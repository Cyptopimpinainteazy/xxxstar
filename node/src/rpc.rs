//! X3 Chain node RPC module wiring.
//!
//! Assembles the full JSON-RPC module used by the service.
//! Merges substrate system RPCs, transaction-payment RPCs, chain RPCs,
//! and the Frontier-compatible ETH/SVM RPC provided by `rpc_frontier`.

use codec::{Decode, Encode};
use crate::atomic_service::AtomicGatewayCommand;
use atomic_swap_orchestrator::{
    AtomicExecutionRequest, AtomicLegExecution, AtomicPair, KernelBundleLeg,
    KernelDeclaredAccess, KernelVmType,
};
use flash_finality::FlashFinalityGadget;
use jsonrpsee::{types::ErrorObjectOwned, RpcModule};
use pallet_x3_kernel::AtlasKernelRuntimeApi;
use sc_client_api::{BlockBackend, StorageProvider};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_core::storage::StorageKey;
use sp_core::{crypto::AccountId32, Pair, H256};
use sp_runtime::generic::Era;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_runtime::transaction_validity::TransactionSource;
use sp_runtime::traits::BlakeTwo256;
use sp_runtime::traits::Hash;
use frame_support::storage::storage_prefix;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use substrate_frame_rpc_system::AccountNonceApi;
use x3_atomic_trade::{AMMPool, SwapRPCServer};
use pallet_x3_atomic_kernel::X3AtomicKernelApi;
use pallet_x3_atomic_kernel::BundleRollbackReason;
use x3_chain_runtime::{
    opaque::Block, AccountId, Address, AssetId, Balance, Runtime, RuntimeCall, Signature,
    SignedExtra, SignedPayload, UncheckedExtrinsic, VERSION,
};
use x3_common::{
    signing::{Ed25519Signer, KeyType, Secp256k1Signer, Signer, Sr25519Signer},
    weight_metering::{WeightConfig, WeightMeter},
};
use x3_cross_vm_bridge::CrossVmOperation;
use x3_rpc::{
    RPCTransaction, SwapRequest, WalletDexApi, WalletDexRpc, WalletServiceApi, WalletServiceRpc,
};
// Simulation-only gas estimator kept for off-chain developer tooling; canonical nodes
// use the Frontier stub (see rpc_frontier).
#[allow(deprecated)]
use x3_rpc::GasEstimationRPC;

use crate::rpc_middleware::RateLimiter;
use crate::service::FullClient;

type RpcError = Box<dyn std::error::Error + Send + Sync>;
type JsonRpseeError = ErrorObjectOwned;

/// Helper to create custom JSON-RPC errors.
fn custom_error(message: impl Into<String>) -> JsonRpseeError {
    ErrorObjectOwned::owned(-32603, message.into(), None::<()>)
}

fn run_on_rpc_thread<T, F>(f: F) -> Result<T, JsonRpseeError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    std::thread::spawn(f)
        .join()
        .map_err(|_| custom_error("RPC worker helper thread panicked"))?
        .map_err(custom_error)
}

/// Decode hex string with "0x" prefix to 32-byte array.
fn decode_hex_32(value: &str, label: &str) -> Result<[u8; 32], JsonRpseeError> {
    let stripped = value.strip_prefix("0x").unwrap_or(value);
    let bytes =
        hex::decode(stripped).map_err(|e| custom_error(format!("{label} decode failed: {e}")))?;
    if bytes.len() != 32 {
        return Err(custom_error(format!(
            "{label} must be 32 bytes, got {}",
            bytes.len()
        )));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

/// Decode hex string with "0x" prefix into raw bytes.
fn decode_hex_bytes(value: &str, label: &str) -> Result<Vec<u8>, JsonRpseeError> {
    let stripped = value.strip_prefix("0x").unwrap_or(value);
    hex::decode(stripped).map_err(|e| custom_error(format!("{label} decode failed: {e}")))
}

/// Decode hex string with "0x" prefix to 20-byte array.
fn decode_hex_20(value: &str, label: &str) -> Result<[u8; 20], JsonRpseeError> {
    let stripped = value.strip_prefix("0x").unwrap_or(value);
    let bytes =
        hex::decode(stripped).map_err(|e| custom_error(format!("{label} decode failed: {e}")))?;
    if bytes.len() != 20 {
        return Err(custom_error(format!(
            "{label} must be 20 bytes, got {}",
            bytes.len()
        )));
    }
    let mut array = [0u8; 20];
    array.copy_from_slice(&bytes);
    Ok(array)
}

/// Parse u128 value from JSON.
fn parse_u128_value(
    value: Option<&serde_json::Value>,
    label: &str,
) -> Result<u128, JsonRpseeError> {
    let val = value.ok_or_else(|| custom_error(format!("Missing {label}")))?;
    if let Some(s) = val.as_str() {
        s.parse::<u128>()
            .map_err(|e| custom_error(format!("{label} parse failed: {e}")))
    } else if let Some(n) = val.as_u64() {
        Ok(n as u128)
    } else {
        Err(custom_error(format!("{label} must be string or number")))
    }
}

#[derive(Debug)]
struct DepositRelayPayload {
    chain_id: u64,
    token_address: [u8; 20],
    depositor: [u8; 20],
    recipient: Vec<u8>,
    amount: u128,
    nonce: u128,
}

const SIGNED_DEPOSIT_RELAY_MAGIC: &[u8] = b"X3DP1";

#[derive(Debug)]
struct SignedDepositRelayEnvelope {
    lock_proof: Vec<u8>,
    deposit_payload: Vec<u8>,
}

fn take<'a>(
    bytes: &'a [u8],
    offset: &mut usize,
    len: usize,
    label: &str,
) -> Result<&'a [u8], JsonRpseeError> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| custom_error(format!("{label} offset overflow")))?;
    let slice = bytes
        .get(*offset..end)
        .ok_or_else(|| custom_error(format!("Proof payload missing {label}")))?;
    *offset = end;
    Ok(slice)
}

fn decode_scale_compact_len(bytes: &[u8], offset: &mut usize) -> Result<usize, JsonRpseeError> {
    let first = *take(bytes, offset, 1, "recipient length")?
        .first()
        .ok_or_else(|| custom_error("Proof payload missing recipient length"))?;
    match first & 0b11 {
        0 => Ok((first >> 2) as usize),
        1 => {
            let second = *take(bytes, offset, 1, "recipient length second byte")?
                .first()
                .ok_or_else(|| {
                    custom_error("Proof payload missing recipient length second byte")
                })?;
            Ok((u16::from_le_bytes([first, second]) >> 2) as usize)
        }
        2 => {
            let rest = take(bytes, offset, 3, "recipient length remaining bytes")?;
            Ok((u32::from_le_bytes([first, rest[0], rest[1], rest[2]]) >> 2) as usize)
        }
        _ => Err(custom_error(
            "Proof payload recipient length uses unsupported SCALE big-integer compact mode",
        )),
    }
}

fn decode_signed_deposit_relay_envelope(
    bytes: &[u8],
) -> Result<SignedDepositRelayEnvelope, JsonRpseeError> {
    if !bytes.starts_with(SIGNED_DEPOSIT_RELAY_MAGIC) {
        return Err(custom_error(
            "Signed deposit relay envelope required; raw deposit payloads are not accepted",
        ));
    }

    let mut offset = SIGNED_DEPOSIT_RELAY_MAGIC.len();
    let proof_len = u32::from_le_bytes(
        take(bytes, &mut offset, 4, "signed envelope proof length")?
            .try_into()
            .map_err(|_| custom_error("Invalid signed envelope proof length bytes"))?,
    ) as usize;
    if proof_len == 0 {
        return Err(custom_error("Signed envelope lock proof must not be empty"));
    }

    let lock_proof = take(bytes, &mut offset, proof_len, "signed envelope lock proof")?.to_vec();
    let deposit_payload = bytes
        .get(offset..)
        .ok_or_else(|| custom_error("Signed envelope missing deposit payload"))?
        .to_vec();
    if deposit_payload.is_empty() {
        return Err(custom_error(
            "Signed envelope deposit payload must not be empty",
        ));
    }

    Ok(SignedDepositRelayEnvelope {
        lock_proof,
        deposit_payload,
    })
}

fn verify_lock_proof_binding(
    operation: &CrossVmOperation,
    lock_proof: &[u8],
) -> Result<(), JsonRpseeError> {
    if lock_proof.len() < 33 {
        return Err(custom_error("Signed envelope lock proof is too short"));
    }
    let expected_operation_hash = sp_core::hashing::blake2_256(&operation.encode());
    if lock_proof[0..32] != expected_operation_hash {
        return Err(custom_error(
            "Signed envelope lock proof is not bound to the decoded bridge operation",
        ));
    }
    if lock_proof[32] == 0 {
        return Err(custom_error(
            "Signed envelope lock proof has no validator signatures",
        ));
    }
    Ok(())
}

fn wrapped_asset_id(chain_id: u32, token_address: &[u8; 20]) -> [u8; 32] {
    let mut preimage = Vec::with_capacity(4 + token_address.len());
    preimage.extend_from_slice(&chain_id.to_le_bytes());
    preimage.extend_from_slice(token_address);
    sp_core::hashing::blake2_256(&preimage)
}

