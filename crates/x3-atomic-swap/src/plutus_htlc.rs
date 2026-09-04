//! # Plutus/eUTXO HTLC Adapter (Cardano)
//!
//! Adapter for Cardano's Plutus/eUTXO chains. Implements [`X3VmAdapter`]
//! with mock/placeholder proof structures.
//!
//! In production, [`lock`] would create a Plutus script UTXO with the given
//! datum (hashlock, receiver, refund_address, timeout), [`claim`] would spend
//! that UTXO with a Claim redeemer providing the preimage, and [`refund`] would
//! spend it with a Refund redeemer after timeout. Finality uses Cardano's
//! Ouroboros consensus (minimum 2 confirmations for safe, standard 6).

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
// Plutus Types
// ─────────────────────────────────────────────────────────────────────────────

/// Cardano network environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PlutusNetwork {
    Mainnet,
    Preprod,
    Preview,
}

impl PlutusNetwork {
    pub fn name(&self) -> &'static str {
        match self {
            PlutusNetwork::Mainnet => "cardano-mainnet",
            PlutusNetwork::Preprod => "cardano-preprod",
            PlutusNetwork::Preview => "cardano-preview",
        }
    }
}

/// Plutus datum for an HTLC smart contract.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlutusDatum {
    pub hashlock: [u8; 32],
    pub receiver_bytes: Vec<u8>,
    pub refund_bytes: Vec<u8>,
    pub timeout: u64,
}

/// Plutus redeemer for spending an HTLC UTXO.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlutusRedeemer {
    Claim { preimage: [u8; 32] },
    Refund {},
}

/// Represents a Plutus script controlling locked funds on Cardano.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlutusScript {
    pub script_hash: String,
    pub datum: PlutusDatum,
    pub address: String,
    pub lovelace_amount: u128,
}

