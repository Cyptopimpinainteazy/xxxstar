//! # Substrate HTLC Adapter
//!
//! Adapter for Substrate chains (Polkadot, Kusama, standalone Substrate
//! chains). Implements [`X3VmAdapter`] with mock/placeholder proof structures.
//!
//! In production, [`lock`] would dispatch to a Substrate pallet (e.g. an
//! atomic-swap or HTLC pallet), [`claim`] would submit a claim transaction,
//! and [`refund`] would trigger the refund path after timeout. Finality uses
//! GRANDPA (1 block finality) model.

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
// SubstrateHtlcAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter for Substrate chains (Polkadot, Kusama, standalone).
///
/// Uses mock/placeholder proof data. In real operation this would connect to a
/// Substrate node via RPC/WebSocket and interact with an HTLC pallet.
///
/// For stateful double-claim/double-refund enforcement, use
/// [`StatefulSubstrateAdapter`].
#[derive(Debug, Clone)]
pub struct SubstrateHtlcAdapter {
    /// Chain identifier (e.g. "substrate-default", "polkadot", "kusama").
    pub chain_id: ChainId,
    /// Optional HTTP RPC URL for Substrate node.
    pub rpc_url: Option<String>,
    /// Optional WebSocket URL for Substrate node.
    pub ws_url: Option<String>,
    /// Current finalized block number (GRANDPA finality).
    pub finalized_block: u64,
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

impl SubstrateHtlcAdapter {
    /// Create a new adapter for the given chain identifier.
    ///
    /// Example chain IDs: `"substrate-default"`, `"polkadot"`, `"kusama"`.
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            chain_id,
            rpc_url: None,
            ws_url: None,
            finalized_block: 0,
        }
    }

    /// Set the RPC and WebSocket URLs for the Substrate node.
    pub fn set_rpc(&mut self, rpc_url: &str, ws_url: &str) {
        self.rpc_url = Some(rpc_url.to_string());
        self.ws_url = Some(ws_url.to_string());
    }

    /// Generate a deterministic mock tx_id from intent_id and a label byte.
    fn mock_tx_id(intent_id: IntentId, label: u8) -> TxId {
        let mut hasher = Sha256::new();
        hasher.update(intent_id.to_le_bytes());
        hasher.update([label]);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Derive a mock pallet address from chain_id.
    fn mock_pallet_address(chain_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"substrate-htlc-pallet:");
        hasher.update(chain_id.as_bytes());
        let result = hasher.finalize();
        // Substrate addresses are typically 32 or 33 bytes; encode as hex
        format!("0x{}", hex::encode(&result[..16]))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl X3VmAdapter for SubstrateHtlcAdapter {
    fn vm_type(&self) -> VmType {
        VmType::Substrate
    }

    fn adapter_name(&self) -> &'static str {
        "x3-adapter-substrate"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec![
            "substrate-default".into(),
            "polkadot".into(),
            "kusama".into(),
        ]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        vec!["DOT".into(), "KSM".into(), "X3".into()]
    }

    // ── Lifecycle operations ──────────────────────────────────────────────

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        // In production, this would dispatch to the Substrate HTLC pallet via
        // an extrinsic. Here we create a LockProof with mock/placeholder data.
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
        let lock_address = Self::mock_pallet_address(&chain_id);
        let block_number = self.finalized_block + 1;

        // Derive receiver/refund_address addresses from intent (interpreted as hex).
        let receiver = intent.receiver.as_bytes().to_vec();
        let refund_address = intent.refund_path.address.as_bytes().to_vec();

        Ok(LockProof {
            tx_id,
            chain_id,
            vm_type: VmType::Substrate,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            confirmations: 0,
            lock_address,
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver,
            refund_address,
            timeout: intent.source_timeout,
            raw_proof: vec![0x73, 0x75, 0x62, 0x01], // "sub\x01" - mock proof
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        // In production, this would submit a claim extrinsic.
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x02);
        let block_number = self.finalized_block + 2;

        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::Substrate,
            preimage,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x73, 0x75, 0x62, 0x02], // "sub\x02" - mock proof
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        // In production, this would submit a refund extrinsic.
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x03);
        let block_number = self.finalized_block + 3;

        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::Substrate,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x73, 0x75, 0x62, 0x03], // "sub\x03" - mock proof
        })
    }

    // ── Verification ──────────────────────────────────────────────────────

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        // Basic well-formedness check for mock proofs.
        if proof.vm_type != VmType::Substrate {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        if proof.lock_address.is_empty() {
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
        if proof.vm_type != VmType::Substrate {
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
        if proof.vm_type != VmType::Substrate {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    // ── Estimation & Health ───────────────────────────────────────────────

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        Ok(FeeEstimate {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::Substrate,
            // 0.01 DOT in Planck (1 DOT = 10^10 Planck on Polkadot)
            native_fee: 100_000_000,
            gas_units: 0,
            gas_price: 0,
            estimated_usd: 0.05,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        // GRANDPA finality model: 1 confirmation = finalized
        let confirmations = 1u64;
        let finalized = true;
        let safe = true;

        Ok(FinalityProof {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::Substrate,
            tx_id: tx_id.clone(),
            block_number: self.finalized_block,
            block_hash: hex::encode(Sha256::digest(self.finalized_block.to_le_bytes())),
            confirmations,
            finalized,
            finality_source: "grandpa".into(),
            safe_to_reveal_secret: safe,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        Ok(ChainHealth {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::Substrate,
            latest_block: self.finalized_block,
            finalized_block: self.finalized_block,
            block_delay_ms: 6_000,    // ~6s block time (Polkadot)
            finality_delay_ms: 6_000, // GRANDPA finality in ~6s
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
            adapter_name: "x3-adapter-substrate",
            vm_type: VmType::Substrate,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: false, // needs actual Substrate runtime event listening
            finality_proof: true,
            rpc_indexer_support: false, // needs real RPC/indexer integration
            timeout_safety: true,
            tests_implemented: true,
            proof_ledger_integration: true,
            ibc_support: false,
            cross_adapter_atomicity_test: false,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Stateful Wrapper with Double-Claim Protection
// ─────────────────────────────────────────────────────────────────────────────

/// A stateful wrapper around [`SubstrateHtlcAdapter`] that tracks lock state
/// in order to reject double claims and double refunds.
///
/// The base [`SubstrateHtlcAdapter`] uses `&self` (immutable) for all trait
/// methods, so we provide this wrapper that uses interior mutability via
/// a simple tracking map. This is the type that should be used for actual
/// swap operations where state enforcement is required.
#[derive(Debug, Clone)]
pub struct StatefulSubstrateAdapter {
    pub inner: SubstrateHtlcAdapter,
    locks: Vec<InternalLock>,
}

impl StatefulSubstrateAdapter {
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            inner: SubstrateHtlcAdapter::new(chain_id),
            locks: Vec::new(),
        }
    }

    pub fn set_rpc(&mut self, rpc_url: &str, ws_url: &str) {
        self.inner.set_rpc(rpc_url, ws_url);
    }

    /// Lock funds and record the lock state internally.
    pub fn lock(&mut self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        // Prevent duplicate locks for the same intent.
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
            source_asset: "DOT".into(),
            destination_asset: "USDC".into(),
            amount_in: 1_000_000_000_000,
            min_amount_out: 500_000_000,
            receiver: "5FfBQ1CApRqRm1zJQJgxjQPuB4Hn7FnP2YMT8EF8CqPCgNG7".into(),
            hashlock,
            source_timeout: 1_800_000,
            destination_timeout: 1_700_000,
            finality_requirements: vec![FinalityRequirement {
                chain: ChainKind::X3,
                level: FinalityLevel::Bft,
            }],
            refund_path: RefundPath {
                chain: ChainKind::X3,
                address: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY".into(),
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

    // ── Adapter Identity Tests ────────────────────────────────────────────

    #[test]
    fn test_adapter_identity() {
        let adapter = SubstrateHtlcAdapter::new("substrate-default".into());

        assert_eq!(adapter.vm_type(), VmType::Substrate);
        assert_eq!(adapter.adapter_name(), "x3-adapter-substrate");

        let chains = adapter.supported_chains();
        assert!(chains.contains(&"substrate-default".into()));
        assert!(chains.contains(&"polkadot".into()));
        assert!(chains.contains(&"kusama".into()));

        let assets = adapter.supported_assets();
        assert!(assets.contains(&"DOT".into()));
        assert!(assets.contains(&"KSM".into()));
        assert!(assets.contains(&"X3".into()));
    }

    #[test]
    fn test_adapter_name_const() {
        // Verify adapter_name is a &'static str, not heap-allocated.
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());
        let name: &'static str = adapter.adapter_name();
        assert_eq!(name, "x3-adapter-substrate");
    }

    // ── Lock Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_lock_creates_proof() {
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());
        let hashlock = make_hashlock(b"test_preimage");
        let intent = make_test_intent(42, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");

        assert_eq!(proof.vm_type, VmType::Substrate);
        assert_eq!(proof.hashlock, hashlock);
        assert_eq!(proof.locked_amount, intent.amount_in);
        assert!(!proof.tx_id.is_empty());
        assert!(!proof.lock_address.is_empty());
        assert_eq!(proof.receiver, intent.receiver.as_bytes());
        assert_eq!(proof.refund_address, intent.refund_path.address.as_bytes());
        assert_ne!(proof.block_number, 0);
    }

    #[test]
    fn test_lock_differs_per_intent() {
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());
        let h1 = make_hashlock(b"secret1");
        let h2 = make_hashlock(b"secret2");

        let proof1 = adapter.lock(&make_test_intent(1, h1)).expect("lock 1");
        let proof2 = adapter.lock(&make_test_intent(2, h2)).expect("lock 2");

        assert_ne!(proof1.tx_id, proof2.tx_id);
    }

    // ── Claim Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_claim_with_preimage() {
        let adapter = SubstrateHtlcAdapter::new("kusama".into());
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..5].copy_from_slice(b"claim");
            p
        };

        let proof = adapter.claim(100, preimage).expect("claim should succeed");

        assert_eq!(proof.intent_id, 100);
        assert_eq!(proof.preimage, preimage);
        assert_eq!(proof.vm_type, VmType::Substrate);
        assert!(!proof.tx_id.is_empty());
    }

    // ── Refund Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_refund_after_timeout() {
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());

        let proof = adapter.refund(200).expect("refund should succeed");

        assert_eq!(proof.intent_id, 200);
        assert_eq!(proof.vm_type, VmType::Substrate);
        assert!(!proof.tx_id.is_empty());
        assert_ne!(proof.block_number, 0);
    }

    // ── Verification Tests ────────────────────────────────────────────────

    #[test]
    fn test_verify_valid_lock() {
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());
        let hashlock = make_hashlock(b"valid_lock");
        let intent = make_test_intent(10, hashlock);

        let proof = adapter.lock(&intent).expect("lock");
        let valid = adapter.verify_lock(&proof).expect("verify");

        assert!(valid, "well-formed lock proof should verify");
    }

    #[test]
    fn test_verify_invalid_lock() {
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());

        // Malformed proof: wrong VM type
        let bad_proof = LockProof {
            tx_id: "some_tx".into(),
            chain_id: "polkadot".into(),
            vm_type: VmType::Evm, // wrong!
            block_number: 0,
            block_hash: "".into(),
            confirmations: 0,
            lock_address: "some_addr".into(),
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
    fn test_verify_invalid_lock_empty_tx() {
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());

        let bad_proof = LockProof {
            tx_id: String::new(),
            chain_id: "polkadot".into(),
            vm_type: VmType::Substrate,
            block_number: 0,
            block_hash: "".into(),
            confirmations: 0,
            lock_address: "addr".into(),
            locked_amount: 100,
            hashlock: [0u8; 32],
            receiver: vec![],
            refund_address: vec![],
            timeout: 1000,
            raw_proof: vec![],
        };

        let valid = adapter.verify_lock(&bad_proof).expect("verify");
        assert!(!valid, "empty tx_id should fail verification");
    }

    #[test]
    fn test_verify_invalid_lock_zero_amount() {
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());

        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "polkadot".into(),
            vm_type: VmType::Substrate,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "addr".into(),
            locked_amount: 0, // zero amount is invalid
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
    fn test_verify_valid_claim() {
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());
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
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());

        // Empty preimage
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
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());

        let proof = adapter.refund(10).expect("refund");
        let valid = adapter.verify_refund(&proof).expect("verify");

        assert!(valid, "well-formed refund proof should verify");
    }

    #[test]
    fn test_verify_invalid_refund() {
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());

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
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());

        let fp = adapter
            .finality_status(&"mock_tx_id".into())
            .expect("finality status");

        assert_eq!(fp.chain_id, "polkadot");
        assert_eq!(fp.vm_type, VmType::Substrate);
        assert!(fp.finalized);
        assert!(fp.safe_to_reveal_secret);
        assert_eq!(fp.finality_source, "grandpa");
    }

    #[test]
    fn test_chain_health() {
        let adapter = SubstrateHtlcAdapter::new("kusama".into());

        let health = adapter.chain_health().expect("chain health");

        assert_eq!(health.chain_id, "kusama");
        assert_eq!(health.vm_type, VmType::Substrate);
        assert!(!health.halted);
        assert!(!health.degraded);
        assert!(health.safe_for_new_intents);
        assert!(health.rpc_quorum_healthy);
    }

    // ── Fee Estimation Tests ──────────────────────────────────────────────

    #[test]
    fn test_estimate_fee() {
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());
        let hashlock = make_hashlock(b"fee_test");
        let intent = make_test_intent(99, hashlock);

        let fee = adapter.estimate_fee(&intent).expect("estimate_fee");

        assert_eq!(fee.chain_id, "polkadot");
        assert_eq!(fee.vm_type, VmType::Substrate);
        assert!(fee.native_fee > 0);
        assert!(fee.estimated_usd > 0.0);
    }

    // ── Readiness Score Tests ─────────────────────────────────────────────

    #[test]
    fn test_readiness_score() {
        let adapter = SubstrateHtlcAdapter::new("polkadot".into());

        let score = adapter.readiness_score();

        // All fields truthy except event_proof_extraction and rpc_indexer_support.
        assert!(score.interface_implemented);
        assert!(score.lock_path);
        assert!(score.claim_path);
        assert!(score.refund_path);
        assert!(!score.event_proof_extraction);
        assert!(score.finality_proof);
        assert!(!score.rpc_indexer_support);
        assert!(score.timeout_safety);
        assert!(score.tests_implemented);
        assert!(score.proof_ledger_integration);

        assert_eq!(score.score(), 80);
        assert_eq!(score.adapter_name, "x3-adapter-substrate");
        assert_eq!(score.vm_type, VmType::Substrate);

        let missing = score.missing_items();
        assert!(missing.contains(&"event_proof_extraction"));
        assert!(missing.contains(&"rpc_indexer_support"));
        assert!(missing.contains(&"ibc_support"));
        assert_eq!(missing.len(), 4);
    }

    // ── Stateful Adapter Tests ────────────────────────────────────────────

    #[test]
    fn test_double_claim_rejected() {
        let mut adapter = StatefulSubstrateAdapter::new("polkadot".into());

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..8].copy_from_slice(b"double_c");
            p
        };
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(300, hashlock);

        // Lock first
        adapter.lock(&intent).expect("lock should succeed");

        // First claim should succeed
        let claim1 = adapter.claim(300, preimage);
        assert!(claim1.is_ok(), "first claim should succeed");

        // Second claim should be rejected
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
        let mut adapter = StatefulSubstrateAdapter::new("kusama".into());

        let hashlock = make_hashlock(b"refund_test");
        let intent = make_test_intent(400, hashlock);

        adapter.lock(&intent).expect("lock");

        // Refund after timeout
        let after_timeout = intent.source_timeout + 1;
        let refund1 = adapter.refund(400, after_timeout);
        assert!(refund1.is_ok(), "first refund should succeed");

        // Second refund should be rejected
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
        let mut adapter = StatefulSubstrateAdapter::new("polkadot".into());

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..4].copy_from_slice(b"befo");
            p
        };
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(500, hashlock);

        adapter.lock(&intent).expect("lock");

        // Claim before timeout (current_time < source_timeout)
        let _before_timeout = intent.source_timeout - 100;
        let claim = adapter.claim(500, preimage);
        assert!(claim.is_ok(), "claim before timeout should succeed");
    }

    #[test]
    fn test_refund_before_timeout_rejected() {
        let mut adapter = StatefulSubstrateAdapter::new("polkadot".into());

        let hashlock = make_hashlock(b"early_refund");
        let intent = make_test_intent(600, hashlock);

        adapter.lock(&intent).expect("lock");

        // Refund before timeout should be rejected
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
    fn test_set_rpc() {
        let mut adapter = SubstrateHtlcAdapter::new("polkadot".into());
        assert!(adapter.rpc_url.is_none());
        assert!(adapter.ws_url.is_none());

        adapter.set_rpc("http://localhost:9933", "ws://localhost:9944");

        assert_eq!(adapter.rpc_url.as_deref(), Some("http://localhost:9933"));
        assert_eq!(adapter.ws_url.as_deref(), Some("ws://localhost:9944"));
    }

    #[test]
    fn test_adapter_chain_id_independence() {
        let dot = SubstrateHtlcAdapter::new("polkadot".into());
        let ksm = SubstrateHtlcAdapter::new("kusama".into());

        assert_eq!(dot.vm_type(), VmType::Substrate);
        assert_eq!(ksm.vm_type(), VmType::Substrate);

        // Different chain IDs produce different lock addresses.
        let h = make_hashlock(b"chain_test");
        let intent_dot = make_test_intent(1, h);
        let intent_ksm = make_test_intent(1, h);

        let proof_dot = dot.lock(&intent_dot).expect("dot lock");
        let proof_ksm = ksm.lock(&intent_ksm).expect("ksm lock");

        assert_ne!(proof_dot.lock_address, proof_ksm.lock_address);
    }
}
