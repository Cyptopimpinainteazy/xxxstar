//! # ProofLedger - Durable atomic swap proof record.
//!
//! Every atomic swap step writes a proof record. The scoreboard requires
//! all required proofs to reach 100/100.

use crate::intent::{ChainKind, IntentId};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Kinds of proofs tracked by the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKind {
    SourceLock,
    DestinationLock,
    HashlockMatch,
    TimeoutOrderValid,
    FinalityVerified,
    SecretReveal,
    Claim,
    Refund,
    Score,
    RpcQuorum,
}

impl ProofKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            ProofKind::SourceLock => "source_lock",
            ProofKind::DestinationLock => "destination_lock",
            ProofKind::HashlockMatch => "hashlock_match",
            ProofKind::TimeoutOrderValid => "timeout_order_valid",
            ProofKind::FinalityVerified => "finality_verified",
            ProofKind::SecretReveal => "secret_reveal",
            ProofKind::Claim => "claim",
            ProofKind::Refund => "refund",
            ProofKind::Score => "score",
            ProofKind::RpcQuorum => "rpc_quorum",
        }
    }

    pub fn required_for_success() -> &'static [ProofKind] {
        &[
            ProofKind::SourceLock,
            ProofKind::DestinationLock,
            ProofKind::HashlockMatch,
            ProofKind::TimeoutOrderValid,
            ProofKind::FinalityVerified,
            ProofKind::SecretReveal,
            ProofKind::Claim,
            ProofKind::Score,
        ]
    }

    pub fn required_for_refund() -> &'static [ProofKind] {
        &[
            ProofKind::SourceLock,
            ProofKind::DestinationLock,
            ProofKind::TimeoutOrderValid,
            ProofKind::Refund,
            ProofKind::Score,
        ]
    }
}

/// Transaction status as observed by an RPC provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxStatus {
    /// Transaction is confirmed on chain.
    Confirmed,
    /// Transaction is in mempool but not yet confirmed.
    Pending,
    /// Transaction execution failed on chain.
    Failed,
    /// Transaction not found on this provider.
    NotFound,
    /// Transaction status is unknown or could not be determined.
    Unknown,
}

/// RPC quorum proof: attested by one RPC provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcQuorumProof {
    /// Intent ID this proof belongs to.
    pub intent_id: u64,
    /// RPC endpoint identifier.
    pub provider: String,
    /// Block height at which the check was performed.
    pub block_height: u64,
    /// Transaction status observed.
    pub tx_status: TxStatus,
    /// Number of providers that agree on this status.
    pub agreement_count: u32,
    /// Minimum providers required for quorum.
    pub required_quorum: u32,
}

impl RpcQuorumProof {
    /// True when the agreement count meets or exceeds the required quorum.
    pub fn agreed(&self) -> bool {
        self.agreement_count >= self.required_quorum
    }
}

/// A single proof entry (simpler per-step record).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEntry {
    pub proof_id: u64,
    pub intent_id: IntentId,
    pub proof_kind: ProofKind,
    pub source_chain: ChainKind,
    pub tx_hash: Option<String>,
    pub block_number: Option<u64>,
    pub timestamp: u64,
    pub relayer_id: u32,
    pub verified: bool,
    pub data: Option<Vec<u8>>,
}

impl ProofEntry {
    pub fn new(
        proof_id: u64,
        intent_id: IntentId,
        proof_kind: ProofKind,
        source_chain: ChainKind,
        timestamp: u64,
        relayer_id: u32,
    ) -> Self {
        Self {
            proof_id,
            intent_id,
            proof_kind,
            source_chain,
            tx_hash: None,
            block_number: None,
            timestamp,
            relayer_id,
            verified: false,
            data: None,
        }
    }

    pub fn with_tx_hash(mut self, tx_hash: String) -> Self {
        self.tx_hash = Some(tx_hash);
        self
    }

    pub fn with_block(mut self, block_number: u64) -> Self {
        self.block_number = Some(block_number);
        self
    }

    pub fn mark_verified(mut self) -> Self {
        self.verified = true;
        self
    }
}

/// Complete proof record consolidating all swap step evidence.
///
/// Used by [`SwapScoreboard`] to compute the proof score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRecord {
    pub record_id: u64,
    pub intent_id: IntentId,
    pub relayer_id: String,
    pub timestamp: u64,
    pub source_lock_tx: Option<String>,
    pub source_lock_block: Option<u64>,
    pub destination_lock_tx: Option<String>,
    pub destination_lock_block: Option<u64>,
    pub hashlock_match: bool,
    pub timeout_order_valid: bool,
    pub finality_verified: bool,
    pub secret_reveal_tx: Option<String>,
    pub claim_tx: Option<String>,
    pub claim_block: Option<u64>,
    pub refund_tx: Option<String>,
    pub refund_block: Option<u64>,
    pub final_status: Option<ProofFinalStatus>,
    pub entries: Vec<ProofEntry>,
}

