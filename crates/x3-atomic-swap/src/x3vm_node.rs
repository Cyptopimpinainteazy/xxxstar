//! Native X3-node transport for atomic lock/claim/refund.
//!
//! This module turns the generic `X3VmLiveTransport` boundary into a real
//! Substrate/X3 node RPC transport. Signing remains outside this crate: callers
//! provide an `X3ExtrinsicSigner` that returns a signed SCALE extrinsic for the
//! exact X3 settlement-engine call. The transport submits that extrinsic with
//! `author_submitExtrinsic`, waits until the exact extrinsic is present in a
//! GRANDPA-finalized block, and returns proof objects backed by finalized block
//! evidence from the node.

use crate::adapter::{
    AdapterReadinessScore, ChainHealth, ChainId, ClaimProof, FeeEstimate, FinalityProof, LockProof,
    RefundProof, TxId, VmType,
};
use crate::error::SwapError;
use crate::intent::{AtomicIntent, IntentId};
use crate::rpc_client::RpcClient;
use crate::x3vm_live::X3VmLiveTransport;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

/// External signing boundary for native X3 settlement calls.
///
/// Implementations may use a local keystore, remote signer, KMS, or HSM. This
/// crate never receives private key material. Returned values MUST be complete
/// signed SCALE extrinsics encoded as `0x`-prefixed hex.
pub trait X3ExtrinsicSigner: Send + Sync + Debug {
    fn sign_lock_escrow(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent: &AtomicIntent,
    ) -> Result<String, SwapError>;

    fn sign_claim_settlement(
        &self,
        chain_id: &ChainId,
        intent_id: IntentId,
        preimage: [u8; 32],
    ) -> Result<String, SwapError>;

    fn sign_refund_settlement(
        &self,
        chain_id: &ChainId,
        intent_id: IntentId,
    ) -> Result<String, SwapError>;
}

/// Finalized inclusion evidence captured directly from the X3 node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct X3FinalizedInclusionProof {
    pub tx_id: String,
    pub block_hash: String,
    pub block_number: u64,
    pub state_root: String,
    pub extrinsic_index: u32,
    pub signed_extrinsic: String,
}

#[derive(Debug, Clone)]
pub struct X3NodeTransportConfig {
    pub chain_id: ChainId,
    pub rpc_url: String,
    /// Number of finalized-head polls before failing closed.
    pub finality_poll_attempts: u32,
    /// Delay between finalized-head polls.
    pub finality_poll_delay_ms: u64,
    /// Expected block time used only for health/readiness reporting.
    pub expected_block_time_ms: u64,
}

impl X3NodeTransportConfig {
    pub fn local(rpc_url: String) -> Self {
        Self {
            chain_id: "x3-local".into(),
            rpc_url,
            finality_poll_attempts: 40,
            finality_poll_delay_ms: 500,
            expected_block_time_ms: 6_000,
        }
    }
}

/// Concrete native X3 node transport.
#[derive(Debug)]
pub struct X3NodeTransport<S: X3ExtrinsicSigner> {
    config: X3NodeTransportConfig,
    rpc: Mutex<RpcClient>,
    signer: S,
}

impl<S: X3ExtrinsicSigner> X3NodeTransport<S> {
    pub fn new(config: X3NodeTransportConfig, signer: S) -> Self {
        let rpc = RpcClient::new(config.rpc_url.clone(), 0);
        Self {
            config,
            rpc: Mutex::new(rpc),
            signer,
        }
    }

