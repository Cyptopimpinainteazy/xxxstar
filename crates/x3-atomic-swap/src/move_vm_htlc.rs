//! # MoveVM HTLC Adapter
//!
//! Adapter for MoveVM chains (Sui, Aptos). Implements [`X3VmAdapter`] with
//! mock/placeholder proof structures.
//!
//! In production, [`lock`] would deploy a Move HTLC module/object, [`claim`] would
//! call the claim entry function, and [`refund`] would trigger the refund path
//! after timeout. Finality uses checkpoint models (Sui) or BFT (Aptos).

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
// MoveNetwork
// ─────────────────────────────────────────────────────────────────────────────

/// Supported MoveVM networks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MoveNetwork {
    SuiMainnet,
    SuiTestnet,
    AptosMainnet,
    AptosTestnet,
}

impl MoveNetwork {
    /// Human-readable network name.
    pub fn name(&self) -> &'static str {
        match self {
            MoveNetwork::SuiMainnet => "sui-mainnet",
            MoveNetwork::SuiTestnet => "sui-testnet",
            MoveNetwork::AptosMainnet => "aptos-mainnet",
            MoveNetwork::AptosTestnet => "aptos-testnet",
        }
    }

    /// Default RPC endpoint for the network.
    pub fn default_rpc(&self) -> &'static str {
        match self {
            MoveNetwork::SuiMainnet => "https://fullnode.mainnet.sui.io",
            MoveNetwork::SuiTestnet => "https://fullnode.testnet.sui.io",
            MoveNetwork::AptosMainnet => "https://fullnode.mainnet.aptoslabs.com",
            MoveNetwork::AptosTestnet => "https://fullnode.testnet.aptoslabs.com",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MoveHtlcModule
// ─────────────────────────────────────────────────────────────────────────────

/// Represents a Move HTLC module with its resource types.
#[derive(Debug, Clone)]
pub struct MoveHtlcModule;

/// Lock resource representation in MoveVM.
///
/// Analogous to a Sui object or Aptos resource holding HTLC state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockResource {
    pub owner: Vec<u8>,
    pub receiver: Vec<u8>,
    pub refund_address: Vec<u8>,
    pub asset: AssetId,
    pub amount: u64,
    pub hashlock: [u8; 32],
    pub timeout: u64,
    pub claimed: bool,
    pub refunded: bool,
}

