//! Unified audit trail — append-only log of all significant swarm events.
//!
//! Every enforcement decision, sanctions change, genesis registration,
//! circuit-breaker trip, and emergency action should be recorded here.
//! The trail is the canonical source of truth for operator review and
//! post-incident analysis.
//!
//! This module is intentionally minimal: it provides the data model and
//! an in-memory append-only store. Production backends (e.g. pallet storage,
//! log sink) wrap this.

use crate::genesis::AgentId;
use serde::{Deserialize, Serialize};

pub type BlockHeight = u64;

/// Category of the audited event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditCategory {
    /// An agent's genesis record was created.
    GenesisCreated,
    /// An agent was terminated (operator-initiated or expiry).
    AgentTerminated,
    /// An agent reached Kill sanction via the misconduct ladder.
    AgentKilled,
    /// An agent's expiry was set or updated.
    ExpirySet,
    /// A misconduct violation was recorded.
    ViolationRecorded,
    /// A sanction level changed.
    SanctionChanged,
    /// An agent spawn was allowed.
    SpawnAllowed,
    /// An agent spawn was rejected (includes reason).
    SpawnRejected,
    /// A circuit breaker was tripped.
    CircuitBreakerTripped,
    /// A circuit breaker was reset.
    CircuitBreakerReset,
    /// A scope was degraded.
    ScopeDegraded,
    /// A scope was undegraded.
    ScopeUndegraded,
    /// A proof was finalised for the first time.
    ProofFinalised,
    /// A double-finalisation attempt was rejected (INV-R-002).
    DoubleFinalityRejected,
    /// An emergency power action was invoked.
    EmergencyPower,
    /// Generic operator action.
    OperatorAction,
}

/// A single immutable audit entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Sequential entry ID (1-indexed).
    pub id: u64,
    /// Block at which the event occurred.
    pub block: BlockHeight,
    /// Category of the event.
    pub category: AuditCategory,
    /// Optional agent id this event relates to.
    pub agent_id: Option<AgentId>,
    /// Human-readable summary (max 256 bytes on insert).
    pub summary: String,
    /// Optional structured payload (JSON string, <= 1024 bytes on insert).
    pub payload: Option<String>,
}

/// Append-only audit log.
#[derive(Debug, Default)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
    next_id: u64,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    /// Append an event to the log. Returns the assigned entry id.
    pub fn record(
        &mut self,
        block: BlockHeight,
        category: AuditCategory,
        agent_id: Option<AgentId>,
        summary: impl Into<String>,
        payload: Option<String>,
    ) -> u64 {
        let summary = {
            let s: String = summary.into();
            if s.len() > 256 {
                s[..256].to_string()
            } else {
                s
            }
        };
        let payload = payload.map(|p| if p.len() > 1024 { p[..1024].to_string() } else { p });

        let id = self.next_id;
        self.entries.push(AuditEntry {
            id,
            block,
            category,
            agent_id,
            summary,
            payload,
        });
        self.next_id += 1;
        id
    }

    /// Returns all entries (immutable slice).
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Returns all entries for a specific agent.
    pub fn entries_for_agent(&self, agent_id: &AgentId) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.agent_id.as_ref() == Some(agent_id))
            .collect()
    }

    /// Returns all entries in a block range [from, to] inclusive.
    pub fn entries_in_range(&self, from: BlockHeight, to: BlockHeight) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.block >= from && e.block <= to)
            .collect()
    }

    /// Total entries logged.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if no events have been recorded.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_increments_ids() {
        let mut log = AuditLog::new();
        let id1 = log.record(1, AuditCategory::OperatorAction, None, "action 1", None);
        let id2 = log.record(2, AuditCategory::OperatorAction, None, "action 2", None);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn filter_by_agent() {
        let mut log = AuditLog::new();
        let agent = [1u8; 32];
        let other = [2u8; 32];
        log.record(10, AuditCategory::GenesisCreated, Some(agent), "created", None);
        log.record(11, AuditCategory::GenesisCreated, Some(other), "other", None);
        log.record(12, AuditCategory::ViolationRecorded, Some(agent), "violation", None);

        let entries = log.entries_for_agent(&agent);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.agent_id == Some(agent)));
    }

    #[test]
    fn filter_by_range() {
        let mut log = AuditLog::new();
        for i in 0u64..10 {
            log.record(i * 10, AuditCategory::OperatorAction, None, "x", None);
        }
        let range = log.entries_in_range(20, 50);
        // blocks: 20, 30, 40, 50 → 4 entries
        assert_eq!(range.len(), 4);
    }

    #[test]
    fn summary_truncated_at_256() {
        let mut log = AuditLog::new();
        let long = "a".repeat(1000);
        log.record(1, AuditCategory::OperatorAction, None, long, None);
        assert_eq!(log.entries()[0].summary.len(), 256);
    }

    #[test]
    fn payload_truncated_at_1024() {
        let mut log = AuditLog::new();
        let long = Some("b".repeat(2000));
        log.record(1, AuditCategory::OperatorAction, None, "s", long);
        assert_eq!(log.entries()[0].payload.as_ref().unwrap().len(), 1024);
    }
}