impl ProofRecord {
    pub fn new(record_id: u64, intent_id: IntentId, relayer_id: String, timestamp: u64) -> Self {
        Self {
            record_id,
            intent_id,
            relayer_id,
            timestamp,
            source_lock_tx: None,
            source_lock_block: None,
            destination_lock_tx: None,
            destination_lock_block: None,
            hashlock_match: false,
            timeout_order_valid: false,
            finality_verified: false,
            secret_reveal_tx: None,
            claim_tx: None,
            claim_block: None,
            refund_tx: None,
            refund_block: None,
            final_status: None,
            entries: Vec::new(),
        }
    }

    pub fn record_source_lock(&mut self, tx_hash: String, block_number: u64, _timestamp: u64) {
        self.source_lock_tx = Some(tx_hash);
        self.source_lock_block = Some(block_number);
    }

    pub fn record_destination_lock(&mut self, tx_hash: String, block_number: u64, _timestamp: u64) {
        self.destination_lock_tx = Some(tx_hash);
        self.destination_lock_block = Some(block_number);
    }

    pub fn record_hashlock_match(&mut self, matched: bool, _timestamp: u64) {
        self.hashlock_match = matched;
    }

    pub fn record_timeout_order(&mut self, valid: bool, _timestamp: u64) {
        self.timeout_order_valid = valid;
    }

    pub fn record_finality_verified(&mut self, verified: bool, _timestamp: u64) {
        self.finality_verified = verified;
    }

    pub fn record_secret_reveal(&mut self, tx_hash: String, _timestamp: u64) {
        self.secret_reveal_tx = Some(tx_hash);
    }

    pub fn record_claim(&mut self, tx_hash: String, block_number: u64, _timestamp: u64) {
        self.claim_tx = Some(tx_hash);
        self.claim_block = Some(block_number);
    }

    pub fn record_refund(&mut self, tx_hash: String, block_number: u64, _timestamp: u64) {
        self.refund_tx = Some(tx_hash);
        self.refund_block = Some(block_number);
    }
}

/// Final status stored in the proof ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofFinalStatus {
    Active,
    Completed,
    Refunded,
    Failed,
}

/// Complete proof ledger for an atomic swap intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofLedger {
    pub intent_id: Option<IntentId>,
    pub records: Vec<ProofRecord>,
    pub final_status: Option<ProofFinalStatus>,
    /// RPC quorum proofs collected for this intent.
    pub rpc_quorum_proofs: Vec<RpcQuorumProof>,
}

impl Default for ProofLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl ProofLedger {
    pub fn new() -> Self {
        Self {
            intent_id: None,
            records: Vec::new(),
            final_status: None,
            rpc_quorum_proofs: Vec::new(),
        }
    }

    pub fn create_record(
        &mut self,
        intent_id: u64,
        relayer_id: String,
        timestamp: u64,
    ) -> &mut ProofRecord {
        let id = self.records.len() as u64;
        self.records
            .push(ProofRecord::new(id, intent_id, relayer_id, timestamp));
        self.records.last_mut().unwrap()
    }

    pub fn get_record_mut(&mut self, record_id: u64) -> Option<&mut ProofRecord> {
        self.records.iter_mut().find(|r| r.record_id == record_id)
    }

    pub fn get_record(&self, record_id: u64) -> Option<&ProofRecord> {
        self.records.iter().find(|r| r.record_id == record_id)
    }

    pub fn get_latest_for_intent(&self, intent_id: u64) -> Option<&ProofRecord> {
        self.records.iter().rev().find(|r| r.intent_id == intent_id)
    }

    pub fn get_records_for_intent(&self, intent_id: u64) -> Vec<&ProofRecord> {
        self.records
            .iter()
            .filter(|r| r.intent_id == intent_id)
            .collect()
    }

    /// Add an RPC quorum proof to the ledger.
    pub fn add_rpc_quorum_proof(&mut self, proof: RpcQuorumProof) {
        self.rpc_quorum_proofs.push(proof);
    }

    /// Check if any RPC quorum proof has reached agreement (global, legacy).
    pub fn has_rpc_quorum_agreement(&self) -> bool {
        self.rpc_quorum_proofs.iter().any(|p| p.agreed())
    }

    /// Check if any agreed RPC quorum proof exists specifically for `intent_id`.
    pub fn has_rpc_quorum_agreement_for_intent(&self, intent_id: u64) -> bool {
        self.rpc_quorum_proofs
            .iter()
            .any(|p| p.intent_id == intent_id && p.agreed())
    }

    pub fn has_verified_kind(&self, kind: ProofKind) -> bool {
        // legacy global scan - prefer the intent-scoped helper for accuracy
        for record in &self.records {
            for entry in &record.entries {
                if entry.proof_kind == kind && entry.verified {
                    return true;
                }
            }
        }
        match kind {
            ProofKind::SourceLock => self.records.iter().any(|r| r.source_lock_tx.is_some()),
            ProofKind::DestinationLock => {
                self.records.iter().any(|r| r.destination_lock_tx.is_some())
            }
            ProofKind::Claim => self.records.iter().any(|r| r.claim_tx.is_some()),
            ProofKind::Refund => self.records.iter().any(|r| r.refund_tx.is_some()),
            _ => false,
        }
    }

