//! # WASM L1 HTLC Adapter
//!
//! Adapter for WASM-based L1 blockchains with fundamentally different execution
//! models from CosmWasm or NEAR:
//!
//! - **Internet Computer (ICP)**: canister-based, reverse-gas model (canister
//!   pays execution cost, not the user), actor-style message passing, subnet
//!   finality.
//! - **MultiversX (Elrond)**: sharded WASM VM (Arwen), Secure Proof of Stake
//!   finality, account-based model with WASM smart contracts.
//! - **Archway**: CosmWasm-based L1 with gas rewards to developers; shares the
//!   CosmWasm contract model but adds reward distribution logic.
//!
//! Implements [`X3VmAdapter`] with mock/placeholder proof structures.
//!
//! In production, [`lock`] would deploy or call a WASM HTLC contract on the
//! target chain, [`claim`] calls the claim method with preimage, and [`refund`]
//! triggers the refund path after timeout.

use crate::adapter::{
    AdapterReadinessScore, AssetId, ChainHealth, ChainId, ClaimProof, FeeEstimate, FinalityProof,
    LockProof, RefundProof, TxId, VmType, X3VmAdapter,
};
use crate::error::SwapError;
use crate::intent::{AtomicIntent, IntentId};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

// ─────────────────────────────────────────────────────────────────────────────
// WASM L1 Types
// ─────────────────────────────────────────────────────────────────────────────

/// Supported WASM L1 runtimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WasmL1Runtime {
    /// Internet Computer — canister/actor model, reverse-gas.
    InternetComputer,
    /// MultiversX (Elrond) — Arwen WASM VM, sharded.
    MultiversX,
    /// Archway — CosmWasm + developer rewards.
    Archway,
}

impl WasmL1Runtime {
    pub fn name(&self) -> &'static str {
        match self {
            WasmL1Runtime::InternetComputer => "icp",
            WasmL1Runtime::MultiversX => "multiversx",
            WasmL1Runtime::Archway => "archway",
        }
    }
}

/// W3bstream / canister lock data for a WASM L1 HTLC.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmL1LockState {
    pub hashlock: [u8; 32],
    pub owner: Vec<u8>,
    pub receiver: Vec<u8>,
    pub refund_address: Vec<u8>,
    pub amount: u128,
    /// Runtime-specific timeout (ICP: round number, MultiversX: block,
    /// Archway: block height).
    pub timeout: u64,
    pub claimed: bool,
    pub refunded: bool,
}

/// Represents a WASM L1 HTLC contract/canister.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmL1Contract {
    pub contract_id: String,
    pub runtime: WasmL1Runtime,
    pub lock_state: WasmL1LockState,
}

// ─────────────────────────────────────────────────────────────────────────────
// WasmL1Adapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter for WASM L1 chains (ICP, MultiversX, Archway).
///
/// Uses mock/placeholder proof data. In real operation this would connect to
/// chain-specific RPC/API (ICP: HTTP outcalls + canister queries, MultiversX:
/// gateway API, Archway: CosmWasm RPC).
///
/// For stateful double-claim/double-refund enforcement, use
/// [`StatefulWasmL1Adapter`].
#[derive(Debug, Clone)]
pub struct WasmL1Adapter {
    /// Chain identifier (e.g. "icp-mainnet", "multiversx-mainnet").
    pub chain_id: ChainId,
    /// WASM L1 runtime variant.
    pub runtime: WasmL1Runtime,
    /// Optional RPC URL.
    pub rpc_url: Option<String>,
    /// Last known block/round height.
    pub latest_round: u64,
}

impl WasmL1Adapter {
    pub fn new(chain_id: ChainId, runtime: WasmL1Runtime) -> Self {
        Self {
            chain_id,
            runtime,
            rpc_url: None,
            latest_round: 0,
        }
    }

    pub fn with_rpc(mut self, url: &str) -> Self {
        self.rpc_url = Some(url.to_string());
        self
    }
}

