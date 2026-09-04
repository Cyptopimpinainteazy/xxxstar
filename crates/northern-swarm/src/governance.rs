//! Swarm Governance Code Enforcement (RC2).
//!
//! Implements the quorum gate, Sentinel-Judge/Scribe enforcement, degrade state
//! machine, and agent kill path as specified in the swarm governance docs.
//!
//! ## Quorum Gate
//! All task results must pass a quorum check before being accepted. The gate
//! compares result hashes from ≥3 executors and requires ≥2/3 match.
//!
//! ## Sentinel-Judge/Scribe
//! - **Sentinel**: monitors executor behavior, detects anomalies
//! - **Judge**: evaluates evidence and issues verdicts
//! - **Scribe**: records verdicts in the immutable audit trail
//!
//! ## Degrade State Machine
//! Executors progress through states: Active → Warning → Degraded → Suspended → Killed.
//! Each state reduces privileges and increases scrutiny.

use crate::types::*;
use std::collections::HashMap;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Quorum Gate
// ---------------------------------------------------------------------------

/// Quorum result from comparing multiple executor result hashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuorumVerdict {
    /// Enough executors agreed; result is trusted.
    Passed { result_hash: String, agreement_count: usize },
    /// Too few agreeing executors; result is rejected.
    Failed { expected: usize, actual: usize },
    /// No results available to compare.
    InsufficientData,
}

/// Minimum number of executor results required for quorum.
pub const QUORUM_MIN_EXECUTORS: usize = 3;

/// Minimum fraction of executors that must agree (2/3).
pub fn quorum_threshold(total: usize) -> usize {
    let needed = total.saturating_mul(2) / 3;
    needed.max(1)
}

/// Run the quorum gate on a set of executor result hashes.
///
/// Returns `QuorumVerdict::Passed` if ≥2/3 of executors agree on the same hash.
pub fn check_quorum(results: &HashMap<ExecutorId, String>) -> QuorumVerdict {
    if results.len() < QUORUM_MIN_EXECUTORS {
        return QuorumVerdict::InsufficientData;
    }

    // Count votes per result hash
    let mut vote_counts: HashMap<&str, usize> = HashMap::new();
    for hash in results.values() {
        *vote_counts.entry(hash.as_str()).or_insert(0) += 1;
    }

    // Find the highest-voted hash
    let (best_hash, best_count) = vote_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .unwrap_or(("", 0));

    let needed = quorum_threshold(results.len());
    if best_count >= needed {
        QuorumVerdict::Passed {
            result_hash: best_hash.to_string(),
            agreement_count: best_count,
        }
    } else {
        QuorumVerdict::Failed {
            expected: needed,
            actual: best_count,
        }
    }
}

// ---------------------------------------------------------------------------
// Sentinel — Anomaly Detection
// ---------------------------------------------------------------------------

/// Types of executor anomalies the Sentinel can detect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anomaly {
    /// Executor failed to submit a result within the timeout.
    Timeout,
    /// Executor's result hash diverged from the quorum majority.
    ResultMismatch,
    /// Executor submitted a result for a task it did not claim.
    UnauthorizedSubmission,
    /// Executor heartbeat stopped.
    HeartbeatLost,
}

/// Sentinel monitors executor behavior across tasks.
pub struct Sentinel {
    /// Per-executor anomaly count
    anomalies: HashMap<ExecutorId, Vec<(TaskId, Anomaly)>>,
    /// Current misconduct ladder state per executor
    ladder_state: HashMap<ExecutorId, MisconductState>,
}

/// Executor misconduct progression ladder.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MisconductState {
    Clean,
    Warning,
    Degraded,
    Suspended,
    Killed,
}

impl Sentinel {
    pub fn new() -> Self {
        Sentinel {
            anomalies: HashMap::new(),
            ladder_state: HashMap::new(),
        }
    }

    /// Record an anomaly and escalate if threshold is exceeded.
    pub fn report_anomaly(&mut self, executor_id: ExecutorId, task_id: TaskId, anomaly: Anomaly) {
        self.anomalies
            .entry(executor_id.clone())
            .or_default()
            .push((task_id, anomaly.clone()));

        self.escalate(&executor_id);
    }

    /// Get current misconduct state for an executor.
    pub fn get_state(&self, executor_id: &str) -> MisconductState {
        self.ladder_state
            .get(executor_id)
            .cloned()
            .unwrap_or(MisconductState::Clean)
    }