    fn rpc_lock(&self) -> Result<std::sync::MutexGuard<'_, RpcClient>, SwapError> {
        self.rpc
            .lock()
            .map_err(|_| SwapError::RpcError("X3 RPC mutex poisoned".into()))
    }

    fn require_chain(&self, chain_id: &ChainId) -> Result<(), SwapError> {
        if chain_id != &self.config.chain_id {
            return Err(SwapError::RpcError(alloc::format!(
                "X3 node transport chain mismatch: configured {}, requested {}",
                self.config.chain_id,
                chain_id
            )));
        }
        Ok(())
    }

    fn submit_extrinsic(&self, signed_extrinsic: &str) -> Result<TxId, SwapError> {
        if !signed_extrinsic.starts_with("0x") || signed_extrinsic.len() <= 2 {
            return Err(SwapError::RpcError(
                "signed X3 extrinsic must be non-empty 0x-prefixed SCALE hex".into(),
            ));
        }
        let mut rpc = self.rpc_lock()?;
        let resp = rpc.call(
            "author_submitExtrinsic",
            vec![Value::String(signed_extrinsic.to_string())],
        )?;
        let tx_id = resp
            .result
            .and_then(|v| v.as_str().map(ToString::to_string))
            .ok_or_else(|| SwapError::RpcError("author_submitExtrinsic returned no tx hash".into()))?;
        if tx_id.is_empty() {
            return Err(SwapError::RpcError(
                "author_submitExtrinsic returned empty tx hash".into(),
            ));
        }
        Ok(tx_id)
    }

    fn finalized_head(&self) -> Result<String, SwapError> {
        let mut rpc = self.rpc_lock()?;
        let resp = rpc.call("chain_getFinalizedHead", Vec::new())?;
        resp.result
            .and_then(|v| v.as_str().map(ToString::to_string))
            .ok_or_else(|| SwapError::RpcError("chain_getFinalizedHead returned no hash".into()))
    }

    fn block_and_header(&self, block_hash: &str) -> Result<(Value, Value), SwapError> {
        let mut rpc = self.rpc_lock()?;
        let block = rpc
            .call(
                "chain_getBlock",
                vec![Value::String(block_hash.to_string())],
            )?
            .result
            .ok_or_else(|| SwapError::RpcError("chain_getBlock returned no block".into()))?;
        let header = rpc
            .call(
                "chain_getHeader",
                vec![Value::String(block_hash.to_string())],
            )?
            .result
            .ok_or_else(|| SwapError::RpcError("chain_getHeader returned no header".into()))?;
        Ok((block, header))
    }

    fn parse_hex_u64(value: &Value, field: &str) -> Result<u64, SwapError> {
        let raw = value
            .as_str()
            .ok_or_else(|| SwapError::RpcError(alloc::format!("{} is not a hex string", field)))?;
        let stripped = raw.strip_prefix("0x").unwrap_or(raw);
        u64::from_str_radix(stripped, 16).map_err(|e| {
            SwapError::RpcError(alloc::format!("invalid {} '{}': {}", field, raw, e))
        })
    }

    fn finalized_head_number(&self) -> Result<u64, SwapError> {
        let head = self.finalized_head()?;
        let mut rpc = self.rpc_lock()?;
        let header = rpc
            .call("chain_getHeader", vec![Value::String(head)])?
            .result
            .ok_or_else(|| SwapError::RpcError("chain_getHeader returned no header".into()))?;
        Self::parse_hex_u64(
            header
                .get("number")
                .ok_or_else(|| SwapError::RpcError("finalized header missing number".into()))?,
            "header.number",
        )
    }

    fn block_hash_at(&self, number: u64) -> Result<Option<String>, SwapError> {
        let mut rpc = self.rpc_lock()?;
        let resp = rpc.call(
            "chain_getBlockHash",
            vec![Value::Number(number.into())],
        )?;
        Ok(resp.result.and_then(|v| v.as_str().map(ToString::to_string)))
    }

    fn inclusion_at_block(
        &self,
        tx_id: &TxId,
        signed_extrinsic: &str,
        block_hash: &str,
    ) -> Result<Option<X3FinalizedInclusionProof>, SwapError> {
        let (block, header) = self.block_and_header(block_hash)?;
        let extrinsics = block
            .pointer("/block/extrinsics")
            .and_then(Value::as_array)
            .ok_or_else(|| SwapError::RpcError("chain_getBlock missing block.extrinsics".into()))?;

        let Some(index) = extrinsics
            .iter()
            .position(|x| x.as_str() == Some(signed_extrinsic))
        else {
            return Ok(None);
        };

        let block_number = Self::parse_hex_u64(
            header
                .get("number")
                .ok_or_else(|| SwapError::RpcError("finalized header missing number".into()))?,
            "header.number",
        )?;
        let state_root = header
            .get("stateRoot")
            .and_then(Value::as_str)
            .ok_or_else(|| SwapError::RpcError("finalized header missing stateRoot".into()))?
            .to_string();

        Ok(Some(X3FinalizedInclusionProof {
            tx_id: tx_id.clone(),
            block_hash: block_hash.to_string(),
            block_number,
            state_root,
            extrinsic_index: index as u32,
            signed_extrinsic: signed_extrinsic.to_string(),
        }))
    }

    /// Polls finalized blocks for `signed_extrinsic`, scanning every finalized
    /// block number since the poll started (not just the latest finalized
    /// head) so that a fast-finalizing dev node cannot skip past the block
    /// that actually contains the extrinsic between two polls.
    fn wait_for_finalized_inclusion(
        &self,
        tx_id: &TxId,
        signed_extrinsic: &str,
    ) -> Result<X3FinalizedInclusionProof, SwapError> {
        let mut next_number: Option<u64> = None;
        for attempt in 0..self.config.finality_poll_attempts {
            let head_number = self.finalized_head_number()?;
            let start = next_number.unwrap_or(head_number);
            if head_number >= start {
                for number in start..=head_number {
                    let hash = self.block_hash_at(number)?.ok_or_else(|| {
                        SwapError::RpcError(alloc::format!(
                            "chain_getBlockHash returned no hash for finalized block {}",
                            number
                        ))
                    })?;
                    if let Some(proof) =
                        self.inclusion_at_block(tx_id, signed_extrinsic, &hash)?
                    {
                        return Ok(proof);
                    }
                }
            }
            next_number = Some(head_number + 1);
            if attempt + 1 < self.config.finality_poll_attempts {
                thread::sleep(Duration::from_millis(self.config.finality_poll_delay_ms));
            }
        }
        Err(SwapError::RpcError(alloc::format!(
            "X3 extrinsic {} was not observed in a finalized block after {} polls",
            tx_id,
            self.config.finality_poll_attempts
        )))
    }

    fn encode_inclusion(proof: &X3FinalizedInclusionProof) -> Result<Vec<u8>, SwapError> {
        serde_json::to_vec(proof)
            .map_err(|e| SwapError::RpcError(alloc::format!("encode X3 inclusion proof: {}", e)))
    }

    fn decode_inclusion(raw: &[u8]) -> Result<X3FinalizedInclusionProof, SwapError> {
        serde_json::from_slice(raw)
            .map_err(|e| SwapError::RpcError(alloc::format!("decode X3 inclusion proof: {}", e)))
    }

    fn verify_inclusion(&self, tx_id: &str, block_hash: &str, raw: &[u8]) -> Result<bool, SwapError> {
        let proof = Self::decode_inclusion(raw)?;
        if proof.tx_id != tx_id || proof.block_hash != block_hash {
            return Ok(false);
        }
        // Always re-read the recorded finalized block. A proof must remain bound
        // to the exact block, height, state root and extrinsic index even when
        // that block is also the node's current finalized head.
        let (block, header) = self.block_and_header(&proof.block_hash)?;
        let extrinsics = block
            .pointer("/block/extrinsics")
            .and_then(Value::as_array)
            .ok_or_else(|| SwapError::RpcError("proof block missing extrinsics".into()))?;
        if extrinsics
            .get(proof.extrinsic_index as usize)
            .and_then(Value::as_str)
            != Some(proof.signed_extrinsic.as_str())
        {
            return Ok(false);
        }
        if header.get("stateRoot").and_then(Value::as_str) != Some(proof.state_root.as_str()) {
            return Ok(false);
        }
        let recorded_number = Self::parse_hex_u64(
            header
                .get("number")
                .ok_or_else(|| SwapError::RpcError("proof header missing number".into()))?,
            "proof header.number",
        )?;
        if recorded_number != proof.block_number {
            return Ok(false);
        }

        // Finally prove that the recorded block is no newer than the current
        // GRANDPA-finalized head. Inclusion in a non-finalized fork is never
        // sufficient.
        let finalized_number = self.finalized_head_number()?;
        Ok(proof.block_number <= finalized_number)
    }

    fn latest_header_numbers(&self) -> Result<(u64, u64), SwapError> {
        let finalized_hash = self.finalized_head()?;
        let (_, finalized_header) = self.block_and_header(&finalized_hash)?;
        let finalized = Self::parse_hex_u64(
            finalized_header
                .get("number")
                .ok_or_else(|| SwapError::RpcError("finalized header missing number".into()))?,
            "finalized number",
        )?;

        let mut rpc = self.rpc_lock()?;
        let latest_header = rpc
            .call("chain_getHeader", Vec::new())?
            .result
            .ok_or_else(|| SwapError::RpcError("latest chain_getHeader returned no header".into()))?;
        let latest = Self::parse_hex_u64(
            latest_header
                .get("number")
                .ok_or_else(|| SwapError::RpcError("latest header missing number".into()))?,
            "latest number",
        )?;
        Ok((latest, finalized))
    }
}

