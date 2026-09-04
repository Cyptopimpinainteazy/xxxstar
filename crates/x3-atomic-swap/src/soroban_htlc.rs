//! # Soroban WASM HTLC Adapter (Stellar)
//!
//! Adapter for Soroban (Stellar's smart contract platform). Implements
//! [`X3VmAdapter`] with mock/placeholder proof structures.
//!
//! In production, [`lock`] would deploy/create an HTLC contract on Stellar,
//! [`claim`] would reveal the preimage to claim funds, and [`refund`] would
//! trigger the refund path after timeout. Finality uses Stellar SCP consensus
//! (ledger close ~5s, pre-verified tx).

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
// Soroban Types
// ─────────────────────────────────────────────────────────────────────────────

/// Soroban network environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SorobanNetwork {
    Mainnet,
    Testnet,
    Futurenet,
}

impl SorobanNetwork {
    /// Get the network name string.
    pub fn name(&self) -> &'static str {
        match self {
            SorobanNetwork::Mainnet => "stellar-mainnet",
            SorobanNetwork::Testnet => "stellar-testnet",
            SorobanNetwork::Futurenet => "stellar-futurenet",
        }
    }

    /// Get the default RPC endpoint for this network.
    pub fn default_rpc(&self) -> &'static str {
        match self {
            SorobanNetwork::Mainnet => "https://rpc.stellar.org",
            SorobanNetwork::Testnet => "https://rpc-testnet.stellar.org",
            SorobanNetwork::Futurenet => "https://rpc-futurenet.stellar.org",
        }
    }
}

/// Lock data stored in a Soroban HTLC contract.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SorobanLockData {
    /// SHA-256 hashlock.
    pub hashlock: [u8; 32],
    /// Owner/locker address (raw bytes).
    pub owner: Vec<u8>,
    /// Receiver/claimant address (raw bytes).
    pub receiver: Vec<u8>,
    /// Refund address (raw bytes).
    pub refund_address: Vec<u8>,
    /// Amount locked (in stroops, 1 XLM = 10^7 stroops).
    pub amount: u128,
    /// Timeout (ledger sequence number or unix timestamp).
    pub timeout: u64,
    /// Whether the lock has been claimed.
    pub claimed: bool,
    /// Whether the lock has been refunded.
    pub refunded: bool,
}

/// Represents a Soroban HTLC smart contract.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SorobanContract {
    /// Contract ID (32-byte hash).
    pub contract_id: [u8; 32],
    /// Lock data for the HTLC.
    pub lock_data: SorobanLockData,
}

// ─────────────────────────────────────────────────────────────────────────────
// SorobanHtlcAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter for Soroban WASM chains (Stellar).
///
/// Uses mock/placeholder proof data. In real operation this would connect to a
/// Stellar RPC node and interact with Soroban HTLC contracts.
///
/// For stateful double-claim/double-refund enforcement, use
/// [`StatefulSorobanAdapter`].
#[derive(Debug, Clone)]
pub struct SorobanHtlcAdapter {
    /// Chain identifier (e.g. "stellar-mainnet", "stellar-testnet").
    pub chain_id: ChainId,
    /// Network variant.
    pub network: SorobanNetwork,
    /// Optional RPC URL.
    pub rpc_url: Option<String>,
    /// Last known ledger sequence number.
    pub last_ledger: u64,
    /// Tracked claimed intent IDs.
    pub claimed_intents: Vec<u64>,
    /// Tracked refunded intent IDs.
    pub refunded_intents: Vec<u64>,
}

/// Internal lock state tracked by the stateful adapter.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct InternalSorobanLock {
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

