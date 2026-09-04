//! X3 Chain node RPC module wiring.
//!
//! Assembles the full JSON-RPC module used by the service.
//! Merges substrate system RPCs, transaction-payment RPCs, chain RPCs,
//! and the Frontier-compatible ETH/SVM RPC provided by `rpc_frontier`.

use codec::{Decode, Encode};
use flash_finality::FlashFinalityGadget;
use jsonrpsee::{types::ErrorObjectOwned, RpcModule};
use pallet_x3_kernel::AtlasKernelRuntimeApi;
use sc_client_api::{BlockBackend, StorageProvider};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_core::storage::StorageKey;
use sp_core::Pair;
use sp_runtime::generic::Era;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_runtime::transaction_validity::TransactionSource;
use std::sync::{Arc, Mutex};
use substrate_frame_rpc_system::AccountNonceApi;
use x3_atomic_trade::{AMMPool, SwapRPCServer};
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
    GasEstimationRPC, RPCTransaction, SwapRequest, WalletDexApi, WalletDexRpc, WalletServiceApi,
    WalletServiceRpc,
};

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

fn decode_agent_law_check() -> Result<pallet_x3_agent_law::AgentLawCheck<Runtime>, JsonRpseeError> {
    codec::Decode::decode(&mut &[][..])
        .map_err(|e| custom_error(format!("decode agent law extension failed: {e}")))
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
) -> Result<RpcModule<()>, RpcError>
where
    P: TransactionPool<Block = Block> + Sync + Send + 'static,
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
{
    let mut module = RpcModule::new(());

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

    // ── x3_submitCrossVmTransaction ─────────────────────
    // Local bridge-testnet ingress for relayer-submitted deposit payloads.
    // Decodes the gateway event payload and submits the real kernel extrinsic
    // so successful relays mutate CanonicalLedger.
    let submit_client = client.clone();
    let cross_vm_limiter = limiter.clone();
    module.register_method(
        "x3_submitCrossVmTransaction",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            use codec::Encode;

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

            let council_call = |proposal: RuntimeCall| -> RuntimeCall {
                let length_bound = proposal.encoded_size() as u32;
                RuntimeCall::Council(pallet_collective::Call::<
                    Runtime,
                    pallet_collective::Instance1,
                >::propose {
                    threshold: 1,
                    proposal: Box::new(proposal),
                    length_bound,
                })
            };

            let kernel_call = RuntimeCall::AtlasKernel(
                pallet_x3_kernel::Call::<Runtime>::submit_cross_vm_operation {
                    operation: operation.clone(),
                    nonce: bridge_nonce,
                    max_fee: 1_000u128,
                    proof: pallet_x3_kernel::CrossChainProof::LockProof(envelope.lock_proof),
                },
            );

            let wrapped_chain_id = u32::try_from(relay_payload.chain_id)
                .map_err(|_| custom_error("Relay chain id does not fit u32"))?;
            let wrapped_asset_id = wrapped_asset_id(wrapped_chain_id, &relay_payload.token_address);
            let wrapped_recipient = AccountId::decode(&mut &relay_payload.recipient[..])
                .map_err(|e| custom_error(format!("Wrapped recipient decode failed: {e}")))?;
            let register_wrapped_call = council_call(RuntimeCall::X3Wrapped(
                pallet_x3_wrapped::Call::<Runtime>::register_wrapped_asset {
                    asset_id: wrapped_asset_id,
                    config: pallet_x3_wrapped::WrappedAssetConfig {
                        native_asset_id: [0u8; 32],
                        max_wrapped_supply: u128::MAX,
                        governance_weight_bps: 10_000,
                        bridge_fee_bps: 0,
                        status: pallet_x3_wrapped::WrappedAssetStatus::Active,
                    },
                },
            ));
            let mint_wrapped_call = council_call(RuntimeCall::X3Wrapped(
                pallet_x3_wrapped::Call::<Runtime>::mint_wrapped {
                    chain_id: wrapped_chain_id,
                    asset_id: wrapped_asset_id,
                    recipient: wrapped_recipient,
                    amount: relay_payload.amount,
                    nonce: bridge_nonce,
                },
            ));

            let tx_hash = submit_call(kernel_call, account_nonce)?;
            let register_nonce = account_nonce
                .checked_add(1)
                .ok_or_else(|| custom_error("Account nonce overflow before wrapped register"))?;
            let mint_nonce = account_nonce
                .checked_add(2)
                .ok_or_else(|| custom_error("Account nonce overflow before wrapped mint"))?;
            let register_tx_hash = submit_call(register_wrapped_call, register_nonce)?;
            let mint_tx_hash = submit_call(mint_wrapped_call, mint_nonce)?;

            let submission_hash = sp_core::hashing::blake2_256(&proof);
            Ok(serde_json::json!({
                "status": "submitted",
                "submission_hash": format!("0x{}", hex::encode(submission_hash)),
                "extrinsic_hash": format!("{tx_hash:?}"),
                "wrapped_register_extrinsic_hash": format!("{register_tx_hash:?}"),
                "wrapped_mint_extrinsic_hash": format!("{mint_tx_hash:?}"),
                "wrapped_chain_id": wrapped_chain_id,
                "wrapped_asset_id": format!("0x{}", hex::encode(wrapped_asset_id)),
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

            // Read locked supply from treasury + staking pallets if available
            let locked: u128 = 0u128; // placeholder — wire to treasury_reserved when pallet active

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

    // ── Gas Estimation RPC ───────────────────────────────
    // Simulation-only gas estimator for off-chain developer tooling.
    // Production nodes should use the Frontier stub in rpc_frontier.
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
