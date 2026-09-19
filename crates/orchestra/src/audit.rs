//! Append-only audit trail — Commandment III, "every action must be logged".
//!
//! Every decision boundary in the Orchestra records what an agent did and why.
//! The log is **bounded**: an unbounded in-memory log is a memory leak in a
//! node that stays up for weeks, so the oldest entries are evicted once the
//! capacity is reached. The number of evicted entries is itself recorded,
//! because a log that quietly drops history is not an audit trail — a reader
//! can tell the difference between "nothing was dropped" and "the beginning is
//! gone".

use crate::score::Commandment;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// The largest capacity [`AuditLog::new`] will pre-reserve for. A larger
/// capacity is honoured, it just does not pre-allocate the whole ring.
const PREALLOC_CEILING: usize = 1024;

/// What an audit entry records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEvent {
    /// A Commandment was violated.
    Violation(Commandment),
    /// A task lifecycle event, e.g. `execution_started`.
    Task {
        /// Name of the lifecycle event.
        event: String,
    },
}

/// A single audit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// The agent this entry is about.
    pub agent_id: u32,
    /// The task this entry is about, when the entry is task-scoped.
    pub task_id: Option<String>,
    /// What happened.
    pub event: AuditEvent,
    /// Human-readable detail.
    pub detail: String,
    /// When the entry was recorded.
    pub timestamp: DateTime<Utc>,
}

impl AuditEntry {
    /// Record a violation of the Score.
    pub fn violation(agent_id: u32, commandment: Commandment, detail: String) -> Self {
        Self {
            agent_id,
            task_id: None,
            event: AuditEvent::Violation(commandment),
            detail,
            timestamp: Utc::now(),
        }
    }

    /// Record a task lifecycle event.
    pub fn task_event(agent_id: u32, task_id: &str, event: &str, detail: &str) -> Self {
        Self {
            agent_id,
            task_id: Some(task_id.to_string()),
            event: AuditEvent::Task {
                event: event.to_string(),
            },
            detail: detail.to_string(),
            timestamp: Utc::now(),
        }
    }

    /// The violated Commandment, if this entry records a violation.
    pub fn violated_commandment(&self) -> Option<Commandment> {
        match self.event {
            AuditEvent::Violation(commandment) => Some(commandment),
            AuditEvent::Task { .. } => None,
        }
    }

    /// The task lifecycle event name, if this entry records one.
    pub fn task_event_name(&self) -> Option<&str> {
        match &self.event {
            AuditEvent::Task { event } => Some(event.as_str()),
            AuditEvent::Violation(_) => None,
        }
    }
}

/// A bounded, append-only audit log.
#[derive(Debug, Clone)]
pub struct AuditLog {
    capacity: usize,
    entries: VecDeque<AuditEntry>,
    evicted: u64,
}

impl AuditLog {
    /// Create a log that retains at most `capacity` entries.
    ///
    /// A capacity of zero would make the log a no-op that silently discards
    /// everything, so it is clamped to one entry.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: VecDeque::with_capacity(capacity.clamp(1, PREALLOC_CEILING)),
            evicted: 0,
        }
    }

    /// Append an entry, evicting the oldest entry once the log is full.
    pub fn append(&mut self, entry: AuditEntry) {
        while self.entries.len() >= self.capacity {
            self.entries.pop_front();
            self.evicted += 1;
        }
        self.entries.push_back(entry);
    }

    /// The configured capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// How many entries the log currently holds.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many entries have been evicted to stay within capacity.
    ///
    /// A non-zero value means the oldest history is no longer in the log.
    pub fn evicted(&self) -> u64 {
        self.evicted
    }

    /// The retained entries, oldest first.
    pub fn entries(&self) -> &VecDeque<AuditEntry> {
        &self.entries
    }

    /// The retained entries, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &AuditEntry> {
        self.entries.iter()
    }

    /// The retained entries that record a Score violation.
    pub fn violations(&self) -> impl Iterator<Item = &AuditEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.violated_commandment().is_some())
    }

    /// The retained entries about one agent.
    pub fn for_agent(&self, agent_id: u32) -> impl Iterator<Item = &AuditEntry> {
        self.entries
            .iter()
            .filter(move |entry| entry.agent_id == agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(agent_id: u32) -> AuditEntry {
        AuditEntry::task_event(agent_id, "task-1", "execution_started", "started")
    }

    #[test]
    fn entries_are_kept_oldest_first() {
        let mut log = AuditLog::new(10);
        log.append(entry(1));
        log.append(entry(2));
        log.append(entry(3));

        let agent_ids: Vec<u32> = log.iter().map(|e| e.agent_id).collect();
        assert_eq!(agent_ids, vec![1, 2, 3]);
        assert_eq!(log.len(), 3);
        assert_eq!(log.evicted(), 0);
    }

    #[test]
    fn full_log_evicts_the_oldest_entry_and_counts_it() {
        let mut log = AuditLog::new(2);
        log.append(entry(1));
        log.append(entry(2));
        log.append(entry(3));

        let agent_ids: Vec<u32> = log.iter().map(|e| e.agent_id).collect();
        assert_eq!(agent_ids, vec![2, 3], "the oldest entry must be dropped");
        assert_eq!(log.len(), 2);
        assert_eq!(log.evicted(), 1, "eviction must not be silent");
    }

    #[test]
    fn zero_capacity_still_retains_the_last_entry() {
        let mut log = AuditLog::new(0);
        log.append(entry(1));
        log.append(entry(2));

        assert_eq!(log.capacity(), 1);
        assert_eq!(log.len(), 1);
        assert_eq!(log.iter().map(|e| e.agent_id).collect::<Vec<_>>(), vec![2]);
        assert_eq!(log.evicted(), 1);
    }

    #[test]
    fn a_violation_entry_carries_its_commandment() {
        let entry = AuditEntry::violation(
            7,
            Commandment::NoAgentSovereignty,
            "claimed sovereign execution".into(),
        );

        assert_eq!(
            entry.violated_commandment(),
            Some(Commandment::NoAgentSovereignty)
        );
        assert_eq!(entry.task_event_name(), None);
        assert_eq!(entry.task_id, None);
    }

    #[test]
    fn a_task_entry_carries_its_task_and_event() {
        let entry = AuditEntry::task_event(7, "task-9", "execution_completed", "ok");

        assert_eq!(entry.violated_commandment(), None);
        assert_eq!(entry.task_event_name(), Some("execution_completed"));
        assert_eq!(entry.task_id.as_deref(), Some("task-9"));
        assert_eq!(entry.agent_id, 7);
    }

    #[test]
    fn violations_and_agent_filters_select_the_right_entries() {
        let mut log = AuditLog::new(10);
        log.append(entry(1));
        log.append(AuditEntry::violation(
            2,
            Commandment::ImmutableLogging,
            "wrote without a log entry".into(),
        ));
        log.append(AuditEntry::violation(
            1,
            Commandment::NoJuryInfluence,
            "messaged a juror mid-rotation".into(),
        ));

        assert_eq!(log.violations().count(), 2);
        assert_eq!(
            log.for_agent(1)
                .map(|e| e.violated_commandment())
                .collect::<Vec<_>>(),
            vec![None, Some(Commandment::NoJuryInfluence)]
        );
    }

    #[test]
    fn an_entry_survives_a_serde_round_trip() {
        let entry = AuditEntry::violation(3, Commandment::OffChainReadOnly, "wrote".into());
        let json = serde_json::to_string(&entry).expect("serialises");
        let restored: AuditEntry = serde_json::from_str(&json).expect("deserialises");

        assert_eq!(entry, restored);
    }
}
