//! Runtime-aware signer for native X3VM atomic settlement calls.
//!
//! This is the concrete implementation of `x3_atomic_swap::X3ExtrinsicSigner`.
//! It uses the exact `x3_chain_runtime::{RuntimeCall, SignedExtra, SignedPayload}`
//! layout used by the node's atomic gateway, queries the live node for genesis
//! hash and account nonce, and signs the X3 settlement-engine calls with sr25519.
//! Local atomic-swap `u64` intent ids are never guessed into runtime ids: callers
//! must explicitly bind them to the real `H256` produced by `create_intent`.

use codec::{Decode, Encode};
use frame_support::storage::storage_prefix;
use pallet_x3_settlement_engine::{AssetSpec, ExternalChainId, TokenId};
use serde_json::Value;
use sp_core::crypto::Ss58Codec;
use sp_core::{Pair as PairTrait, H256};
use sp_runtime::generic::Era;
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;
use x3_atomic_swap::intent::IntentId;
use x3_atomic_swap::{AtomicIntent, ChainId, RpcClient, SwapError, X3ExtrinsicSigner};
use x3_chain_runtime::{
    AccountId, Address, Runtime, RuntimeCall, RuntimeEvent, Signature, SignedExtra, SignedPayload,
    UncheckedExtrinsic, VERSION,
};

/// A signed create-intent transaction. The runtime intent id CANNOT be known
/// before the extrinsic executes on-chain: `generate_intent_id` hashes in
/// `T::UnixTime::now()`, a value only the runtime observes at execution time.
/// Callers must submit `signed_extrinsic`, wait for finality, then call
/// [`X3RuntimeSigner::resolve_intent_id`] with the finalized block hash to
/// deterministically reconstruct the exact id the runtime allocated.
#[derive(Debug, Clone)]
pub struct PreparedX3Intent {
    /// Maker account (the signer) used as a hash input.
    pub maker: AccountId,
    /// Taker account used as a hash input.
    pub taker: AccountId,
    /// `TotalIntents` value observed before submission; this becomes the
    /// intent's `nonce` hash input as long as no other `create_intent` from
    /// any account lands in an earlier block first (true for single-signer
    /// test/orchestration flows).
    pub nonce: u64,
    /// Complete signed SCALE extrinsic encoded for `author_submitExtrinsic`.
    pub signed_extrinsic: String,
}

/// Concrete runtime-aware signer for X3 settlement-engine extrinsics.
pub struct X3RuntimeSigner {
    chain_id: ChainId,
    rpc: Mutex<RpcClient>,
    pair: sp_core::sr25519::Pair,
    intent_bindings: Mutex<BTreeMap<IntentId, H256>>,
}

impl fmt::Debug for X3RuntimeSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("X3RuntimeSigner")
            .field("chain_id", &self.chain_id)
            .field("account", &self.account())
            .finish_non_exhaustive()
    }
}

impl X3RuntimeSigner {
    /// Load an sr25519 signer URI and attach it to a live X3 RPC endpoint.
    pub fn from_uri(
        chain_id: ChainId,
        rpc_url: String,
        uri: &str,
    ) -> Result<Self, SwapError> {
        let pair = sp_core::sr25519::Pair::from_string(uri, None).map_err(|e| {
            SwapError::Internal(format!("failed to load X3 sr25519 signer {uri}: {e:?}"))
        })?;
        Ok(Self {
            chain_id,
            rpc: Mutex::new(RpcClient::new(rpc_url, 0)),
            pair,
            intent_bindings: Mutex::new(BTreeMap::new()),
        })
    }

    /// Account id controlled by this signer.
    pub fn account(&self) -> AccountId {
        <Signature as Verify>::Signer::from(self.pair.public()).into_account()
    }

    /// Explicitly bind an off-chain atomic-swap id to its real runtime H256 id.
    pub fn bind_intent(
        &self,
        local_intent_id: IntentId,
        runtime_intent_id: H256,
    ) -> Result<(), SwapError> {
        if runtime_intent_id == H256::zero() {
            return Err(SwapError::Internal(
                "refusing to bind local intent to zero runtime intent id".into(),
            ));
        }
        let mut bindings = self
            .intent_bindings
            .lock()
            .map_err(|_| SwapError::Internal("X3 intent binding mutex poisoned".into()))?;
        if let Some(existing) = bindings.get(&local_intent_id) {
            if *existing != runtime_intent_id {
                return Err(SwapError::Internal(format!(
                    "local intent {local_intent_id} already bound to a different runtime id"
                )));
            }
        }
        bindings.insert(local_intent_id, runtime_intent_id);
        Ok(())
    }

