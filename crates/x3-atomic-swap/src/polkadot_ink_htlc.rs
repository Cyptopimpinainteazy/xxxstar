//! # Polkadot ink! / PVM HTLC Adapter
//!
//! Adapter for Polkadot's ink! smart contract platform and Polkadot Virtual
//! Machine (PVM). Implements [`X3VmAdapter`] with mock/placeholder proof
//! structures.
//!
//! In production, [`lock`] would deploy/create an ink! HTLC contract on a
//! parachain, [`claim`] would call the claim method with preimage, and
//! [`refund`] would trigger the refund path after timeout. Finality uses
//! GRANDPA finality (same as Substrate, 1 block finality).

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
// Ink! Types
// ─────────────────────────────────────────────────────────────────────────────

/// ink! / Polkadot network environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InkNetwork {
    PolkadotMainnet,
    PolkadotTestnet,
    Westend,
    Rococo,
}

impl InkNetwork {
    /// Get the network name string.
    pub fn name(&self) -> &'static str {
        match self {
            InkNetwork::PolkadotMainnet => "polkadot-mainnet",
            InkNetwork::PolkadotTestnet => "polkadot-testnet",
            InkNetwork::Westend => "polkadot-westend",
            InkNetwork::Rococo => "rococo-contracts",
        }
    }
}

/// State of an HTLC contract on an ink! chain.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InkHtlcContract {
    /// Contract address (WASM blob hash prefixed).
    pub contract_address: Vec<u8>,
    /// SHA-256 hashlock.
    pub hashlock: [u8; 32],
    /// Owner/locker address.
    pub owner: Vec<u8>,
    /// Receiver/claimant address.
    pub receiver: Vec<u8>,
    /// Refund address.
    pub refund_address: Vec<u8>,
    /// Amount locked (in Planck units).
    pub amount: u128,
    /// Timeout (block number or unix timestamp).
    pub timeout: u64,
    /// Whether the lock has been claimed.
    pub claimed: bool,
    /// Whether the lock has been refunded.
    pub refunded: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// InkHtlcAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter for Polkadot ink! / PVM chains.
///
/// Uses mock/placeholder proof data. In real operation this would connect to a
/// Substrate/Polkadot RPC node and interact with ink! HTLC contracts.
///
/// For stateful double-claim/double-refund enforcement, use
/// [`StatefulInkAdapter`].
#[derive(Debug, Clone)]
pub struct InkHtlcAdapter {
    /// Chain identifier (e.g. "polkadot-mainnet", "rococo-contracts").
    pub chain_id: ChainId,
    /// Network variant.
    pub network: InkNetwork,
    /// Optional RPC URL.
    pub rpc_url: Option<String>,
    /// Last known finalized block number.
    pub finalized_block: u64,
    /// Tracked claimed intent IDs.
    pub claimed_intents: Vec<u64>,
    /// Tracked refunded intent IDs.
    pub refunded_intents: Vec<u64>,
    /// Whether XCM (cross-chain messaging) is supported.
    pub xcm_supported: bool,
}

/// Internal lock state tracked by the stateful adapter.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct InternalInkLock {
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

impl InkHtlcAdapter {
    /// Create a new adapter for the given chain identifier.
    ///
    /// Example chain IDs: `"polkadot-mainnet"`, `"polkadot-westend"`,
    /// `"rococo-contracts"`, `"astar"`, `"moonbeam"`.
    pub fn new(chain_id: ChainId, network: InkNetwork) -> Self {
        Self {
            chain_id,
            network,
            rpc_url: None,
            finalized_block: 0,
            claimed_intents: Vec::new(),
            refunded_intents: Vec::new(),
            xcm_supported: true,
        }
    }

    /// Set the RPC URL.
    pub fn set_rpc(&mut self, rpc_url: &str) {
        self.rpc_url = Some(rpc_url.to_string());
    }

    /// Enable or disable XCM support.
    pub fn set_xcm_supported(&mut self, supported: bool) {
        self.xcm_supported = supported;
    }