fn decode_deposit_relay_payload(bytes: &[u8]) -> Result<DepositRelayPayload, JsonRpseeError> {
    let mut offset = 0usize;
    let chain_id = u64::from_le_bytes(
        take(bytes, &mut offset, 8, "chain id")?
            .try_into()
            .map_err(|_| custom_error("Invalid chain id bytes"))?,
    );
    let _message_id = take(bytes, &mut offset, 32, "message id")?;
    let token_address_bytes = take(bytes, &mut offset, 20, "token address")?;
    let mut token_address = [0u8; 20];
    token_address.copy_from_slice(token_address_bytes);

    let depositor_bytes = take(bytes, &mut offset, 20, "depositor")?;
    let mut depositor = [0u8; 20];
    depositor.copy_from_slice(depositor_bytes);

    let recipient_len = decode_scale_compact_len(bytes, &mut offset)?;
    let recipient = take(bytes, &mut offset, recipient_len, "x3 recipient")?.to_vec();
    if recipient.len() != 32 {
        return Err(custom_error(format!(
            "x3 recipient must be 32 bytes, got {}",
            recipient.len()
        )));
    }

    let amount = u128::from_le_bytes(
        take(bytes, &mut offset, 16, "amount")?
            .try_into()
            .map_err(|_| custom_error("Invalid amount bytes"))?,
    );
    if amount == 0 {
        return Err(custom_error("Deposit amount must be non-zero"));
    }

    let nonce = u128::from_le_bytes(
        take(bytes, &mut offset, 16, "nonce")?
            .try_into()
            .map_err(|_| custom_error("Invalid nonce bytes"))?,
    );

    let _gateway_address = take(bytes, &mut offset, 20, "gateway address")?;
    let _gateway_block_number = take(bytes, &mut offset, 8, "gateway block number")?;
    if offset != bytes.len() {
        return Err(custom_error(format!(
            "Proof payload has {} trailing bytes",
            bytes.len() - offset
        )));
    }

    Ok(DepositRelayPayload {
        chain_id,
        token_address,
        depositor,
        recipient,
        amount,
        nonce,
    })
}

fn account_from_public(public: sp_core::sr25519::Public) -> AccountId {
    <Signature as Verify>::Signer::from(public).into_account()
}

fn load_sr25519_pair_from_env(name: &str) -> Result<sp_core::sr25519::Pair, JsonRpseeError> {
    let seed = std::env::var(name).map_err(|_| {
        custom_error(format!(
            "{name} env var not set — required for the two-member council flow"
        ))
    })?;
    sp_core::sr25519::Pair::from_string(&seed, None)
        .map_err(|e| custom_error(format!("load {name} key failed: {e:?}")))
}

fn read_u32_storage(
    client: &FullClient,
    at: H256,
    pallet: &[u8],
    item: &[u8],
) -> Result<u32, JsonRpseeError> {
    use codec::Decode;
    let key = StorageKey(storage_prefix(pallet, item).to_vec());
    let Some(data) = StorageProvider::storage(client, at, &key)
        .map_err(|e| custom_error(format!("read {pallet:?}/{item:?} storage failed: {e}")))?
    else {
        // StorageValue counters are uninitialized until the first write.
        return Ok(0);
    };
    u32::decode(&mut &data.0[..])
        .map_err(|e| custom_error(format!("decode {pallet:?}/{item:?} storage failed: {e}")))
}



fn read_settlement_intent_state(
    client: &FullClient,
    at: H256,
    intent_id: H256,
) -> Result<Option<pallet_x3_settlement_engine::IntentState>, JsonRpseeError> {
    let intent_key = StorageKey(
        pallet_x3_settlement_engine::SettlementIntents::<Runtime>::hashed_key_for(intent_id),
    );
    let intent_exists = StorageProvider::storage(client, at, &intent_key)
        .map_err(|e| custom_error(format!("read settlement intent failed: {e}")))?
        .is_some();
    if !intent_exists {
        return Ok(None);
    }

    let state_key = StorageKey(
        pallet_x3_settlement_engine::IntentStates::<Runtime>::hashed_key_for(intent_id),
    );
    let state = match StorageProvider::storage(client, at, &state_key)
        .map_err(|e| custom_error(format!("read settlement intent state failed: {e}")))?
    {
        Some(data) => pallet_x3_settlement_engine::IntentState::decode(&mut &data.0[..])
            .map_err(|e| custom_error(format!("decode settlement intent state failed: {e}")))?,
        None => pallet_x3_settlement_engine::IntentState::default(),
    };
    Ok(Some(state))
}

fn sign_runtime_call(
    pair: &sp_core::sr25519::Pair,
    account: &AccountId,
    genesis_hash: H256,
    nonce: u32,
    call: RuntimeCall,
) -> Result<UncheckedExtrinsic, JsonRpseeError> {
    use codec::Encode;
    let extra: SignedExtra = (
        frame_system::CheckNonZeroSender::<Runtime>::new(),
        frame_system::CheckSpecVersion::<Runtime>::new(),
        frame_system::CheckTxVersion::<Runtime>::new(),
        frame_system::CheckGenesis::<Runtime>::new(),
        frame_system::CheckEra::<Runtime>::from(Era::Immortal),
        frame_system::CheckNonce::<Runtime>::from(nonce),
        frame_system::CheckWeight::<Runtime>::new(),
        pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
        pallet_x3_invariants::InvariantCheck::<Runtime>::new(),
        decode_agent_law_check()?,
    );
    let payload = SignedPayload::from_raw(
        call.clone(),
        extra.clone(),
        (
            (),
            VERSION.spec_version,
            VERSION.transaction_version,
            genesis_hash,
            genesis_hash,
            (),
            (),
            (),
            (),
            (),
        ),
    );
    let signature = payload.using_encoded(|payload| Signature::from(pair.sign(payload)));
    Ok(UncheckedExtrinsic::new_signed(
        call,
        Address::Id(account.clone()),
        signature,
        extra,
    ))
}

fn submit_to_pool<P>(pool: &P, best_hash: H256, extrinsic: UncheckedExtrinsic) -> Result<H256, JsonRpseeError>
where
    P: TransactionPool<Block = Block, Hash = H256> + Sync,
{
    futures::executor::block_on(pool.submit_one(
        best_hash,
        TransactionSource::External,
        extrinsic.into(),
    ))
    .map_err(|e| custom_error(format!("Runtime extrinsic submission failed: {e}")))
}

fn wait_for_best_block_advance(
    client: &FullClient,
    baseline: u32,
    attempts: usize,
    label: &str,
) -> Result<(), JsonRpseeError> {
    for _ in 0..attempts {
        let current = client.info().best_number;
        if current > baseline {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(750));
    }
    Err(custom_error(format!(
        "{label}: best block did not advance from #{baseline} within timeout"
    )))
}

fn decode_agent_law_check() -> Result<pallet_x3_agent_law::AgentLawCheck<Runtime>, JsonRpseeError> {
    codec::Decode::decode(&mut &[][..])
        .map_err(|e| custom_error(format!("decode agent law extension failed: {e}")))
}

fn parse_overlay_legs(value: &serde_json::Value) -> Result<Vec<KernelBundleLeg>, JsonRpseeError> {
    let legs = value
        .get("legs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| custom_error("Missing legs array"))?;
    legs.iter()
        .map(|leg| {
            let vm_type = match leg
                .get("vm_type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
            {
                "evm" => KernelVmType::Evm,
                "svm" => KernelVmType::Svm,
                "x3" => KernelVmType::X3,
                "cross" => KernelVmType::Cross,
                other => {
                    return Err(custom_error(format!("Invalid vm_type: {other}")));
                }
            };
            let token_in = decode_hex_32(
                leg.get("token_in")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| custom_error("Missing token_in"))?,
                "token_in",
            )?;
            let token_out = decode_hex_32(
                leg.get("token_out")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| custom_error("Missing token_out"))?,
                "token_out",
            )?;
            let amount_in = parse_u128_value(leg.get("amount_in"), "amount_in")?;
            let min_amount_out =
                parse_u128_value(leg.get("min_amount_out"), "min_amount_out")?;
            let deadline = parse_u128_value(leg.get("deadline"), "deadline")? as u64;
            let reads = leg
                .get("access")
                .and_then(|a| a.get("reads"))
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|h| {
                            let s = h
                                .as_str()
                                .ok_or_else(|| custom_error("access read must be a string"))?;
                            decode_hex_32(s, "access read").map(H256)
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?
                .unwrap_or_default();
            let writes = leg
                .get("access")
                .and_then(|a| a.get("writes"))
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|h| {
                            let s = h
                                .as_str()
                                .ok_or_else(|| custom_error("access write must be a string"))?;
                            decode_hex_32(s, "access write").map(H256)
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?
                .unwrap_or_default();
            let access = KernelDeclaredAccess {
                reads,
                writes,
            };
            Ok(KernelBundleLeg {
                vm_type,
                token_in: H256(token_in),
                token_out: H256(token_out),
                amount_in,
                min_amount_out,
                deadline,
                access,
            })
        })
        .collect()
}

fn parse_executions(value: &serde_json::Value) -> Result<Vec<AtomicLegExecution>, JsonRpseeError> {
    let Some(executions) = value.get("executions").and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };
    executions
        .iter()
        .map(|execution| {
            let vm = execution
                .get("vm")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match vm {
                "evm" => {
                    let caller = decode_hex_20(
                        execution
                            .get("caller")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| custom_error("Missing evm caller"))?,
                        "caller",
                    )?;
                    let target = decode_hex_20(
                        execution
                            .get("target")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| custom_error("Missing evm target"))?,
                        "target",
                    )?;
                    let value = parse_u128_value(execution.get("value"), "value")?;
                    let input = decode_hex_bytes(
                        execution
                            .get("input")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                        "input",
                    )?;
                    Ok(AtomicLegExecution::Evm {
                        caller,
                        target,
                        value,
                        input,
                    })
                }
                "svm" => {
                    let caller = decode_hex_32(
                        execution
                            .get("caller")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| custom_error("Missing svm caller"))?,
                        "caller",
                    )?;
                    let program_id = decode_hex_32(
                        execution
                            .get("program_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| custom_error("Missing svm program_id"))?,
                        "program_id",
                    )?;
                    let instruction = decode_hex_bytes(
                        execution
                            .get("instruction")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                        "instruction",
                    )?;
                    Ok(AtomicLegExecution::Svm {
                        caller,
                        program_id,
                        instruction,
                    })
                }
                "x3" => {
                    let caller = decode_hex_32(
                        execution
                            .get("caller")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| custom_error("Missing x3 caller"))?,
                        "caller",
                    )?;
                    let selector_raw = decode_hex_bytes(
                        execution
                            .get("selector")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| custom_error("Missing x3 selector"))?,
                        "selector",
                    )?;
                    let selector: [u8; 4] = selector_raw
                        .try_into()
                        .map_err(|_| custom_error("X3 selector must be 4 bytes"))?;
                    let payload = decode_hex_bytes(
                        execution
                            .get("payload")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                        "payload",
                    )?;
                    Ok(AtomicLegExecution::X3 {
                        caller,
                        selector,
                        payload,
                    })
                }
                "transfer" => {
                    let vm = match execution
                        .get("vm")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                    {
                        "evm" => KernelVmType::Evm,
                        "svm" => KernelVmType::Svm,
                        "x3" => KernelVmType::X3,
                        other => {
                            return Err(custom_error(format!("Invalid transfer vm: {other}")));
                        }
                    };
                    let from = decode_hex_bytes(
                        execution
                            .get("from")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| custom_error("Missing transfer from"))?,
                        "from",
                    )?;
                    let to = decode_hex_bytes(
                        execution
                            .get("to")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| custom_error("Missing transfer to"))?,
                        "to",
                    )?;
                    let amount = parse_u128_value(execution.get("amount"), "amount")?;
                    Ok(AtomicLegExecution::Transfer {
                        vm,
                        from,
                        to,
                        amount,
                    })
                }
                other => Err(custom_error(format!("Invalid execution vm: {other}"))),
            }
        })
        .collect()
}