    fn runtime_intent_id(&self, local_intent_id: IntentId) -> Result<H256, SwapError> {
        self.intent_bindings
            .lock()
            .map_err(|_| SwapError::Internal("X3 intent binding mutex poisoned".into()))?
            .get(&local_intent_id)
            .copied()
            .ok_or_else(|| {
                SwapError::Internal(format!(
                    "local intent {local_intent_id} is not bound to an on-chain X3 settlement intent"
                ))
            })
    }

    fn rpc_call(&self, method: &str, params: Vec<Value>) -> Result<Value, SwapError> {
        let mut rpc = self
            .rpc
            .lock()
            .map_err(|_| SwapError::RpcError("X3 signer RPC mutex poisoned".into()))?;
        let response = rpc.call(method, params)?;
        Ok(response.result.unwrap_or(Value::Null))
    }

    fn genesis_hash(&self) -> Result<H256, SwapError> {
        let result = self.rpc_call("chain_getBlockHash", vec![Value::from(0u64)])?;
        let raw = result
            .as_str()
            .ok_or_else(|| SwapError::RpcError("chain_getBlockHash(0) was not a string".into()))?;
        let bytes = hex::decode(raw.strip_prefix("0x").unwrap_or(raw))
            .map_err(|e| SwapError::RpcError(format!("decode genesis hash: {e}")))?;
        if bytes.len() != 32 {
            return Err(SwapError::RpcError(format!(
                "genesis hash must be 32 bytes, got {}",
                bytes.len()
            )));
        }
        Ok(H256::from_slice(&bytes))
    }

    fn account_nonce(&self) -> Result<u32, SwapError> {
        let account = self.account().to_ss58check();
        let result = self.rpc_call("system_accountNextIndex", vec![Value::String(account)])?;
        let nonce = if let Some(n) = result.as_u64() {
            n
        } else if let Some(raw) = result.as_str() {
            if let Some(hex) = raw.strip_prefix("0x") {
                u64::from_str_radix(hex, 16).map_err(|e| {
                    SwapError::RpcError(format!("decode account nonce '{raw}': {e}"))
                })?
            } else {
                raw.parse::<u64>().map_err(|e| {
                    SwapError::RpcError(format!("decode account nonce '{raw}': {e}"))
                })?
            }
        } else {
            return Err(SwapError::RpcError(
                "system_accountNextIndex returned unsupported nonce shape".into(),
            ));
        };
        u32::try_from(nonce)
            .map_err(|_| SwapError::RpcError(format!("account nonce {nonce} exceeds u32")))
    }

    fn total_intents(&self) -> Result<u64, SwapError> {
        let key = storage_prefix(b"X3SettlementEngine", b"TotalIntents");
        let result = self.rpc_call(
            "state_getStorage",
            vec![Value::String(format!("0x{}", hex::encode(key)))],
        )?;
        if result.is_null() {
            return Ok(0);
        }
        let raw = result
            .as_str()
            .ok_or_else(|| SwapError::RpcError("TotalIntents storage was not hex".into()))?;
        let bytes = hex::decode(raw.strip_prefix("0x").unwrap_or(raw))
            .map_err(|e| SwapError::RpcError(format!("decode TotalIntents storage: {e}")))?;
        u64::decode(&mut &bytes[..])
            .map_err(|e| SwapError::RpcError(format!("SCALE decode TotalIntents: {e}")))
    }

    fn signed_extrinsic(&self, call: RuntimeCall) -> Result<String, SwapError> {
        let genesis_hash = self.genesis_hash()?;
        let nonce = self.account_nonce()?;
        let account = self.account();
        let agent_law_check = pallet_x3_agent_law::AgentLawCheck::<Runtime>::decode(&mut &[][..])
            .map_err(|e| SwapError::Internal(format!("decode agent-law extension: {e}")))?;

        // This tuple and additional-signed payload intentionally mirror
        // node/src/atomic_gateway.rs. The order is consensus-critical.
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
            agent_law_check,
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
        let signature = payload.using_encoded(|bytes| Signature::from(self.pair.sign(bytes)));
        let xt = UncheckedExtrinsic::new_signed(
            call,
            Address::Id(account),
            signature,
            extra,
        );
        Ok(format!("0x{}", hex::encode(xt.encode())))
    }

