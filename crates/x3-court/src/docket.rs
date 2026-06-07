//! Court docket — registry of all disputes and verdicts.

use crate::error::CourtError;
use crate::types::*;
use std::collections::HashMap;
use x3_proof::types::BlockHeight;

/// The court docket — permanent record of all disputes.
pub struct CourtDocket {
    /// All disputes by ID.
    disputes: HashMap<u64, Dispute>,
    /// Disputes by respondent pubkey.
    by_respondent: HashMap<[u8; 32], Vec<u64>>,
    /// Active (unresolved) dispute IDs.
    active: Vec<u64>,
}

impl CourtDocket {
    /// Create a new empty docket.
    pub fn new() -> Self {
        Self {
            disputes: HashMap::new(),
            by_respondent: HashMap::new(),
            active: Vec::new(),
        }
    }

    /// Register a dispute.
    pub fn register(&mut self, dispute: Dispute) -> Result<(), CourtError> {
        let id = dispute.id.0;
        if self.disputes.contains_key(&id) {
            return Err(CourtError::DuplicateDispute(dispute.id));
        }

        self.by_respondent
            .entry(dispute.respondent.pubkey)
            .or_default()
            .push(id);

        self.active.push(id);
        self.disputes.insert(id, dispute);
        Ok(())
    }

    /// Get a dispute by ID.
    pub fn get(&self, id: DisputeId) -> Option<&Dispute> {
        self.disputes.get(&id.0)
    }

    /// Get a mutable dispute by ID.
    pub fn get_mut(&mut self, id: DisputeId) -> Option<&mut Dispute> {
        self.disputes.get_mut(&id.0)
    }

    /// Get all disputes for a respondent.
    pub fn get_by_respondent(&self, pubkey: &[u8; 32]) -> Vec<&Dispute> {
        self.by_respondent
            .get(pubkey)
            .map(|ids| ids.iter().filter_map(|id| self.disputes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all active (unresolved) disputes.
    pub fn active_disputes(&self) -> Vec<&Dispute> {
        self.active
            .iter()
            .filter_map(|id| self.disputes.get(id))
            .filter(|d| matches!(d.state, DisputeState::Filed | DisputeState::Replaying))
            .collect()
    }

    /// Process timed-out disputes.
    pub fn process_timeouts(&mut self, current_block: BlockHeight) -> Vec<DisputeId> {
        let mut timed_out = Vec::new();

        for id in &self.active {
            if let Some(dispute) = self.disputes.get(id) {
                if current_block > dispute.deadline
                    && matches!(dispute.state, DisputeState::Filed | DisputeState::Replaying)
                {
                    timed_out.push(DisputeId(*id));
                }
            }
        }

        for id in &timed_out {
            if let Some(dispute) = self.disputes.get_mut(&id.0) {
                dispute.state = DisputeState::Dismissed;
            }
        }

        self.active.retain(|id| {
            self.disputes
                .get(id)
                .map(|d| matches!(d.state, DisputeState::Filed | DisputeState::Replaying))
                .unwrap_or(false)
        });

        timed_out
    }

    /// Get all verdicts.
    pub fn verdicts(&self) -> Vec<&VerdictRecord> {
        self.disputes
            .values()
            .filter_map(|d| d.verdict.as_ref())
            .collect()
    }

    /// Total disputes filed.
    pub fn total_count(&self) -> usize {
        self.disputes.len()
    }

    /// Number of active disputes.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Number of guilty verdicts.
    pub fn guilty_count(&self) -> usize {
        self.disputes
            .values()
            .filter_map(|d| d.verdict.as_ref())
            .filter(|v| v.outcome == VerdictOutcome::Guilty)
            .count()
    }
}

impl Default for CourtDocket {
    fn default() -> Self {
        Self::new()
    }
}
