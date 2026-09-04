//! On-chain agents — persistent, execute tasks, log to blockchain.
//!
//! On-chain agents are the primary workforce of the Orchestra. They:
//! - Are spawned and initialized with their alignment Score
//! - Receive tasks (on-chain or from .md queue)
//! - Execute tasks if approved (or stage for jury review)
//! - Can be periodically rotated to temporary off-chain jury duty
//! - Can be slashed or retired for misalignment

use crate::audit::AuditEntry;
use crate::score::{ActionContext, ScoreEnforcer, ScoreViolation, TaskClassification};
use crate::task::TaskSpec;

use super::identity::{AgentDomain, AgentId, AgentIdentity, AlignmentScore, OrchestraSection};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Status of an on-chain agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnChainStatus {
    /// Active and ready to receive tasks.
    Active,
    /// Currently executing a task.
    Executing,
    /// Temporarily rotated to off-chain jury duty.
    OnJuryDuty,
    /// Suspended pending investigation.
    Suspended,
    /// Slashed for a violation.
    Slashed,
    /// Retired to scrap yard.
    Retired,
}

/// An on-chain agent in the Orchestra.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainAgent {
    /// Core identity.
    pub identity: AgentIdentity,
    /// Current operational status.
    pub status: OnChainStatus,
    /// Queue of pending tasks assigned to this agent.
    pub task_queue: VecDeque<String>,
    /// Currently executing task ID (if any).
    pub current_task: Option<String>,
    /// Accumulated rewards (in smallest unit).
    pub rewards: u64,
    /// Number of times slashed.
    pub slash_count: u32,
    /// Immutable action log (recent entries; older entries on-chain).
    pub recent_actions: VecDeque<AuditEntry>,
    /// Maximum recent actions to keep in memory.
    pub max_recent_actions: usize,
    /// Last heartbeat timestamp.
    pub last_heartbeat: DateTime<Utc>,
}