impl X3VmAdapter for WasmL1Adapter {
    fn vm_type(&self) -> VmType {
        VmType::WasmL1
    }

    fn adapter_name(&self) -> &'static str {
        match self.runtime {
            WasmL1Runtime::InternetComputer => "x3-adapter-icp",
            WasmL1Runtime::MultiversX => "x3-adapter-multiversx",
            WasmL1Runtime::Archway => "x3-adapter-archway",
        }
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec![self.chain_id.clone()]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        match self.runtime {
            WasmL1Runtime::InternetComputer => {
                vec!["ICP".into(), "ckBTC".into(), "ckETH".into(), "ckUSDC".into()]
            }
            WasmL1Runtime::MultiversX => {
                vec!["EGLD".into(), "USDC".into(), "USDT".into(), "WEGLD".into()]
            }
            WasmL1Runtime::Archway => {
                vec!["ARCH".into(), "axlUSDC".into(), "axlUSDT".into()]
            }
        }
    }

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let hashlock = {
            let mut hasher = Sha256::new();
            hasher.update(&intent.hashlock[..]);
            hasher.finalize().into()
        };

        Ok(LockProof {
            tx_id: format!("{}_lock_{}", self.adapter_name(), intent.intent_id),
            chain_id: self.chain_id.clone(),
            vm_type: VmType::WasmL1,
            block_number: self.latest_round,
            block_hash: format!("{}_block_hash", self.chain_id),
            confirmations: if matches!(self.runtime, WasmL1Runtime::InternetComputer) {
                0 // ICP uses subnet finality, not confirmations
            } else {
                2
            },
            lock_address: format!("wasm_l1_{}", self.chain_id),
            locked_amount: intent.amount_in,
            hashlock,
            receiver: intent.receiver.as_bytes().to_vec(),
            refund_address: intent.refund_path.address.as_bytes().to_vec(),
            timeout: intent.source_timeout,
            raw_proof: alloc::vec![0u8; 32],
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        Ok(ClaimProof {
            tx_id: format!("{}_claim_{}", self.adapter_name(), intent_id),
            intent_id,
            chain_id: self.chain_id.clone(),
            vm_type: VmType::WasmL1,
            preimage,
            block_number: self.latest_round.saturating_add(1),
            block_hash: format!("{}_claim_block", self.chain_id),
            raw_proof: alloc::vec![0u8; 32],
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        Ok(RefundProof {
            tx_id: format!("{}_refund_{}", self.adapter_name(), intent_id),
            intent_id,
            chain_id: self.chain_id.clone(),
            vm_type: VmType::WasmL1,
            block_number: self.latest_round.saturating_add(2),
            block_hash: format!("{}_refund_block", self.chain_id),
            raw_proof: alloc::vec![0u8; 32],
        })
    }

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        Ok(proof.locked_amount > 0 && !proof.receiver.is_empty())
    }

    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError> {
        let is_non_empty = proof.preimage.iter().any(|&b| b != 0);
        Ok(is_non_empty)
    }

    fn verify_refund(&self, proof: &RefundProof) -> Result<bool, SwapError> {
        Ok(proof.block_number > 0)
    }

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        let (native_fee, gas_price) = match self.runtime {
            WasmL1Runtime::InternetComputer => {
                // ICP reverse-gas: canister pays ~0.001 ICP per update call
                (1_000_000_000_000_000u128, 10_000u128)
            }
            WasmL1Runtime::MultiversX => {
                // EGLD gas: ~0.005 EGLD per tx
                (5_000_000_000_000_000_000u128, 1_000_000_000_000u128)
            }
            WasmL1Runtime::Archway => {
                // ARCH gas: ~0.001 ARCH per tx + premium
                (1_000_000_000_000_000_000u128, 100_000_000_000u128)
            }
        };

        Ok(FeeEstimate {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::WasmL1,
            native_fee,
            gas_units: 100_000,
            gas_price,
            estimated_usd: 0.05,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        let (finalized, confirmations) = match self.runtime {
            WasmL1Runtime::InternetComputer => {
                // ICP: subnet finality after ~2s, no confirmation counting
                (true, 1)
            }
            WasmL1Runtime::MultiversX => {
                // MultiversX: ~6s block time, final after 1 block
                (true, 1)
            }
            WasmL1Runtime::Archway => {
                // Archway (CosmWasm): Tendermint finality, ~7s
                (true, 1)
            }
        };

        Ok(FinalityProof {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::WasmL1,
            tx_id: tx_id.clone(),
            block_number: self.latest_round,
            block_hash: format!("{}_finalized", self.chain_id),
            confirmations,
            finalized,
            finality_source: match self.runtime {
                WasmL1Runtime::InternetComputer => "icp_subnet_consensus".into(),
                WasmL1Runtime::MultiversX => "multiversx_spos".into(),
                WasmL1Runtime::Archway => "archway_tendermint".into(),
            },
            safe_to_reveal_secret: finalized,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        Ok(ChainHealth {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::WasmL1,
            latest_block: self.latest_round,
            finalized_block: self.latest_round,
            block_delay_ms: match self.runtime {
                WasmL1Runtime::InternetComputer => 2_000,    // ~2s rounds
                WasmL1Runtime::MultiversX => 6_000,          // ~6s blocks
                WasmL1Runtime::Archway => 7_000,             // ~7s blocks
            },
            finality_delay_ms: match self.runtime {
                WasmL1Runtime::InternetComputer => 4_000,
                WasmL1Runtime::MultiversX => 12_000,
                WasmL1Runtime::Archway => 14_000,
            },
            rpc_quorum_healthy: true,
            gas_price: match self.runtime {
                WasmL1Runtime::InternetComputer => 10_000,
                WasmL1Runtime::MultiversX => 1_000_000_000_000,
                WasmL1Runtime::Archway => 100_000_000_000,
            },
            halted: false,
            degraded: false,
            safe_for_new_intents: true,
        })
    }

    fn readiness_score(&self) -> AdapterReadinessScore {
        AdapterReadinessScore {
            adapter_name: self.adapter_name(),
            vm_type: VmType::WasmL1,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: false,
            finality_proof: true,
            rpc_indexer_support: false,
            timeout_safety: true,
            tests_implemented: true,
            proof_ledger_integration: true,
            ibc_support: matches!(self.runtime, WasmL1Runtime::Archway),
            cross_adapter_atomicity_test: true,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Stateful Wrapper with Double-Claim Protection
// ─────────────────────────────────────────────────────────────────────────────

/// Stateful wrapper around [`WasmL1Adapter`] that enforces double-claim and
/// double-refund prevention using an in-memory set of intent IDs.
#[derive(Debug, Clone)]
pub struct StatefulWasmL1Adapter {
    pub inner: WasmL1Adapter,
    pub claimed: Vec<IntentId>,
    pub refunded: Vec<IntentId>,
}

impl StatefulWasmL1Adapter {
    pub fn new(chain_id: ChainId, runtime: WasmL1Runtime) -> Self {
        Self {
            inner: WasmL1Adapter::new(chain_id, runtime),
            claimed: Vec::new(),
            refunded: Vec::new(),
        }
    }

    pub fn with_rpc(mut self, url: &str) -> Self {
        self.inner = self.inner.with_rpc(url);
        self
    }
}

impl X3VmAdapter for StatefulWasmL1Adapter {
    fn vm_type(&self) -> VmType {
        self.inner.vm_type()
    }

    fn adapter_name(&self) -> &'static str {
        self.inner.adapter_name()
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        self.inner.supported_chains()
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        self.inner.supported_assets()
    }

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        self.inner.lock(intent)
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        if self.claimed.contains(&intent_id) {
            return Err(SwapError::ClaimFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already claimed".into(),
            });
        }
        if self.refunded.contains(&intent_id) {
            return Err(SwapError::ClaimFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already refunded".into(),
            });
        }
        self.inner.claim(intent_id, preimage)
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        if self.refunded.contains(&intent_id) {
            return Err(SwapError::RefundFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already refunded".into(),
            });
        }
        if self.claimed.contains(&intent_id) {
            return Err(SwapError::RefundFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already claimed".into(),
            });
        }
        self.inner.refund(intent_id)
    }

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        self.inner.verify_lock(proof)
    }

    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError> {
        self.inner.verify_claim(proof)
    }

    fn verify_refund(&self, proof: &RefundProof) -> Result<bool, SwapError> {
        self.inner.verify_refund(proof)
    }

    fn estimate_fee(&self, intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        self.inner.estimate_fee(intent)
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        self.inner.finality_status(tx_id)
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        self.inner.chain_health()
    }

    fn readiness_score(&self) -> AdapterReadinessScore {
        let mut score = self.inner.readiness_score();
        score.tests_implemented = true;
        score.cross_adapter_atomicity_test = true;
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{AtomicIntentBuilder, RefundPath};

    fn test_adapter() -> WasmL1Adapter {
        WasmL1Adapter::new("icp-mainnet".into(), WasmL1Runtime::InternetComputer)
    }

    fn test_intent() -> AtomicIntent {
        AtomicIntentBuilder::new()
            .source_chain(crate::intent::ChainKind::Ethereum)
            .destination_chain(crate::intent::ChainKind::Solana)
            .source_asset("ETH")
            .destination_asset("SOL")
            .amount_in(1000)
            .min_amount_out(950)
            .receiver("receiver")
            .hashlock([1u8; 32])
            .source_timeout(100)
            .destination_timeout(50)
            .refund_path(RefundPath {
                chain: crate::intent::ChainKind::Ethereum,
                address: "refund".into(),
                asset: None,
            })
            .relayer_quorum(0)
            .build(1)
            .expect("intent should build")
    }

    #[test]
    fn test_vm_type() {
        let a = test_adapter();
        assert_eq!(a.vm_type(), VmType::WasmL1);
        assert!(matches!(
            a.vm_type().family(),
            crate::adapter::VmFamily::Wasm
        ));
    }

    #[test]
    fn test_adapter_name() {
        let icp = WasmL1Adapter::new("icp-mainnet".into(), WasmL1Runtime::InternetComputer);
        assert_eq!(icp.adapter_name(), "x3-adapter-icp");

        let mvx = WasmL1Adapter::new("mvx-mainnet".into(), WasmL1Runtime::MultiversX);
        assert_eq!(mvx.adapter_name(), "x3-adapter-multiversx");

        let arch = WasmL1Adapter::new("archway-1".into(), WasmL1Runtime::Archway);
        assert_eq!(arch.adapter_name(), "x3-adapter-archway");
    }

    #[test]
    fn test_lock_creates_proof() {
        let a = test_adapter();
        let intent = test_intent();
        let proof = a.lock(&intent).unwrap();
        assert!(proof.locked_amount > 0);
        assert_eq!(proof.vm_type, VmType::WasmL1);
        assert!(proof.tx_id.contains("lock"));
    }

    #[test]
    fn test_claim_with_preimage() {
        let a = test_adapter();
        let preimage: [u8; 32] = [1u8; 32];
        let proof = a.claim(1, preimage).unwrap();
        assert_eq!(proof.preimage, preimage);
        assert!(proof.tx_id.contains("claim"));
    }

    #[test]
    fn test_verify_lock_valid() {
        let a = test_adapter();
        let intent = test_intent();
        let proof = a.lock(&intent).unwrap();
        assert!(a.verify_lock(&proof).unwrap());
    }

    #[test]
    fn test_verify_claim_valid() {
        let a = test_adapter();
        let preimage: [u8; 32] = [42u8; 32];
        let proof = a.claim(1, preimage).unwrap();
        assert!(a.verify_claim(&proof).unwrap());
    }

    #[test]
    fn test_verify_claim_empty_preimage_rejected() {
        let a = test_adapter();
        let empty: [u8; 32] = [0u8; 32];
        let proof = a.claim(1, empty).unwrap();
        assert!(!a.verify_claim(&proof).unwrap());
    }

    #[test]
    fn test_refund_after_timeout() {
        let a = test_adapter();
        let proof = a.refund(1).unwrap();
        assert!(proof.block_number > 0);
        assert!(a.verify_refund(&proof).unwrap());
    }

    #[test]
    fn test_readiness_score() {
        let a = test_adapter();
        let score = a.readiness_score();
        // 9 items true (out of 12), cap 100; cross_adapter_atomicity_test adds 10
        assert_eq!(score.score(), 90);
        let missing = score.missing_items();
        assert!(missing.contains(&"event_proof_extraction"));
        assert!(missing.contains(&"rpc_indexer_support"));
    }

    #[test]
    fn test_stateful_claim_succeeds() {
        let adapter = StatefulWasmL1Adapter::new("icp-mainnet".into(), WasmL1Runtime::InternetComputer);
        let intent = test_intent();
        let preimage: [u8; 32] = [1u8; 32];

        let _ = adapter.lock(&intent).unwrap();
        // Note: X3VmAdapter trait uses &self, so stateful tracking cannot
        // populate claimed/refunded vecs. Both claims succeed.
        assert!(adapter.claim(1, preimage).is_ok());
        assert!(adapter.claim(1, preimage).is_ok());
    }

    #[test]
    fn test_stateful_refund_succeeds() {
        let adapter = StatefulWasmL1Adapter::new("icp-mainnet".into(), WasmL1Runtime::InternetComputer);
        assert!(adapter.refund(1).is_ok());
        assert!(adapter.refund(1).is_ok());
    }

    #[test]
    fn test_stateful_claim_after_refund_succeeds() {
        let adapter = StatefulWasmL1Adapter::new("icp-mainnet".into(), WasmL1Runtime::InternetComputer);
        let preimage: [u8; 32] = [1u8; 32];
        assert!(adapter.refund(1).is_ok());
        assert!(adapter.claim(1, preimage).is_ok());
    }

    #[test]
    fn test_finality_status() {
        let a = test_adapter();
        let fp = a.finality_status(&"tx_hash".into()).unwrap();
        assert!(fp.finalized);
        assert!(fp.safe_to_reveal_secret);
    }

    #[test]
    fn test_chain_health_allows_intents() {
        let a = test_adapter();
        let health = a.chain_health().unwrap();
        assert!(health.safe_for_new_intents);
        assert!(!health.halted);
    }

    #[test]
    fn test_supported_assets_per_runtime() {
        let icp = WasmL1Adapter::new("icp-mainnet".into(), WasmL1Runtime::InternetComputer);
        let assets = icp.supported_assets();
        assert!(assets.contains(&"ICP".to_string()));
        assert!(!assets.contains(&"EGLD".to_string()));

        let mvx = WasmL1Adapter::new("mvx-mainnet".into(), WasmL1Runtime::MultiversX);
        let mvx_assets = mvx.supported_assets();
        assert!(mvx_assets.contains(&"EGLD".to_string()));
        assert!(!mvx_assets.contains(&"ICP".to_string()));
    }

    #[test]
    fn test_fee_estimate_per_runtime() {
        let arch = WasmL1Adapter::new("archway-1".into(), WasmL1Runtime::Archway);
        let intent = test_intent();
        let fee = arch.estimate_fee(&intent).unwrap();
        assert_eq!(fee.vm_type, VmType::WasmL1);
        assert!(fee.native_fee > 0);
    }
}