    /// Read `Timestamp::Now` (the `u64` unix-seconds moment set by the
    /// mandatory `pallet_timestamp` inherent) as observed *at* a specific
    /// block. This is the exact same value `T::UnixTime::now()` returns to
    /// any pallet executing within that block, including
    /// `generate_intent_id`'s internal timestamp read.
    fn timestamp_at(&self, block_hash: H256) -> Result<u64, SwapError> {
        let key = storage_prefix(b"Timestamp", b"Now");
        let result = self.rpc_call(
            "state_getStorage",
            vec![
                Value::String(format!("0x{}", hex::encode(key))),
                Value::String(format!("0x{}", hex::encode(block_hash.as_bytes()))),
            ],
        )?;
        let raw = result.as_str().ok_or_else(|| {
            SwapError::RpcError("Timestamp::Now storage was not hex".into())
        })?;
        let bytes = hex::decode(raw.strip_prefix("0x").unwrap_or(raw))
            .map_err(|e| SwapError::RpcError(format!("decode Timestamp::Now storage: {e}")))?;
        // pallet_timestamp stores the moment in milliseconds; the settlement
        // engine's `T::UnixTime::now().as_secs()` divides by 1000 internally
        // via `sp_timestamp::InherentDataProvider`/`UnixTime` blanket impl.
        let millis = u64::decode(&mut &bytes[..])
            .map_err(|e| SwapError::RpcError(format!("SCALE decode Timestamp::Now: {e}")))?;
        Ok(millis / 1000)
    }

    /// Reconstruct the exact `H256` id the runtime allocated to a
    /// `create_intent` call once it has finalized, by re-deriving the same
    /// `blake2_256(maker ++ taker ++ nonce ++ unix_secs)` hash the pallet
    /// computes internally, using the real on-chain timestamp read from the
    /// finalized block instead of guessing it beforehand.
    pub fn resolve_intent_id(
        &self,
        prepared: &PreparedX3Intent,
        finalized_block_hash: H256,
    ) -> Result<H256, SwapError> {
        let unix_secs = self.timestamp_at(finalized_block_hash)?;
        let mut data = prepared.maker.encode();
        data.extend(prepared.taker.encode());
        data.extend(prepared.nonce.to_le_bytes());
        data.extend(unix_secs.to_le_bytes());
        Ok(H256::from(sp_core::hashing::blake2_256(&data)))
    }

    /// Build the on-chain `create_intent` call. The runtime intent id cannot
    /// be predicted here (see [`PreparedX3Intent`]); callers must submit,
    /// wait for finality, then call [`Self::resolve_intent_id`].
    pub fn prepare_create_intent(
        &self,
        taker: AccountId,
        asset_a: AssetSpec,
        asset_b: AssetSpec,
        secret_hash: H256,
        timeout_seconds: Option<u64>,
    ) -> Result<PreparedX3Intent, SwapError> {
        let nonce = self.total_intents()?;
        let call = RuntimeCall::X3SettlementEngine(
            pallet_x3_settlement_engine::Call::<Runtime>::create_intent {
                taker: taker.clone(),
                asset_a,
                asset_b,
                secret_hash,
                timeout_seconds,
            },
        );
        Ok(PreparedX3Intent {
            maker: self.account(),
            taker,
            nonce,
            signed_extrinsic: self.signed_extrinsic(call)?,
        })
    }

    /// Sign an explicit settlement escrow leg. Used by multi-leg orchestration
    /// and the live lifecycle test after the primary adapter lock.
    pub fn sign_lock_escrow_leg(
        &self,
        runtime_intent_id: H256,
        leg_index: u32,
        chain: ExternalChainId,
        amount: u128,
        escrow_data: Vec<u8>,
    ) -> Result<String, SwapError> {
        let call = RuntimeCall::X3SettlementEngine(
            pallet_x3_settlement_engine::Call::<Runtime>::lock_escrow {
                intent_id: runtime_intent_id,
                leg_index,
                chain,
                amount,
                escrow_data,
            },
        );
        self.signed_extrinsic(call)
    }

    /// Convenience native-asset spec for local X3 lifecycle tests/tools.
    pub fn x3_native_asset(amount: u128) -> AssetSpec {
        AssetSpec {
            chain: ExternalChainId::X3Native,
            token: TokenId::Native,
            amount,
        }
    }
}