    /// Generate a deterministic mock tx_id from intent_id and a label byte.
    fn mock_tx_id(intent_id: IntentId, label: u8) -> TxId {
        let mut hasher = Sha256::new();
        hasher.update(intent_id.to_le_bytes());
        hasher.update([label]);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Generate a mock ink! contract address (wasm blob hash prefixed).
    fn mock_contract_address(chain_id: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"x3-ink-htlc:");
        hasher.update(chain_id.as_bytes());
        let result = hasher.finalize();
        // ink! contract addresses are typically 32-byte account IDs.
        result.to_vec()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl X3VmAdapter for InkHtlcAdapter {
    fn vm_type(&self) -> VmType {
        VmType::InkWasm
    }

    fn adapter_name(&self) -> &'static str {
        "x3-adapter-ink"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec![
            "polkadot-mainnet".into(),
            "polkadot-westend".into(),
            "rococo-contracts".into(),
            "astar".into(),
            "moonbeam".into(),
        ]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        vec![
            "DOT".into(),
            "WND".into(),
            "ROC".into(),
            "ASTR".into(),
            "GLMR".into(),
        ]
    }

    // ── Lifecycle operations ──────────────────────────────────────────────

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
        let lock_address = hex::encode(Self::mock_contract_address(&chain_id));
        let block_number = self.finalized_block + 1;

        let receiver = intent.receiver.as_bytes().to_vec();
        let refund_address = intent.refund_path.address.as_bytes().to_vec();

        Ok(LockProof {
            tx_id,
            chain_id,
            vm_type: VmType::InkWasm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            confirmations: 0,
            lock_address,
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver,
            refund_address,
            timeout: intent.source_timeout,
            raw_proof: vec![0x69, 0x6e, 0x6b, 0x01], // "ink\x01" - mock proof
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x02);
        let block_number = self.finalized_block + 2;

        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::InkWasm,
            preimage,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x69, 0x6e, 0x6b, 0x02], // "ink\x02" - mock proof
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x03);
        let block_number = self.finalized_block + 3;

        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::InkWasm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x69, 0x6e, 0x6b, 0x03], // "ink\x03" - mock proof
        })
    }

    // ── Verification ──────────────────────────────────────────────────────

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::InkWasm {
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
        if proof.vm_type != VmType::InkWasm {
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
        if proof.vm_type != VmType::InkWasm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    // ── Estimation & Health ───────────────────────────────────────────────

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        // ~0.01 DOT for an ink! contract call
        Ok(FeeEstimate {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::InkWasm,
            native_fee: 10_000_000_000_000, // 0.01 DOT in Planck
            gas_units: 0,
            gas_price: 0,
            estimated_usd: 0.05,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        // GRANDPA finality: 1 block finality for Substrate chains
        let finalized = self.finalized_block >= 1;
        let safe = self.finalized_block >= 2;

        Ok(FinalityProof {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::InkWasm,
            tx_id: tx_id.clone(),
            block_number: self.finalized_block,
            block_hash: hex::encode(Sha256::digest(self.finalized_block.to_le_bytes())),
            confirmations: if finalized { 1 } else { 0 },
            finalized,
            finality_source: "grandpa".into(),
            safe_to_reveal_secret: safe,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        Ok(ChainHealth {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::InkWasm,
            latest_block: self.finalized_block,
            finalized_block: if self.finalized_block >= 1 {
                self.finalized_block
            } else {
                0
            },
            block_delay_ms: 12_000,    // ~12s Polkadot block time
            finality_delay_ms: 12_000, // GRANDPA finality in 1 block
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
            adapter_name: "x3-adapter-ink",
            vm_type: VmType::InkWasm,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: false, // needs actual ink! event extraction
            finality_proof: true,
            rpc_indexer_support: false, // needs real RPC/indexer integration
            timeout_safety: true,
            tests_implemented: true,
            proof_ledger_integration: false, // needs proof ledger integration
            ibc_support: false,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Stateful Wrapper with Double-Claim Protection
// ─────────────────────────────────────────────────────────────────────────────

/// A stateful wrapper around [`InkHtlcAdapter`] that tracks lock state
/// in order to reject double claims and double refunds.
#[derive(Debug, Clone)]
pub struct StatefulInkAdapter {
    pub inner: InkHtlcAdapter,
    locks: Vec<InternalInkLock>,
}

impl StatefulInkAdapter {
    pub fn new(chain_id: ChainId, network: InkNetwork) -> Self {
        Self {
            inner: InkHtlcAdapter::new(chain_id, network),
            locks: Vec::new(),
        }
    }

    pub fn set_rpc(&mut self, rpc_url: &str) {
        self.inner.set_rpc(rpc_url);
    }

    pub fn set_xcm_supported(&mut self, supported: bool) {
        self.inner.set_xcm_supported(supported);
    }

    /// Lock funds and record the lock state internally.
    pub fn lock(&mut self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        if self.locks.iter().any(|l| l.intent_id == intent.intent_id) {
            return Err(SwapError::AlreadyLocked {
                chain: intent.source_chain,
            });
        }

        let proof = self.inner.lock(intent)?;

        self.locks.push(InternalInkLock {
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
            amount_in: 1_000_000_000_000_000, // 0.001 DOT in Planck
            min_amount_out: 500_000_000,
            receiver: "alice".into(),
            hashlock,
            source_timeout: 1_800_000,
            destination_timeout: 1_700_000,
            finality_requirements: vec![FinalityRequirement {
                chain: ChainKind::X3,
                level: FinalityLevel::Bft,
            }],
            refund_path: RefundPath {
                chain: ChainKind::X3,
                address: "refund".into(),
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

    // ── Ink Network Tests ─────────────────────────────────────────────────

    #[test]
    fn test_ink_network_name() {
        assert_eq!(InkNetwork::PolkadotMainnet.name(), "polkadot-mainnet");
        assert_eq!(InkNetwork::PolkadotTestnet.name(), "polkadot-testnet");
        assert_eq!(InkNetwork::Westend.name(), "polkadot-westend");
        assert_eq!(InkNetwork::Rococo.name(), "rococo-contracts");
    }

    #[test]
    fn test_ink_network_equality() {
        assert_eq!(InkNetwork::PolkadotMainnet, InkNetwork::PolkadotMainnet);
        assert_ne!(InkNetwork::PolkadotMainnet, InkNetwork::Westend);
        assert_ne!(InkNetwork::PolkadotMainnet, InkNetwork::Rococo);
    }

    // ── Ink Type Tests ────────────────────────────────────────────────────

    #[test]
    fn test_ink_htlc_contract() {
        let contract = InkHtlcContract {
            contract_address: vec![0x01; 32],
            hashlock: [0xabu8; 32],
            owner: vec![0x02],
            receiver: vec![0x03],
            refund_address: vec![0x04],
            amount: 1_000_000,
            timeout: 1000,
            claimed: false,
            refunded: false,
        };
        assert_eq!(contract.amount, 1_000_000);
        assert_eq!(contract.contract_address.len(), 32);
        assert!(!contract.claimed);
        assert!(!contract.refunded);
    }

    // ── Adapter Identity Tests ────────────────────────────────────────────

    #[test]
    fn test_adapter_identity() {
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);

        assert_eq!(adapter.vm_type(), VmType::InkWasm);
        assert_eq!(adapter.adapter_name(), "x3-adapter-ink");

        let chains = adapter.supported_chains();
        assert!(chains.contains(&"polkadot-mainnet".into()));
        assert!(chains.contains(&"polkadot-westend".into()));
        assert!(chains.contains(&"rococo-contracts".into()));
        assert!(chains.contains(&"astar".into()));
        assert!(chains.contains(&"moonbeam".into()));

        let assets = adapter.supported_assets();
        assert!(assets.contains(&"DOT".into()));
        assert!(assets.contains(&"WND".into()));
        assert!(assets.contains(&"ROC".into()));
        assert!(assets.contains(&"ASTR".into()));
        assert!(assets.contains(&"GLMR".into()));
    }

    #[test]
    fn test_adapter_name_const() {
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let name: &'static str = adapter.adapter_name();
        assert_eq!(name, "x3-adapter-ink");
    }

    #[test]
    fn test_xcm_supported_default() {
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        assert!(adapter.xcm_supported);
    }

    // ── Lock Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_lock_creates_proof() {
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let hashlock = make_hashlock(b"test_preimage");
        let intent = make_test_intent(42, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");

        assert_eq!(proof.vm_type, VmType::InkWasm);
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
        let adapter = InkHtlcAdapter::new("rococo-contracts".into(), InkNetwork::Rococo);
        let h1 = make_hashlock(b"secret1");
        let h2 = make_hashlock(b"secret2");

        let proof1 = adapter.lock(&make_test_intent(1, h1)).expect("lock 1");
        let proof2 = adapter.lock(&make_test_intent(2, h2)).expect("lock 2");

        assert_ne!(proof1.tx_id, proof2.tx_id);
    }

    // ── Claim Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_claim_with_preimage() {
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..5].copy_from_slice(b"claim");
            p
        };

        let proof = adapter.claim(100, preimage).expect("claim should succeed");

        assert_eq!(proof.intent_id, 100);
        assert_eq!(proof.preimage, preimage);
        assert_eq!(proof.vm_type, VmType::InkWasm);
        assert!(!proof.tx_id.is_empty());
    }

    // ── Refund Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_refund_after_timeout() {
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let proof = adapter.refund(200).expect("refund should succeed");

        assert_eq!(proof.intent_id, 200);
        assert_eq!(proof.vm_type, VmType::InkWasm);
        assert!(!proof.tx_id.is_empty());
        assert_ne!(proof.block_number, 0);
    }

    // ── Verification Tests ────────────────────────────────────────────────

    #[test]
    fn test_verify_valid_lock() {
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let hashlock = make_hashlock(b"valid_lock");
        let intent = make_test_intent(10, hashlock);

        let proof = adapter.lock(&intent).expect("lock");
        let valid = adapter.verify_lock(&proof).expect("verify");

        assert!(valid, "well-formed lock proof should verify");
    }

    #[test]
    fn test_verify_invalid_lock_wrong_vm() {
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let bad_proof = LockProof {
            tx_id: "some_tx".into(),
            chain_id: "polkadot-mainnet".into(),
            vm_type: VmType::Evm,
            block_number: 0,
            block_hash: "".into(),
            confirmations: 0,
            lock_address: "contract".into(),
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
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let bad_proof = LockProof {
            tx_id: String::new(),
            chain_id: "polkadot-mainnet".into(),
            vm_type: VmType::InkWasm,
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
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "polkadot-mainnet".into(),
            vm_type: VmType::InkWasm,
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
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
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
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
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

    // ── Stateful Adapter Tests ────────────────────────────────────────────

    #[test]
    fn test_stateful_lock_and_claim() {
        let mut adapter =
            StatefulInkAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let preimage = make_hashlock(b"secret");
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(1, hashlock);

        let lock_proof = adapter.lock(&intent).expect("lock");
        assert!(!lock_proof.tx_id.is_empty());

        let claim_proof = adapter.claim(1, preimage).expect("claim");
        assert_eq!(claim_proof.intent_id, 1);

        assert!(adapter.is_claimed(1));
        assert!(!adapter.is_refunded(1));
    }

    #[test]
    fn test_stateful_double_claim_rejected() {
        let mut adapter =
            StatefulInkAdapter::new("polkadot-testnet".into(), InkNetwork::PolkadotTestnet);
        let preimage = make_hashlock(b"double_secret");
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(2, hashlock);

        adapter.lock(&intent).expect("lock");
        adapter.claim(2, preimage).expect("first claim");

        let result = adapter.claim(2, preimage);
        assert!(result.is_err(), "double claim should be rejected");
    }

    #[test]
    fn test_stateful_double_refund_rejected() {
        let mut adapter =
            StatefulInkAdapter::new("polkadot-testnet".into(), InkNetwork::PolkadotTestnet);
        let hashlock = make_hashlock(b"refund_test");
        let intent = make_test_intent(3, hashlock);

        adapter.lock(&intent).expect("lock");

        let current_time = intent.source_timeout + 1;
        adapter.refund(3, current_time).expect("first refund");

        let result = adapter.refund(3, current_time);
        assert!(result.is_err(), "double refund should be rejected");
    }

    #[test]
    fn test_stateful_refund_before_timeout_fails() {
        let mut adapter =
            StatefulInkAdapter::new("polkadot-testnet".into(), InkNetwork::PolkadotTestnet);
        let hashlock = make_hashlock(b"too_early");
        let intent = make_test_intent(4, hashlock);

        adapter.lock(&intent).expect("lock");

        let current_time = intent.source_timeout - 1;
        let result = adapter.refund(4, current_time);
        assert!(result.is_err(), "refund before timeout should fail");
    }

    #[test]
    fn test_stateful_claim_wrong_preimage_fails() {
        let mut adapter =
            StatefulInkAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let hashlock = make_hashlock(b"real_secret");
        let wrong_preimage = make_hashlock(b"wrong_secret");
        let intent = make_test_intent(5, hashlock);

        adapter.lock(&intent).expect("lock");
        let result = adapter.claim(5, wrong_preimage);
        assert!(result.is_err(), "wrong preimage should fail");
    }

    #[test]
    fn test_stateful_claim_nonexistent_intent_fails() {
        let mut adapter =
            StatefulInkAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let preimage = make_hashlock(b"ghost");
        let result = adapter.claim(999, preimage);
        assert!(result.is_err(), "claiming nonexistent intent should fail");
    }

    #[test]
    fn test_stateful_lock_duplicate_rejected() {
        let mut adapter =
            StatefulInkAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let hashlock = make_hashlock(b"dup");
        let intent = make_test_intent(10, hashlock);

        adapter.lock(&intent).expect("first lock");
        let result = adapter.lock(&intent);
        assert!(result.is_err(), "duplicate lock should be rejected");
    }

    // ── Readiness Score Tests ─────────────────────────────────────────────

    #[test]
    fn test_readiness_score() {
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let score = adapter.readiness_score();
        assert_eq!(score.score(), 70);
        assert!(score.missing_items().contains(&"event_proof_extraction"));
        assert!(score.missing_items().contains(&"rpc_indexer_support"));
        assert!(score.missing_items().contains(&"proof_ledger_integration"));
    }

    // ── Fee & Finality Tests ──────────────────────────────────────────────

    #[test]
    fn test_estimate_fee() {
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let hashlock = make_hashlock(b"fee_test");
        let intent = make_test_intent(50, hashlock);
        let fee = adapter.estimate_fee(&intent).expect("fee estimate");
        assert_eq!(fee.native_fee, 10_000_000_000_000);
        assert_eq!(fee.vm_type, VmType::InkWasm);
    }

    #[test]
    fn test_finality_status() {
        let mut adapter =
            InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        adapter.finalized_block = 5;
        let tx_id = String::from("tx_abc");
        let status = adapter.finality_status(&tx_id).expect("finality");
        assert_eq!(status.finality_source, "grandpa");
        assert!(status.finalized);
    }

    #[test]
    fn test_chain_health() {
        let adapter = InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        let health = adapter.chain_health().expect("health");
        assert!(health.safe_for_new_intents);
        assert!(!health.halted);
        assert!(!health.degraded);
        assert_eq!(health.block_delay_ms, 12_000);
    }

    #[test]
    fn test_xcm_flag() {
        let mut adapter =
            InkHtlcAdapter::new("polkadot-mainnet".into(), InkNetwork::PolkadotMainnet);
        assert!(adapter.xcm_supported);
        adapter.set_xcm_supported(false);
        assert!(!adapter.xcm_supported);
    }
}
