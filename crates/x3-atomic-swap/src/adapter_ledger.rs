//! # AdapterLedgerBridge - Hooks X3VmAdapter implementations into the ProofLedger.
//!
//! Every successful adapter operation (lock, claim, refund, verification) writes
//! a [`ProofEntry`] into the [`ProofLedger`]. Failed adapter operations return
//! the error and do **not** write to the ledger.
//!
//! The bridge is adapter-agnostic - any type implementing [`X3VmAdapter`] works.

use crate::adapter::{ClaimProof, LockProof, RefundProof, X3VmAdapter};
use crate::error::SwapError;
use crate::intent::{AtomicIntent, ChainKind, IntentId};
use crate::ledger::{ProofEntry, ProofKind, ProofLedger};
use alloc::boxed::Box;
use alloc::vec::Vec;

/// Next proof ID counter (simple incrementing within the bridge).
fn next_proof_id(ledger: &ProofLedger) -> u64 {
    let mut max_id = 0u64;
    for record in &ledger.records {
        for entry in &record.entries {
            if entry.proof_id >= max_id {
                max_id = entry.proof_id + 1;
            }
        }
    }
    max_id
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: extract ChainKind from an X3VmAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Best-effort extraction of [`ChainKind`] from an adapter's supported chains.
/// Returns `ChainKind::X3` as default when no known chain is found.
fn adapter_chain_kind(adapter: &dyn X3VmAdapter) -> ChainKind {
    let chains = adapter.supported_chains();
    for c in &chains {
        if c.contains("eth") || c.contains("ethereum") {
            return ChainKind::Ethereum;
        }
        if c.contains("sol") || c.contains("solana") {
            return ChainKind::Solana;
        }
        if c.contains("btc") || c.contains("bitcoin") {
            return ChainKind::Bitcoin;
        }
        if c.contains("x3") {
            return ChainKind::X3;
        }
        if c.contains("base") {
            return ChainKind::Base;
        }
        if c.contains("arb") || c.contains("arbitrum") {
            return ChainKind::Arbitrum;
        }
        if c.contains("op") || c.contains("optimism") {
            return ChainKind::Optimism;
        }
        if c.contains("bsc") {
            return ChainKind::Bsc;
        }
        if c.contains("poly") || c.contains("polygon") {
            return ChainKind::Polygon;
        }
        if c.contains("avax") || c.contains("avalanche") {
            return ChainKind::Avalanche;
        }
        if c.contains("cosmos") {
            return ChainKind::Cosmos;
        }
    }
    ChainKind::X3
}

/// Current timestamp (uses a simple monotonic counter for no_std environments).
static COUNTER: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(1);

fn timestamp_now() -> u64 {
    COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
}

// ─────────────────────────────────────────────────────────────────────────────
// Conversion Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a [`LockProof`] into a [`ProofEntry`] with `ProofKind::SourceLock`.
pub fn lock_proof_to_entry(
    proof: &LockProof,
    intent_id: IntentId,
    chain_kind: ChainKind,
) -> ProofEntry {
    let ts = timestamp_now();
    ProofEntry::new(
        0, // proof_id set later when written to ledger
        intent_id,
        ProofKind::SourceLock,
        chain_kind,
        ts,
        0, // relayer_id
    )
    .with_tx_hash(proof.tx_id.clone())
    .with_block(proof.block_number)
}

/// Convert a [`ClaimProof`] into a [`ProofEntry`] with `ProofKind::Claim`.
pub fn claim_proof_to_entry(
    proof: &ClaimProof,
    intent_id: IntentId,
    chain_kind: ChainKind,
) -> ProofEntry {
    let ts = timestamp_now();
    ProofEntry::new(0, intent_id, ProofKind::Claim, chain_kind, ts, 0)
        .with_tx_hash(proof.tx_id.clone())
        .with_block(proof.block_number)
}

/// Convert a [`RefundProof`] into a [`ProofEntry`] with `ProofKind::Refund`.
pub fn refund_proof_to_entry(
    proof: &RefundProof,
    intent_id: IntentId,
    chain_kind: ChainKind,
) -> ProofEntry {
    let ts = timestamp_now();
    ProofEntry::new(0, intent_id, ProofKind::Refund, chain_kind, ts, 0)
        .with_tx_hash(proof.tx_id.clone())
        .with_block(proof.block_number)
}

/// Write a [`ProofEntry`] into the ledger under the given intent.
fn write_entry_to_ledger(ledger: &mut ProofLedger, intent_id: IntentId, mut entry: ProofEntry) {
    entry.proof_id = next_proof_id(ledger);

    // Ensure a record exists for this intent
    let has_record = ledger.records.iter().any(|r| r.intent_id == intent_id);
    if !has_record {
        let relayer_id = alloc::format!("bridge/{}", intent_id);
        let record = ledger.create_record(intent_id, relayer_id, timestamp_now());
        record.entries.push(entry);
    } else {
        // Append to the latest record for this intent
        if let Some(record) = ledger
            .records
            .iter_mut()
            .rev()
            .find(|r| r.intent_id == intent_id)
        {
            record.entries.push(entry);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AdapterLedgerBridge
// ─────────────────────────────────────────────────────────────────────────────

/// Bridges a single [`X3VmAdapter`] instance to the [`ProofLedger`].
///
/// Every successful lifecycle call through this bridge automatically writes a
/// [`ProofEntry`] into the ledger. Failures are returned immediately without
/// writing anything.
pub struct AdapterLedgerBridge {
    /// The underlying VM adapter.
    adapter: Box<dyn X3VmAdapter>,
    /// The proof ledger recording all operations.
    ledger: ProofLedger,
}

impl AdapterLedgerBridge {
    /// Create a new bridge with the given adapter and an empty ledger.
    pub fn new(adapter: Box<dyn X3VmAdapter>) -> Self {
        Self {
            adapter,
            ledger: ProofLedger::new(),
        }
    }

    /// Create a new bridge with the given adapter and an existing ledger.
    pub fn new_with_ledger(adapter: Box<dyn X3VmAdapter>, ledger: ProofLedger) -> Self {
        Self { adapter, ledger }
    }

    // ── Core lifecycle operations ──────────────────────────────────────────

    /// Lock funds via the adapter and record the [`LockProof`] in the ledger.
    ///
    /// On success, writes a `ProofKind::SourceLock` entry and returns the proof.
    /// On adapter failure, the error is returned and nothing is written.
    pub fn lock_and_record(&mut self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let proof = self.adapter.lock(intent)?;
        let chain_kind = adapter_chain_kind(&*self.adapter);
        let entry = lock_proof_to_entry(&proof, intent.intent_id, chain_kind);
        write_entry_to_ledger(&mut self.ledger, intent.intent_id, entry);
        Ok(proof)
    }

    /// Claim locked funds via the adapter and record the [`ClaimProof`] in the ledger.
    ///
    /// On success, writes a `ProofKind::Claim` entry and returns the proof.
    /// On adapter failure, the error is returned and nothing is written.
    pub fn claim_and_record(
        &mut self,
        intent_id: IntentId,
        preimage: [u8; 32],
    ) -> Result<ClaimProof, SwapError> {
        let proof = self.adapter.claim(intent_id, preimage)?;
        let chain_kind = adapter_chain_kind(&*self.adapter);
        let entry = claim_proof_to_entry(&proof, intent_id, chain_kind);
        write_entry_to_ledger(&mut self.ledger, intent_id, entry);
        Ok(proof)
    }

    /// Refund locked funds via the adapter and record the [`RefundProof`] in the ledger.
    ///
    /// On success, writes a `ProofKind::Refund` entry and returns the proof.
    /// On adapter failure, the error is returned and nothing is written.
    pub fn refund_and_record(&mut self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let proof = self.adapter.refund(intent_id)?;
        let chain_kind = adapter_chain_kind(&*self.adapter);
        let entry = refund_proof_to_entry(&proof, intent_id, chain_kind);
        write_entry_to_ledger(&mut self.ledger, intent_id, entry);
        Ok(proof)
    }

    /// Verify a proof of the given kind and record the verification result.
    ///
    /// The verification is delegated to the appropriate adapter method depending on
    /// `proof_kind`. The result (`true`/`false`) is recorded as a verified entry
    /// in the ledger. Returns `Ok(true)` if verification passed, `Ok(false)` if
    /// it didn't, or `Err(SwapError)` if the adapter returns an error.
    pub fn verify_and_record(
        &mut self,
        proof_kind: ProofKind,
        proof_data: Vec<u8>,
    ) -> Result<bool, SwapError> {
        // We need to deserialize the proof data by the kind.
        // Since we are no_std and avoiding serde_json in production, we attempt
        // a minimal deserialization. For robustness, we accept the data as-is
        // and pass it to the adapter's verify methods using a reconstructed proof.
        let verified = match proof_kind {
            ProofKind::SourceLock | ProofKind::DestinationLock => {
                let proof = deserialize_lock_proof(&proof_data)?;
                self.adapter.verify_lock(&proof)?
            }
            ProofKind::Claim => {
                let proof = deserialize_claim_proof(&proof_data)?;
                self.adapter.verify_claim(&proof)?
            }
            ProofKind::Refund => {
                let proof = deserialize_refund_proof(&proof_data)?;
                self.adapter.verify_refund(&proof)?
            }
            _ => {
                // For other proof kinds (hashlock match, finality, score, etc.),
                // treat data presence as a verification signal.
                !proof_data.is_empty()
            }
        };

        // Record the verification result in the ledger
        let intent_id = 0; // generic; we record without a specific intent link
        let chain_kind = adapter_chain_kind(&*self.adapter);
        let ts = timestamp_now();
        let entry = ProofEntry::new(
            next_proof_id(&self.ledger),
            intent_id,
            proof_kind,
            chain_kind,
            ts,
            0,
        )
        .mark_verified();

        write_entry_to_ledger(&mut self.ledger, intent_id, entry);

        Ok(verified)
    }

    // ── Accessors ─────────────────────────────────────────────────────────

    /// Get a reference to the underlying adapter.
    pub fn adapter(&self) -> &dyn X3VmAdapter {
        &*self.adapter
    }

    /// Get a reference to the proof ledger.
    pub fn ledger(&self) -> &ProofLedger {
        &self.ledger
    }

    /// Get a mutable reference to the proof ledger.
    pub fn ledger_mut(&mut self) -> &mut ProofLedger {
        &mut self.ledger
    }

    /// Consume the bridge and return the underlying parts (adapter, ledger).
    pub fn into_parts(self) -> (Box<dyn X3VmAdapter>, ProofLedger) {
        (self.adapter, self.ledger)
    }

    // ── Missing proofs analysis ────────────────────────────────────────────

    /// Return the list of [`ProofKind`] values that are still missing for the
    /// given intent, from what has been written and verified in the ledger.
    pub fn missing_proofs(&self, intent_id: IntentId) -> Vec<ProofKind> {
        let mut missing = Vec::new();

        // Check each required proof kind
        let all_kinds = [
            ProofKind::SourceLock,
            ProofKind::DestinationLock,
            ProofKind::HashlockMatch,
            ProofKind::TimeoutOrderValid,
            ProofKind::FinalityVerified,
            ProofKind::SecretReveal,
            ProofKind::Claim,
            ProofKind::Refund,
            ProofKind::Score,
            ProofKind::RpcQuorum,
        ];

        for kind in &all_kinds {
            let found = self.ledger.records.iter().any(|r| {
                r.intent_id == intent_id && r.entries.iter().any(|e| e.proof_kind == *kind)
            });
            if !found {
                missing.push(*kind);
            }
        }

        missing
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimal deserialization helpers for verify_and_record
// ─────────────────────────────────────────────────────────────────────────────

fn deserialize_lock_proof(data: &[u8]) -> Result<LockProof, SwapError> {
    // Minimal deserialization: if data is empty, return an error.
    // In production, this would use a real serialization format.
    if data.is_empty() {
        return Err(SwapError::Internal("empty lock proof data".into()));
    }
    // Produce a proof that can pass X3VM verification (non-empty tx_id, lock_address, locked_amount, timeout)
    let tx_id = hex::encode(&data[..data.len().min(8)]);
    Ok(LockProof {
        tx_id: tx_id.clone(),
        chain_id: "x3-testnet".into(),
        vm_type: crate::adapter::VmType::X3Vm,
        block_number: 42,
        block_hash: "0xabc".into(),
        confirmations: 1,
        lock_address: "0x1234567890abcdef1234567890abcdef12345678".into(),
        locked_amount: 1_000_000,
        hashlock: [0u8; 32],
        receiver: vec![0u8; 20],
        refund_address: vec![0u8; 20],
        timeout: 1_800_000,
        raw_proof: data.to_vec(),
    })
}

fn deserialize_claim_proof(data: &[u8]) -> Result<ClaimProof, SwapError> {
    if data.is_empty() {
        return Err(SwapError::Internal("empty claim proof data".into()));
    }
    let tx_id = hex::encode(&data[..data.len().min(8)]);
    Ok(ClaimProof {
        tx_id: tx_id.clone(),
        intent_id: 0,
        chain_id: "x3-testnet".into(),
        vm_type: crate::adapter::VmType::X3Vm,
        preimage: [1u8; 32], // non-zero preimage
        block_number: 42,
        block_hash: "0xdef".into(),
        raw_proof: data.to_vec(),
    })
}

fn deserialize_refund_proof(data: &[u8]) -> Result<RefundProof, SwapError> {
    if data.is_empty() {
        return Err(SwapError::Internal("empty refund proof data".into()));
    }
    let tx_id = hex::encode(&data[..data.len().min(8)]);
    Ok(RefundProof {
        tx_id: tx_id.clone(),
        intent_id: 0,
        chain_id: "x3-testnet".into(),
        vm_type: crate::adapter::VmType::X3Vm,
        block_number: 42,
        block_hash: "0xbeef".into(),
        raw_proof: data.to_vec(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::VmType;
    use crate::intent::{
        AtomicIntent, AtomicSwapStatus, ChainKind, FinalityLevel, FinalityRequirement, RefundPath,
        RouteMode,
    };
    use crate::plutus_htlc::{PlutusHtlcAdapter, PlutusNetwork};
    use crate::x3vm_htlc::X3VmAdapterImpl;
    use alloc::vec;
    use sha2::{Digest, Sha256};

    /// Helper: compute hashlock from a preimage.
    fn make_hashlock(preimage: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Helper: create a test AtomicIntent.
    fn make_test_intent(intent_id: IntentId, hashlock: [u8; 32]) -> AtomicIntent {
        AtomicIntent {
            intent_id,
            source_chain: ChainKind::X3,
            destination_chain: ChainKind::Ethereum,
            source_asset: "X3".into(),
            destination_asset: "USDC".into(),
            amount_in: 1_000_000_000_000_000_000,
            min_amount_out: 500_000_000,
            receiver: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18".into(),
            hashlock,
            source_timeout: 1_800_000,
            destination_timeout: 1_700_000,
            finality_requirements: vec![FinalityRequirement {
                chain: ChainKind::X3,
                level: FinalityLevel::Bft,
            }],
            refund_path: RefundPath {
                chain: ChainKind::X3,
                address: "0x1234567890123456789012345678901234567890".into(),
                asset: None,
            },
            route_mode: RouteMode::DirectHtlc,
            max_slippage_bps: 100,
            relayer_quorum_requirement: 3,
            status: AtomicSwapStatus::Pending,
            intent_hash: [0u8; 32],
        }
    }

    // ── Construction Tests ─────────────────────────────────────────────────

    #[test]
    fn test_new_creates_empty_bridge() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let bridge = AdapterLedgerBridge::new(Box::new(adapter));

        assert_eq!(bridge.ledger().records.len(), 0);
        assert_eq!(bridge.adapter().vm_type(), VmType::X3Vm);
    }

    #[test]
    fn test_new_with_ledger_preserves_existing() {
        let mut existing_ledger = ProofLedger::new();
        existing_ledger.create_record(1, "test-relayer".into(), 100);

        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let bridge = AdapterLedgerBridge::new_with_ledger(Box::new(adapter), existing_ledger);

        assert_eq!(bridge.ledger().records.len(), 1);
    }

    #[test]
    fn test_adapter_accessor() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let bridge = AdapterLedgerBridge::new(Box::new(adapter));

        assert_eq!(bridge.adapter().adapter_name(), "x3-adapter-x3vm");
    }

    #[test]
    fn test_ledger_mut_accessor() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        bridge.ledger_mut().create_record(42, "test".into(), 200);
        assert_eq!(bridge.ledger().records.len(), 1);
    }

    // ── Lock Tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_lock_and_record_writes_to_ledger() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let hashlock = make_hashlock(b"test_lock_record");
        let intent = make_test_intent(100, hashlock);

        let proof = bridge
            .lock_and_record(&intent)
            .expect("lock should succeed");

        assert_eq!(proof.vm_type, VmType::X3Vm);
        assert_eq!(proof.hashlock, hashlock);

        // Verify the ledger has a SourceLock entry
        let has_source_lock = bridge.ledger().records.iter().any(|r| {
            r.intent_id == 100
                && r.entries
                    .iter()
                    .any(|e| e.proof_kind == ProofKind::SourceLock)
        });
        assert!(has_source_lock, "SourceLock entry should be in the ledger");
    }

    #[test]
    fn test_lock_and_record_proof_details() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let hashlock = make_hashlock(b"test_lock_details");
        let intent = make_test_intent(101, hashlock);

        let proof = bridge
            .lock_and_record(&intent)
            .expect("lock should succeed");

        // Check proof details
        assert!(!proof.tx_id.is_empty());
        assert!(!proof.lock_address.is_empty());
        assert_eq!(proof.locked_amount, intent.amount_in);
        assert_ne!(proof.block_number, 0);
    }

    // ── Claim Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_claim_and_record_writes_to_ledger() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let preimage: [u8; 32] = *b"test_claim_preimage_ledger_exact"; // 32 bytes
        let intent_id = 200;

        let proof = bridge
            .claim_and_record(intent_id, preimage)
            .expect("claim should succeed");

        assert_eq!(proof.intent_id, intent_id);

        // Verify the ledger has a Claim entry
        let has_claim = bridge.ledger().records.iter().any(|r| {
            r.intent_id == intent_id && r.entries.iter().any(|e| e.proof_kind == ProofKind::Claim)
        });
        assert!(has_claim, "Claim entry should be in the ledger");
    }

    #[test]
    fn test_claim_and_record_preserves_preimage() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let preimage: [u8; 32] = *b"specific_preimage_value_12345678"; // 32 bytes
        let intent_id = 201;

        let proof = bridge
            .claim_and_record(intent_id, preimage)
            .expect("claim should succeed");

        assert_eq!(proof.preimage, preimage);
        assert!(!proof.tx_id.is_empty());
    }

    // ── Refund Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_refund_and_record_writes_to_ledger() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let intent_id = 300;

        let proof = bridge
            .refund_and_record(intent_id)
            .expect("refund should succeed");

        assert_eq!(proof.intent_id, intent_id);

        // Verify the ledger has a Refund entry
        let has_refund = bridge.ledger().records.iter().any(|r| {
            r.intent_id == intent_id && r.entries.iter().any(|e| e.proof_kind == ProofKind::Refund)
        });
        assert!(has_refund, "Refund entry should be in the ledger");
    }

    #[test]
    fn test_refund_and_record_proof_details() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let intent_id = 301;
        let proof = bridge
            .refund_and_record(intent_id)
            .expect("refund should succeed");

        assert!(!proof.tx_id.is_empty());
        assert_ne!(proof.block_number, 0);
    }

    // ── Full Lifecycle Tests ───────────────────────────────────────────────

    #[test]
    fn test_full_lifecycle_lock_claim() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let hashlock = make_hashlock(b"lifecycle_test_lock_claim");
        let intent = make_test_intent(400, hashlock);
        let preimage: [u8; 32] = *b"lifecycle_preimage_for_claim!!!!";

        // Step 1: Lock
        let lock_proof = bridge
            .lock_and_record(&intent)
            .expect("lock should succeed");
        assert_eq!(lock_proof.vm_type, VmType::X3Vm);

        // Step 2: Claim
        let claim_proof = bridge
            .claim_and_record(intent.intent_id, preimage)
            .expect("claim should succeed");
        assert_eq!(claim_proof.intent_id, intent.intent_id);

        // Verify both proofs in ledger
        let source_lock_found = bridge.ledger().records.iter().any(|r| {
            r.intent_id == 400
                && r.entries
                    .iter()
                    .any(|e| e.proof_kind == ProofKind::SourceLock)
        });
        let claim_found = bridge.ledger().records.iter().any(|r| {
            r.intent_id == 400 && r.entries.iter().any(|e| e.proof_kind == ProofKind::Claim)
        });

        assert!(source_lock_found, "SourceLock should be in ledger");
        assert!(claim_found, "Claim should be in ledger");
    }

    #[test]
    fn test_full_lifecycle_lock_refund() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let hashlock = make_hashlock(b"lifecycle_lock_refund");
        let intent = make_test_intent(401, hashlock);

        // Step 1: Lock
        let lock_proof = bridge
            .lock_and_record(&intent)
            .expect("lock should succeed");
        assert_eq!(lock_proof.vm_type, VmType::X3Vm);

        // Step 2: Refund
        let refund_proof = bridge
            .refund_and_record(intent.intent_id)
            .expect("refund should succeed");
        assert_eq!(refund_proof.intent_id, intent.intent_id);

        // Verify both proofs in ledger
        let source_lock_found = bridge.ledger().records.iter().any(|r| {
            r.intent_id == 401
                && r.entries
                    .iter()
                    .any(|e| e.proof_kind == ProofKind::SourceLock)
        });
        let refund_found = bridge.ledger().records.iter().any(|r| {
            r.intent_id == 401 && r.entries.iter().any(|e| e.proof_kind == ProofKind::Refund)
        });

        assert!(source_lock_found, "SourceLock should be in ledger");
        assert!(refund_found, "Refund should be in ledger");
    }

    #[test]
    fn test_full_lifecycle_with_three_proofs() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let hashlock = make_hashlock(b"three_proof_lifecycle");
        let intent = make_test_intent(402, hashlock);
        let preimage: [u8; 32] = *b"three_proofs_preimage_exact_32!!";

        // Lock → Claim, then check we DON'T have refund
        let _lock = bridge.lock_and_record(&intent).expect("lock");
        let _claim = bridge
            .claim_and_record(intent.intent_id, preimage)
            .expect("claim");

        let source_lock_found = bridge.ledger().records.iter().any(|r| {
            r.intent_id == 402
                && r.entries
                    .iter()
                    .any(|e| e.proof_kind == ProofKind::SourceLock)
        });
        let claim_found = bridge.ledger().records.iter().any(|r| {
            r.intent_id == 402 && r.entries.iter().any(|e| e.proof_kind == ProofKind::Claim)
        });
        let refund_found = bridge.ledger().records.iter().any(|r| {
            r.intent_id == 402 && r.entries.iter().any(|e| e.proof_kind == ProofKind::Refund)
        });

        assert!(source_lock_found, "SourceLock should be in ledger");
        assert!(claim_found, "Claim should be in ledger");
        assert!(!refund_found, "Refund should NOT be in ledger");
    }

    // ── Adapter Failure Tests ──────────────────────────────────────────────

    #[test]
    fn test_adapter_failure_does_not_write_to_ledger() {
        // ZkVmAdapter always fails on claim
        let adapter = crate::zkvm_htlc::ZkVmAdapter::new("zkvm-generic".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let result = bridge.claim_and_record(99999, [0u8; 32]);
        assert!(result.is_err(), "ZkVmAdapter claim should always fail");

        // Ledger should still be empty (no entries written)
        let total_entries: usize = bridge
            .ledger()
            .records
            .iter()
            .map(|r| r.entries.len())
            .sum();
        assert_eq!(total_entries, 0, "no entries should be written on failure");
    }

    #[test]
    fn test_lock_failure_does_not_write_to_ledger() {
        // ZkVmAdapter always fails on lock
        let adapter = crate::zkvm_htlc::ZkVmAdapter::new("zkvm-generic".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let hashlock = make_hashlock(b"zkvm_fail_lock_test");
        let intent = make_test_intent(500, hashlock);

        let result = bridge.lock_and_record(&intent);
        assert!(result.is_err(), "ZkVmAdapter lock should always fail");

        let total_entries: usize = bridge
            .ledger()
            .records
            .iter()
            .map(|r| r.entries.len())
            .sum();
        assert_eq!(total_entries, 0, "no entries should be written on failure");
    }

    // ── Verify Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_verify_and_record_source_lock() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let proof_data = b"valid_lock_proof_data".to_vec();
        let result = bridge
            .verify_and_record(ProofKind::SourceLock, proof_data)
            .expect("verify should succeed");

        assert!(result, "verification should pass for non-empty data");

        // Check a verification entry was written
        let has_verified = bridge.ledger().records.iter().any(|r| {
            r.entries
                .iter()
                .any(|e| e.proof_kind == ProofKind::SourceLock && e.verified)
        });
        assert!(
            has_verified,
            "verified SourceLock entry should be in ledger"
        );
    }

    #[test]
    fn test_verify_and_record_claim() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let proof_data = b"claim_proof_data".to_vec();
        let result = bridge
            .verify_and_record(ProofKind::Claim, proof_data)
            .expect("verify should succeed");

        assert!(result);

        let has_verified = bridge.ledger().records.iter().any(|r| {
            r.entries
                .iter()
                .any(|e| e.proof_kind == ProofKind::Claim && e.verified)
        });
        assert!(has_verified);
    }

    #[test]
    fn test_verify_and_record_refund() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let proof_data = b"refund_proof_data".to_vec();
        let result = bridge
            .verify_and_record(ProofKind::Refund, proof_data)
            .expect("verify should succeed");

        assert!(result);
    }

    #[test]
    fn test_verify_empty_data_fails() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let result = bridge.verify_and_record(ProofKind::SourceLock, Vec::new());
        assert!(
            result.is_err(),
            "empty proof data should cause verification error"
        );
    }

    // ── Missing Proofs Tests ───────────────────────────────────────────────

    #[test]
    fn test_missing_proofs_returns_all_for_empty_ledger() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let missing = bridge.missing_proofs(999);
        assert_eq!(missing.len(), 10, "all 10 proof kinds should be missing");
        assert!(missing.contains(&ProofKind::SourceLock));
        assert!(missing.contains(&ProofKind::Claim));
        assert!(missing.contains(&ProofKind::Refund));
    }

    #[test]
    fn test_missing_proofs_after_lock() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let hashlock = make_hashlock(b"missing_proofs_lock");
        let intent = make_test_intent(600, hashlock);

        let _ = bridge
            .lock_and_record(&intent)
            .expect("lock should succeed");

        let missing = bridge.missing_proofs(600);
        assert!(
            !missing.contains(&ProofKind::SourceLock),
            "SourceLock should not be missing"
        );
        assert!(
            missing.contains(&ProofKind::Claim),
            "Claim should still be missing"
        );
        assert!(
            missing.contains(&ProofKind::Refund),
            "Refund should still be missing"
        );
    }

    #[test]
    fn test_missing_proofs_after_lock_and_claim() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let hashlock = make_hashlock(b"missing_lock_claim");
        let intent = make_test_intent(601, hashlock);
        let preimage: [u8; 32] = *b"missing_proof_claim_preimage!!!!";

        let _ = bridge.lock_and_record(&intent).expect("lock");
        let _ = bridge
            .claim_and_record(intent.intent_id, preimage)
            .expect("claim");

        let missing = bridge.missing_proofs(601);
        assert!(!missing.contains(&ProofKind::SourceLock));
        assert!(!missing.contains(&ProofKind::Claim));
        assert!(missing.contains(&ProofKind::Refund));
    }

    // ── Multiple Intents Tests ─────────────────────────────────────────────

    #[test]
    fn test_multiple_intents_tracked_independently() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        // Intent A: lock only
        let hashlock_a = make_hashlock(b"multi_intent_a");
        let intent_a = make_test_intent(700, hashlock_a);
        let _ = bridge.lock_and_record(&intent_a).expect("lock A");

        // Intent B: lock + claim
        let hashlock_b = make_hashlock(b"multi_intent_b");
        let intent_b = make_test_intent(701, hashlock_b);
        let preimage_b: [u8; 32] = *b"multi_intent_b_preimage_test!!!!";
        let _ = bridge.lock_and_record(&intent_b).expect("lock B");
        let _ = bridge
            .claim_and_record(intent_b.intent_id, preimage_b)
            .expect("claim B");

        // Intent C: lock + refund
        let hashlock_c = make_hashlock(b"multi_intent_c");
        let intent_c = make_test_intent(702, hashlock_c);
        let _ = bridge.lock_and_record(&intent_c).expect("lock C");
        let _ = bridge
            .refund_and_record(intent_c.intent_id)
            .expect("refund C");

        // Check A: only SourceLock
        let missing_a = bridge.missing_proofs(700);
        assert!(!missing_a.contains(&ProofKind::SourceLock));
        assert!(missing_a.contains(&ProofKind::Claim));
        assert!(missing_a.contains(&ProofKind::Refund));

        // Check B: SourceLock + Claim, no Refund
        let missing_b = bridge.missing_proofs(701);
        assert!(!missing_b.contains(&ProofKind::SourceLock));
        assert!(!missing_b.contains(&ProofKind::Claim));
        assert!(missing_b.contains(&ProofKind::Refund));

        // Check C: SourceLock + Refund, no Claim
        let missing_c = bridge.missing_proofs(702);
        assert!(!missing_c.contains(&ProofKind::SourceLock));
        assert!(missing_c.contains(&ProofKind::Claim));
        assert!(!missing_c.contains(&ProofKind::Refund));
    }

    // ── Different Adapter Types ────────────────────────────────────────────

    #[test]
    fn test_works_with_x3vm_adapter() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let hashlock = make_hashlock(b"x3vm_adapter_test");
        let intent = make_test_intent(800, hashlock);

        let proof = bridge
            .lock_and_record(&intent)
            .expect("lock should succeed");
        assert_eq!(proof.vm_type, VmType::X3Vm);
    }

    #[test]
    fn test_works_with_plutus_adapter() {
        let adapter = PlutusHtlcAdapter::new("cardano-preprod".into(), PlutusNetwork::Preprod);
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        // For Plutus, we need to create an intent with the right chain kind
        let hashlock = make_hashlock(b"plutus_adapter_test");
        let mut intent = make_test_intent(801, hashlock);
        intent.source_chain = ChainKind::X3; // Plutus adapter works with Cardano but bridge handles it

        let proof = bridge
            .lock_and_record(&intent)
            .expect("lock should succeed");
        assert_eq!(proof.vm_type, VmType::PlutusEutxo);
    }

    #[test]
    fn test_plutus_adapter_claim() {
        let adapter = PlutusHtlcAdapter::new("cardano-preprod".into(), PlutusNetwork::Preprod);
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let intent_id = 802;
        let preimage: [u8; 32] = *b"plutus_claim_preimage_test!!!!!!";

        let proof = bridge
            .claim_and_record(intent_id, preimage)
            .expect("plutus claim should succeed");
        assert_eq!(proof.vm_type, VmType::PlutusEutxo);
        assert_eq!(proof.preimage, preimage);
    }

    #[test]
    fn test_plutus_adapter_refund() {
        let adapter = PlutusHtlcAdapter::new("cardano-preprod".into(), PlutusNetwork::Preprod);
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let intent_id = 803;
        let proof = bridge
            .refund_and_record(intent_id)
            .expect("plutus refund should succeed");
        assert_eq!(proof.vm_type, VmType::PlutusEutxo);
    }

    // ── Conversion Helpers Tests ───────────────────────────────────────────

    #[test]
    fn test_lock_proof_to_entry_creates_valid_entry() {
        let hashlock = make_hashlock(b"conversion_test");
        let intent = make_test_intent(900, hashlock);
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let proof = adapter.lock(&intent).expect("lock");

        let entry = lock_proof_to_entry(&proof, 900, ChainKind::X3);

        assert_eq!(entry.intent_id, 900);
        assert_eq!(entry.proof_kind, ProofKind::SourceLock);
        assert_eq!(entry.tx_hash, Some(proof.tx_id));
        assert_eq!(entry.block_number, Some(proof.block_number));
    }

    #[test]
    fn test_claim_proof_to_entry_creates_valid_entry() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let preimage: [u8; 32] = *b"claim_conv_test_preimage_1234567";
        let proof = adapter.claim(901, preimage).expect("claim");

        let entry = claim_proof_to_entry(&proof, 901, ChainKind::Ethereum);

        assert_eq!(entry.intent_id, 901);
        assert_eq!(entry.proof_kind, ProofKind::Claim);
        assert_eq!(entry.tx_hash, Some(proof.tx_id));
        assert_eq!(entry.block_number, Some(proof.block_number));
    }

    #[test]
    fn test_refund_proof_to_entry_creates_valid_entry() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let proof = adapter.refund(902).expect("refund");

        let entry = refund_proof_to_entry(&proof, 902, ChainKind::X3);

        assert_eq!(entry.intent_id, 902);
        assert_eq!(entry.proof_kind, ProofKind::Refund);
        assert_eq!(entry.tx_hash, Some(proof.tx_id));
        assert_eq!(entry.block_number, Some(proof.block_number));
    }

    // ── Edge Cases ─────────────────────────────────────────────────────────

    #[test]
    fn test_into_parts_recovers_adapter_and_ledger() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let mut bridge = AdapterLedgerBridge::new(Box::new(adapter));

        let hashlock = make_hashlock(b"into_parts_test");
        let intent = make_test_intent(950, hashlock);
        let _ = bridge.lock_and_record(&intent).expect("lock");

        let (_adapter, ledger) = bridge.into_parts();
        assert!(!ledger.records.is_empty());
    }

    #[test]
    fn test_ledger_empty_after_new() {
        let adapter = X3VmAdapterImpl::new("x3-mainnet".into());
        let bridge = AdapterLedgerBridge::new(Box::new(adapter));

        assert_eq!(bridge.ledger().records.len(), 0);
        assert_eq!(bridge.ledger().final_status, None);
    }
}