impl X3ExtrinsicSigner for X3RuntimeSigner {
    fn sign_lock_escrow(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent: &AtomicIntent,
    ) -> Result<String, SwapError> {
        if chain_id != &self.chain_id {
            return Err(SwapError::Internal(format!(
                "X3 signer chain mismatch: configured {}, requested {}",
                self.chain_id, chain_id
            )));
        }
        let runtime_intent_id = self.runtime_intent_id(intent.intent_id)?;
        self.sign_lock_escrow_leg(
            runtime_intent_id,
            0,
            ExternalChainId::X3Native,
            intent.amount_in,
            escrow_address.to_vec(),
        )
    }

    fn sign_claim_settlement(
        &self,
        chain_id: &ChainId,
        intent_id: IntentId,
        preimage: [u8; 32],
    ) -> Result<String, SwapError> {
        if chain_id != &self.chain_id {
            return Err(SwapError::Internal("X3 signer chain mismatch".into()));
        }
        let runtime_intent_id = self.runtime_intent_id(intent_id)?;
        let call = RuntimeCall::X3SettlementEngine(
            pallet_x3_settlement_engine::Call::<Runtime>::claim_settlement {
                intent_id: runtime_intent_id,
                secret: H256::from(preimage),
            },
        );
        self.signed_extrinsic(call)
    }

    fn sign_refund_settlement(
        &self,
        chain_id: &ChainId,
        intent_id: IntentId,
    ) -> Result<String, SwapError> {
        if chain_id != &self.chain_id {
            return Err(SwapError::Internal("X3 signer chain mismatch".into()));
        }
        let runtime_intent_id = self.runtime_intent_id(intent_id)?;
        let call = RuntimeCall::X3SettlementEngine(
            pallet_x3_settlement_engine::Call::<Runtime>::refund_settlement {
                intent_id: runtime_intent_id,
            },
        );
        self.signed_extrinsic(call)
    }

    fn verify_finalized_dispatch(
        &self,
        block_hash: &str,
        extrinsic_index: u32,
    ) -> Result<(), SwapError> {
        let key = storage_prefix(b"System", b"Events");
        let result = self.rpc_call(
            "state_getStorage",
            vec![
                Value::String(format!("0x{}", hex::encode(key))),
                Value::String(block_hash.to_string()),
            ],
        )?;
        let raw = result
            .as_str()
            .ok_or_else(|| SwapError::RpcError("System::Events storage was not hex".into()))?;
        let bytes = hex::decode(raw.strip_prefix("0x").unwrap_or(raw))
            .map_err(|e| SwapError::RpcError(format!("decode System::Events storage: {e}")))?;
        let records =
            Vec::<frame_system::EventRecord<RuntimeEvent, H256>>::decode(&mut &bytes[..]).map_err(
                |e| SwapError::RpcError(format!("SCALE decode System::Events: {e}")),
            )?;

        let mut saw_failed_dispatch = false;
        for record in records {
            let frame_system::Phase::ApplyExtrinsic(index) = record.phase else {
                continue;
            };
            if index != extrinsic_index {
                continue;
            }
            match record.event {
                RuntimeEvent::System(frame_system::Event::ExtrinsicSuccess { .. }) => return Ok(()),
                RuntimeEvent::System(frame_system::Event::ExtrinsicFailed { .. }) => {
                    saw_failed_dispatch = true;
                }
                _ => {}
            }
        }

        if saw_failed_dispatch {
            Err(SwapError::RpcError(format!(
                "X3 extrinsic at finalized index {extrinsic_index} dispatched with ExtrinsicFailed"
            )))
        } else {
            Err(SwapError::RpcError(format!(
                "no terminal System dispatch event found for finalized extrinsic index {extrinsic_index}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unbound_local_intent() {
        let signer = X3RuntimeSigner::from_uri(
            "x3-local".into(),
            "http://127.0.0.1:1".into(),
            "//Alice",
        )
        .expect("dev key loads without RPC");
        let err = signer.runtime_intent_id(7).unwrap_err();
        assert!(err.to_string().contains("not bound"));
    }

    #[test]
    fn binding_is_stable_and_cannot_be_repointed() {
        let signer = X3RuntimeSigner::from_uri(
            "x3-local".into(),
            "http://127.0.0.1:1".into(),
            "//Alice",
        )
        .expect("dev key loads without RPC");
        let a = H256([1u8; 32]);
        let b = H256([2u8; 32]);
        signer.bind_intent(7, a).unwrap();
        signer.bind_intent(7, a).unwrap();
        assert_eq!(signer.runtime_intent_id(7).unwrap(), a);
        assert!(signer.bind_intent(7, b).is_err());
    }
}