impl<S: X3ExtrinsicSigner> X3VmLiveTransport for X3NodeTransport<S> {
    fn lock(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent: &AtomicIntent,
    ) -> Result<LockProof, SwapError> {
        self.require_chain(chain_id)?;
        let signed = self
            .signer
            .sign_lock_escrow(chain_id, escrow_address, intent)?;
        let tx_id = self.submit_extrinsic(&signed)?;
        let inclusion = self.wait_for_finalized_inclusion(&tx_id, &signed)?;
        Ok(LockProof {
            tx_id,
            chain_id: chain_id.clone(),
            vm_type: VmType::X3Vm,
            block_number: inclusion.block_number,
            block_hash: inclusion.block_hash.clone(),
            confirmations: 1,
            lock_address: String::from_utf8(escrow_address.to_vec())
                .unwrap_or_else(|_| alloc::format!("0x{}", hex::encode(escrow_address))),
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver: intent.receiver.as_bytes().to_vec(),
            refund_address: intent.refund_path.address.as_bytes().to_vec(),
            timeout: intent.source_timeout,
            raw_proof: Self::encode_inclusion(&inclusion)?,
        })
    }

    fn claim(
        &self,
        chain_id: &ChainId,
        _escrow_address: &[u8],
        intent_id: IntentId,
        preimage: [u8; 32],
    ) -> Result<ClaimProof, SwapError> {
        self.require_chain(chain_id)?;
        let signed = self
            .signer
            .sign_claim_settlement(chain_id, intent_id, preimage)?;
        let tx_id = self.submit_extrinsic(&signed)?;
        let inclusion = self.wait_for_finalized_inclusion(&tx_id, &signed)?;
        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id: chain_id.clone(),
            vm_type: VmType::X3Vm,
            preimage,
            block_number: inclusion.block_number,
            block_hash: inclusion.block_hash.clone(),
            raw_proof: Self::encode_inclusion(&inclusion)?,
        })
    }

    fn refund(
        &self,
        chain_id: &ChainId,
        _escrow_address: &[u8],
        intent_id: IntentId,
    ) -> Result<RefundProof, SwapError> {
        self.require_chain(chain_id)?;
        let signed = self.signer.sign_refund_settlement(chain_id, intent_id)?;
        let tx_id = self.submit_extrinsic(&signed)?;
        let inclusion = self.wait_for_finalized_inclusion(&tx_id, &signed)?;
        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id: chain_id.clone(),
            vm_type: VmType::X3Vm,
            block_number: inclusion.block_number,
            block_hash: inclusion.block_hash.clone(),
            raw_proof: Self::encode_inclusion(&inclusion)?,
        })
    }

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        self.verify_inclusion(&proof.tx_id, &proof.block_hash, &proof.raw_proof)
    }

    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError> {
        self.verify_inclusion(&proof.tx_id, &proof.block_hash, &proof.raw_proof)
    }

    fn verify_refund(&self, proof: &RefundProof) -> Result<bool, SwapError> {
        self.verify_inclusion(&proof.tx_id, &proof.block_hash, &proof.raw_proof)
    }

    fn estimate_fee(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent: &AtomicIntent,
    ) -> Result<FeeEstimate, SwapError> {
        self.require_chain(chain_id)?;
        // Fee estimation must query the exact lock payload that would be
        // submitted, including the real escrow address.
        let signed = self
            .signer
            .sign_lock_escrow(chain_id, escrow_address, intent)?;
        let mut rpc = self.rpc_lock()?;
        let info = rpc
            .call(
                "payment_queryInfo",
                vec![Value::String(signed), Value::Null],
            )?
            .result
            .ok_or_else(|| SwapError::RpcError("payment_queryInfo returned no result".into()))?;
        let partial_fee = info
            .get("partialFee")
            .and_then(Value::as_str)
            .ok_or_else(|| SwapError::RpcError("payment_queryInfo missing partialFee".into()))?;
        let native_fee = partial_fee.parse::<u128>().map_err(|e| {
            SwapError::RpcError(alloc::format!("invalid partialFee '{}': {}", partial_fee, e))
        })?;
        let weight = info
            .pointer("/weight/refTime")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(FeeEstimate {
            chain_id: chain_id.clone(),
            vm_type: VmType::X3Vm,
            native_fee,
            gas_units: weight,
            gas_price: 0,
            estimated_usd: 0.0,
        })
    }

    fn finality_status(
        &self,
        chain_id: &ChainId,
        tx_id: &TxId,
    ) -> Result<FinalityProof, SwapError> {
        self.require_chain(chain_id)?;
        let head = self.finalized_head()?;
        let (_, header) = self.block_and_header(&head)?;
        let block_number = Self::parse_hex_u64(
            header
                .get("number")
                .ok_or_else(|| SwapError::RpcError("finalized header missing number".into()))?,
            "header.number",
        )?;
        Ok(FinalityProof {
            chain_id: chain_id.clone(),
            vm_type: VmType::X3Vm,
            tx_id: tx_id.clone(),
            block_number,
            block_hash: head,
            confirmations: 1,
            finalized: true,
            finality_source: "GRANDPA finalized head".into(),
            safe_to_reveal_secret: true,
        })
    }

    fn chain_health(&self, chain_id: &ChainId) -> Result<ChainHealth, SwapError> {
        self.require_chain(chain_id)?;
        let (latest, finalized) = self.latest_header_numbers()?;
        let mut rpc = self.rpc_lock()?;
        let health = rpc.call("system_health", Vec::new())?.result.unwrap_or_else(|| json!({}));
        let syncing = health
            .get("isSyncing")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let peers = health
            .get("peers")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let lag = latest.saturating_sub(finalized);
        let degraded = syncing || lag > 8;
        Ok(ChainHealth {
            chain_id: chain_id.clone(),
            vm_type: VmType::X3Vm,
            latest_block: latest,
            finalized_block: finalized,
            block_delay_ms: self.config.expected_block_time_ms,
            finality_delay_ms: lag.saturating_mul(self.config.expected_block_time_ms),
            rpc_quorum_healthy: peers > 0 && !syncing,
            gas_price: 0,
            halted: false,
            degraded,
            safe_for_new_intents: !degraded,
        })
    }

    fn readiness_score(&self) -> AdapterReadinessScore {
        AdapterReadinessScore {
            adapter_name: "x3-node-native",
            vm_type: VmType::X3Vm,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: false,
            finality_proof: true,
            rpc_indexer_support: true,
            timeout_safety: true,
            tests_implemented: false,
            proof_ledger_integration: false,
            ibc_support: false,
            cross_adapter_atomicity_test: false,
        }
    }
}