impl SorobanHtlcAdapter {
    /// Create a new adapter for the given chain identifier.
    ///
    /// Example chain IDs: `"stellar-mainnet"`, `"stellar-testnet"`, `"stellar-futurenet"`.
    pub fn new(chain_id: ChainId, network: SorobanNetwork) -> Self {
        Self {
            chain_id,
            network,
            rpc_url: Some(network.default_rpc().to_string()),
            last_ledger: 0,
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

    /// Generate a mock Stellar G-prefixed address (base32-encoded ed25519 key).
    fn mock_stellar_address(chain_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"x3-soroban-htlc:");
        hasher.update(chain_id.as_bytes());
        let result = hasher.finalize();
        // Stellar addresses start with 'G' followed by base32-encoded ed25519 key.
        // For mock purposes, we use a recognizable format.
        format!("G{}", hex::encode(&result[..30]).to_uppercase())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl X3VmAdapter for SorobanHtlcAdapter {
    fn vm_type(&self) -> VmType {
        VmType::SorobanWasm
    }

    fn adapter_name(&self) -> &'static str {
        "x3-adapter-soroban"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec![
            "stellar-mainnet".into(),
            "stellar-testnet".into(),
            "stellar-futurenet".into(),
        ]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        vec!["XLM".into(), "USDC".into(), "yUSDC".into()]
    }

    // ── Lifecycle operations ──────────────────────────────────────────────

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
        let lock_address = Self::mock_stellar_address(&chain_id);
        let block_number = self.last_ledger + 1;

        let receiver = intent.receiver.as_bytes().to_vec();
        let refund_address = intent.refund_path.address.as_bytes().to_vec();

        Ok(LockProof {
            tx_id,
            chain_id,
            vm_type: VmType::SorobanWasm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            confirmations: 0,
            lock_address,
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver,
            refund_address,
            timeout: intent.source_timeout,
            raw_proof: vec![0x73, 0x6f, 0x72, 0x01], // "sor\x01" - mock proof
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x02);
        let block_number = self.last_ledger + 2;

        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::SorobanWasm,
            preimage,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x73, 0x6f, 0x72, 0x02], // "sor\x02" - mock proof
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x03);
        let block_number = self.last_ledger + 3;

        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::SorobanWasm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x73, 0x6f, 0x72, 0x03], // "sor\x03" - mock proof
        })
    }

    // ── Verification ──────────────────────────────────────────────────────

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::SorobanWasm {
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
        if proof.vm_type != VmType::SorobanWasm {
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
        if proof.vm_type != VmType::SorobanWasm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    // ── Estimation & Health ───────────────────────────────────────────────

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        // ~0.001 XLM for a Soroban contract call (in stroops)
        Ok(FeeEstimate {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::SorobanWasm,
            native_fee: 100_000, // 0.001 XLM in stroops
            gas_units: 0,
            gas_price: 0,
            estimated_usd: 0.001,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        // Stellar SCP consensus: ledger close ~5s, pre-verified tx
        let finalized = self.last_ledger >= 1;
        let safe = self.last_ledger >= 2;

        Ok(FinalityProof {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::SorobanWasm,
            tx_id: tx_id.clone(),
            block_number: self.last_ledger,
            block_hash: hex::encode(Sha256::digest(self.last_ledger.to_le_bytes())),
            confirmations: if finalized { 1 } else { 0 },
            finalized,
            finality_source: "scp".into(),
            safe_to_reveal_secret: safe,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        Ok(ChainHealth {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::SorobanWasm,
            latest_block: self.last_ledger,
            finalized_block: if self.last_ledger >= 1 {
                self.last_ledger
            } else {
                0
            },
            block_delay_ms: 5_000,    // ~5s Stellar block time
            finality_delay_ms: 5_000, // SCP finality is immediate on close
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
            adapter_name: "x3-adapter-soroban",
            vm_type: VmType::SorobanWasm,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: false, // needs actual Soroban event extraction
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

/// A stateful wrapper around [`SorobanHtlcAdapter`] that tracks lock state
/// in order to reject double claims and double refunds.
#[derive(Debug, Clone)]
pub struct StatefulSorobanAdapter {
    pub inner: SorobanHtlcAdapter,
    locks: Vec<InternalSorobanLock>,
}

impl StatefulSorobanAdapter {
    pub fn new(chain_id: ChainId, network: SorobanNetwork) -> Self {
        Self {
            inner: SorobanHtlcAdapter::new(chain_id, network),
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

        self.locks.push(InternalSorobanLock {
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
            source_asset: "XLM".into(),
            destination_asset: "USDC".into(),
            amount_in: 10_000_000_000, // 1000 XLM in stroops
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

    // ── Soroban Network Tests ─────────────────────────────────────────────

    #[test]
    fn test_soroban_network_name() {
        assert_eq!(SorobanNetwork::Mainnet.name(), "stellar-mainnet");
        assert_eq!(SorobanNetwork::Testnet.name(), "stellar-testnet");
        assert_eq!(SorobanNetwork::Futurenet.name(), "stellar-futurenet");
    }

    #[test]
    fn test_soroban_network_default_rpc() {
        assert_eq!(
            SorobanNetwork::Mainnet.default_rpc(),
            "https://rpc.stellar.org"
        );
        assert_eq!(
            SorobanNetwork::Testnet.default_rpc(),
            "https://rpc-testnet.stellar.org"
        );
        assert_eq!(
            SorobanNetwork::Futurenet.default_rpc(),
            "https://rpc-futurenet.stellar.org"
        );
    }

    #[test]
    fn test_soroban_network_equality() {
        assert_eq!(SorobanNetwork::Mainnet, SorobanNetwork::Mainnet);
        assert_ne!(SorobanNetwork::Mainnet, SorobanNetwork::Testnet);
        assert_ne!(SorobanNetwork::Mainnet, SorobanNetwork::Futurenet);
    }

    // ── Soroban Type Tests ────────────────────────────────────────────────

    #[test]
    fn test_soroban_lock_data() {
        let data = SorobanLockData {
            hashlock: [0xabu8; 32],
            owner: vec![0x01],
            receiver: vec![0x02],
            refund_address: vec![0x03],
            amount: 10_000_000,
            timeout: 1000,
            claimed: false,
            refunded: false,
        };
        assert_eq!(data.amount, 10_000_000);
        assert!(!data.claimed);
        assert!(!data.refunded);
    }

    #[test]
    fn test_soroban_contract() {
        let contract = SorobanContract {
            contract_id: [0x42u8; 32],
            lock_data: SorobanLockData {
                hashlock: [0u8; 32],
                owner: vec![],
                receiver: vec![],
                refund_address: vec![],
                amount: 0,
                timeout: 0,
                claimed: false,
                refunded: false,
            },
        };
        assert_eq!(contract.contract_id, [0x42u8; 32]);
    }

    // ── Adapter Identity Tests ────────────────────────────────────────────

    #[test]
    fn test_adapter_identity() {
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);

        assert_eq!(adapter.vm_type(), VmType::SorobanWasm);
        assert_eq!(adapter.adapter_name(), "x3-adapter-soroban");

        let chains = adapter.supported_chains();
        assert!(chains.contains(&"stellar-mainnet".into()));
        assert!(chains.contains(&"stellar-testnet".into()));
        assert!(chains.contains(&"stellar-futurenet".into()));

        let assets = adapter.supported_assets();
        assert!(assets.contains(&"XLM".into()));
        assert!(assets.contains(&"USDC".into()));
        assert!(assets.contains(&"yUSDC".into()));
    }

    #[test]
    fn test_adapter_name_const() {
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let name: &'static str = adapter.adapter_name();
        assert_eq!(name, "x3-adapter-soroban");
    }

    // ── Lock Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_lock_creates_proof() {
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let hashlock = make_hashlock(b"test_preimage");
        let intent = make_test_intent(42, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");

        assert_eq!(proof.vm_type, VmType::SorobanWasm);
        assert_eq!(proof.hashlock, hashlock);
        assert_eq!(proof.locked_amount, intent.amount_in);
        assert!(!proof.tx_id.is_empty());
        assert!(!proof.lock_address.is_empty());
        assert!(proof.lock_address.starts_with('G'));
        assert_eq!(proof.receiver, intent.receiver.as_bytes());
        assert_eq!(proof.refund_address, intent.refund_path.address.as_bytes());
        assert_ne!(proof.block_number, 0);
    }

    #[test]
    fn test_lock_differs_per_intent() {
        let adapter = SorobanHtlcAdapter::new("stellar-testnet".into(), SorobanNetwork::Testnet);
        let h1 = make_hashlock(b"secret1");
        let h2 = make_hashlock(b"secret2");

        let proof1 = adapter.lock(&make_test_intent(1, h1)).expect("lock 1");
        let proof2 = adapter.lock(&make_test_intent(2, h2)).expect("lock 2");

        assert_ne!(proof1.tx_id, proof2.tx_id);
    }

    // ── Claim Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_claim_with_preimage() {
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..5].copy_from_slice(b"claim");
            p
        };

        let proof = adapter.claim(100, preimage).expect("claim should succeed");

        assert_eq!(proof.intent_id, 100);
        assert_eq!(proof.preimage, preimage);
        assert_eq!(proof.vm_type, VmType::SorobanWasm);
        assert!(!proof.tx_id.is_empty());
    }

    // ── Refund Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_refund_after_timeout() {
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let proof = adapter.refund(200).expect("refund should succeed");

        assert_eq!(proof.intent_id, 200);
        assert_eq!(proof.vm_type, VmType::SorobanWasm);
        assert!(!proof.tx_id.is_empty());
        assert_ne!(proof.block_number, 0);
    }

    // ── Verification Tests ────────────────────────────────────────────────

    #[test]
    fn test_verify_valid_lock() {
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let hashlock = make_hashlock(b"valid_lock");
        let intent = make_test_intent(10, hashlock);

        let proof = adapter.lock(&intent).expect("lock");
        let valid = adapter.verify_lock(&proof).expect("verify");

        assert!(valid, "well-formed lock proof should verify");
    }

    #[test]
    fn test_verify_invalid_lock_wrong_vm() {
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let bad_proof = LockProof {
            tx_id: "some_tx".into(),
            chain_id: "stellar-mainnet".into(),
            vm_type: VmType::Evm,
            block_number: 0,
            block_hash: "".into(),
            confirmations: 0,
            lock_address: "GABC".into(),
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
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let bad_proof = LockProof {
            tx_id: String::new(),
            chain_id: "stellar-mainnet".into(),
            vm_type: VmType::SorobanWasm,
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
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "stellar-mainnet".into(),
            vm_type: VmType::SorobanWasm,
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
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
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
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
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
            StatefulSorobanAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
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
            StatefulSorobanAdapter::new("stellar-testnet".into(), SorobanNetwork::Testnet);
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
            StatefulSorobanAdapter::new("stellar-testnet".into(), SorobanNetwork::Testnet);
        let hashlock = make_hashlock(b"refund_test");
        let intent = make_test_intent(3, hashlock);

        adapter.lock(&intent).expect("lock");

        // Refund after timeout
        let current_time = intent.source_timeout + 1;
        adapter.refund(3, current_time).expect("first refund");

        let result = adapter.refund(3, current_time);
        assert!(result.is_err(), "double refund should be rejected");
    }

    #[test]
    fn test_stateful_refund_before_timeout_fails() {
        let mut adapter =
            StatefulSorobanAdapter::new("stellar-testnet".into(), SorobanNetwork::Testnet);
        let hashlock = make_hashlock(b"too_early");
        let intent = make_test_intent(4, hashlock);

        adapter.lock(&intent).expect("lock");

        // Try refund before timeout
        let current_time = intent.source_timeout - 1;
        let result = adapter.refund(4, current_time);
        assert!(result.is_err(), "refund before timeout should fail");
    }

    #[test]
    fn test_stateful_claim_wrong_preimage_fails() {
        let mut adapter =
            StatefulSorobanAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
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
            StatefulSorobanAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let preimage = make_hashlock(b"ghost");
        let result = adapter.claim(999, preimage);
        assert!(result.is_err(), "claiming nonexistent intent should fail");
    }

    #[test]
    fn test_stateful_lock_duplicate_rejected() {
        let mut adapter =
            StatefulSorobanAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let hashlock = make_hashlock(b"dup");
        let intent = make_test_intent(10, hashlock);

        adapter.lock(&intent).expect("first lock");
        let result = adapter.lock(&intent);
        assert!(result.is_err(), "duplicate lock should be rejected");
    }

    // ── Readiness Score Tests ─────────────────────────────────────────────

    #[test]
    fn test_readiness_score() {
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let score = adapter.readiness_score();
        assert_eq!(score.score(), 70);
        assert!(score.missing_items().contains(&"event_proof_extraction"));
        assert!(score.missing_items().contains(&"rpc_indexer_support"));
        assert!(score.missing_items().contains(&"proof_ledger_integration"));
    }

    // ── Fee & Finality Tests ──────────────────────────────────────────────

    #[test]
    fn test_estimate_fee() {
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let hashlock = make_hashlock(b"fee_test");
        let intent = make_test_intent(50, hashlock);
        let fee = adapter.estimate_fee(&intent).expect("fee estimate");
        assert_eq!(fee.native_fee, 100_000);
        assert_eq!(fee.vm_type, VmType::SorobanWasm);
    }

    #[test]
    fn test_finality_status() {
        let mut adapter =
            SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        adapter.last_ledger = 5;
        let tx_id = String::from("tx_abc");
        let status = adapter.finality_status(&tx_id).expect("finality");
        assert_eq!(status.finality_source, "scp");
        assert!(status.finalized);
    }

    #[test]
    fn test_chain_health() {
        let adapter = SorobanHtlcAdapter::new("stellar-mainnet".into(), SorobanNetwork::Mainnet);
        let health = adapter.chain_health().expect("health");
        assert!(health.safe_for_new_intents);
        assert!(!health.halted);
        assert!(!health.degraded);
        assert_eq!(health.block_delay_ms, 5_000);
    }
}