    /// Get all anomalies for an executor.
    pub fn get_anomalies(&self, executor_id: &str) -> &[(TaskId, Anomaly)] {
        self.anomalies
            .get(executor_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Escalate an executor through the misconduct ladder based on anomaly count.
    fn escalate(&mut self, executor_id: &str) {
        let count = self
            .anomalies
            .get(executor_id)
            .map(|v| v.len())
            .unwrap_or(0);

        let new_state = match count {
            0 => MisconductState::Clean,
            1..=2 => MisconductState::Warning,
            3..=4 => MisconductState::Degraded,
            5..=9 => MisconductState::Suspended,
            _ => MisconductState::Killed,
        };

        let current = self
            .ladder_state
            .entry(executor_id.to_string())
            .or_insert(MisconductState::Clean);

        if *current != new_state {
            info!(
                executor_id = %executor_id,
                from = ?current,
                to = ?new_state,
                anomalies = count,
                "Sentinel escalated executor through misconduct ladder",
            );
            *current = new_state;
        }
    }
}

// ---------------------------------------------------------------------------
// Judge — Verdict Engine
// ---------------------------------------------------------------------------

/// A verdict issued by the Judge after evaluating Sentinel reports.
#[derive(Debug, Clone)]
pub struct Verdict {
    pub executor_id: ExecutorId,
    pub task_id: TaskId,
    pub verdict: VerdictType,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerdictType {
    /// Executor cleared of wrongdoing.
    Exonerated,
    /// Executor's result accepted despite minor anomaly.
    Warning,
    /// Executor's result rejected; reputation penalty.
    Slashed,
    /// Executor permanently banned.
    Banned,
    /// Task assigned to another executor.
    Reassigned,
}

/// Judge evaluates evidence from the Sentinel and produces verdicts.
pub struct Judge;

impl Judge {
    /// Evaluate a Sentinel anomaly report and produce a verdict.
    pub fn evaluate(
        executor_id: &str,
        task_id: &str,
        anomaly: &Anomaly,
        ladder_state: &MisconductState,
    ) -> Verdict {
        match (anomaly, ladder_state) {
            (Anomaly::ResultMismatch, MisconductState::Clean)
            | (Anomaly::Timeout, MisconductState::Clean) => Verdict {
                executor_id: executor_id.to_string(),
                task_id: task_id.to_string(),
                verdict: VerdictType::Warning,
                reason: "First-time offense; warning issued".into(),
            },
            (Anomaly::ResultMismatch, MisconductState::Warning) => Verdict {
                executor_id: executor_id.to_string(),
                task_id: task_id.to_string(),
                verdict: VerdictType::Slashed,
                reason: "Repeated result mismatch; reputation slashed".into(),
            },
            (Anomaly::HeartbeatLost, _) => Verdict {
                executor_id: executor_id.to_string(),
                task_id: task_id.to_string(),
                verdict: VerdictType::Reassigned,
                reason: "Executor offline; tasks reassigned".into(),
            },
            (_, MisconductState::Degraded) => Verdict {
                executor_id: executor_id.to_string(),
                task_id: task_id.to_string(),
                verdict: VerdictType::Slashed,
                reason: "Executor in degraded state; all results slashed".into(),
            },
            (_, MisconductState::Suspended) | (_, MisconductState::Killed) => Verdict {
                executor_id: executor_id.to_string(),
                task_id: task_id.to_string(),
                verdict: VerdictType::Banned,
                reason: "Executor suspended or killed; permanently banned".into(),
            },
            _ => Verdict {
                executor_id: executor_id.to_string(),
                task_id: task_id.to_string(),
                verdict: VerdictType::Warning,
                reason: "Unrecognized anomaly pattern; warning issued".into(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Scribe — Audit Trail
// ---------------------------------------------------------------------------

/// A single entry in the immutable audit trail.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: i64,
    pub executor_id: ExecutorId,
    pub task_id: TaskId,
    pub event: AuditEvent,
}

#[derive(Debug, Clone)]
pub enum AuditEvent {
    Anomaly(Anomaly),
    Verdict(Verdict),
    StateChange(MisconductState),
    QuorumResult(QuorumVerdict),
    KillPath(String),
}

/// Scribe records all governance events in an append-only audit trail.
pub struct Scribe {
    trail: Vec<AuditEntry>,
}

impl Scribe {
    pub fn new() -> Self {
        Scribe { trail: Vec::new() }
    }

    /// Record an event in the audit trail.
    pub fn record(
        &mut self,
        executor_id: ExecutorId,
        task_id: TaskId,
        event: AuditEvent,
    ) {
        let entry = AuditEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            executor_id,
            task_id,
            event,
        };
        self.trail.push(entry);
    }

    /// Get the full audit trail.
    pub fn trail(&self) -> &[AuditEntry] {
        &self.trail
    }

    /// Get audit entries for a specific executor.
    pub fn entries_for(&self, executor_id: &str) -> Vec<&AuditEntry> {
        self.trail
            .iter()
            .filter(|e| e.executor_id == executor_id)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Kill Path
// ---------------------------------------------------------------------------

/// Kill an executor — permanently remove from the active set and slash full bond.
pub fn kill_executor(
    executor_id: &str,
    reason: &str,
    scribe: &mut Scribe,
) -> KillResult {
    info!(
        executor_id = %executor_id,
        reason = %reason,
        "KILL PATH activated for executor",
    );

    scribe.record(
        executor_id.to_string(),
        "KILL".into(),
        AuditEvent::KillPath(format!("Executor killed: {reason}")),
    );

    KillResult {
        executor_id: executor_id.to_string(),
        slashed: true,
        banned: true,
    }
}

/// Result of executing the kill path on an executor.
#[derive(Debug, Clone)]
pub struct KillResult {
    pub executor_id: ExecutorId,
    pub slashed: bool,
    pub banned: bool,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quorum_passes_with_2_3_majority() {
        let mut results = HashMap::new();
        results.insert("exec1".into(), "hash_a".into());
        results.insert("exec2".into(), "hash_a".into());
        results.insert("exec3".into(), "hash_b".into());

        let verdict = check_quorum(&results);
        assert_eq!(
            verdict,
            QuorumVerdict::Passed {
                result_hash: "hash_a".into(),
                agreement_count: 2,
            }
        );
    }

    #[test]
    fn test_quorum_fails_with_minority() {
        let mut results = HashMap::new();
        results.insert("exec1".into(), "hash_a".into());
        results.insert("exec2".into(), "hash_b".into());
        results.insert("exec3".into(), "hash_c".into());

        let verdict = check_quorum(&results);
        assert_eq!(
            verdict,
            QuorumVerdict::Failed {
                expected: 2,
                actual: 1,
            }
        );
    }

    #[test]
    fn test_quorum_insufficient_executors() {
        let mut results = HashMap::new();
        results.insert("exec1".into(), "hash_a".into());

        let verdict = check_quorum(&results);
        assert_eq!(verdict, QuorumVerdict::InsufficientData);
    }

    #[test]
    fn test_sentinel_escalation() {
        let mut sentinel = Sentinel::new();

        // 1 anomaly → Warning
        sentinel.report_anomaly("exec1".into(), "task1".into(), Anomaly::Timeout);
        assert_eq!(
            sentinel.get_state("exec1"),
            MisconductState::Warning
        );

        // 3 anomalies → Degraded
        sentinel.report_anomaly("exec1".into(), "task2".into(), Anomaly::ResultMismatch);
        sentinel.report_anomaly("exec1".into(), "task3".into(), Anomaly::ResultMismatch);
        assert_eq!(
            sentinel.get_state("exec1"),
            MisconductState::Degraded
        );

        // 5 anomalies → Suspended
        sentinel.report_anomaly("exec1".into(), "task4".into(), Anomaly::Timeout);
        sentinel.report_anomaly("exec1".into(), "task5".into(), Anomaly::ResultMismatch);
        assert_eq!(
            sentinel.get_state("exec1"),
            MisconductState::Suspended
        );

        // 10 anomalies → Killed
        for i in 6..=11 {
            sentinel.report_anomaly(
                "exec1".into(),
                format!("task{i}"),
                Anomaly::HeartbeatLost,
            );
        }
        assert_eq!(sentinel.get_state("exec1"), MisconductState::Killed);
    }

    #[test]
    fn test_judge_issues_verdict() {
        let verdict = Judge::evaluate("exec1", "task1", &Anomaly::Timeout, &MisconductState::Clean);
        assert_eq!(verdict.verdict, VerdictType::Warning);

        let verdict = Judge::evaluate(
            "exec1",
            "task1",
            &Anomaly::ResultMismatch,
            &MisconductState::Warning,
        );
        assert_eq!(verdict.verdict, VerdictType::Slashed);

        let verdict = Judge::evaluate(
            "exec1",
            "task1",
            &Anomaly::HeartbeatLost,
            &MisconductState::Clean,
        );
        assert_eq!(verdict.verdict, VerdictType::Reassigned);
    }

    #[test]
    fn test_scribe_audit_trail() {
        let mut scribe = Scribe::new();
        scribe.record(
            "exec1".into(),
            "task1".into(),
            AuditEvent::Anomaly(Anomaly::Timeout),
        );

        assert_eq!(scribe.trail().len(), 1);
        assert_eq!(scribe.entries_for("exec1").len(), 1);
        assert_eq!(scribe.entries_for("nonexistent").len(), 0);
    }

    #[test]
    fn test_kill_path() {
        let mut scribe = Scribe::new();
        let result = kill_executor("rogue-executor", "Malicious behavior detected", &mut scribe);

        assert!(result.slashed);
        assert!(result.banned);
        assert_eq!(scribe.entries_for("rogue-executor").len(), 1);
    }
}