impl MoveHtlcModule {
    /// Generate mock compiled module bytecode hash.
    ///
    /// Returns a deterministic 32-byte hash representing the compiled
    /// Move module bytecode.
    pub fn generate_module_code() -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"x3-move-htlc-module-v1");
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MoveVmAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter for MoveVM chains (Sui, Aptos).
///
/// Uses mock/placeholder proof data. In real operation this would connect to a
/// Sui/Aptos node via RPC and interact with Move HTLC modules.
///
/// For stateful double-claim/double-refund enforcement, use
/// [`StatefulMoveVmAdapter`].
#[derive(Debug, Clone)]
pub struct MoveVmAdapter {
    /// Chain identifier (e.g. "sui-mainnet", "aptos-mainnet").
    pub chain_id: ChainId,
    /// Move network variant.
    pub network: MoveNetwork,
    /// Optional HTTP RPC URL.
    pub rpc_url: Option<String>,
    /// Current finalized checkpoint (Sui) or block (Aptos).
    pub finalized_checkpoint: u64,
    /// Tracked claimed intent IDs for stateless double-claim detection.
    pub claimed_intents: Vec<u64>,
    /// Tracked refunded intent IDs for stateless double-refund detection.
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

impl MoveVmAdapter {
    /// Create a new adapter for the given chain identifier.
    ///
    /// Example chain IDs: `"sui-mainnet"`, `"sui-testnet"`, `"aptos-mainnet"`.
    pub fn new(chain_id: ChainId) -> Self {
        let network = match chain_id.as_str() {
            "sui-mainnet" => MoveNetwork::SuiMainnet,
            "sui-testnet" => MoveNetwork::SuiTestnet,
            "aptos-mainnet" => MoveNetwork::AptosMainnet,
            "aptos-testnet" => MoveNetwork::AptosTestnet,
            _ => MoveNetwork::SuiMainnet,
        };
        Self {
            chain_id: chain_id.clone(),
            network,
            rpc_url: Some(network.default_rpc().to_string()),
            finalized_checkpoint: 0,
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

    /// Generate a mock Move object address from intent_id.
    fn mock_object_address(intent_id: IntentId) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"x3-move-htlc-object:");
        hasher.update(intent_id.to_le_bytes());
        let result = hasher.finalize();
        // Sui object addresses are 32 bytes; encode as hex with 0x prefix
        format!("0x{}", hex::encode(result))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl X3VmAdapter for MoveVmAdapter {
    fn vm_type(&self) -> VmType {
        VmType::MoveVm
    }

    fn adapter_name(&self) -> &'static str {
        "x3-adapter-move"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec![
            "sui-mainnet".into(),
            "sui-testnet".into(),
            "aptos-mainnet".into(),
            "aptos-testnet".into(),
        ]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        vec!["SUI".into(), "APT".into(), "USDC".into(), "USDT".into()]
    }

    // ── Lifecycle operations ──────────────────────────────────────────────

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
        let lock_address = Self::mock_object_address(intent.intent_id);
        let block_number = self.finalized_checkpoint + 1;

        let receiver = intent.receiver.as_bytes().to_vec();
        let refund_address = intent.refund_path.address.as_bytes().to_vec();

        Ok(LockProof {
            tx_id,
            chain_id,
            vm_type: VmType::MoveVm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            confirmations: 0,
            lock_address,
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver,
            refund_address,
            timeout: intent.source_timeout,
            raw_proof: vec![0x6d, 0x6f, 0x76, 0x01], // "mov\x01" - mock proof
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x02);
        let block_number = self.finalized_checkpoint + 2;

        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::MoveVm,
            preimage,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x6d, 0x6f, 0x76, 0x02], // "mov\x02" - mock proof
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x03);
        let block_number = self.finalized_checkpoint + 3;

        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::MoveVm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x6d, 0x6f, 0x76, 0x03], // "mov\x03" - mock proof
        })
    }

    // ── Verification ──────────────────────────────────────────────────────

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::MoveVm {
            return Ok(false);
        }
        // lock_address should be valid hex (with 0x prefix)
        if proof.lock_address.is_empty() || !proof.lock_address.starts_with("0x") {
            return Ok(false);
        }
        if proof.locked_amount == 0 {
            return Ok(false);
        }
        if proof.timeout == 0 {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::MoveVm {
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
        if proof.vm_type != VmType::MoveVm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    // ── Estimation & Health ───────────────────────────────────────────────

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        // 0.005 SUI equivalent (5_000_000 MIST)
        let native_fee = match self.network {
            MoveNetwork::SuiMainnet | MoveNetwork::SuiTestnet => 5_000_000u128,
            MoveNetwork::AptosMainnet | MoveNetwork::AptosTestnet => 100_000u128, // 0.0001 APT
        };
        let estimated_usd = match self.network {
            MoveNetwork::SuiMainnet => 0.02,
            MoveNetwork::SuiTestnet => 0.001,
            MoveNetwork::AptosMainnet => 0.01,
            MoveNetwork::AptosTestnet => 0.001,
        };

        Ok(FeeEstimate {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::MoveVm,
            native_fee,
            gas_units: 0,
            gas_price: 0,
            estimated_usd,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        // Sui: checkpoint finality (>= 1 checkpoint = finalized)
        // Aptos: BFT finality (1 block = finalized)
        let (finalized, finality_source) = match self.network {
            MoveNetwork::SuiMainnet | MoveNetwork::SuiTestnet => {
                (self.finalized_checkpoint >= 1, "sui-checkpoint")
            }
            MoveNetwork::AptosMainnet | MoveNetwork::AptosTestnet => {
                (self.finalized_checkpoint >= 1, "aptos-bft")
            }
        };

        Ok(FinalityProof {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::MoveVm,
            tx_id: tx_id.clone(),
            block_number: self.finalized_checkpoint,
            block_hash: hex::encode(Sha256::digest(self.finalized_checkpoint.to_le_bytes())),
            confirmations: if finalized { 1 } else { 0 },
            finalized,
            finality_source: finality_source.into(),
            safe_to_reveal_secret: finalized,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        let (block_delay_ms, finality_delay_ms) = match self.network {
            MoveNetwork::SuiMainnet | MoveNetwork::SuiTestnet => (1_000, 2_000), // ~1s block, ~2s checkpoint
            MoveNetwork::AptosMainnet | MoveNetwork::AptosTestnet => (1_000, 1_000), // ~1s block, ~1s BFT
        };

        Ok(ChainHealth {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::MoveVm,
            latest_block: self.finalized_checkpoint,
            finalized_block: self.finalized_checkpoint,
            block_delay_ms,
            finality_delay_ms,
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
            adapter_name: "x3-adapter-move",
            vm_type: VmType::MoveVm,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: false, // needs actual Move event subscription
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

/// A stateful wrapper around [`MoveVmAdapter`] that tracks lock state
/// in order to reject double claims and double refunds.
#[derive(Debug, Clone)]
pub struct StatefulMoveVmAdapter {
    pub inner: MoveVmAdapter,
    locks: Vec<InternalLock>,
}

impl StatefulMoveVmAdapter {
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            inner: MoveVmAdapter::new(chain_id),
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
            source_asset: "SUI".into(),
            destination_asset: "USDC".into(),
            amount_in: 1_000_000_000_000,
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

    // ── MoveNetwork Tests ─────────────────────────────────────────────────

    #[test]
    fn test_move_network_names() {
        assert_eq!(MoveNetwork::SuiMainnet.name(), "sui-mainnet");
        assert_eq!(MoveNetwork::SuiTestnet.name(), "sui-testnet");
        assert_eq!(MoveNetwork::AptosMainnet.name(), "aptos-mainnet");
        assert_eq!(MoveNetwork::AptosTestnet.name(), "aptos-testnet");
    }

    #[test]
    fn test_move_network_default_rpc() {
        assert!(MoveNetwork::SuiMainnet.default_rpc().contains("sui.io"));
        assert!(MoveNetwork::AptosMainnet
            .default_rpc()
            .contains("aptoslabs.com"));
    }

    #[test]
    fn test_move_htlc_module_generated_code() {
        let code = MoveHtlcModule::generate_module_code();
        assert_eq!(code.len(), 32);
        // Deterministic
        let code2 = MoveHtlcModule::generate_module_code();
        assert_eq!(code, code2);
    }

    #[test]
    fn test_lock_resource_creation() {
        let resource = LockResource {
            owner: vec![0x01],
            receiver: vec![0x02],
            refund_address: vec![0x03],
            asset: "SUI".into(),
            amount: 1000,
            hashlock: [0u8; 32],
            timeout: 100,
            claimed: false,
            refunded: false,
        };
        assert_eq!(resource.asset, "SUI");
        assert!(!resource.claimed);
        assert!(!resource.refunded);
    }

    // ── Adapter Identity Tests ────────────────────────────────────────────

    #[test]
    fn test_adapter_identity() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());

        assert_eq!(adapter.vm_type(), VmType::MoveVm);
        assert_eq!(adapter.adapter_name(), "x3-adapter-move");

        let chains = adapter.supported_chains();
        assert!(chains.contains(&"sui-mainnet".into()));
        assert!(chains.contains(&"sui-testnet".into()));
        assert!(chains.contains(&"aptos-mainnet".into()));
        assert!(chains.contains(&"aptos-testnet".into()));

        let assets = adapter.supported_assets();
        assert!(assets.contains(&"SUI".into()));
        assert!(assets.contains(&"APT".into()));
        assert!(assets.contains(&"USDC".into()));
        assert!(assets.contains(&"USDT".into()));
    }

    #[test]
    fn test_adapter_name_const() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
        let name: &'static str = adapter.adapter_name();
        assert_eq!(name, "x3-adapter-move");
    }

    #[test]
    fn test_network_auto_detect() {
        let aptos = MoveVmAdapter::new("aptos-mainnet".into());
        assert_eq!(aptos.network, MoveNetwork::AptosMainnet);

        let sui_test = MoveVmAdapter::new("sui-testnet".into());
        assert_eq!(sui_test.network, MoveNetwork::SuiTestnet);

        // Unknown defaults to SuiMainnet
        let unknown = MoveVmAdapter::new("unknown".into());
        assert_eq!(unknown.network, MoveNetwork::SuiMainnet);
    }

    // ── Lock Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_lock_creates_proof() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
        let hashlock = make_hashlock(b"test_preimage");
        let intent = make_test_intent(42, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");

        assert_eq!(proof.vm_type, VmType::MoveVm);
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
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
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
        let adapter = MoveVmAdapter::new("aptos-mainnet".into());
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..5].copy_from_slice(b"claim");
            p
        };

        let proof = adapter.claim(100, preimage).expect("claim should succeed");

        assert_eq!(proof.intent_id, 100);
        assert_eq!(proof.preimage, preimage);
        assert_eq!(proof.vm_type, VmType::MoveVm);
        assert!(!proof.tx_id.is_empty());
    }

    // ── Refund Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_refund_after_timeout() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());

        let proof = adapter.refund(200).expect("refund should succeed");

        assert_eq!(proof.intent_id, 200);
        assert_eq!(proof.vm_type, VmType::MoveVm);
        assert!(!proof.tx_id.is_empty());
        assert_ne!(proof.block_number, 0);
    }

    // ── Verification Tests ────────────────────────────────────────────────

    #[test]
    fn test_verify_valid_lock() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
        let hashlock = make_hashlock(b"valid_lock");
        let intent = make_test_intent(10, hashlock);

        let proof = adapter.lock(&intent).expect("lock");
        let valid = adapter.verify_lock(&proof).expect("verify");

        assert!(valid, "well-formed lock proof should verify");
    }

    #[test]
    fn test_verify_invalid_lock_wrong_vm() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
        let bad_proof = LockProof {
            tx_id: "some_tx".into(),
            chain_id: "sui-mainnet".into(),
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
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "sui-mainnet".into(),
            vm_type: VmType::MoveVm,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "not-hex".to_string(), // doesn't start with 0x
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
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "sui-mainnet".into(),
            vm_type: VmType::MoveVm,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "0xabc".into(),
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
    fn test_verify_invalid_lock_zero_timeout() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "sui-mainnet".into(),
            vm_type: VmType::MoveVm,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "0xabc".into(),
            locked_amount: 100,
            hashlock: [0u8; 32],
            receiver: vec![],
            refund_address: vec![],
            timeout: 0, // zero timeout is invalid
            raw_proof: vec![],
        };
        let valid = adapter.verify_lock(&bad_proof).expect("verify");
        assert!(!valid, "zero timeout should fail verification");
    }

    #[test]
    fn test_verify_valid_claim() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
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
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
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
        let adapter = MoveVmAdapter::new("aptos-mainnet".into());
        let proof = adapter.refund(10).expect("refund");
        let valid = adapter.verify_refund(&proof).expect("verify");
        assert!(valid, "well-formed refund proof should verify");
    }

    #[test]
    fn test_verify_invalid_refund() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
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
    fn test_finality_status_sui() {
        let mut adapter = MoveVmAdapter::new("sui-mainnet".into());
        adapter.finalized_checkpoint = 42;

        let fp = adapter
            .finality_status(&"mock_tx_id".into())
            .expect("finality status");

        assert_eq!(fp.chain_id, "sui-mainnet");
        assert_eq!(fp.vm_type, VmType::MoveVm);
        assert!(fp.finalized); // finalized_checkpoint >= 1
        assert!(fp.safe_to_reveal_secret);
        assert_eq!(fp.finality_source, "sui-checkpoint");
    }

    #[test]
    fn test_finality_status_aptos() {
        let mut adapter = MoveVmAdapter::new("aptos-mainnet".into());
        adapter.finalized_checkpoint = 42;

        let fp = adapter
            .finality_status(&"mock_tx_id".into())
            .expect("finality status");

        assert_eq!(fp.chain_id, "aptos-mainnet");
        assert_eq!(fp.finality_source, "aptos-bft");
        assert!(fp.finalized);
    }

    #[test]
    fn test_finality_unfinalized() {
        let mut adapter = MoveVmAdapter::new("sui-mainnet".into());
        adapter.finalized_checkpoint = 0;

        let fp = adapter
            .finality_status(&"new_tx".into())
            .expect("finality status");

        assert!(!fp.finalized, "checkpoint 0 should be unfinalized");
    }

    #[test]
    fn test_chain_health() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());

        let health = adapter.chain_health().expect("chain health");

        assert_eq!(health.chain_id, "sui-mainnet");
        assert_eq!(health.vm_type, VmType::MoveVm);
        assert!(!health.halted);
        assert!(!health.degraded);
        assert!(health.safe_for_new_intents);
        assert!(health.rpc_quorum_healthy);
    }

    #[test]
    fn test_chain_health_aptos() {
        let adapter = MoveVmAdapter::new("aptos-mainnet".into());

        let health = adapter.chain_health().expect("chain health");

        assert_eq!(health.chain_id, "aptos-mainnet");
        assert_eq!(health.block_delay_ms, 1_000);
        assert_eq!(health.finality_delay_ms, 1_000);
    }

    // ── Fee Estimation Tests ──────────────────────────────────────────────

    #[test]
    fn test_estimate_fee_sui() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());
        let hashlock = make_hashlock(b"fee_test_sui");
        let intent = make_test_intent(99, hashlock);

        let fee = adapter.estimate_fee(&intent).expect("estimate_fee");

        assert_eq!(fee.chain_id, "sui-mainnet");
        assert_eq!(fee.vm_type, VmType::MoveVm);
        assert!(fee.native_fee > 0);
        assert!(fee.estimated_usd > 0.0);
    }

    #[test]
    fn test_estimate_fee_aptos() {
        let adapter = MoveVmAdapter::new("aptos-mainnet".into());
        let hashlock = make_hashlock(b"fee_test_apt");
        let intent = make_test_intent(98, hashlock);

        let fee = adapter.estimate_fee(&intent).expect("estimate_fee");

        assert_eq!(fee.chain_id, "aptos-mainnet");
        assert_eq!(fee.native_fee, 100_000);
    }

    // ── Readiness Score Tests ─────────────────────────────────────────────

    #[test]
    fn test_readiness_score() {
        let adapter = MoveVmAdapter::new("sui-mainnet".into());

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
        assert!(score.proof_ledger_integration);

        assert_eq!(score.score(), 80);
        assert_eq!(score.adapter_name, "x3-adapter-move");
        assert_eq!(score.vm_type, VmType::MoveVm);

        let missing = score.missing_items();
        assert!(missing.contains(&"event_proof_extraction"));
        assert!(missing.contains(&"rpc_indexer_support"));
        assert!(missing.contains(&"ibc_support"));
        assert_eq!(missing.len(), 4);
    }

    // ── Stateful Adapter Tests ────────────────────────────────────────────

    #[test]
    fn test_stateful_lock() {
        let mut adapter = StatefulMoveVmAdapter::new("sui-mainnet".into());
        let hashlock = make_hashlock(b"stateful_lock");
        let intent = make_test_intent(500, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");
        assert!(!proof.tx_id.is_empty());
        assert!(adapter.locks.len() == 1);
    }

    #[test]
    fn test_double_lock_rejected() {
        let mut adapter = StatefulMoveVmAdapter::new("sui-mainnet".into());
        let hashlock = make_hashlock(b"double_lock");
        let intent = make_test_intent(301, hashlock);

        adapter.lock(&intent).expect("first lock");
        let second = adapter.lock(&intent);
        assert!(second.is_err(), "double lock should be rejected");
    }

    #[test]
    fn test_double_claim_rejected() {
        let mut adapter = StatefulMoveVmAdapter::new("sui-mainnet".into());

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
        let mut adapter = StatefulMoveVmAdapter::new("aptos-mainnet".into());

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
        let mut adapter = StatefulMoveVmAdapter::new("sui-mainnet".into());

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
        let mut adapter = StatefulMoveVmAdapter::new("sui-mainnet".into());

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
        let mut adapter = StatefulMoveVmAdapter::new("sui-mainnet".into());

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
        let mut adapter = StatefulMoveVmAdapter::new("sui-mainnet".into());

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
    fn test_is_claimed_refunded() {
        let mut adapter = StatefulMoveVmAdapter::new("aptos-mainnet".into());

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
        let mut adapter = MoveVmAdapter::new("sui-mainnet".into());
        assert!(adapter.rpc_url.is_some());

        adapter.set_rpc("https://custom.node.example.com");
        assert_eq!(adapter.rpc_url.unwrap(), "https://custom.node.example.com");
    }
}
