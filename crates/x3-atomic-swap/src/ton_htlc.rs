//! # TON TVM HTLC Adapter
//!
//! Adapter for TON blockchain (TVM). Implements [`X3VmAdapter`] with
//! mock/placeholder proof structures.
//!
//! In production, [`lock`] would deploy or call a TON FunC/Tact HTLC contract,
//! [`claim`] would call the claim method with preimage, and [`refund`] would
//! trigger the refund path after timeout. Finality uses TON masterchain
//! confirmation (~5s finality).

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
// TON Types
// ─────────────────────────────────────────────────────────────────────────────

/// TON network environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TonNetwork {
    Mainnet,
    Testnet,
}

impl TonNetwork {
    pub fn name(&self) -> &'static str {
        match self {
            TonNetwork::Mainnet => "ton-mainnet",
            TonNetwork::Testnet => "ton-testnet",
        }
    }
}

/// Data stored in a TON HTLC smart contract.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TonLockData {
    pub hashlock: [u8; 32],
    pub owner: Vec<u8>,
    pub receiver: Vec<u8>,
    pub refund_address: Vec<u8>,
    pub amount: u128,
    pub timeout: u64,
    pub claimed: bool,
}

/// Represents a TON HTLC contract.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TonContract {
    pub address: String,
    pub lock_data: TonLockData,
}

