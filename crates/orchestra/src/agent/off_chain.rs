//! Off-chain agents — jury members, adversarial auditors, ephemeral stress-testers.
//!
//! Off-chain agents are:
//! - Fully isolated from on-chain state (except read-only snapshots)
//! - No writeback until majority vote resolves
//! - Anonymous voting enforced; only outcome is visible
//!
//! Lifecycle:
//! 1. Spawned for specific jury session
//! 2. Receives tasks and law proposals
//! 3. Votes Yes/No on each
//! 4. Logs encrypted, submitted for audit
//! 5. Retired or recycled (scrap yard) if misaligned

use crate::audit::AuditEntry;
use crate::score::{ActionContext, ScoreEnforcer, ScoreViolation};

use super::identity::{AgentDomain, AgentId, AgentIdentity, AlignmentScore, OrchestraSection};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Role of an off-chain agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OffChainRole {
    /// Jury member for major task approval.
    JuryMember,
    /// Adversarial auditor — tests system resilience.
    AdversarialAuditor,
    /// Ephemeral stress-tester — simulates failure scenarios.
    StressTester,
}

impl std::fmt::Display for OffChainRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JuryMember => write!(f, "jury-member"),
            Self::AdversarialAuditor => write!(f, "adversarial-auditor"),
            Self::StressTester => write!(f, "stress-tester"),
        }
    }
}

/// Status of an off-chain agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OffChainStatus {
    /// Spawned and ready for session.
    Ready,
    /// Currently reviewing tasks.
    Reviewing,
    /// Currently voting on tasks.
    Voting,
    /// Generating audit reports.
    Auditing,
    /// Running stress test simulations.
    StressTesting,
    /// Session complete, awaiting retirement or recycling.
    SessionComplete,
    /// Retired to scrap yard.
    Retired,
}

/// Snapshot of on-chain state provided to off-chain agents (read-only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSnapshot {
    /// Block number at snapshot time.
    pub block_number: u64,
    /// Block hash for verification.
    pub block_hash: String,
    /// Timestamp of snapshot.
    pub timestamp: DateTime<Utc>,
    /// Relevant agent states (sanitized, no private keys).
    pub agent_states: Vec<AgentSnapshotEntry>,
    /// Pending tasks requiring jury review.
    pub pending_tasks: Vec<String>,
    /// Recent audit log entries.
    pub recent_audit_entries: Vec<AuditEntry>,
}

/// Sanitized agent state in a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshotEntry {
    pub agent_id: AgentId,
    pub section: OrchestraSection,
    pub alignment: AlignmentScore,
    pub tasks_completed: u64,
    pub violations: u64,
}

/// An off-chain agent in the Orchestra.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffChainAgent {
    /// Core identity.
    pub identity: AgentIdentity,
    /// Off-chain role.
    pub role: OffChainRole,
    /// Current operational status.
    pub status: OffChainStatus,
    /// Chain snapshot this agent is operating on (read-only).
    pub snapshot: Option<ChainSnapshot>,
    /// Session ID this agent is participating in.
    pub session_id: Option<String>,
    /// Tasks reviewed in this session.
    pub tasks_reviewed: Vec<String>,
    /// Encrypted vote commitments (hash of vote + nonce).
    pub vote_commitments: Vec<VoteCommitment>,
    /// Audit findings generated during session.
    pub audit_findings: VecDeque<AuditEntry>,
    /// Stress test results (if role is StressTester).
    pub stress_test_results: Vec<StressTestResult>,
}

/// A blinded vote commitment — preserves anonymity per Commandment V.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteCommitment {
    /// Task ID being voted on.
    pub task_id: String,
    /// Blake3 hash of (vote || nonce || agent_identity_hash).
    /// The actual vote is hidden until reveal phase (if ever).
    pub commitment_hash: [u8; 32],
    /// Timestamp of commitment.
    pub committed_at: DateTime<Utc>,
}

/// Result of a stress test simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResult {
    /// What scenario was tested.
    pub scenario: String,
    /// Whether the system maintained integrity.
    pub integrity_maintained: bool,
    /// Failure mode observed (if any).
    pub failure_mode: Option<String>,
    /// Recommendations.
    pub recommendations: Vec<String>,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
}

impl OffChainAgent {
    /// Create a new off-chain agent for a specific session.
    pub fn new(
        id: AgentId,
        display_name: String,
        section: OrchestraSection,
        role: OffChainRole,
    ) -> Self {
        let now = Utc::now();
        let identity_hash = {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(id.to_le_bytes());
            hasher.update(now.timestamp().to_le_bytes());
            hasher.update(b"off-chain");
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            hash
        };

        Self {
            identity: AgentIdentity {
                id,
                display_name,
                section,
                alignment: AlignmentScore::NEUTRAL,
                domain: AgentDomain::OffChain,
                spawned_at: now,
                tasks_completed: 0,
                jury_sessions_served: 0,
                violations: 0,
                identity_hash,
            },
            role,
            status: OffChainStatus::Ready,
            snapshot: None,
            session_id: None,
            tasks_reviewed: Vec::new(),
            vote_commitments: Vec::new(),
            audit_findings: VecDeque::new(),
            stress_test_results: Vec::new(),
        }
    }