/// Full RPC extension creation.
///
/// Called by the service to build the RPC module for each connection.
pub fn create_full<P>(
    client: Arc<FullClient>,
    pool: Arc<P>,
    gadget: Option<Arc<FlashFinalityGadget>>,
    limiter: Arc<RateLimiter>,
    _subscription_executor: sc_rpc::SubscriptionTaskExecutor,
    enable_demo_wallet_rpc: bool,
    enable_bridge_ingress_rpc: bool,
    atomic_gateway_tx: Option<mpsc::Sender<AtomicGatewayCommand>>,
) -> Result<RpcModule<()>, RpcError>
where
    P: TransactionPool<Block = Block, Hash = H256> + Sync + Send + 'static,
    FullClient: ProvideRuntimeApi<Block>,
    FullClient: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
    FullClient: BlockBackend<Block>,
    <FullClient as ProvideRuntimeApi<Block>>::Api: BlockBuilder<Block>,
    <FullClient as ProvideRuntimeApi<Block>>::Api:
        substrate_frame_rpc_system::AccountNonceApi<Block, x3_chain_runtime::AccountId, u32>,
    <FullClient as ProvideRuntimeApi<Block>>::Api:
        pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<
            Block,
            x3_chain_runtime::Balance,
        >,
    <FullClient as ProvideRuntimeApi<Block>>::Api:
        pallet_x3_kernel::AtlasKernelRuntimeApi<Block, AccountId, Balance, AssetId>,
    <FullClient as ProvideRuntimeApi<Block>>::Api:
        pallet_x3_atomic_kernel::X3AtomicKernelApi<Block>,
{
    let mut module = RpcModule::new(());

    let client_for_settlement_state = client.clone();
    module.register_method(
        "x3_settlementState",
        move |params, _, _| -> Result<serde_json::Value, ErrorObjectOwned> {
            let (intent_hex,): (String,) = params.parse()?;
            let intent_id = H256(decode_hex_32(&intent_hex, "runtime intent id")?);
            let at = client_for_settlement_state.info().best_hash;
            match read_settlement_intent_state(
                client_for_settlement_state.as_ref(),
                at,
                intent_id,
            )? {
                Some(state) => Ok(serde_json::json!({
                    "intent_id": intent_hex,
                    "state": format!("{state:?}"),
                    "at": format!("0x{}", hex::encode(at)),
                })),
                None => Ok(serde_json::json!({
                    "intent_id": intent_hex,
                    "state": "Unknown",
                    "at": format!("0x{}", hex::encode(at)),
                })),
            }
        },
    )?;


    if let Some(atomic_gateway_tx) = atomic_gateway_tx {
        let rollback_tx = atomic_gateway_tx.clone();
        module.register_method(
            "atomic_submitAtomicBundle",
            move |params, _, _| -> Result<serde_json::Value, ErrorObjectOwned> {
            let req: serde_json::Value = params.parse::<(serde_json::Value,)>().map(|(v,)| v)?;
            let legs = parse_overlay_legs(&req)?;
            let deadline_blocks = req
                .get("deadline_blocks")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| custom_error("Missing deadline_blocks"))? as u32;
            let chain_id = req
                .get("chain_id")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| custom_error("Missing chain_id"))? as u32;
            let nonce = req
                .get("nonce")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| custom_error("Missing nonce"))?;
            let svm_tx = decode_hex_bytes(
                req.get("svm_tx")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| custom_error("Missing svm_tx"))?,
                "svm_tx",
            )?;
            let evm_tx = decode_hex_bytes(
                req.get("evm_tx")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| custom_error("Missing evm_tx"))?,
                "evm_tx",
            )?;
            let sequence_nonce = req
                .get("sequence_nonce")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let request = AtomicExecutionRequest {
                pair: AtomicPair {
                    swap_id: Vec::new(),
                    svm_tx,
                    evm_tx,
                    sequence_nonce,
                    pallet_bundle_id: None,
                },
                legs,
                deadline_blocks,
                chain_id,
                nonce,
                executions: parse_executions(&req)?,
            };
            let hold_for_rollback = req
                .get("hold_for_rollback")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let command = if hold_for_rollback {
                AtomicGatewayCommand::SubmitBundleHoldForRollback(request)
            } else {
                AtomicGatewayCommand::SubmitBundle(request)
            };
            atomic_gateway_tx
                .try_send(command)
                .map_err(|e| custom_error(format!("atomic gateway queue full: {e}")))?;
            Ok(serde_json::json!({ "status": "accepted" }))
            },
        )?;
        module.register_method(
            "atomic_rollbackBundle",
            move |params, _, _| -> Result<serde_json::Value, ErrorObjectOwned> {
                let req: serde_json::Value = params.parse::<(serde_json::Value,)>().map(|(v,)| v)?;
                let bundle_id = H256(decode_hex_32(
                    req.get("bundle_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing bundle_id"))?,
                    "bundle_id",
                )?);
                let reason = match req
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("submitter_cancelled")
                {
                    "execution_failed" => BundleRollbackReason::ExecutionFailed,
                    "access_set_violation" => BundleRollbackReason::AccessSetViolation,
                    "deadline_exceeded" => BundleRollbackReason::DeadlineExceeded,
                    _ => BundleRollbackReason::SubmitterCancelled,
                };
                rollback_tx
                    .try_send(AtomicGatewayCommand::Rollback { bundle_id, reason })
                    .map_err(|e| custom_error(format!("atomic gateway queue full: {e}")))?;
                Ok(serde_json::json!({ "status": "accepted" }))
            },
        )?;
    }

    let client_for_find = client.clone();
    module.register_method(
        "atomic_findBundle",
        move |params, _, _| -> Result<serde_json::Value, ErrorObjectOwned> {
            let req: serde_json::Value = params.parse::<(serde_json::Value,)>().map(|(v,)| v)?;
            let submitter_hex = req
                .get("submitter")
                .and_then(|v| v.as_str())
                .ok_or_else(|| custom_error("Missing submitter"))?;
            let submitter = AccountId32::new(decode_hex_32(submitter_hex, "submitter")?);
            let legs_hash = H256(decode_hex_32(
                req.get("legs_hash")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| custom_error("Missing legs_hash"))?,
                "legs_hash",
            )?);
            let at = client_for_find.info().best_hash;
            match client_for_find.runtime_api().find_bundle(at, submitter, legs_hash) {
                Ok(Some((bundle_id, status))) => Ok(serde_json::json!({
                    "bundle_id": format!("0x{}", hex::encode(bundle_id)),
                    "status": format!("{status:?}"),
                })),
                Ok(None) => Ok(serde_json::json!({
                    "bundle_id": null,
                    "status": null,
                })),
                Err(e) => Err(custom_error(format!("runtime find_bundle failed: {e}"))),
            }
        },
    )?;

    let client_for_status = client.clone();
    module.register_method(
        "atomic_getBundleStatus",
        move |params, _, _| -> Result<serde_json::Value, ErrorObjectOwned> {
            let req: serde_json::Value = params.parse::<(serde_json::Value,)>().map(|(v,)| v)?;
            let bundle_id = H256(decode_hex_32(
                req.get("bundle_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| custom_error("Missing bundle_id"))?,
                "bundle_id",
            )?);
            let at = client_for_status.info().best_hash;
            let status = client_for_status
                .runtime_api()
                .get_bundle_status(at, bundle_id)
                .map_err(|e| custom_error(format!("get_bundle_status failed: {e}")))?;
            Ok(serde_json::json!({
                "bundle_id": format!("0x{}", hex::encode(bundle_id)),
                "status": status.map(|s| format!("{s:?}")),
            }))
        },
    )?;

    let client_for_proof = client.clone();
    module.register_method(
        "atomic_getPoAEProof",
        move |params, _, _| -> Result<serde_json::Value, ErrorObjectOwned> {
            let req: serde_json::Value = params.parse::<(serde_json::Value,)>().map(|(v,)| v)?;
            let bundle_id = H256(decode_hex_32(
                req.get("bundle_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| custom_error("Missing bundle_id"))?,
                "bundle_id",
            )?);
            let at = client_for_proof.info().best_hash;
            let proof = client_for_proof
                .runtime_api()
                .get_poae_proof(at, bundle_id)
                .map_err(|e| custom_error(format!("get_poae_proof failed: {e}")))?;
            Ok(match proof {
                Some(proof) => serde_json::json!({
                    "bundle_id": format!("0x{}", hex::encode(proof.bundle_id)),
                    "receipt_root": format!("0x{}", hex::encode(proof.receipt_root)),
                    "finalized_block": proof.finalized_block,
                    "finality_cert": format!("0x{}", hex::encode(proof.finality_cert)),
                    "legs_hash": format!("0x{}", hex::encode(proof.legs_hash)),
                    "leg_count": proof.leg_count,
                }),
                None => serde_json::json!(null),
            })
        },
    )?;

    let tx_pool = pool.clone();
    let system_rpc = substrate_frame_rpc_system::System::new(client.clone(), pool);
    module.merge(substrate_frame_rpc_system::SystemApiServer::into_rpc(
        system_rpc,
    ))?;

    let tx_payment_rpc = pallet_transaction_payment_rpc::TransactionPayment::new(client.clone());
    module.merge(
        pallet_transaction_payment_rpc::TransactionPaymentApiServer::into_rpc(tx_payment_rpc),
    )?;

    // Merge Frontier ETH-compatible JSON-RPC endpoints.
    let frontier_module = crate::rpc_frontier::create_frontier_rpc(client.clone())?;
    module.merge(frontier_module)?;

    // Merge SVM-compatible JSON-RPC endpoints.
    let svm_module = crate::rpc_frontier::create_svm_rpc(client.clone())?;
    module.merge(svm_module)?;

    // Initialize DEX RPC integration.
    let wallet_dex = Arc::new(WalletDexRpc::<Block, FullClient>::new(client.clone()));
    let swap_rpc = Arc::new(Mutex::new(SwapRPCServer::new()));

    // Register default AMM pool (X3/USDC).
    {
        let mut engine = swap_rpc
            .lock()
            .map_err(|_| custom_error("Swap engine lock poisoned"))?;

        let _ = engine.register_pool(AMMPool {
            id: "default_x3_usdc".to_string(),
            token_a: "X3".to_string(),
            token_b: "USDC".to_string(),
            reserve_a: 10_000_000_000_000,
            reserve_b: 10_000_000_000_000,
            fee_bps: 30,
            tvl_usd: 20_000_000.0,
        });
    }

    // Register walletDex_estimateSwap RPC method.
    let wallet_dex_estimate = wallet_dex.clone();
    module.register_method(
        "walletDex_estimateSwap",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let req: serde_json::Value = params.parse::<(serde_json::Value,)>().map(|(v,)| v)?;
            let request = SwapRequest {
                token_in: decode_hex_32(
                    req.get("token_in")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing token_in"))?,
                    "token_in",
                )?,
                token_out: decode_hex_32(
                    req.get("token_out")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing token_out"))?,
                    "token_out",
                )?,
                amount_in: parse_u128_value(req.get("amount_in"), "amount_in")?,
                min_amount_out: parse_u128_value(req.get("min_amount_out"), "min_amount_out")?,
                wallet_id: decode_hex_32(
                    req.get("wallet_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing wallet_id"))?,
                    "wallet_id",
                )?,
                require_approval: req
                    .get("require_approval")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                approval_threshold: parse_u128_value(
                    req.get("approval_threshold"),
                    "approval_threshold",
                )?,
            };

            wallet_dex_estimate
                .estimate_swap(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("walletDex_estimateSwap failed: {e}")))
        },
    )?;

    // Register walletDex_executeSwap RPC method.
    let wallet_dex_execute = wallet_dex.clone();
    let dex_exec_limiter = limiter.clone();
    module.register_method(
        "walletDex_executeSwap",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            dex_exec_limiter
                .check_request(0, "walletDex_executeSwap")
                .map_err(|e| custom_error(e.to_string()))?;
            let req: serde_json::Value = params.parse::<(serde_json::Value,)>().map(|(v,)| v)?;
            let request = SwapRequest {
                token_in: decode_hex_32(
                    req.get("token_in")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing token_in"))?,
                    "token_in",
                )?,
                token_out: decode_hex_32(
                    req.get("token_out")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing token_out"))?,
                    "token_out",
                )?,
                amount_in: parse_u128_value(req.get("amount_in"), "amount_in")?,
                min_amount_out: parse_u128_value(req.get("min_amount_out"), "min_amount_out")?,
                wallet_id: decode_hex_32(
                    req.get("wallet_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing wallet_id"))?,
                    "wallet_id",
                )?,
                require_approval: req
                    .get("require_approval")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                approval_threshold: parse_u128_value(
                    req.get("approval_threshold"),
                    "approval_threshold",
                )?,
            };

            wallet_dex_execute
                .execute_swap(request, vec![])
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("walletDex_executeSwap failed: {e}")))
        },
    )?;

    // Demo wallet RPC (wallet_*): every method in this service is a fabricated,
    // non-cryptographic placeholder (fake balances, a substring "signature",
    // a hardcoded default mnemonic when none is supplied — see CRITICAL-TX-1
    // in audit-artifacts/mainnet-readiness/2026-09-06-fbd4613b-claude/).
    // It is retained only as a disabled-by-default demo surface for local UI
    // development against `--dev`/explicitly-opted-in chains, per this repo's
    // AGENTS.md rule against reachable fake stubs in production paths. It MUST
    // NOT be reachable on any non-dev chain spec.
    if enable_demo_wallet_rpc {
    // Initialize Wallet Service RPC
    let wallet_service = Arc::new(WalletServiceRpc::<Block, FullClient>::new(client.clone()));

    // Register wallet service RPC methods
    module.register_method("wallet_createWallet", {
        let wallet_service = wallet_service.clone();
        let create_wallet_limiter = limiter.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            create_wallet_limiter
                .check_request(0, "wallet_createWallet")
                .map_err(|e| custom_error(e.to_string()))?;
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::CreateWalletRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .create_wallet(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_createWallet failed: {e}")))
        }
    })?;

    module.register_method("wallet_importWallet", {
        let wallet_service = wallet_service.clone();
        let import_wallet_limiter = limiter.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            import_wallet_limiter
                .check_request(0, "wallet_importWallet")
                .map_err(|e| custom_error(e.to_string()))?;
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::ImportWalletRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .import_wallet(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_importWallet failed: {e}")))
        }
    })?;

    module.register_method("wallet_backupWallet", {
        let wallet_service = wallet_service.clone();
        let backup_wallet_limiter = limiter.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            backup_wallet_limiter
                .check_request(0, "wallet_backupWallet")
                .map_err(|e| custom_error(e.to_string()))?;
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::BackupWalletRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .backup_wallet(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_backupWallet failed: {e}")))
        }
    })?;

    module.register_method("wallet_getBalance", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::GetBalanceRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .get_balance(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_getBalance failed: {e}")))
        }
    })?;

    module.register_method("wallet_signTransaction", {
        let wallet_service = wallet_service.clone();
        let sign_tx_limiter = limiter.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            sign_tx_limiter
                .check_request(0, "wallet_signTransaction")
                .map_err(|e| custom_error(e.to_string()))?;
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::SignTransactionRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .sign_transaction(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_signTransaction failed: {e}")))
        }
    })?;

    module.register_method("wallet_submitTransaction", {
        let wallet_service = wallet_service.clone();
        let submit_tx_limiter = limiter.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            submit_tx_limiter
                .check_request(0, "wallet_submitTransaction")
                .map_err(|e| custom_error(e.to_string()))?;
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::SubmitTransactionRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .submit_transaction(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_submitTransaction failed: {e}")))
        }
    })?;

    module.register_method("wallet_getTransactions", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::GetTransactionsRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .get_transactions(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_getTransactions failed: {e}")))
        }
    })?;

    module.register_method("wallet_getWalletStatus", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::GetWalletStatusRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .get_wallet_status(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_getWalletStatus failed: {e}")))
        }
    })?;

    module.register_method("wallet_listWallets", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::ListWalletsRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .list_wallets(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_listWallets failed: {e}")))
        }
    })?;

    module.register_method("wallet_setNetwork", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::SetNetworkRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .set_network(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_setNetwork failed: {e}")))
        }
    })?;

    module.register_method("wallet_getNetworks", {
        let wallet_service = wallet_service.clone();
        move |_, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            wallet_service
                .get_networks()
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_getNetworks failed: {e}")))
        }
    })?;
    } // enable_demo_wallet_rpc

    // Register signing RPC methods
    module.register_method(
        "x3_sign_ed25519",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (message_hex, secret_hex): (String, String) = params.parse()?;
            let message = hex::decode(message_hex.strip_prefix("0x").unwrap_or(&message_hex))
                .map_err(|e| custom_error(format!("Invalid message hex: {e}")))?;
            let secret = hex::decode(secret_hex.strip_prefix("0x").unwrap_or(&secret_hex))
                .map_err(|e| custom_error(format!("Invalid secret hex: {e}")))?;

            if secret.len() != 32 {
                return Err(custom_error("Secret key must be 32 bytes"));
            }

            let mut secret_array = [0u8; 32];
            secret_array.copy_from_slice(&secret);

            let signer = Ed25519Signer::from_secret_key(&secret_array);
            let signature = signer.sign(&message);

            Ok(serde_json::Value::String(format!(
                "0x{}",
                hex::encode(signature.as_bytes())
            )))
        },
    )?;

    module.register_method(
        "x3_sign_secp256k1",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (message_hex, secret_hex): (String, String) = params.parse()?;
            let message = hex::decode(message_hex.strip_prefix("0x").unwrap_or(&message_hex))
                .map_err(|e| custom_error(format!("Invalid message hex: {e}")))?;
            let secret = hex::decode(secret_hex.strip_prefix("0x").unwrap_or(&secret_hex))
                .map_err(|e| custom_error(format!("Invalid secret hex: {e}")))?;

            if secret.len() != 32 {
                return Err(custom_error("Secret key must be 32 bytes"));
            }

            let mut secret_array = [0u8; 32];
            secret_array.copy_from_slice(&secret);

            let signer = Secp256k1Signer::from_secret_key(&secret_array)
                .map_err(|e| custom_error(format!("Invalid secret key: {e}")))?;
            let signature = signer.sign(&message);

            Ok(serde_json::Value::String(format!(
                "0x{}",
                hex::encode(signature.as_bytes())
            )))
        },
    )?;

    module.register_method(
        "x3_sign_sr25519",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (message_hex, secret_hex): (String, String) = params.parse()?;
            let message = hex::decode(message_hex.strip_prefix("0x").unwrap_or(&message_hex))
                .map_err(|e| custom_error(format!("Invalid message hex: {e}")))?;
            let secret = hex::decode(secret_hex.strip_prefix("0x").unwrap_or(&secret_hex))
                .map_err(|e| custom_error(format!("Invalid secret hex: {e}")))?;

            if secret.len() != 32 {
                return Err(custom_error("Secret key must be 32 bytes"));
            }

            let mut secret_array = [0u8; 32];
            secret_array.copy_from_slice(&secret);

            let signer = Sr25519Signer::from_secret_key(&secret_array);
            let signature = signer.sign(&message);

            Ok(serde_json::Value::String(format!(
                "0x{}",
                hex::encode(signature.as_bytes())
            )))
        },
    )?;

    module.register_method(
        "x3_verify_signature",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (message_hex, signature_hex, public_key_hex, key_type_str): (
                String,
                String,
                String,
                String,
            ) = params.parse()?;
            let message = hex::decode(message_hex.strip_prefix("0x").unwrap_or(&message_hex))
                .map_err(|e| custom_error(format!("Invalid message hex: {e}")))?;
            let signature = hex::decode(signature_hex.strip_prefix("0x").unwrap_or(&signature_hex))
                .map_err(|e| custom_error(format!("Invalid signature hex: {e}")))?;
            let public_key =
                hex::decode(public_key_hex.strip_prefix("0x").unwrap_or(&public_key_hex))
                    .map_err(|e| custom_error(format!("Invalid public key hex: {e}")))?;

            let key_type = match key_type_str.to_lowercase().as_str() {
                "ed25519" => KeyType::Ed25519,
                "secp256k1" => KeyType::Secp256k1,
                "sr25519" => KeyType::Sr25519,
                _ => {
                    return Err(custom_error(
                        "Invalid key type. Must be ed25519, secp256k1, or sr25519",
                    ))
                }
            };

            let valid =
                x3_common::signing::verify_signature(&signature, &message, &public_key, key_type);

            Ok(serde_json::Value::Bool(valid))
        },
    )?;

    module.register_method(
        "x3_weight_meter",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let config: serde_json::Value = params.parse::<(serde_json::Value,)>().map(|(v,)| v)?;

            let max_compute_units = config
                .get("max_compute_units")
                .and_then(|v| v.as_u64())
                .unwrap_or(200_000);
            let max_gas = config
                .get("max_gas")
                .and_then(|v| v.as_u64())
                .unwrap_or(1_000_000);

            let mut meter = WeightMeter::new(WeightConfig {
                max_compute_units,
                max_gas,
                ..Default::default()
            });

            // Consume some compute units for demonstration
            meter
                .consume_compute(1000)
                .map_err(|e| custom_error(format!("Compute limit: {e}")))?;

            Ok(serde_json::json!({
                "remaining_compute": meter.remaining_compute(),
                "remaining_gas": meter.remaining_gas(),
                "consumed_compute": meter.consumed_compute(),
                "consumed_gas": meter.consumed_gas(),
            }))
        },
    )?;

    // Initialize Validator RPC — wired to the Substrate client for live
    // authority set queries instead of returning hardcoded stubs.
    let validator_rpc = x3_rpc::create_validator_rpc(client.clone())?;
    module.merge(validator_rpc)?;

    // ── Gateway RPC ────────────────────────────────────
    let read_storage: x3_rpc::StorageReadFn = {
        let client = client.clone();
        Arc::new(move |key: StorageKey, hash: [u8; 32]| {
            StorageProvider::storage(&*client, sp_core::H256::from(hash), &key)
                .map(|opt| opt.map(|d| d.0))
                .map_err(|e| e.to_string())
        })
    };
    let read_keys: x3_rpc::StorageKeysFn = {
        let client = client.clone();
        Arc::new(move |prefix: StorageKey, hash: [u8; 32]| {
            StorageProvider::storage_keys(&*client, sp_core::H256::from(hash), Some(&prefix), None)
                .map(|iter| iter.into_iter().map(|key| key.0).collect())
                .map_err(|e| e.to_string())
        })
    };
    let best_hash: [u8; 32] = client.info().best_hash.into();
    let gateway_rpc = x3_rpc::create_gateway_rpc(read_storage, read_keys, best_hash)?;
    module.merge(gateway_rpc)?;

    // ── x3_getCanonicalBalance ──────────────────────────
    let ledger_client = client.clone();
    module.register_method(
        "x3_getCanonicalBalance",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (account_hex, asset_id): (String, AssetId) = params
                .parse()
                .map_err(|e| custom_error(format!("Invalid balance parameters: {e}")))?;
            let account_bytes = decode_hex_32(&account_hex, "account")?;
            let account = AccountId::decode(&mut &account_bytes[..])
                .map_err(|e| custom_error(format!("Account decode failed: {e}")))?;
            let block_hash = ledger_client.info().best_hash;
            let balance = ledger_client
                .runtime_api()
                .get_canonical_balance(block_hash, account, asset_id)
                .map_err(|e| custom_error(format!("Runtime balance query failed: {e}")))?;

            Ok(serde_json::json!({
                "account": account_hex,
                "asset_id": asset_id,
                "balance": balance.to_string(),
            }))
        },
    )?;

    let kernel_state_client = client.clone();
    module.register_method(
        "x3_getKernelBridgeState",
        move |_, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let block_hash = kernel_state_client.info().best_hash;
            let api = kernel_state_client.runtime_api();
            let authorities = api
                .get_authorities(block_hash)
                .map_err(|e| custom_error(format!("Runtime authorities query failed: {e}")))?;
            let authorized_accounts = api.get_authorized_accounts(block_hash).map_err(|e| {
                custom_error(format!("Runtime authorized accounts query failed: {e}"))
            })?;

            Ok(serde_json::json!({
                "authorities": authorities
                    .into_iter()
                    .map(|account| format!("0x{}", hex::encode(account.encode())))
                    .collect::<Vec<_>>(),
                "authorized_accounts": authorized_accounts
                    .into_iter()
                    .map(|account| format!("0x{}", hex::encode(account.encode())))
                    .collect::<Vec<_>>(),
            }))
        },
    )?;

    let wrapped_accounting_client = client.clone();
    module.register_method(
        "x3_getWrappedAccounting",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (account_hex, chain_id, wrapped_asset_hex): (String, u32, String) = params
                .parse()
                .map_err(|e| custom_error(format!("Invalid wrapped accounting parameters: {e}")))?;
            let account_bytes = decode_hex_32(&account_hex, "account")?;
            let account = AccountId::decode(&mut &account_bytes[..])
                .map_err(|e| custom_error(format!("Account decode failed: {e}")))?;
            let wrapped_asset_id = decode_hex_32(&wrapped_asset_hex, "wrapped asset id")?;

            let block_hash = wrapped_accounting_client.info().best_hash;
            let api = wrapped_accounting_client.runtime_api();
            let balance = api
                .get_wrapped_balance(block_hash, account, chain_id, wrapped_asset_id)
                .map_err(|e| custom_error(format!("Runtime wrapped balance query failed: {e}")))?;
            let supply = api
                .get_wrapped_supply(block_hash, chain_id, wrapped_asset_id)
                .map_err(|e| custom_error(format!("Runtime wrapped supply query failed: {e}")))?;
            let total_supply = api.get_total_wrapped_supply(block_hash).map_err(|e| {
                custom_error(format!("Runtime total wrapped supply query failed: {e}"))
            })?;

            Ok(serde_json::json!({
                "account": account_hex,
                "chain_id": chain_id,
                "wrapped_asset_id": format!("0x{}", hex::encode(wrapped_asset_id)),
                "balance": balance.to_string(),
                "supply": supply.to_string(),
                "total_supply": total_supply.to_string(),
            }))
        },
    )?;

    let wrapped_token_accounting_client = client.clone();
    module.register_method(
        "x3_getWrappedAccountingForToken",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (account_hex, chain_id, token_address_hex): (String, u32, String) =
                params.parse().map_err(|e| {
                    custom_error(format!("Invalid wrapped token accounting parameters: {e}"))
                })?;
            let account_bytes = decode_hex_32(&account_hex, "account")?;
            let account = AccountId::decode(&mut &account_bytes[..])
                .map_err(|e| custom_error(format!("Account decode failed: {e}")))?;
            let token_address = decode_hex_20(&token_address_hex, "token address")?;
            let asset_id = wrapped_asset_id(chain_id, &token_address);

            let block_hash = wrapped_token_accounting_client.info().best_hash;
            let api = wrapped_token_accounting_client.runtime_api();
            let balance = api
                .get_wrapped_balance(block_hash, account, chain_id, asset_id)
                .map_err(|e| custom_error(format!("Runtime wrapped balance query failed: {e}")))?;
            let supply = api
                .get_wrapped_supply(block_hash, chain_id, asset_id)
                .map_err(|e| custom_error(format!("Runtime wrapped supply query failed: {e}")))?;
            let total_supply = api.get_total_wrapped_supply(block_hash).map_err(|e| {
                custom_error(format!("Runtime total wrapped supply query failed: {e}"))
            })?;

            Ok(serde_json::json!({
                "account": account_hex,
                "chain_id": chain_id,
                "token_address": token_address_hex,
                "wrapped_asset_id": format!("0x{}", hex::encode(asset_id)),
                "balance": balance.to_string(),
                "supply": supply.to_string(),
                "total_supply": total_supply.to_string(),
            }))
        },
    )?;

    // Shared pool handle for the two-member wrapped council flow. Captured
    // before tx_pool is moved into the bridge-ingress closure below.
    let council_flow_pool = tx_pool.clone();

    // ── x3_submitCrossVmTransaction ─────────────────────
    // Development/local bridge ingress for relayer-submitted deposit payloads.
    // Decodes the gateway event payload and submits the real kernel extrinsic
    // so successful relays mutate CanonicalLedger. It is intentionally not
    // registered on Live chain specs, and it never auto-mints wrapped assets:
    // wrapped register/mint require a two-member council proposal executed by
    // independent governance tooling (see HIGH-TX-2).
    let submit_client = client.clone();
    let cross_vm_limiter = limiter.clone();
    module.register_method(
        "x3_submitCrossVmTransaction",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            use codec::Encode;

            if !enable_bridge_ingress_rpc {
                return Err(custom_error(
                    "x3_submitCrossVmTransaction is only available on Development/Local chain specs",
                ));
            }

            cross_vm_limiter
                .check_request(0, "x3_submitCrossVmTransaction")
                .map_err(|e| custom_error(e.to_string()))?;

            let proof_hex: String = params
                .parse::<(String,)>()
                .map(|(v,)| v)
                .map_err(|e| custom_error(format!("Invalid proof parameter: {e}")))?;
            let stripped = proof_hex.strip_prefix("0x").unwrap_or(&proof_hex);
            let proof = hex::decode(stripped)
                .map_err(|e| custom_error(format!("Proof hex decode failed: {e}")))?;
            if proof.is_empty() {
                return Err(custom_error("Proof payload must not be empty"));
            }

            let envelope = decode_signed_deposit_relay_envelope(&proof)?;
            let relay_payload = decode_deposit_relay_payload(&envelope.deposit_payload)?;
            if relay_payload.chain_id == 0 {
                return Err(custom_error("Relay payload chain_id must be non-zero"));
            }
            if relay_payload.token_address == [0u8; 20] {
                return Err(custom_error(
                    "Relay payload token_address must not be the zero address",
                ));
            }
            let bridge_nonce = u64::try_from(relay_payload.nonce)
                .map_err(|_| custom_error("Relay nonce does not fit u64"))?;
            let operation = CrossVmOperation::TransferToSvm {
                source: relay_payload.depositor,
                destination: relay_payload.recipient.clone(),
                amount: relay_payload.amount,
            };
            verify_lock_proof_binding(&operation, &envelope.lock_proof)?;

            let seed = std::env::var("X3_SUBMITTER_SEED").map_err(|_| {
                custom_error(
                    "X3_SUBMITTER_SEED env var not set — required for x3_submitCrossVmTransaction. \
                     Set to an sr25519 seed phrase for the submission key."
                )
            })?;
            let pair = sp_core::sr25519::Pair::from_string(&seed, None).map_err(|e| {
                custom_error(format!(
                    "load local submitter key from X3_SUBMITTER_SEED failed: {e:?}"
                ))
            })?;
            let account = account_from_public(pair.public());
            let best_hash = submit_client.info().best_hash;
            let genesis_hash = submit_client
                .block_hash(0)
                .map_err(|e| custom_error(format!("Genesis hash lookup failed: {e}")))?
                .ok_or_else(|| custom_error("Genesis block hash not found"))?;
            let account_nonce = submit_client
                .runtime_api()
                .account_nonce(best_hash, account.clone())
                .map_err(|e| custom_error(format!("Account nonce lookup failed: {e}")))?;

            let submit_call = |call: RuntimeCall, nonce: u32| -> Result<_, JsonRpseeError> {
                let extra: SignedExtra = (
                    frame_system::CheckNonZeroSender::<Runtime>::new(),
                    frame_system::CheckSpecVersion::<Runtime>::new(),
                    frame_system::CheckTxVersion::<Runtime>::new(),
                    frame_system::CheckGenesis::<Runtime>::new(),
                    frame_system::CheckEra::<Runtime>::from(Era::Immortal),
                    frame_system::CheckNonce::<Runtime>::from(nonce),
                    frame_system::CheckWeight::<Runtime>::new(),
                    pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
                    pallet_x3_invariants::InvariantCheck::<Runtime>::new(),
                    decode_agent_law_check()?,
                );
                let payload = SignedPayload::from_raw(
                    call.clone(),
                    extra.clone(),
                    (
                        (),
                        VERSION.spec_version,
                        VERSION.transaction_version,
                        genesis_hash,
                        genesis_hash,
                        (),
                        (),
                        (),
                        (),
                        (),
                    ),
                );
                let signature =
                    payload.using_encoded(|payload| Signature::from(pair.sign(payload)));
                let extrinsic = UncheckedExtrinsic::new_signed(
                    call,
                    Address::Id(account.clone()),
                    signature,
                    extra,
                );
                futures::executor::block_on(tx_pool.submit_one(
                    best_hash,
                    TransactionSource::External,
                    extrinsic.into(),
                ))
                .map_err(|e| custom_error(format!("Runtime extrinsic submission failed: {e}")))
            };

            let kernel_call = RuntimeCall::AtlasKernel(
                pallet_x3_kernel::Call::<Runtime>::submit_cross_vm_operation {
                    operation: operation.clone(),
                    nonce: bridge_nonce,
                    max_fee: 1_000u128,
                    proof: pallet_x3_kernel::CrossChainProof::LockProof(envelope.lock_proof),
                },
            );

            let tx_hash = submit_call(kernel_call, account_nonce)?;

            let submission_hash = sp_core::hashing::blake2_256(&proof);
            Ok(serde_json::json!({
                "status": "submitted",
                "submission_hash": format!("0x{}", hex::encode(submission_hash)),
                "extrinsic_hash": format!("{tx_hash:?}"),
                "wrapped_mint": "requires_two_member_council_proposal",
                "recipient": format!("0x{}", hex::encode(match operation {
                    CrossVmOperation::TransferToSvm { ref destination, .. } => destination,
                    _ => unreachable!(),
                })),
                "amount": relay_payload.amount.to_string(),
                "bridge_nonce": bridge_nonce,
                "bytes": proof.len(),
            }))
        },
    )?;

    // ── x3_proposeWrappedCouncil ─────────────────────────
    // Two-member wrapped-asset governance, step 1: a council member proposes
    // register/mint with threshold 2 and the node waits for the motion to be
    // stored. The returned proposal hash/index are consumed by
    // x3_executeWrappedCouncil (step 2: Alice vote + Bob vote + close).
    let propose_client = client.clone();
    let propose_pool = council_flow_pool.clone();
    let propose_limiter = limiter.clone();
    module.register_method(
        "x3_proposeWrappedCouncil",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            use codec::Encode;
            use sp_core::crypto::Pair;

            if !enable_bridge_ingress_rpc {
                return Err(custom_error(
                    "x3_proposeWrappedCouncil is only available on Development/Local chain specs",
                ));
            }
            propose_limiter
                .check_request(0, "x3_proposeWrappedCouncil")
                .map_err(|e| custom_error(e.to_string()))?;

            let (req,): (serde_json::Value,) =
                params.parse().map_err(|e| custom_error(format!("Invalid params: {e}")))?;
            let action = req
                .get("action")
                .and_then(|v| v.as_str())
                .ok_or_else(|| custom_error("Missing action (register|mint)"))?;
            let chain_id = u32::try_from(
                req.get("chain_id")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| custom_error("Missing chain_id"))?,
            )
            .map_err(|_| custom_error("chain_id does not fit u32"))?;
            let token_address_hex = req
                .get("token_address")
                .and_then(|v| v.as_str())
                .ok_or_else(|| custom_error("Missing token_address"))?;
            let token_address = decode_hex_20(token_address_hex, "token_address")?;
            let wrapped_id = wrapped_asset_id(chain_id, &token_address);

            let proposer_pair =
                load_sr25519_pair_from_env("X3_SUBMITTER_SEED")?;
            let proposer_account = account_from_public(proposer_pair.public());
            let best_hash = propose_client.info().best_hash;
            let baseline_number = propose_client.info().best_number;
            let genesis_hash = propose_client
                .block_hash(0)
                .map_err(|e| custom_error(format!("Genesis hash lookup failed: {e}")))?
                .ok_or_else(|| custom_error("Genesis block hash not found"))?;
            let proposer_nonce = propose_client
                .runtime_api()
                .account_nonce(best_hash, proposer_account.clone())
                .map_err(|e| custom_error(format!("Proposer nonce lookup failed: {e}")))?;

            let proposal = match action {
                "register" => RuntimeCall::X3Wrapped(
                    pallet_x3_wrapped::Call::<Runtime>::register_wrapped_asset {
                        asset_id: wrapped_id,
                        config: pallet_x3_wrapped::WrappedAssetConfig {
                            native_asset_id: [0u8; 32],
                            max_wrapped_supply: u128::MAX,
                            governance_weight_bps: 10_000,
                            bridge_fee_bps: 0,
                            status: pallet_x3_wrapped::WrappedAssetStatus::Active,
                        },
                    },
                ),
                "mint" => {
                    let recipient_hex = req
                        .get("recipient")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing recipient"))?;
                    let recipient = decode_hex_32(recipient_hex, "recipient")?;
                    let amount = parse_u128_value(req.get("amount"), "amount")?;
                    let nonce = req
                        .get("nonce")
                        .and_then(|v| v.as_str())
                        .and_then(|v| v.parse::<u64>().ok())
                        .ok_or_else(|| custom_error("Missing numeric nonce"))?;
                    RuntimeCall::X3Wrapped(
                        pallet_x3_wrapped::Call::<Runtime>::mint_wrapped {
                            chain_id,
                            asset_id: wrapped_id,
                            recipient: AccountId::new(recipient),
                            amount,
                            nonce,
                        },
                    )
                }
                _ => return Err(custom_error("action must be 'register' or 'mint'")),
            };
            let length_bound = proposal.encoded_size() as u32;
            let council_call = RuntimeCall::Council(
                pallet_collective::Call::<Runtime, pallet_collective::Instance1>::propose {
                    threshold: 2,
                    proposal: Box::new(proposal.clone()),
                    length_bound,
                },
            );
            let extrinsic = sign_runtime_call(
                &proposer_pair,
                &proposer_account,
                genesis_hash,
                proposer_nonce,
                council_call,
            )?;
            let before_count = read_u32_storage(&propose_client, best_hash, b"Council", b"ProposalCount")?;
            submit_to_pool(propose_pool.as_ref(), best_hash, extrinsic)?;
            wait_for_best_block_advance(&propose_client, baseline_number, 120, "council proposal inclusion")?;

            let after_hash = propose_client.info().best_hash;
            let after_count = read_u32_storage(&propose_client, after_hash, b"Council", b"ProposalCount")?;
            if after_count <= before_count {
                return Err(custom_error(
                    "Council proposal was not stored; threshold/membership check failed",
                ));
            }
            let proposal_hash = BlakeTwo256::hash_of(&proposal);
            Ok(serde_json::json!({
                "status": "pending_second_approval",
                "proposal_hash": format!("0x{}", hex::encode(proposal_hash.as_bytes())),
                "proposal_index": after_count - 1,
                "wrapped_asset_id": format!("0x{}", hex::encode(wrapped_id)),
                "action": action,
            }))
        },
    )?;

    // ── x3_executeWrappedCouncil ────────────────────────
    // Two-member wrapped-asset governance, step 2: Alice and Bob each cast an
    // Aye vote on the stored motion, then close it so the wrapped call executes.
    let execute_client = client.clone();
    let execute_pool = council_flow_pool.clone();
    let execute_limiter = limiter.clone();
    module.register_method(
        "x3_executeWrappedCouncil",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            use sp_core::crypto::Pair;

            if !enable_bridge_ingress_rpc {
                return Err(custom_error(
                    "x3_executeWrappedCouncil is only available on Development/Local chain specs",
                ));
            }
            execute_limiter
                .check_request(0, "x3_executeWrappedCouncil")
                .map_err(|e| custom_error(e.to_string()))?;

            let (req,): (serde_json::Value,) =
                params.parse().map_err(|e| custom_error(format!("Invalid params: {e}")))?;
            let proposal_hash_hex = req
                .get("proposal_hash")
                .and_then(|v| v.as_str())
                .ok_or_else(|| custom_error("Missing proposal_hash"))?;
            let proposal_hash_bytes = decode_hex_32(proposal_hash_hex, "proposal_hash")?;
            let proposal_hash = H256::from(proposal_hash_bytes);
            let proposal_index = req
                .get("proposal_index")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| custom_error("Missing proposal_index"))? as u32;

            let alice_pair = load_sr25519_pair_from_env("X3_SUBMITTER_SEED")?;
            let bob_pair = load_sr25519_pair_from_env("X3_APPROVER_SEED")?;
            if alice_pair.public() == bob_pair.public() {
                return Err(custom_error(
                    "X3_SUBMITTER_SEED and X3_APPROVER_SEED must be independent keys",
                ));
            }
            let alice_account = account_from_public(alice_pair.public());
            let bob_account = account_from_public(bob_pair.public());
            let genesis_hash = execute_client
                .block_hash(0)
                .map_err(|e| custom_error(format!("Genesis hash lookup failed: {e}")))?
                .ok_or_else(|| custom_error("Genesis block hash not found"))?;

            let mut baseline = execute_client.info().best_number;
            let best_hash = execute_client.info().best_hash;
            let alice_nonce = execute_client
                .runtime_api()
                .account_nonce(best_hash, alice_account.clone())
                .map_err(|e| custom_error(format!("Alice nonce lookup failed: {e}")))?;
            let alice_vote = RuntimeCall::Council(
                pallet_collective::Call::<Runtime, pallet_collective::Instance1>::vote {
                    proposal: proposal_hash,
                    index: proposal_index,
                    approve: true,
                },
            );
            let extrinsic = sign_runtime_call(
                &alice_pair,
                &alice_account,
                genesis_hash,
                alice_nonce,
                alice_vote,
            )?;
            submit_to_pool(execute_pool.as_ref(), best_hash, extrinsic)
                .map_err(|e| custom_error(format!("Alice vote submit: {e}")))?;
            wait_for_best_block_advance(&execute_client, baseline, 120, "Alice council vote")?;

            baseline = execute_client.info().best_number;
            let best_hash = execute_client.info().best_hash;
            let bob_nonce = execute_client
                .runtime_api()
                .account_nonce(best_hash, bob_account.clone())
                .map_err(|e| custom_error(format!("Bob nonce lookup failed: {e}")))?;
            let bob_vote = RuntimeCall::Council(
                pallet_collective::Call::<Runtime, pallet_collective::Instance1>::vote {
                    proposal: proposal_hash,
                    index: proposal_index,
                    approve: true,
                },
            );
            let extrinsic = sign_runtime_call(
                &bob_pair,
                &bob_account,
                genesis_hash,
                bob_nonce,
                bob_vote,
            )?;
            submit_to_pool(execute_pool.as_ref(), best_hash, extrinsic)
                .map_err(|e| custom_error(format!("Bob vote submit: {e}")))?;
            wait_for_best_block_advance(&execute_client, baseline, 120, "Bob council vote")?;

            baseline = execute_client.info().best_number;
            let best_hash = execute_client.info().best_hash;
            let bob_close_nonce = execute_client
                .runtime_api()
                .account_nonce(best_hash, bob_account.clone())
                .map_err(|e| custom_error(format!("Bob close nonce lookup failed: {e}")))?;
            let close = RuntimeCall::Council(
                pallet_collective::Call::<Runtime, pallet_collective::Instance1>::close {
                    proposal_hash,
                    index: proposal_index,
                    proposal_weight_bound: sp_runtime::Weight::from_parts(
                        1_000_000_000,
                        1_000_000,
                    ),
                    length_bound: 1_000_000,
                },
            );
            let extrinsic = sign_runtime_call(
                &bob_pair,
                &bob_account,
                genesis_hash,
                bob_close_nonce,
                close,
            )?;
            submit_to_pool(execute_pool.as_ref(), best_hash, extrinsic)
                .map_err(|e| custom_error(format!("Council close submit: {e}")))?;
            wait_for_best_block_advance(&execute_client, baseline, 120, "council close")?;

            Ok(serde_json::json!({
                "status": "executed",
                "proposal_hash": format!("0x{}", hex::encode(proposal_hash.as_bytes())),
                "proposal_index": proposal_index,
            }))
        },
    )?;

    // ── crossVm_getRecentTransfers ──────────────────────
    // Returns recent cross-VM asset transfers from the bridge pallet state.
    let cross_vm_client = client.clone();
    module.register_method(
        "crossVm_getRecentTransfers",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let limit: u32 = params
                .parse::<(serde_json::Value,)>()
                .ok()
                .and_then(|(v,)| v.get("limit").and_then(|l| l.as_u64()))
                .unwrap_or(20) as u32;
            // Query the bridge pallet's recent transfers storage (if available).
            // Fall back to an empty array when the runtime API is unavailable.
            let api = cross_vm_client.runtime_api();
            let block_hash = cross_vm_client.info().best_hash;

            let transfers: Vec<serde_json::Value> = match api.get_cross_vm_transfers(block_hash) {
                Ok(encoded) => {
                    use codec::Decode;
                    let pairs: Vec<(sp_core::H256, Vec<u8>)> =
                        Decode::decode(&mut &encoded[..]).unwrap_or_default();
                    pairs
                        .into_iter()
                        .take(limit as usize)
                        .map(|(id, record_bytes)| {
                            serde_json::json!({
                                "message_id": format!("0x{}", hex::encode(id.as_bytes())),
                                "encoded_record": format!("0x{}", hex::encode(&record_bytes)),
                            })
                        })
                        .collect()
                }
                Err(_) => vec![],
            };

            Ok(serde_json::json!({
                "transfers": transfers,
                "total": transfers.len(),
            }))
        },
    )?;

    // ── token_getSupply ──────────────────────────────────
    // Returns the total token supply from the balances pallet.
    let supply_client = client.clone();
    module.register_method(
        "token_getSupply",
        move |_params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let block_hash = supply_client.info().best_hash;
            let total_issuance = supply_client
                .runtime_api()
                .get_total_issuance(block_hash)
                .unwrap_or_default();

            // Read locked (protocol-held, non-circulating) native supply from chain state.
            let locked = supply_client
                .runtime_api()
                .native_locked_supply(block_hash)
                .unwrap_or_default();

            let circulating = total_issuance.saturating_sub(locked);

            Ok(serde_json::json!({
                "total_supply": total_issuance.to_string(),
                "circulating_supply": circulating.to_string(),
                "locked_supply": locked.to_string(),
            }))
        },
    )?;

    // ── swarm_getMetrics ─────────────────────────────────
    // Proxies to x3-swarm-api at :8787 for swarm telemetry.
    module.register_method(
        "swarm_getMetrics",
        move |_params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let result = run_on_rpc_thread(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("Failed to start swarm RPC helper runtime: {e}"))?;
                Ok(rt
                    .block_on(async {
                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_millis(800))
                            .build()
                            .ok()?;
                        let health: serde_json::Value = client
                            .get("http://127.0.0.1:8787/health")
                            .send()
                            .await
                            .ok()?
                            .json()
                            .await
                            .ok()?;
                        let scoreboard: serde_json::Value = client
                            .get("http://127.0.0.1:8787/scoreboard")
                            .send()
                            .await
                            .ok()?
                            .json()
                            .await
                            .ok()?;
                        Some(serde_json::json!({
                            "health": health,
                            "scoreboard": scoreboard,
                        }))
                    })
                    .unwrap_or(serde_json::json!({
                        "health": { "status": "unreachable" },
                        "scoreboard": { "tasks_total": 0, "success_rate": 0.0 },
                        "swarm_api": "http://127.0.0.1:8787",
                    })))
            })?;
            Ok(result)
        },
    )?;

    // ── swarm_getRecentTasks ──────────────────────────────
    // Proxies to x3-swarm-api :8787/tasks for recent task list.
    module.register_method(
        "swarm_getRecentTasks",
        move |_params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let result = run_on_rpc_thread(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("Failed to start swarm RPC helper runtime: {e}"))?;
                Ok(rt
                    .block_on(async {
                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_millis(800))
                            .build()
                            .ok()?;
                        let tasks: serde_json::Value = client
                            .get("http://127.0.0.1:8787/tasks")
                            .send()
                            .await
                            .ok()?
                            .json()
                            .await
                            .ok()?;
                        Some(serde_json::json!({ "tasks": tasks }))
                    })
                    .unwrap_or(serde_json::json!({
                        "tasks": [],
                        "swarm_api": "http://127.0.0.1:8787",
                    })))
            })?;
            Ok(result)
        },
    )?;

    // ── flash_getCertificate ──────────────────────────────
    // Returns the latest Flash Finality certificate for a given block number.
    if let Some(gadget) = gadget.as_ref() {
        let gadget_cert = gadget.clone();
        module.register_method(
            "flash_getCertificate",
            move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
                let block_number: u64 = params
                    .parse::<(u64,)>()
                    .map(|(v,)| v)
                    .map_err(|e| custom_error(format!("Invalid block_number parameter: {e}")))?;
                let gadget_cert = gadget_cert.clone();
                let cert = run_on_rpc_thread(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| format!("Failed to start flash RPC helper runtime: {e}"))?;
                    Ok(rt.block_on(gadget_cert.get_certificate_by_number(block_number)))
                })?;
                Ok(match cert {
                    Some(c) => serde_json::to_value(c).unwrap_or_default(),
                    None => serde_json::Value::Null,
                })
            },
        )?;
    }

    // ── flash_getMetrics ──────────────────────────────────
    // Returns Flash Finality metrics snapshot.
    if let Some(gadget) = gadget.as_ref() {
        let gadget_metrics = gadget.clone();
        module.register_method(
            "flash_getMetrics",
            move |_params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
                let gadget_metrics = gadget_metrics.clone();
                let metrics = run_on_rpc_thread(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| format!("Failed to start flash RPC helper runtime: {e}"))?;
                    Ok(rt.block_on(gadget_metrics.metrics()))
                })?;
                Ok(serde_json::to_value(metrics).unwrap_or_default())
            },
        )?;
    }

    // ── Gas Estimation RPC ──────────────────────────────
    // Simulation-only gas estimator for off-chain developer tooling.
    // Production nodes should use the Frontier stub in rpc_frontier.
    #[allow(deprecated)] // see note above: intentional simulation-only estimator.
    let gas_estimator = std::sync::Arc::new(GasEstimationRPC::new());
    {
        let ge = gas_estimator.clone();
        module.register_method(
            "x3_estimateGas",
            move |params, _, _| -> Result<serde_json::Value, JsonRpseeError> {
                let tx: RPCTransaction = params
                    .parse()
                    .map_err(|e| custom_error(format!("Invalid tx params: {e}")))?;
                ge.estimate_gas(&tx)
                    .map(|est| serde_json::to_value(est).unwrap_or_default())
                    .map_err(|e| custom_error(e))
            },
        )?;
    }
    {
        let ge = gas_estimator.clone();
        module.register_method(
            "x3_estimateGasMany",
            move |params, _, _| -> Result<serde_json::Value, JsonRpseeError> {
                let txs: Vec<RPCTransaction> = params
                    .parse()
                    .map_err(|e| custom_error(format!("Invalid batch tx params: {e}")))?;
                ge.estimate_gas_many(&txs)
                    .map(|ests| serde_json::to_value(ests).unwrap_or_default())
                    .map_err(|e| custom_error(e))
            },
        )?;
    }
    {
        let ge = gas_estimator.clone();
        module.register_method(
            "x3_call",
            move |params, _, _| -> Result<serde_json::Value, JsonRpseeError> {
                let tx: RPCTransaction = params
                    .parse()
                    .map_err(|e| custom_error(format!("Invalid call params: {e}")))?;
                ge.call(&tx)
                    .map(|output| {
                        serde_json::json!({ "output": format!("0x{}", hex::encode(output)) })
                    })
                    .map_err(|e| custom_error(e))
            },
        )?;
    }

    // ── Benchmark RPC ────────────────────────────────────
    // Benchmark RPC requires PostgreSQL database (PgPool) which is not
    // available in the default node startup. Methods return an error until
    // a DatabaseBenchmarkService with a configured pool is wired.
    module.register_method(
        "x3_benchmarkSubmitJob",
        move |_, _, _| -> Result<serde_json::Value, JsonRpseeError> {
            Err(custom_error(
                "Benchmark RPC requires PostgreSQL database — not configured on this node",
            ))
        },
    )?;
    module.register_method(
        "x3_benchmarkGetJob",
        move |_, _, _| -> Result<serde_json::Value, JsonRpseeError> {
            Err(custom_error(
                "Benchmark RPC requires PostgreSQL database — not configured on this node",
            ))
        },
    )?;
    module.register_method(
        "x3_benchmarkGetReport",
        move |_, _, _| -> Result<serde_json::Value, JsonRpseeError> {
            Err(custom_error(
                "Benchmark RPC requires PostgreSQL database — not configured on this node",
            ))
        },
    )?;
    module.register_method(
        "x3_benchmarkListJobs",
        move |_, _, _| -> Result<serde_json::Value, JsonRpseeError> {
            Err(custom_error(
                "Benchmark RPC requires PostgreSQL database — not configured on this node",
            ))
        },
    )?;

    // ── network_subscribeMetrics (subscription) ───────────
    // Subscribes to live network metrics via WebSocket.
    // Polls system_health + system_peers every 5s and pushes updates.
    let metrics_client = client.clone();
    module.register_subscription(
        "network_subscribeMetrics",
        "network_subscribeMetrics",
        "network_unsubscribeMetrics",
        move |_params, pending, _ctx, _ext| {
            let metrics_client = metrics_client.clone();
            async move {
                let sink = match pending.accept().await {
                    Ok(sink) => sink,
                    Err(_) => return,
                };

                let client = metrics_client.clone();
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

                loop {
                    interval.tick().await;
                    let block_hash = client.info().best_hash;

                    let peers = client.runtime_api().get_peer_count(block_hash).unwrap_or(0);

                    let metrics = serde_json::json!({
                        "peers": peers,
                        "best_block": block_hash.to_string(),
                        "timestamp": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                            .to_string(),
                    });

                    if let Ok(msg) = jsonrpsee::server::SubscriptionMessage::from_json(&metrics) {
                        if sink.send(msg).await.is_err() {
                            break; // client disconnected
                        }
                    }
                }
            }
        },
    )?;

    Ok(module)
}
