//! # CairoVM HTLC Adapter
//!
//! Adapter for CairoVM chains (StarkNet). Implements [`X3VmAdapter`] with
//! mock/placeholder proof structures.
//!
//! In production, [`lock`] would deploy a Cairo HTLC contract with felt252-compatible
//! addresses, [`claim`] would call the claim function, and [`refund`] would trigger
//! the refund path after timeout. Finality requires L1 settlement proof for full
//! finality on StarkNet.

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
// Cairo Contract Types
// ─────────────────────────────────────────────────────────────────────────────

/// Cairo HTLC contract representation using felt252-compatible values.
///
/// In Cairo/StarkNet, addresses and field elements are 252-bit numbers
/// represented as bytes or hex strings. Here we use raw byte representations
/// for simplicity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CairoHtlcContract {
    /// Hashlock encoded as felt252-compatible bytes.
    pub felt_hashlock: [u8; 32],
    /// Receiver address as felt252-compatible bytes.
    pub felt_receiver: Vec<u8>,
    /// Refund address as felt252-compatible bytes.
    pub felt_refund_address: Vec<u8>,
    /// The on-chain contract address (StarkNet contract).
    pub contract_address: String,
    /// Asset identifier.
    pub asset: AssetId,
    /// Locked amount in smallest unit (wei/fri).
    pub amount: u128,
    /// Timeout block number.
    pub timeout_block: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// CairoVmAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter for CairoVM chains (StarkNet).
///
/// Uses mock/placeholder proof data. In real operation this would connect to a
/// StarkNet node via RPC and interact with Cairo HTLC contracts.
///
/// For stateful double-claim/double-refund enforcement, use
/// [`StatefulCairoVmAdapter`].
#[derive(Debug, Clone)]
pub struct CairoVmAdapter {
    /// Chain identifier (e.g. "starknet-mainnet", "starknet-testnet").
    pub chain_id: ChainId,
    /// Optional RPC URL for StarkNet node.
    pub rpc_url: Option<String>,
    /// Current finalized block (L2 block, needs L1 settlement for full finality).
    pub last_finalized_block: u64,
    /// Tracked claimed intent IDs.
    pub claimed_intents: Vec<u64>,
    /// Tracked refunded intent IDs.
    pub refunded_intents: Vec<u64>,
}

/// Internal lock state tracked by the stateful adapter.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct InternalLock {
    intent_id: IntentId,
    hashlock: [u8; 32],
    receiver: Vec<u8>,
    refund_address: Vec<u8>,
    timeout: u64,
    tx_id: TxId,
    block_number: u64,
    claimed: bool,
    refunded: bool,
}