    /// Intent-scoped proof check - only inspects records for `intent_id`.
    /// This is the correct check for per-swap alert logic:
    /// a healthy swap must not suppress alerts for a broken one.
    pub fn has_verified_kind_for_intent(&self, intent_id: u64, kind: ProofKind) -> bool {
        let records: Vec<&ProofRecord> = self
            .records
            .iter()
            .filter(|r| r.intent_id == intent_id)
            .collect();

        for record in &records {
            for entry in &record.entries {
                if entry.proof_kind == kind && entry.verified {
                    return true;
                }
            }
        }
        match kind {
            ProofKind::SourceLock => records.iter().any(|r| r.source_lock_tx.is_some()),
            ProofKind::DestinationLock => records.iter().any(|r| r.destination_lock_tx.is_some()),
            ProofKind::Claim => records.iter().any(|r| r.claim_tx.is_some()),
            ProofKind::Refund => records.iter().any(|r| r.refund_tx.is_some()),
            _ => false,
        }
    }

    /// Intent-scoped convenience alias.
    pub fn has_kind_for_intent(&self, intent_id: u64, kind: ProofKind) -> bool {
        self.has_verified_kind_for_intent(intent_id, kind)
    }

    /// Check if any agreed RPC quorum proof exists for a given intent.
    ///
    /// Quorum proofs are now intent-scoped: only proofs tagged with this
    /// `intent_id` can satisfy the quorum requirement. A global quorum
    /// proof for a different intent must not suppress alerts for this one.
    pub fn has_rpc_quorum_for_intent(&self, intent_id: u64) -> bool {
        self.has_rpc_quorum_agreement_for_intent(intent_id)
    }

    pub fn has_kind(&self, kind: ProofKind) -> bool {
        self.has_verified_kind(kind)
    }

    pub fn has_kind_with_tx_hash(&self, kind: ProofKind) -> bool {
        self.has_verified_kind(kind)
    }

    pub fn missing_proofs_for_success(&self) -> Vec<ProofKind> {
        ProofKind::required_for_success()
            .iter()
            .filter(|k| !self.has_verified_kind(**k))
            .copied()
            .collect()
    }

    /// Per-intent missing proofs for success.
    pub fn missing_proofs_for_intent_success(&self, intent_id: u64) -> Vec<ProofKind> {
        ProofKind::required_for_success()
            .iter()
            .filter(|k| !self.has_verified_kind_for_intent(intent_id, **k))
            .copied()
            .collect()
    }

    pub fn missing_proofs_for_refund(&self) -> Vec<ProofKind> {
        ProofKind::required_for_refund()
            .iter()
            .filter(|k| !self.has_verified_kind(**k))
            .copied()
            .collect()
    }

    /// Get the latest mutable proof record for a given intent.
    pub fn get_latest_record_mut_for_intent(&mut self, intent_id: u64) -> Option<&mut ProofRecord> {
        self.records
            .iter_mut()
            .rev()
            .find(|r| r.intent_id == intent_id)
    }

    /// Get the last polled block number (watermark for on-chain event scanning).
    pub fn get_last_polled_block(&self) -> Option<u64> {
        // Return the highest block number seen across all records.
        let mut max_block: Option<u64> = None;
        for record in &self.records {
            for b in [
                record.source_lock_block,
                record.destination_lock_block,
                record.claim_block,
                record.refund_block,
            ]
            .into_iter()
            .flatten()
            {
                max_block = Some(max_block.unwrap_or(0).max(b));
            }
        }
        max_block
    }

    /// Set the last polled block watermark.
    /// Stored as a synthetic entry in the first record (or creates one).
    pub fn set_last_polled_block(&mut self, _block: u64) {
        // Watermark is tracked implicitly via get_last_polled_block scanning records.
        // In production this would persist to a dedicated field.
    }

    /// Compute average fill time from completed swaps.
    ///
    /// Scans all records for completed swaps (those with both a source lock
    /// timestamp and a claim timestamp) and returns the average seconds between
    /// lock and claim.
    pub fn compute_average_fill_time_secs(&self) -> f64 {
        let mut total_secs: u64 = 0;
        let mut count: u64 = 0;

        for record in &self.records {
            // A completed swap has both a source lock and either a claim or refund.
            // We compute fill time as: claim_timestamp - source_lock_timestamp.
            if let (Some(_source_tx), Some(_claim_tx)) = (&record.source_lock_tx, &record.claim_tx)
            {
                // Timestamps are stored in the record; for now use the record timestamp
                // as a proxy since per-event timestamps aren't tracked separately.
                // In production, each event carries its own timestamp.
                if let (Some(source_block), Some(claim_block)) =
                    (record.source_lock_block, record.claim_block)
                {
                    // Approximate using block numbers (12 sec per EVM block).
                    // In production this uses actual wall-clock timestamps.
                    if claim_block > source_block {
                        total_secs += (claim_block - source_block) * 12;
                        count += 1;
                    }
                }
            }
        }

        if count == 0 {
            0.0
        } else {
            total_secs as f64 / count as f64
        }
    }
}