    /// Create an off-chain agent from an existing on-chain agent (rotation).
    pub fn from_on_chain(
        on_chain_identity: AgentIdentity,
        role: OffChainRole,
        session_id: String,
    ) -> Self {
        Self {
            identity: AgentIdentity {
                domain: AgentDomain::OffChain,
                ..on_chain_identity
            },
            role,
            status: OffChainStatus::Ready,
            snapshot: None,
            session_id: Some(session_id),
            tasks_reviewed: Vec::new(),
            vote_commitments: Vec::new(),
            audit_findings: VecDeque::new(),
            stress_test_results: Vec::new(),
        }
    }

    /// Receive a chain snapshot (read-only view of on-chain state).
    pub fn receive_snapshot(&mut self, snapshot: ChainSnapshot) {
        self.snapshot = Some(snapshot);
        self.status = OffChainStatus::Reviewing;
    }

    /// Inspect a task spec and produce a vote commitment.
    /// The actual vote (Yes/No) is blinded by the commitment hash.
    pub fn vote_on_task(
        &mut self,
        task_id: &str,
        vote_yes: bool,
    ) -> Result<VoteCommitment, ScoreViolation> {
        // Validate against Score — ensure no identity leakage or influence
        ScoreEnforcer::validate_jury_vote(self.identity.id, false, false)?;

        // Validate off-chain constraints
        let ctx = ActionContext {
            action_type: format!("jury_vote:{}", task_id),
            is_protocol_bound: true,
            claims_sovereignty: false,
            is_loggable: true,
            writes_to_chain: false, // off-chain agents don't write to chain
        };
        ScoreEnforcer::validate_off_chain_action(self.identity.id, &ctx)?;

        // Generate blinded commitment: H(vote || nonce || identity_hash)
        let nonce: [u8; 16] = rand::random();
        let commitment_hash = {
            let mut data = Vec::new();
            data.push(if vote_yes { 1u8 } else { 0u8 });
            data.extend_from_slice(&nonce);
            data.extend_from_slice(&self.identity.identity_hash);
            *blake3::hash(&data).as_bytes()
        };

        let commitment = VoteCommitment {
            task_id: task_id.to_string(),
            commitment_hash,
            committed_at: Utc::now(),
        };

        self.vote_commitments.push(commitment.clone());
        self.tasks_reviewed.push(task_id.to_string());
        self.status = OffChainStatus::Voting;

        Ok(commitment)
    }

    /// Run a stress test simulation (StressTester role only).
    pub fn run_stress_test(&mut self, scenario: &str) -> Option<StressTestResult> {
        if self.role != OffChainRole::StressTester {
            return None;
        }

        self.status = OffChainStatus::StressTesting;

        // Simulate — in production this would run actual scenario execution
        let result = StressTestResult {
            scenario: scenario.to_string(),
            integrity_maintained: true, // placeholder
            failure_mode: None,
            recommendations: vec![],
            timestamp: Utc::now(),
        };

        self.stress_test_results.push(result.clone());
        Some(result)
    }

    /// Complete the session — prepare for retirement or recycling.
    pub fn complete_session(&mut self) {
        self.status = OffChainStatus::SessionComplete;
    }

    /// Retire this agent to the scrap yard.
    pub fn retire(&mut self) {
        self.status = OffChainStatus::Retired;
        self.identity.domain = AgentDomain::Retired;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_chain_agent_creation() {
        let agent = OffChainAgent::new(
            42,
            "juror-42".into(),
            OrchestraSection::Brass,
            OffChainRole::JuryMember,
        );
        assert_eq!(agent.status, OffChainStatus::Ready);
        assert_eq!(agent.role, OffChainRole::JuryMember);
        assert_eq!(agent.identity.domain, AgentDomain::OffChain);
    }

    #[test]
    fn vote_commitment_is_blinded() {
        let mut agent = OffChainAgent::new(
            1,
            "voter".into(),
            OrchestraSection::Strings,
            OffChainRole::JuryMember,
        );

        let commitment = agent.vote_on_task("task-001", true).unwrap();
        assert_eq!(commitment.task_id, "task-001");
        // Commitment hash is non-zero (blinded)
        assert_ne!(commitment.commitment_hash, [0u8; 32]);
        // Cannot determine the vote from the hash alone
        assert_eq!(agent.vote_commitments.len(), 1);
    }

    #[test]
    fn stress_tester_can_run_tests() {
        let mut agent = OffChainAgent::new(
            5,
            "stressor".into(),
            OrchestraSection::Woodwinds,
            OffChainRole::StressTester,
        );

        let result = agent.run_stress_test("network-partition").unwrap();
        assert_eq!(result.scenario, "network-partition");
        assert!(result.integrity_maintained);
    }

    #[test]
    fn non_stress_tester_cannot_run_tests() {
        let mut agent = OffChainAgent::new(
            5,
            "juror".into(),
            OrchestraSection::Woodwinds,
            OffChainRole::JuryMember,
        );
        assert!(agent.run_stress_test("network-partition").is_none());
    }
}