// ─────────────────────────────────────────────────────────────────────────────
// PlutusHtlcAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter for Cardano's Plutus/eUTXO chains.
///
/// Uses mock/placeholder proof data. In real operation this would connect to a
/// Cardano node via Ogmios/Kupo/CardanoDB and interact with Plutus scripts.
///
/// For stateful double-claim/double-refund enforcement, use
/// [`StatefulPlutusAdapter`].
#[derive(Debug, Clone)]
pub struct PlutusHtlcAdapter {
    /// Chain identifier (e.g. "cardano-mainnet", "cardano-preprod").
    pub chain_id: ChainId,
    /// Network variant.
    pub network: PlutusNetwork,
    /// Optional RPC URL for node access.
    pub rpc_url: Option<String>,
    /// Last known slot number.
    pub last_slot: u64,
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

impl PlutusHtlcAdapter {
    /// Create a new adapter for the given chain identifier and network.
    ///
    /// Example chain IDs: `"cardano-mainnet"`, `"cardano-preprod"`, `"cardano-preview"`.
    pub fn new(chain_id: ChainId, network: PlutusNetwork) -> Self {
        Self {
            chain_id,
            network,
            rpc_url: None,
            last_slot: 0,
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

    /// Generate a mock Cardano address from chain_id.
    fn mock_address(chain_id: &str, network: &PlutusNetwork) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"x3-plutus-htlc:");
        hasher.update(chain_id.as_bytes());
        let result = hasher.finalize();
        let prefix = match network {
            PlutusNetwork::Mainnet => "addr1",
            PlutusNetwork::Preprod | PlutusNetwork::Preview => "addr_test",
        };
        format!("{}{}", prefix, hex::encode(&result[..20]))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl X3VmAdapter for PlutusHtlcAdapter {
    fn vm_type(&self) -> VmType {
        VmType::PlutusEutxo
    }

    fn adapter_name(&self) -> &'static str {
        "x3-adapter-plutus"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec![
            "cardano-mainnet".into(),
            "cardano-preprod".into(),
            "cardano-preview".into(),
        ]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        vec!["ADA".into(), "USDM".into(), "iUSD".into()]
    }

    // ── Lifecycle operations ──────────────────────────────────────────────

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
        let lock_address = Self::mock_address(&chain_id, &self.network);
        let block_number = self.last_slot + 1;

        let receiver = intent.receiver.as_bytes().to_vec();
        let refund_address = intent.refund_path.address.as_bytes().to_vec();

        Ok(LockProof {
            tx_id,
            chain_id,
            vm_type: VmType::PlutusEutxo,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            confirmations: 0,
            lock_address,
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver,
            refund_address,
            timeout: intent.source_timeout,
            raw_proof: vec![0x70, 0x6c, 0x75, 0x01], // "plu\x01" - mock proof
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x02);
        let block_number = self.last_slot + 2;

        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::PlutusEutxo,
            preimage,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x70, 0x6c, 0x75, 0x02], // "plu\x02" - mock proof
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x03);
        let block_number = self.last_slot + 3;

        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::PlutusEutxo,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x70, 0x6c, 0x75, 0x03], // "plu\x03" - mock proof
        })
    }

    // ── Verification ──────────────────────────────────────────────────────

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::PlutusEutxo {
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
        if proof.vm_type != VmType::PlutusEutxo {
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
        if proof.vm_type != VmType::PlutusEutxo {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    // ── Estimation & Health ───────────────────────────────────────────────

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        // ~0.17 ADA for a simple Plutus script execution
        Ok(FeeEstimate {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::PlutusEutxo,
            native_fee: 170_000, // 0.17 ADA in lovelace
            gas_units: 0,
            gas_price: 0,
            estimated_usd: 0.12,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        // Cardano Ouroboros: min 2 confirmations safe, standard 6
        let confirmations = if self.last_slot >= 6 {
            6
        } else {
            self.last_slot
        };
        let finalized = confirmations >= 2;
        let safe = confirmations >= 6;

        Ok(FinalityProof {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::PlutusEutxo,
            tx_id: tx_id.clone(),
            block_number: self.last_slot,
            block_hash: hex::encode(Sha256::digest(self.last_slot.to_le_bytes())),
            confirmations,
            finalized,
            finality_source: "ouroboros".into(),
            safe_to_reveal_secret: safe,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        Ok(ChainHealth {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::PlutusEutxo,
            latest_block: self.last_slot,
            finalized_block: self.last_slot.saturating_sub(2),
            block_delay_ms: 20_000,    // ~20s Cardano block time
            finality_delay_ms: 40_000, // ~2 blocks for safe finality
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
            adapter_name: "x3-adapter-plutus",
            vm_type: VmType::PlutusEutxo,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: false, // needs actual Plutus event extraction
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

/// A stateful wrapper around [`PlutusHtlcAdapter`] that tracks lock state
/// in order to reject double claims and double refunds.
#[derive(Debug, Clone)]
pub struct StatefulPlutusAdapter {
    pub inner: PlutusHtlcAdapter,
    locks: Vec<InternalLock>,
}

impl StatefulPlutusAdapter {
    pub fn new(chain_id: ChainId, network: PlutusNetwork) -> Self {
        Self {
            inner: PlutusHtlcAdapter::new(chain_id, network),
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
            source_asset: "ADA".into(),
            destination_asset: "USDC".into(),
            amount_in: 10_000_000, // 10 ADA in lovelace
            min_amount_out: 500_000_000,
            receiver: "addr1abc123def456".into(),
            hashlock,
            source_timeout: 1_800_000,
            destination_timeout: 1_700_000,
            finality_requirements: vec![FinalityRequirement {
                chain: ChainKind::X3,
                level: FinalityLevel::Bft,
            }],
            refund_path: RefundPath {
                chain: ChainKind::X3,
                address: "addr_test1refund789".into(),
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

    // ── Plutus Type Tests ─────────────────────────────────────────────────

    #[test]
    fn test_plutus_network_name() {
        assert_eq!(PlutusNetwork::Mainnet.name(), "cardano-mainnet");
        assert_eq!(PlutusNetwork::Preprod.name(), "cardano-preprod");
        assert_eq!(PlutusNetwork::Preview.name(), "cardano-preview");
    }

    #[test]
    fn test_plutus_network_equality() {
        assert_eq!(PlutusNetwork::Mainnet, PlutusNetwork::Mainnet);
        assert_ne!(PlutusNetwork::Mainnet, PlutusNetwork::Preprod);
    }

    #[test]
    fn test_plutus_datum() {
        let datum = PlutusDatum {
            hashlock: [0xabu8; 32],
            receiver_bytes: vec![0x01, 0x02],
            refund_bytes: vec![0x03, 0x04],
            timeout: 1000,
        };
        assert_eq!(datum.hashlock[0], 0xab);
        assert_eq!(datum.timeout, 1000);
    }

    #[test]
    fn test_plutus_redeemer_claim() {
        let preimage = [0x42u8; 32];
        let redeemer = PlutusRedeemer::Claim { preimage };
        match redeemer {
            PlutusRedeemer::Claim { preimage: p } => {
                assert_eq!(p[0], 0x42);
            }
            _ => panic!("expected Claim variant"),
        }
    }

    #[test]
    fn test_plutus_redeemer_refund() {
        let redeemer = PlutusRedeemer::Refund {};
        match redeemer {
            PlutusRedeemer::Refund {} => {}
            _ => panic!("expected Refund variant"),
        }
    }

    #[test]
    fn test_plutus_script() {
        let script = PlutusScript {
            script_hash: "abc123".into(),
            datum: PlutusDatum {
                hashlock: [0u8; 32],
                receiver_bytes: vec![],
                refund_bytes: vec![],
                timeout: 0,
            },
            address: "addr1test".into(),
            lovelace_amount: 5_000_000,
        };
        assert_eq!(script.script_hash, "abc123");
        assert_eq!(script.lovelace_amount, 5_000_000);
    }

    // ── Adapter Identity Tests ────────────────────────────────────────────

    #[test]
    fn test_adapter_identity() {
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);

        assert_eq!(adapter.vm_type(), VmType::PlutusEutxo);
        assert_eq!(adapter.adapter_name(), "x3-adapter-plutus");

        let chains = adapter.supported_chains();
        assert!(chains.contains(&"cardano-mainnet".into()));
        assert!(chains.contains(&"cardano-preprod".into()));
        assert!(chains.contains(&"cardano-preview".into()));

        let assets = adapter.supported_assets();
        assert!(assets.contains(&"ADA".into()));
        assert!(assets.contains(&"USDM".into()));
        assert!(assets.contains(&"iUSD".into()));
    }

    #[test]
    fn test_adapter_name_const() {
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let name: &'static str = adapter.adapter_name();
        assert_eq!(name, "x3-adapter-plutus");
    }

    // ── Lock Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_lock_creates_proof() {
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let hashlock = make_hashlock(b"test_preimage");
        let intent = make_test_intent(42, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");

        assert_eq!(proof.vm_type, VmType::PlutusEutxo);
        assert_eq!(proof.hashlock, hashlock);
        assert_eq!(proof.locked_amount, intent.amount_in);
        assert!(!proof.tx_id.is_empty());
        assert!(!proof.lock_address.is_empty());
        assert!(
            proof.lock_address.starts_with("addr1") || proof.lock_address.starts_with("addr_test")
        );
        assert_eq!(proof.receiver, intent.receiver.as_bytes());
        assert_eq!(proof.refund_address, intent.refund_path.address.as_bytes());
        assert_ne!(proof.block_number, 0);
    }

    #[test]
    fn test_lock_differs_per_intent() {
        let adapter = PlutusHtlcAdapter::new("cardano-preprod".into(), PlutusNetwork::Preprod);
        let h1 = make_hashlock(b"secret1");
        let h2 = make_hashlock(b"secret2");

        let proof1 = adapter.lock(&make_test_intent(1, h1)).expect("lock 1");
        let proof2 = adapter.lock(&make_test_intent(2, h2)).expect("lock 2");

        assert_ne!(proof1.tx_id, proof2.tx_id);
    }

    // ── Claim Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_claim_with_preimage() {
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..5].copy_from_slice(b"claim");
            p
        };

        let proof = adapter.claim(100, preimage).expect("claim should succeed");

        assert_eq!(proof.intent_id, 100);
        assert_eq!(proof.preimage, preimage);
        assert_eq!(proof.vm_type, VmType::PlutusEutxo);
        assert!(!proof.tx_id.is_empty());
    }

    // ── Refund Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_refund_after_timeout() {
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let proof = adapter.refund(200).expect("refund should succeed");

        assert_eq!(proof.intent_id, 200);
        assert_eq!(proof.vm_type, VmType::PlutusEutxo);
        assert!(!proof.tx_id.is_empty());
        assert_ne!(proof.block_number, 0);
    }

    // ── Verification Tests ────────────────────────────────────────────────

    #[test]
    fn test_verify_valid_lock() {
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let hashlock = make_hashlock(b"valid_lock");
        let intent = make_test_intent(10, hashlock);

        let proof = adapter.lock(&intent).expect("lock");
        let valid = adapter.verify_lock(&proof).expect("verify");

        assert!(valid, "well-formed lock proof should verify");
    }

    #[test]
    fn test_verify_invalid_lock_wrong_vm() {
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let bad_proof = LockProof {
            tx_id: "some_tx".into(),
            chain_id: "cardano-mainnet".into(),
            vm_type: VmType::Evm, // wrong!
            block_number: 0,
            block_hash: "".into(),
            confirmations: 0,
            lock_address: "addr1xxx".into(),
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
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let bad_proof = LockProof {
            tx_id: String::new(),
            chain_id: "cardano-mainnet".into(),
            vm_type: VmType::PlutusEutxo,
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
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "cardano-mainnet".into(),
            vm_type: VmType::PlutusEutxo,
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
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
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
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
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
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let proof = adapter.refund(10).expect("refund");
        let valid = adapter.verify_refund(&proof).expect("verify");
        assert!(valid, "well-formed refund proof should verify");
    }

    #[test]
    fn test_verify_invalid_refund() {
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
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
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let fp = adapter
            .finality_status(&"mock_tx_id".into())
            .expect("finality status");

        assert_eq!(fp.chain_id, "cardano-mainnet");
        assert_eq!(fp.vm_type, VmType::PlutusEutxo);
        assert_eq!(fp.finality_source, "ouroboros");
    }

    #[test]
    fn test_chain_health() {
        let adapter = PlutusHtlcAdapter::new("cardano-preprod".into(), PlutusNetwork::Preprod);
        let health = adapter.chain_health().expect("chain health");

        assert_eq!(health.chain_id, "cardano-preprod");
        assert_eq!(health.vm_type, VmType::PlutusEutxo);
        assert!(!health.halted);
        assert!(!health.degraded);
        assert!(health.safe_for_new_intents);
        assert!(health.rpc_quorum_healthy);
    }

    // ── Fee Estimation Tests ──────────────────────────────────────────────

    #[test]
    fn test_estimate_fee() {
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let hashlock = make_hashlock(b"fee_test");
        let intent = make_test_intent(99, hashlock);

        let fee = adapter.estimate_fee(&intent).expect("estimate_fee");

        assert_eq!(fee.chain_id, "cardano-mainnet");
        assert_eq!(fee.vm_type, VmType::PlutusEutxo);
        assert!(fee.native_fee > 0);
        assert!(fee.estimated_usd > 0.0);
    }

    // ── Readiness Score Tests ─────────────────────────────────────────────

    #[test]
    fn test_readiness_score() {
        let adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);

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
        assert_eq!(score.adapter_name, "x3-adapter-plutus");
        assert_eq!(score.vm_type, VmType::PlutusEutxo);

        let missing = score.missing_items();
        assert!(missing.contains(&"event_proof_extraction"));
        assert!(missing.contains(&"rpc_indexer_support"));
        assert!(missing.contains(&"proof_ledger_integration"));
        assert!(missing.contains(&"ibc_support"));
        assert_eq!(missing.len(), 4);
    }

    // ── Stateful Adapter Tests ────────────────────────────────────────────

    #[test]
    fn test_double_claim_rejected() {
        let mut adapter =
            StatefulPlutusAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);

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
        let mut adapter =
            StatefulPlutusAdapter::new("cardano-preprod".into(), PlutusNetwork::Preprod);

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
        let mut adapter =
            StatefulPlutusAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);

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
        let mut adapter =
            StatefulPlutusAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);

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
        let mut adapter = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        assert!(adapter.rpc_url.is_none());

        adapter.set_rpc("https://cardano-mainnet.node:8080");

        assert_eq!(
            adapter.rpc_url.as_deref(),
            Some("https://cardano-mainnet.node:8080")
        );
    }

    #[test]
    fn test_adapter_chain_id_independence() {
        let mainnet = PlutusHtlcAdapter::new("cardano-mainnet".into(), PlutusNetwork::Mainnet);
        let preprod = PlutusHtlcAdapter::new("cardano-preprod".into(), PlutusNetwork::Preprod);

        assert_eq!(mainnet.vm_type(), VmType::PlutusEutxo);
        assert_eq!(preprod.vm_type(), VmType::PlutusEutxo);

        let h = make_hashlock(b"chain_test");
        let intent_mainnet = make_test_intent(1, h);
        let intent_preprod = make_test_intent(1, h);

        let proof_mainnet = mainnet.lock(&intent_mainnet).expect("mainnet lock");
        let proof_preprod = preprod.lock(&intent_preprod).expect("preprod lock");

        assert_ne!(proof_mainnet.lock_address, proof_preprod.lock_address);
        assert!(proof_mainnet.lock_address.starts_with("addr1"));
        assert!(proof_preprod.lock_address.starts_with("addr_test"));
    }
}