impl CairoVmAdapter {
    /// Create a new adapter for the given chain identifier.
    ///
    /// Example chain IDs: `"starknet-mainnet"`, `"starknet-testnet"`, `"starknet-sepolia"`.
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            chain_id,
            rpc_url: None,
            last_finalized_block: 0,
            claimed_intents: Vec::new(),
            refunded_intents: Vec::new(),
        }
    }

    /// Set the RPC URL.
    pub fn set_rpc(&mut self, rpc_url: &str) {
        self.rpc_url = Some(rpc_url.to_string());
    }

    /// Generate a deterministic mock tx_id from intent_id and a label byte.
    fn mock_tx_id(intent_id: IntentId, label: u8) -> TxId {
        let mut hasher = Sha256::new();
        hasher.update(intent_id.to_le_bytes());
        hasher.update([label]);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Generate a mock Cairo contract address (felt252 format).
    fn mock_cairo_address(intent_id: IntentId) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"x3-cairo-htlc:");
        hasher.update(intent_id.to_le_bytes());
        let result = hasher.finalize();
        // StarkNet addresses are 32-byte felt252 values; use first 31 bytes (felt252 range)
        format!("0x{}", hex::encode(&result[..31]))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl X3VmAdapter for CairoVmAdapter {
    fn vm_type(&self) -> VmType {
        VmType::CairoVm
    }

    fn adapter_name(&self) -> &'static str {
        "x3-adapter-cairo"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec![
            "starknet-mainnet".into(),
            "starknet-testnet".into(),
            "starknet-sepolia".into(),
        ]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        vec!["ETH".into(), "STRK".into(), "USDC".into()]
    }

    // ── Lifecycle operations ──────────────────────────────────────────────

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
        let lock_address = Self::mock_cairo_address(intent.intent_id);
        let block_number = self.last_finalized_block + 1;

        let receiver = intent.receiver.as_bytes().to_vec();
        let refund_address = intent.refund_path.address.as_bytes().to_vec();

        Ok(LockProof {
            tx_id,
            chain_id,
            vm_type: VmType::CairoVm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            confirmations: 0,
            lock_address,
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver,
            refund_address,
            timeout: intent.source_timeout,
            raw_proof: vec![0x63, 0x61, 0x69, 0x01], // "cai\x01" - mock proof
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x02);
        let block_number = self.last_finalized_block + 2;

        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::CairoVm,
            preimage,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x63, 0x61, 0x69, 0x02], // "cai\x02" - mock proof
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x03);
        let block_number = self.last_finalized_block + 3;

        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::CairoVm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x63, 0x61, 0x69, 0x03], // "cai\x03" - mock proof
        })
    }

    // ── Verification ──────────────────────────────────────────────────────

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::CairoVm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        if proof.lock_address.is_empty() {
            return Ok(false);
        }
        // Cairo addresses must start with 0x (felt252 hex format)
        if !proof.lock_address.starts_with("0x") {
            return Ok(false);
        }
        if proof.locked_amount == 0 {
            return Ok(false);
        }
        if proof.timeout == 0 {
            return Ok(false);
        }
        Ok(true)
    }

    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::CairoVm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        if proof.preimage == [0u8; 32] {
            return Ok(false);
        }
        Ok(true)
    }

    fn verify_refund(&self, proof: &RefundProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::CairoVm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    // ── Estimation & Health ───────────────────────────────────────────────

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        // StarkNet L2 fee ~0.0005 ETH equivalent (500_000_000_000 wei)
        Ok(FeeEstimate {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::CairoVm,
            native_fee: 500_000_000_000, // 0.0005 ETH in wei
            gas_units: 0,
            gas_price: 0,
            estimated_usd: 0.01,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        // StarkNet: L2 finality requires L1 settlement proof for full finality.
        // Here we simulate depending on last_finalized_block >= 1.
        let finalized = self.last_finalized_block >= 1;
        // L1 settlement proof is not available in mock mode
        let safe_to_reveal = finalized && self.last_finalized_block >= 10;

        Ok(FinalityProof {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::CairoVm,
            tx_id: tx_id.clone(),
            block_number: self.last_finalized_block,
            block_hash: hex::encode(Sha256::digest(self.last_finalized_block.to_le_bytes())),
            confirmations: self.last_finalized_block,
            finalized,
            finality_source: "starknet-l2-settlement".into(),
            safe_to_reveal_secret: safe_to_reveal,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        Ok(ChainHealth {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::CairoVm,
            latest_block: self.last_finalized_block,
            finalized_block: self.last_finalized_block,
            block_delay_ms: 2_000,     // ~2s block time (StarkNet)
            finality_delay_ms: 60_000, // L1 settlement ~1 min
            rpc_quorum_healthy: true,
            gas_price: 0,
            halted: false,
            degraded: false,
            safe_for_new_intents: true,
        })
    }

    // ── Readiness ─────────────────────────────────────────────────────────

    fn readiness_score(&self) -> AdapterReadinessScore {
        AdapterReadinessScore {
            adapter_name: "x3-adapter-cairo",
            vm_type: VmType::CairoVm,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: false, // needs Cairo event extraction
            finality_proof: true,
            rpc_indexer_support: false, // needs StarkNet indexer integration
            timeout_safety: true,
            tests_implemented: true,
            proof_ledger_integration: false, // proof ledger not fully wired
            ibc_support: false,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Stateful Wrapper with Double-Claim Protection
// ─────────────────────────────────────────────────────────────────────────────

/// A stateful wrapper around [`CairoVmAdapter`] that tracks lock state
/// in order to reject double claims and double refunds.
#[derive(Debug, Clone)]
pub struct StatefulCairoVmAdapter {
    pub inner: CairoVmAdapter,
    locks: Vec<InternalLock>,
}

impl StatefulCairoVmAdapter {
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            inner: CairoVmAdapter::new(chain_id),
            locks: Vec::new(),
        }
    }

    pub fn set_rpc(&mut self, rpc_url: &str) {
        self.inner.set_rpc(rpc_url);
    }

    /// Lock funds and record the lock state internally.
    pub fn lock(&mut self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        if self.locks.iter().any(|l| l.intent_id == intent.intent_id) {
            return Err(SwapError::AlreadyLocked {
                chain: intent.source_chain,
            });
        }

        let proof = self.inner.lock(intent)?;

        self.locks.push(InternalLock {
            intent_id: intent.intent_id,
            hashlock: intent.hashlock,
            receiver: intent.receiver.as_bytes().to_vec(),
            refund_address: intent.refund_path.address.as_bytes().to_vec(),
            timeout: intent.source_timeout,
            tx_id: proof.tx_id.clone(),
            block_number: proof.block_number,
            claimed: false,
            refunded: false,
        });

        Ok(proof)
    }

    /// Claim with preimage, enforcing no double-claim.
    pub fn claim(
        &mut self,
        intent_id: IntentId,
        preimage: [u8; 32],
    ) -> Result<ClaimProof, SwapError> {
        let lock = self
            .locks
            .iter_mut()
            .find(|l| l.intent_id == intent_id)
            .ok_or_else(|| SwapError::ClaimFailed {
                chain: self.inner.chain_id.clone(),
                reason: "no lock record found for this intent".into(),
            })?;

        if lock.claimed {
            return Err(SwapError::ClaimFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already claimed".into(),
            });
        }

        if lock.refunded {
            return Err(SwapError::ClaimFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already refunded".into(),
            });
        }

        // Verify preimage matches hashlock.
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let result = hasher.finalize();
        let mut computed = [0u8; 32];
        computed.copy_from_slice(&result);
        if computed != lock.hashlock {
            return Err(SwapError::ClaimFailed {
                chain: self.inner.chain_id.clone(),
                reason: "hashlock mismatch: preimage does not match hashlock".into(),
            });
        }

        let proof = self.inner.claim(intent_id, preimage)?;
        lock.claimed = true;
        Ok(proof)
    }

    /// Refund after timeout, enforcing no double-refund.
    pub fn refund(
        &mut self,
        intent_id: IntentId,
        current_time: u64,
    ) -> Result<RefundProof, SwapError> {
        let lock = self
            .locks
            .iter_mut()
            .find(|l| l.intent_id == intent_id)
            .ok_or_else(|| SwapError::RefundFailed {
                chain: self.inner.chain_id.clone(),
                reason: "no lock record found for this intent".into(),
            })?;

        if lock.claimed {
            return Err(SwapError::RefundFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already claimed".into(),
            });
        }

        if lock.refunded {
            return Err(SwapError::RefundFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already refunded".into(),
            });
        }

        if current_time < lock.timeout {
            return Err(SwapError::RefundFailed {
                chain: self.inner.chain_id.clone(),
                reason: "timeout has not yet elapsed".into(),
            });
        }

        let proof = self.inner.refund(intent_id)?;
        lock.refunded = true;
        Ok(proof)
    }

    /// Check if a given intent has been claimed.
    pub fn is_claimed(&self, intent_id: IntentId) -> bool {
        self.locks
            .iter()
            .find(|l| l.intent_id == intent_id)
            .map(|l| l.claimed)
            .unwrap_or(false)
    }

    /// Check if a given intent has been refunded.
    pub fn is_refunded(&self, intent_id: IntentId) -> bool {
        self.locks
            .iter()
            .find(|l| l.intent_id == intent_id)
            .map(|l| l.refunded)
            .unwrap_or(false)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::X3VmAdapter;
    use crate::intent::{
        AtomicIntent, AtomicSwapStatus, ChainKind, FinalityLevel, FinalityRequirement, RefundPath,
        RouteMode,
    };

    /// Helper: create a simple test intent.
    fn make_test_intent(intent_id: IntentId, hashlock: [u8; 32]) -> AtomicIntent {
        AtomicIntent {
            intent_id,
            source_chain: ChainKind::X3,
            destination_chain: ChainKind::Ethereum,
            source_asset: "ETH".into(),
            destination_asset: "USDC".into(),
            amount_in: 1_000_000_000_000_000_000,
            min_amount_out: 500_000_000,
            receiver: "0xabc123def456".into(),
            hashlock,
            source_timeout: 1_800_000,
            destination_timeout: 1_700_000,
            finality_requirements: vec![FinalityRequirement {
                chain: ChainKind::X3,
                level: FinalityLevel::Bft,
            }],
            refund_path: RefundPath {
                chain: ChainKind::X3,
                address: "0xrefundaddr789".into(),
                asset: None,
            },
            route_mode: RouteMode::DirectHtlc,
            max_slippage_bps: 100,
            relayer_quorum_requirement: 3,
            status: AtomicSwapStatus::Pending,
            intent_hash: [0u8; 32],
        }
    }

    /// Helper: compute hashlock from preimage.
    fn make_hashlock(preimage: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    // ── Cairo Contract Type Tests ─────────────────────────────────────────

    #[test]
    fn test_cairo_htlc_contract_creation() {
        let contract = CairoHtlcContract {
            felt_hashlock: [0u8; 32],
            felt_receiver: vec![0x01, 0x02],
            felt_refund_address: vec![0x03, 0x04],
            contract_address: "0xabc123".into(),
            asset: "ETH".into(),
            amount: 1_000_000,
            timeout_block: 100_000,
        };
        assert_eq!(contract.asset, "ETH");
        assert_eq!(contract.contract_address, "0xabc123");
        assert_eq!(contract.felt_receiver, vec![0x01, 0x02]);
    }

    // ── Adapter Identity Tests ────────────────────────────────────────────

    #[test]
    fn test_adapter_identity() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());

        assert_eq!(adapter.vm_type(), VmType::CairoVm);
        assert_eq!(adapter.adapter_name(), "x3-adapter-cairo");

        let chains = adapter.supported_chains();
        assert!(chains.contains(&"starknet-mainnet".into()));
        assert!(chains.contains(&"starknet-testnet".into()));
        assert!(chains.contains(&"starknet-sepolia".into()));

        let assets = adapter.supported_assets();
        assert!(assets.contains(&"ETH".into()));
        assert!(assets.contains(&"STRK".into()));
        assert!(assets.contains(&"USDC".into()));
    }

    #[test]
    fn test_adapter_name_const() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let name: &'static str = adapter.adapter_name();
        assert_eq!(name, "x3-adapter-cairo");
    }

    // ── Lock Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_lock_creates_proof() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let hashlock = make_hashlock(b"test_preimage");
        let intent = make_test_intent(42, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");

        assert_eq!(proof.vm_type, VmType::CairoVm);
        assert_eq!(proof.hashlock, hashlock);
        assert_eq!(proof.locked_amount, intent.amount_in);
        assert!(!proof.tx_id.is_empty());
        assert!(proof.lock_address.starts_with("0x"));
        assert_eq!(proof.receiver, intent.receiver.as_bytes());
        assert_eq!(proof.refund_address, intent.refund_path.address.as_bytes());
        assert_ne!(proof.block_number, 0);
    }

    #[test]
    fn test_lock_differs_per_intent() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let h1 = make_hashlock(b"secret1");
        let h2 = make_hashlock(b"secret2");

        let proof1 = adapter.lock(&make_test_intent(1, h1)).expect("lock 1");
        let proof2 = adapter.lock(&make_test_intent(2, h2)).expect("lock 2");

        assert_ne!(proof1.tx_id, proof2.tx_id);
        assert_ne!(proof1.lock_address, proof2.lock_address);
    }

    // ── Claim Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_claim_with_preimage() {
        let adapter = CairoVmAdapter::new("starknet-testnet".into());
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..5].copy_from_slice(b"claim");
            p
        };

        let proof = adapter.claim(100, preimage).expect("claim should succeed");

        assert_eq!(proof.intent_id, 100);
        assert_eq!(proof.preimage, preimage);
        assert_eq!(proof.vm_type, VmType::CairoVm);
        assert!(!proof.tx_id.is_empty());
    }

    // ── Refund Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_refund_after_timeout() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let proof = adapter.refund(200).expect("refund should succeed");

        assert_eq!(proof.intent_id, 200);
        assert_eq!(proof.vm_type, VmType::CairoVm);
        assert!(!proof.tx_id.is_empty());
        assert_ne!(proof.block_number, 0);
    }

    // ── Verification Tests ────────────────────────────────────────────────

    #[test]
    fn test_verify_valid_lock() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let hashlock = make_hashlock(b"valid_lock");
        let intent = make_test_intent(10, hashlock);

        let proof = adapter.lock(&intent).expect("lock");
        let valid = adapter.verify_lock(&proof).expect("verify");

        assert!(valid, "well-formed lock proof should verify");
    }

    #[test]
    fn test_verify_invalid_lock_wrong_vm() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let bad_proof = LockProof {
            tx_id: "some_tx".into(),
            chain_id: "starknet-mainnet".into(),
            vm_type: VmType::Evm, // wrong!
            block_number: 0,
            block_hash: "".into(),
            confirmations: 0,
            lock_address: "0xabc".into(),
            locked_amount: 100,
            hashlock: [0u8; 32],
            receiver: vec![],
            refund_address: vec![],
            timeout: 1000,
            raw_proof: vec![],
        };
        let valid = adapter.verify_lock(&bad_proof).expect("verify");
        assert!(!valid, "wrong VM type should fail verification");
    }

    #[test]
    fn test_verify_invalid_lock_bad_address() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "starknet-mainnet".into(),
            vm_type: VmType::CairoVm,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "no-prefix".to_string(), // doesn't start with 0x
            locked_amount: 100,
            hashlock: [0u8; 32],
            receiver: vec![],
            refund_address: vec![],
            timeout: 1000,
            raw_proof: vec![],
        };
        let valid = adapter.verify_lock(&bad_proof).expect("verify");
        assert!(!valid, "non-0x address should fail verification");
    }

    #[test]
    fn test_verify_invalid_lock_zero_amount() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "starknet-mainnet".into(),
            vm_type: VmType::CairoVm,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "0xabc".into(),
            locked_amount: 0,
            hashlock: [0u8; 32],
            receiver: vec![],
            refund_address: vec![],
            timeout: 1000,
            raw_proof: vec![],
        };
        let valid = adapter.verify_lock(&bad_proof).expect("verify");
        assert!(!valid, "zero amount should fail verification");
    }

    #[test]
    fn test_verify_invalid_lock_zero_timeout() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "starknet-mainnet".into(),
            vm_type: VmType::CairoVm,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "0xabc".into(),
            locked_amount: 100,
            hashlock: [0u8; 32],
            receiver: vec![],
            refund_address: vec![],
            timeout: 0,
            raw_proof: vec![],
        };
        let valid = adapter.verify_lock(&bad_proof).expect("verify");
        assert!(!valid, "zero timeout should fail verification");
    }

    #[test]
    fn test_verify_valid_claim() {
        let adapter = CairoVmAdapter::new("starknet-sepolia".into());
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..4].copy_from_slice(b"test");
            p
        };

        let proof = adapter.claim(10, preimage).expect("claim");
        let valid = adapter.verify_claim(&proof).expect("verify");

        assert!(valid, "well-formed claim proof should verify");
    }

    #[test]
    fn test_verify_invalid_claim() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let bad_proof = ClaimProof {
            tx_id: "".into(),
            intent_id: 0,
            chain_id: "".into(),
            vm_type: VmType::Evm,
            preimage: [0u8; 32],
            block_number: 0,
            block_hash: "".into(),
            raw_proof: vec![],
        };
        let valid = adapter.verify_claim(&bad_proof).expect("verify");
        assert!(!valid, "malformed claim proof should fail");
    }

    #[test]
    fn test_verify_valid_refund() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let proof = adapter.refund(10).expect("refund");
        let valid = adapter.verify_refund(&proof).expect("verify");
        assert!(valid, "well-formed refund proof should verify");
    }

    #[test]
    fn test_verify_invalid_refund() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let bad_proof = RefundProof {
            tx_id: "".into(),
            intent_id: 0,
            chain_id: "".into(),
            vm_type: VmType::Evm,
            block_number: 0,
            block_hash: "".into(),
            raw_proof: vec![],
        };
        let valid = adapter.verify_refund(&bad_proof).expect("verify");
        assert!(!valid, "malformed refund proof should fail");
    }

    // ── Finality & Health Tests ───────────────────────────────────────────

    #[test]
    fn test_finality_status() {
        let mut adapter = CairoVmAdapter::new("starknet-mainnet".into());
        adapter.last_finalized_block = 10;

        let fp = adapter
            .finality_status(&"mock_tx_id".into())
            .expect("finality status");

        assert_eq!(fp.chain_id, "starknet-mainnet");
        assert_eq!(fp.vm_type, VmType::CairoVm);
        assert!(fp.finalized);
        assert!(fp.safe_to_reveal_secret);
        assert_eq!(fp.finality_source, "starknet-l2-settlement");
    }

    #[test]
    fn test_finality_unfinalized() {
        let mut adapter = CairoVmAdapter::new("starknet-mainnet".into());
        adapter.last_finalized_block = 0;

        let fp = adapter
            .finality_status(&"new_tx".into())
            .expect("finality status");

        assert!(!fp.finalized, "block 0 should be unfinalized");
        assert!(!fp.safe_to_reveal_secret);
    }

    #[test]
    fn test_finality_not_safe_below_10() {
        let mut adapter = CairoVmAdapter::new("starknet-mainnet".into());
        adapter.last_finalized_block = 5;

        let fp = adapter
            .finality_status(&"tx".into())
            .expect("finality status");

        assert!(fp.finalized);
        assert!(
            !fp.safe_to_reveal_secret,
            "below 10 blocks not safe for StarkNet"
        );
    }

    #[test]
    fn test_chain_health() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());

        let health = adapter.chain_health().expect("chain health");

        assert_eq!(health.chain_id, "starknet-mainnet");
        assert_eq!(health.vm_type, VmType::CairoVm);
        assert!(!health.halted);
        assert!(!health.degraded);
        assert!(health.safe_for_new_intents);
        assert!(health.rpc_quorum_healthy);
        assert_eq!(health.block_delay_ms, 2_000);
        assert_eq!(health.finality_delay_ms, 60_000);
    }

    // ── Fee Estimation Tests ──────────────────────────────────────────────

    #[test]
    fn test_estimate_fee() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());
        let hashlock = make_hashlock(b"fee_test");
        let intent = make_test_intent(99, hashlock);

        let fee = adapter.estimate_fee(&intent).expect("estimate_fee");

        assert_eq!(fee.chain_id, "starknet-mainnet");
        assert_eq!(fee.vm_type, VmType::CairoVm);
        assert!(fee.native_fee > 0);
        assert!(fee.estimated_usd > 0.0);
    }

    // ── Readiness Score Tests ─────────────────────────────────────────────

    #[test]
    fn test_readiness_score() {
        let adapter = CairoVmAdapter::new("starknet-mainnet".into());

        let score = adapter.readiness_score();

        assert!(score.interface_implemented);
        assert!(score.lock_path);
        assert!(score.claim_path);
        assert!(score.refund_path);
        assert!(!score.event_proof_extraction);
        assert!(score.finality_proof);
        assert!(!score.rpc_indexer_support);
        assert!(score.timeout_safety);
        assert!(score.tests_implemented);
        assert!(!score.proof_ledger_integration);

        assert_eq!(score.score(), 70);
        assert_eq!(score.adapter_name, "x3-adapter-cairo");
        assert_eq!(score.vm_type, VmType::CairoVm);

        let missing = score.missing_items();
        assert!(missing.contains(&"event_proof_extraction"));
        assert!(missing.contains(&"rpc_indexer_support"));
        assert!(missing.contains(&"proof_ledger_integration"));
        assert!(missing.contains(&"ibc_support"));
        assert_eq!(missing.len(), 4);
    }

    // ── Stateful Adapter Tests ────────────────────────────────────────────

    #[test]
    fn test_stateful_lock() {
        let mut adapter = StatefulCairoVmAdapter::new("starknet-mainnet".into());
        let hashlock = make_hashlock(b"stateful_lock");
        let intent = make_test_intent(500, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");
        assert!(!proof.tx_id.is_empty());
        assert_eq!(adapter.locks.len(), 1);
    }

    #[test]
    fn test_double_lock_rejected() {
        let mut adapter = StatefulCairoVmAdapter::new("starknet-mainnet".into());
        let hashlock = make_hashlock(b"double_lock");
        let intent = make_test_intent(301, hashlock);

        adapter.lock(&intent).expect("first lock");
        let second = adapter.lock(&intent);
        assert!(second.is_err(), "double lock should be rejected");
    }

    #[test]
    fn test_double_claim_rejected() {
        let mut adapter = StatefulCairoVmAdapter::new("starknet-mainnet".into());

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..8].copy_from_slice(b"double_c");
            p
        };
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(300, hashlock);

        adapter.lock(&intent).expect("lock should succeed");

        let claim1 = adapter.claim(300, preimage);
        assert!(claim1.is_ok(), "first claim should succeed");

        let claim2 = adapter.claim(300, preimage);
        assert!(claim2.is_err(), "double claim should be rejected");

        match claim2 {
            Err(SwapError::ClaimFailed { reason, .. }) => {
                assert!(reason.contains("already claimed"));
            }
            _ => panic!("expected ClaimFailed error"),
        }
    }

    #[test]
    fn test_double_refund_rejected() {
        let mut adapter = StatefulCairoVmAdapter::new("starknet-testnet".into());

        let hashlock = make_hashlock(b"refund_test");
        let intent = make_test_intent(400, hashlock);

        adapter.lock(&intent).expect("lock");

        let after_timeout = intent.source_timeout + 1;
        let refund1 = adapter.refund(400, after_timeout);
        assert!(refund1.is_ok(), "first refund should succeed");

        let refund2 = adapter.refund(400, after_timeout);
        assert!(refund2.is_err(), "double refund should be rejected");

        match refund2 {
            Err(SwapError::RefundFailed { reason, .. }) => {
                assert!(reason.contains("already refunded"));
            }
            _ => panic!("expected RefundFailed error"),
        }
    }

    #[test]
    fn test_claim_before_timeout_succeeds() {
        let mut adapter = StatefulCairoVmAdapter::new("starknet-mainnet".into());

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..4].copy_from_slice(b"befo");
            p
        };
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(501, hashlock);

        adapter.lock(&intent).expect("lock");

        let claim = adapter.claim(501, preimage);
        assert!(claim.is_ok(), "claim before timeout should succeed");
    }

    #[test]
    fn test_refund_before_timeout_rejected() {
        let mut adapter = StatefulCairoVmAdapter::new("starknet-mainnet".into());

        let hashlock = make_hashlock(b"early_refund");
        let intent = make_test_intent(600, hashlock);

        adapter.lock(&intent).expect("lock");

        let before_timeout = intent.source_timeout - 1;
        let refund = adapter.refund(600, before_timeout);

        match refund {
            Err(SwapError::RefundFailed { reason, .. }) => {
                assert!(reason.contains("timeout has not yet elapsed"));
            }
            other => panic!("expected timeout error, got: {:?}", other),
        }
    }

    #[test]
    fn test_claim_after_refund_rejected() {
        let mut adapter = StatefulCairoVmAdapter::new("starknet-mainnet".into());

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..7].copy_from_slice(b"after_r");
            p
        };
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(700, hashlock);

        adapter.lock(&intent).expect("lock");

        let after_timeout = intent.source_timeout + 1;
        adapter.refund(700, after_timeout).expect("refund");

        let claim = adapter.claim(700, preimage);
        match claim {
            Err(SwapError::ClaimFailed { reason, .. }) => {
                assert!(reason.contains("already refunded"));
            }
            other => panic!(
                "expected ClaimFailed for already refunded, got: {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_is_claimed_and_is_refunded() {
        let mut adapter = StatefulCairoVmAdapter::new("starknet-sepolia".into());

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..3].copy_from_slice(b"st1");
            p
        };
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(800, hashlock);

        adapter.lock(&intent).expect("lock");
        assert!(!adapter.is_claimed(800));
        assert!(!adapter.is_refunded(800));

        adapter.claim(800, preimage).expect("claim");
        assert!(adapter.is_claimed(800));
        assert!(!adapter.is_refunded(800));
    }

    #[test]
    fn test_is_refunded_state() {
        let mut adapter = StatefulCairoVmAdapter::new("starknet-mainnet".into());

        let hashlock = make_hashlock(b"state_refund");
        let intent = make_test_intent(900, hashlock);

        adapter.lock(&intent).expect("lock");
        let after_timeout = intent.source_timeout + 1;
        adapter.refund(900, after_timeout).expect("refund");

        assert!(!adapter.is_claimed(900));
        assert!(adapter.is_refunded(900));
    }

    #[test]
    fn test_set_rpc() {
        let mut adapter = CairoVmAdapter::new("starknet-mainnet".into());
        assert!(adapter.rpc_url.is_none());

        adapter.set_rpc("https://starknet.custom.node.example.com");
        assert_eq!(
            adapter.rpc_url.unwrap(),
            "https://starknet.custom.node.example.com"
        );
    }

    #[test]
    fn test_cairo_address_format() {
        let address = CairoVmAdapter::mock_cairo_address(42);
        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 64); // 0x + 62 hex chars = 31 bytes
    }
}