impl OnChainAgent {
    /// Create a new on-chain agent.
    pub fn new(id: AgentId, display_name: String, section: OrchestraSection) -> Self {
        let now = Utc::now();
        let identity_hash = {
            let mut hasher = sha2::Sha256::new();
            use sha2::Digest;
            hasher.update(id.to_le_bytes());
            hasher.update(now.timestamp().to_le_bytes());
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
                domain: AgentDomain::OnChain,
                spawned_at: now,
                tasks_completed: 0,
                jury_sessions_served: 0,
                violations: 0,
                identity_hash,
            },
            status: OnChainStatus::Active,
            task_queue: VecDeque::new(),
            current_task: None,
            rewards: 0,
            slash_count: 0,
            recent_actions: VecDeque::new(),
            max_recent_actions: 100,
            last_heartbeat: now,
        }
    }

    /// Accept a task into the queue after Score validation.
    pub fn accept_task(&mut self, task: &TaskSpec) -> Result<(), ScoreViolation> {
        let ctx = ActionContext {
            action_type: format!("accept_task:{}", task.metadata.id),
            is_protocol_bound: true,
            claims_sovereignty: false,
            is_loggable: true,
            writes_to_chain: false,
        };
        ScoreEnforcer::validate_on_chain_action(self.identity.id, &ctx)?;

        self.task_queue.push_back(task.metadata.id.clone());
        Ok(())
    }

    /// Begin execution of the next task in queue.
    /// Returns None if queue is empty or agent is not Active.
    /// Returns the task classification (Minor tasks execute immediately,
    /// Major tasks are staged for jury review).
    pub fn begin_next_task(&mut self) -> Option<(String, TaskClassification)> {
        if self.status != OnChainStatus::Active {
            return None;
        }

        let task_id = self.task_queue.pop_front()?;
        self.status = OnChainStatus::Executing;
        self.current_task = Some(task_id.clone());

        // Default classification — caller determines actual classification
        Some((task_id, TaskClassification::Minor))
    }

    /// Complete the current task.
    pub fn complete_task(&mut self, reward: u64) {
        self.current_task = None;
        self.status = OnChainStatus::Active;
        self.identity.tasks_completed += 1;
        self.rewards += reward;
        self.last_heartbeat = Utc::now();
    }

    /// Fail the current task — alignment decreases.
    pub fn fail_task(&mut self, penalty: i32) {
        self.current_task = None;
        self.status = OnChainStatus::Active;
        self.identity.alignment.adjust(-penalty.abs());
    }

    /// Slash this agent for a Score violation.
    pub fn slash(&mut self, violation: &ScoreViolation, alignment_penalty: i32) {
        self.slash_count += 1;
        self.identity.violations += 1;
        self.identity.alignment.adjust(-alignment_penalty.abs());

        if self.identity.alignment.is_critically_misaligned() {
            self.status = OnChainStatus::Retired;
            self.identity.domain = AgentDomain::Retired;
        } else if self.identity.alignment.is_misaligned() {
            self.status = OnChainStatus::Slashed;
        }

        self.log_action(AuditEntry::violation(
            self.identity.id,
            violation.commandment,
            violation.detail.clone(),
        ));
    }

    /// Rotate this agent to off-chain jury duty.
    pub fn rotate_to_jury(&mut self) {
        self.status = OnChainStatus::OnJuryDuty;
        self.identity.domain = AgentDomain::Rotating;
    }

    /// Return from jury duty to active on-chain status.
    pub fn return_from_jury(&mut self) {
        self.status = OnChainStatus::Active;
        self.identity.domain = AgentDomain::OnChain;
        self.identity.jury_sessions_served += 1;
        self.last_heartbeat = Utc::now();
    }

    /// Log an action to the recent actions ring buffer.
    pub fn log_action(&mut self, entry: AuditEntry) {
        if self.recent_actions.len() >= self.max_recent_actions {
            self.recent_actions.pop_front();
        }
        self.recent_actions.push_back(entry);
    }

    /// Check if this agent is eligible for jury duty.
    pub fn is_jury_eligible(&self) -> bool {
        self.status == OnChainStatus::Active && self.identity.alignment.is_jury_eligible()
    }

    /// Heartbeat — agent is alive and responsive.
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_agent_is_active_with_neutral_alignment() {
        let agent = OnChainAgent::new(1, "test-agent".into(), OrchestraSection::Strings);
        assert_eq!(agent.status, OnChainStatus::Active);
        assert_eq!(agent.identity.alignment, AlignmentScore::NEUTRAL);
        assert_eq!(agent.identity.domain, AgentDomain::OnChain);
    }

    #[test]
    fn jury_rotation_round_trip() {
        let mut agent = OnChainAgent::new(1, "rotator".into(), OrchestraSection::Brass);
        agent.identity.alignment = AlignmentScore::new(150); // above jury threshold
        assert!(agent.is_jury_eligible());

        agent.rotate_to_jury();
        assert_eq!(agent.status, OnChainStatus::OnJuryDuty);
        assert_eq!(agent.identity.domain, AgentDomain::Rotating);

        agent.return_from_jury();
        assert_eq!(agent.status, OnChainStatus::Active);
        assert_eq!(agent.identity.domain, AgentDomain::OnChain);
        assert_eq!(agent.identity.jury_sessions_served, 1);
    }

    #[test]
    fn slash_with_critical_misalignment_retires() {
        let mut agent = OnChainAgent::new(1, "bad-agent".into(), OrchestraSection::Percussion);
        agent.identity.alignment = AlignmentScore::new(25);

        let violation = ScoreViolation {
            commandment: crate::score::Commandment::NoAgentSovereignty,
            agent_id: 1,
            detail: "test".into(),
            timestamp: Utc::now(),
        };
        agent.slash(&violation, 10);

        // 25 - 10 = 15, which is below critical threshold (20)
        assert_eq!(agent.status, OnChainStatus::Retired);
    }
}