// ─────────────────────────────────────────────────────────────────────────────
// TonHtlcAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter for TON chains.
///
/// Uses mock/placeholder proof data. In real operation this would connect to a
/// TON node via ADNL/Lite API and interact with FunC/Tact HTLC contracts.
///
/// For stateful double-claim/double-refund enforcement, use
/// [`StatefulTonAdapter`].
#[derive(Debug, Clone)]
pub struct TonHtlcAdapter {
    /// Chain identifier (e.g. "ton-mainnet", "ton-testnet").
    pub chain_id: ChainId,
    /// Network variant.
    pub network: TonNetwork,
    /// Optional HTTP RPC URL.
    pub rpc_url: Option<String>,
    /// Last seen masterchain seqno.
    pub last_seqno: u64,
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

impl TonHtlcAdapter {
    /// Create a new adapter for the given chain identifier.
    ///
    /// Example chain IDs: `"ton-mainnet"`, `"ton-testnet"`.
    pub fn new(chain_id: ChainId, network: TonNetwork) -> Self {
        Self {
            chain_id,
            network,
            rpc_url: None,
            last_seqno: 0,
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

    /// Generate a mock TON contract address.
    fn mock_address(chain_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"x3-ton-htlc:");
        hasher.update(chain_id.as_bytes());
        let result = hasher.finalize();
        format!("EQ{}", hex::encode(&result[..30]))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl X3VmAdapter for TonHtlcAdapter {
    fn vm_type(&self) -> VmType {
        VmType::TonTvm
    }

    fn adapter_name(&self) -> &'static str {
        "x3-adapter-ton-tvm"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec!["ton-mainnet".into(), "ton-testnet".into()]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        vec!["TON".into(), "USDT".into(), "NOT".into()]
    }

    // ── Lifecycle operations ──────────────────────────────────────────────

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
        let lock_address = Self::mock_address(&chain_id);
        let block_number = self.last_seqno + 1;

        let receiver = intent.receiver.as_bytes().to_vec();
        let refund_address = intent.refund_path.address.as_bytes().to_vec();

        Ok(LockProof {
            tx_id,
            chain_id,
            vm_type: VmType::TonTvm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            confirmations: 0,
            lock_address,
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver,
            refund_address,
            timeout: intent.source_timeout,
            raw_proof: vec![0x74, 0x6f, 0x6e, 0x01], // "ton\x01" - mock proof
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x02);
        let block_number = self.last_seqno + 2;

        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::TonTvm,
            preimage,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x74, 0x6f, 0x6e, 0x02], // "ton\x02" - mock proof
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x03);
        let block_number = self.last_seqno + 3;

        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::TonTvm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x74, 0x6f, 0x6e, 0x03], // "ton\x03" - mock proof
        })
    }

    // ── Verification ──────────────────────────────────────────────────────

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::TonTvm {
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
        if proof.vm_type != VmType::TonTvm {
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
        if proof.vm_type != VmType::TonTvm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    // ── Estimation & Health ───────────────────────────────────────────────

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        // ~0.005 TON for a simple contract call
        Ok(FeeEstimate {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::TonTvm,
            native_fee: 5_000_000, // 0.005 TON in nanoTON
            gas_units: 0,
            gas_price: 0,
            estimated_usd: 0.03,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        // TON masterchain finality: ~5s, 1 confirmation is usually enough
        let finalized = self.last_seqno >= 1;
        let safe = self.last_seqno >= 2;

        Ok(FinalityProof {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::TonTvm,
            tx_id: tx_id.clone(),
            block_number: self.last_seqno,
            block_hash: hex::encode(Sha256::digest(self.last_seqno.to_le_bytes())),
            confirmations: if finalized { 1 } else { 0 },
            finalized,
            finality_source: "ton-masterchain".into(),
            safe_to_reveal_secret: safe,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        Ok(ChainHealth {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::TonTvm,
            latest_block: self.last_seqno,
            finalized_block: if self.last_seqno >= 1 {
                self.last_seqno
            } else {
                0
            },
            block_delay_ms: 5_000,    // ~5s TON block time
            finality_delay_ms: 5_000, // ~5s finality
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
            adapter_name: "x3-adapter-ton-tvm",
            vm_type: VmType::TonTvm,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: false, // needs actual TON event extraction
            finality_proof: true,
            rpc_indexer_support: false, // needs real RPC/indexer integration
            timeout_safety: true,
            tests_implemented: true,
            proof_ledger_integration: false, // needs proof ledger integration
            ibc_support: false,
            cross_adapter_atomicity_test: false,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Stateful Wrapper with Double-Claim Protection
// ─────────────────────────────────────────────────────────────────────────────

/// A stateful wrapper around [`TonHtlcAdapter`] that tracks lock state
/// in order to reject double claims and double refunds.
#[derive(Debug, Clone)]
pub struct StatefulTonAdapter {
    pub inner: TonHtlcAdapter,
    locks: Vec<InternalLock>,
}

impl StatefulTonAdapter {
    pub fn new(chain_id: ChainId, network: TonNetwork) -> Self {
        Self {
            inner: TonHtlcAdapter::new(chain_id, network),
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
            source_asset: "TON".into(),
            destination_asset: "USDC".into(),
            amount_in: 1_000_000_000, // 1 TON in nanoTON
            min_amount_out: 500_000_000,
            receiver: "UQABC123DEF456".into(),
            hashlock,
            source_timeout: 1_800_000,
            destination_timeout: 1_700_000,
            finality_requirements: vec![FinalityRequirement {
                chain: ChainKind::X3,
                level: FinalityLevel::Bft,
            }],
            refund_path: RefundPath {
                chain: ChainKind::X3,
                address: "UQAREFUND789".into(),
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

    // ── TON Type Tests ────────────────────────────────────────────────────

    #[test]
    fn test_ton_network_name() {
        assert_eq!(TonNetwork::Mainnet.name(), "ton-mainnet");
        assert_eq!(TonNetwork::Testnet.name(), "ton-testnet");
    }

    #[test]
    fn test_ton_network_equality() {
        assert_eq!(TonNetwork::Mainnet, TonNetwork::Mainnet);
        assert_ne!(TonNetwork::Mainnet, TonNetwork::Testnet);
    }

    #[test]
    fn test_ton_lock_data() {
        let data = TonLockData {
            hashlock: [0xabu8; 32],
            owner: vec![0x01],
            receiver: vec![0x02],
            refund_address: vec![0x03],
            amount: 1_000_000_000,
            timeout: 1000,
            claimed: false,
        };
        assert_eq!(data.amount, 1_000_000_000);
        assert!(!data.claimed);
    }

    #[test]
    fn test_ton_contract() {
        let contract = TonContract {
            address: "EQABCDEF".into(),
            lock_data: TonLockData {
                hashlock: [0u8; 32],
                owner: vec![],
                receiver: vec![],
                refund_address: vec![],
                amount: 0,
                timeout: 0,
                claimed: false,
            },
        };
        assert_eq!(contract.address, "EQABCDEF");
    }

    // ── Adapter Identity Tests ────────────────────────────────────────────

    #[test]
    fn test_adapter_identity() {
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);

        assert_eq!(adapter.vm_type(), VmType::TonTvm);
        assert_eq!(adapter.adapter_name(), "x3-adapter-ton-tvm");

        let chains = adapter.supported_chains();
        assert!(chains.contains(&"ton-mainnet".into()));
        assert!(chains.contains(&"ton-testnet".into()));

        let assets = adapter.supported_assets();
        assert!(assets.contains(&"TON".into()));
        assert!(assets.contains(&"USDT".into()));
        assert!(assets.contains(&"NOT".into()));
    }

    #[test]
    fn test_adapter_name_const() {
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let name: &'static str = adapter.adapter_name();
        assert_eq!(name, "x3-adapter-ton-tvm");
    }

    // ── Lock Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_lock_creates_proof() {
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let hashlock = make_hashlock(b"test_preimage");
        let intent = make_test_intent(42, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");

        assert_eq!(proof.vm_type, VmType::TonTvm);
        assert_eq!(proof.hashlock, hashlock);
        assert_eq!(proof.locked_amount, intent.amount_in);
        assert!(!proof.tx_id.is_empty());
        assert!(!proof.lock_address.is_empty());
        assert!(proof.lock_address.starts_with("EQ"));
        assert_eq!(proof.receiver, intent.receiver.as_bytes());
        assert_eq!(proof.refund_address, intent.refund_path.address.as_bytes());
        assert_ne!(proof.block_number, 0);
    }

    #[test]
    fn test_lock_differs_per_intent() {
        let adapter = TonHtlcAdapter::new("ton-testnet".into(), TonNetwork::Testnet);
        let h1 = make_hashlock(b"secret1");
        let h2 = make_hashlock(b"secret2");

        let proof1 = adapter.lock(&make_test_intent(1, h1)).expect("lock 1");
        let proof2 = adapter.lock(&make_test_intent(2, h2)).expect("lock 2");

        assert_ne!(proof1.tx_id, proof2.tx_id);
    }

    // ── Claim Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_claim_with_preimage() {
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..5].copy_from_slice(b"claim");
            p
        };

        let proof = adapter.claim(100, preimage).expect("claim should succeed");

        assert_eq!(proof.intent_id, 100);
        assert_eq!(proof.preimage, preimage);
        assert_eq!(proof.vm_type, VmType::TonTvm);
        assert!(!proof.tx_id.is_empty());
    }

    // ── Refund Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_refund_after_timeout() {
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let proof = adapter.refund(200).expect("refund should succeed");

        assert_eq!(proof.intent_id, 200);
        assert_eq!(proof.vm_type, VmType::TonTvm);
        assert!(!proof.tx_id.is_empty());
        assert_ne!(proof.block_number, 0);
    }

    // ── Verification Tests ────────────────────────────────────────────────

    #[test]
    fn test_verify_valid_lock() {
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let hashlock = make_hashlock(b"valid_lock");
        let intent = make_test_intent(10, hashlock);

        let proof = adapter.lock(&intent).expect("lock");
        let valid = adapter.verify_lock(&proof).expect("verify");

        assert!(valid, "well-formed lock proof should verify");
    }

    #[test]
    fn test_verify_invalid_lock_wrong_vm() {
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let bad_proof = LockProof {
            tx_id: "some_tx".into(),
            chain_id: "ton-mainnet".into(),
            vm_type: VmType::Evm,
            block_number: 0,
            block_hash: "".into(),
            confirmations: 0,
            lock_address: "EQxxx".into(),
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
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let bad_proof = LockProof {
            tx_id: String::new(),
            chain_id: "ton-mainnet".into(),
            vm_type: VmType::TonTvm,
            block_number: 1,
            block_hash: "hash".into(),
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
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "ton-mainnet".into(),
            vm_type: VmType::TonTvm,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "addr".into(),
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
    fn test_verify_valid_claim() {
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
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
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
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
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let proof = adapter.refund(10).expect("refund");
        let valid = adapter.verify_refund(&proof).expect("verify");
        assert!(valid, "well-formed refund proof should verify");
    }

    #[test]
    fn test_verify_invalid_refund() {
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
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
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let fp = adapter
            .finality_status(&"mock_tx_id".into())
            .expect("finality status");

        assert_eq!(fp.chain_id, "ton-mainnet");
        assert_eq!(fp.vm_type, VmType::TonTvm);
        assert_eq!(fp.finality_source, "ton-masterchain");
    }

    #[test]
    fn test_chain_health() {
        let adapter = TonHtlcAdapter::new("ton-testnet".into(), TonNetwork::Testnet);
        let health = adapter.chain_health().expect("chain health");

        assert_eq!(health.chain_id, "ton-testnet");
        assert_eq!(health.vm_type, VmType::TonTvm);
        assert!(!health.halted);
        assert!(!health.degraded);
        assert!(health.safe_for_new_intents);
    }

    // ── Fee Estimation Tests ──────────────────────────────────────────────

    #[test]
    fn test_estimate_fee() {
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let hashlock = make_hashlock(b"fee_test");
        let intent = make_test_intent(99, hashlock);

        let fee = adapter.estimate_fee(&intent).expect("estimate_fee");

        assert_eq!(fee.chain_id, "ton-mainnet");
        assert_eq!(fee.vm_type, VmType::TonTvm);
        assert!(fee.native_fee > 0);
        assert!(fee.estimated_usd > 0.0);
    }

    // ── Readiness Score Tests ─────────────────────────────────────────────

    #[test]
    fn test_readiness_score() {
        let adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);

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
        assert_eq!(score.adapter_name, "x3-adapter-ton-tvm");
        assert_eq!(score.vm_type, VmType::TonTvm);

        let missing = score.missing_items();
        assert!(missing.contains(&"event_proof_extraction"));
        assert!(missing.contains(&"rpc_indexer_support"));
        assert!(missing.contains(&"proof_ledger_integration"));
        assert!(missing.contains(&"ibc_support"));
        assert_eq!(missing.len(), 5);
    }

    // ── Stateful Adapter Tests ────────────────────────────────────────────

    #[test]
    fn test_double_claim_rejected() {
        let mut adapter = StatefulTonAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);

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
        let mut adapter = StatefulTonAdapter::new("ton-testnet".into(), TonNetwork::Testnet);

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
        let mut adapter = StatefulTonAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..4].copy_from_slice(b"befo");
            p
        };
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(500, hashlock);

        adapter.lock(&intent).expect("lock");

        let claim = adapter.claim(500, preimage);
        assert!(claim.is_ok(), "claim before timeout should succeed");
    }

    #[test]
    fn test_refund_before_timeout_rejected() {
        let mut adapter = StatefulTonAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);

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
    fn test_set_rpc() {
        let mut adapter = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        assert!(adapter.rpc_url.is_none());

        adapter.set_rpc("https://ton-mainnet.rpc:8080");

        assert_eq!(
            adapter.rpc_url.as_deref(),
            Some("https://ton-mainnet.rpc:8080")
        );
    }

    #[test]
    fn test_adapter_chain_id_independence() {
        let mainnet = TonHtlcAdapter::new("ton-mainnet".into(), TonNetwork::Mainnet);
        let testnet = TonHtlcAdapter::new("ton-testnet".into(), TonNetwork::Testnet);

        assert_eq!(mainnet.vm_type(), VmType::TonTvm);
        assert_eq!(testnet.vm_type(), VmType::TonTvm);

        let h = make_hashlock(b"chain_test");
        let intent_main = make_test_intent(1, h);
        let intent_test = make_test_intent(1, h);

        let proof_main = mainnet.lock(&intent_main).expect("mainnet lock");
        let proof_test = testnet.lock(&intent_test).expect("testnet lock");

        assert_ne!(proof_main.lock_address, proof_test.lock_address);
    }
}
