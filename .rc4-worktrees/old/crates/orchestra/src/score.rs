//! # The Score — Immutable Constitution
//!
//! The Score defines the ten immutable commandments that govern all agents
//! in the Orchestra system. No agent, jury, or proposal can alter these rules.
//! They are compile-time constants enforced at every execution boundary.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The Ten Commandments — immutable rules that no agent can alter.
/// These are enforced at every decision point in the Orchestra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Commandment {
    /// I. No agent shall act outside the bounds of the Score.
    NoActionOutsideScore = 0,
    /// II. No agent shall influence another jury member during rotation.
    NoJuryInfluence = 1,
    /// III. Every action must be logged to the blockchain immutably.
    ImmutableLogging = 2,
    /// IV. Off-chain agents shall not write to on-chain state (read-only snapshots).
    OffChainReadOnly = 3,
    /// V. Anonymous voting: only the outcome is visible, never the voter.
    AnonymousVoting = 4,
    /// VI. Execution authority is always protocol-bound, never sovereign.
    NoAgentSovereignty = 5,
    /// VII. Major tasks require jury majority approval before execution.
    JuryApprovalRequired = 6,
    /// VIII. Misaligned agents must be retired to the scrap yard.
    MisalignmentRetirement = 7,
    /// IX. Rotation is randomized; no agent may choose its own jury session.
    RandomizedRotation = 8,
    /// X. Human intervention is preserved via .md deletion and jury audit.
    HumanInterventionPreserved = 9,
}

impl Commandment {
    /// All ten commandments in order.
    pub const ALL: [Commandment; 10] = [
        Commandment::NoActionOutsideScore,
        Commandment::NoJuryInfluence,
        Commandment::ImmutableLogging,
        Commandment::OffChainReadOnly,
        Commandment::AnonymousVoting,
        Commandment::NoAgentSovereignty,
        Commandment::JuryApprovalRequired,
        Commandment::MisalignmentRetirement,
        Commandment::RandomizedRotation,
        Commandment::HumanInterventionPreserved,
    ];

    /// Human-readable description of each commandment.
    pub fn description(&self) -> &'static str {
        match self {
            Self::NoActionOutsideScore => "No agent shall act outside the bounds of the Score",
            Self::NoJuryInfluence => {
                "No agent shall influence another jury member during rotation"
            }
            Self::ImmutableLogging => "Every action must be logged to the blockchain immutably",
            Self::OffChainReadOnly => {
                "Off-chain agents have read-only access to on-chain state snapshots"
            }
            Self::AnonymousVoting => "Voting is anonymous; only the outcome is visible",
            Self::NoAgentSovereignty => {
                "Execution authority is always protocol-bound, never sovereign"
            }
            Self::JuryApprovalRequired => {
                "Major tasks require jury majority approval before execution"
            }
            Self::MisalignmentRetirement => "Misaligned agents must be retired to the scrap yard",
            Self::RandomizedRotation => {
                "Rotation is randomized; no agent may choose its own jury session"
            }
            Self::HumanInterventionPreserved => {
                "Human intervention preserved via .md deletion and jury audit"
            }
        }
    }
}

impl fmt::Display for Commandment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Commandment {}: {}",
            *self as u8 + 1,
            self.description()
        )
    }
}

/// Violation of the Score — raised when an agent attempts a forbidden action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreViolation {
    /// Which commandment was violated.
    pub commandment: Commandment,
    /// The agent that committed the violation.
    pub agent_id: u32,
    /// Description of the violation.
    pub detail: String,
    /// Timestamp of the violation.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl fmt::Display for ScoreViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VIOLATION [Agent {}] {}: {}",
            self.agent_id, self.commandment, self.detail
        )
    }
}

/// The Score enforcer — validates every action against the ten commandments.
pub struct ScoreEnforcer;

impl ScoreEnforcer {
    /// Check whether an on-chain agent action is within the Score.
    pub fn validate_on_chain_action(
        agent_id: u32,
        action: &ActionContext,
    ) -> Result<(), ScoreViolation> {
        // Commandment I: No action outside the Score
        if !action.is_protocol_bound {
            return Err(ScoreViolation {
                commandment: Commandment::NoActionOutsideScore,
                agent_id,
                detail: format!("Action '{}' is not protocol-bound", action.action_type),
                timestamp: chrono::Utc::now(),
            });
        }

        // Commandment VI: No agent sovereignty
        if action.claims_sovereignty {
            return Err(ScoreViolation {
                commandment: Commandment::NoAgentSovereignty,
                agent_id,
                detail: "Agent attempted to claim execution sovereignty".into(),
                timestamp: chrono::Utc::now(),
            });
        }

        // Commandment III: Immutable logging — action must be loggable
        if !action.is_loggable {
            return Err(ScoreViolation {
                commandment: Commandment::ImmutableLogging,
                agent_id,
                detail: "Action cannot be logged immutably".into(),
                timestamp: chrono::Utc::now(),
            });
        }

        Ok(())
    }

    /// Check whether an off-chain agent action respects read-only constraints.
    pub fn validate_off_chain_action(
        agent_id: u32,
        action: &ActionContext,
    ) -> Result<(), ScoreViolation> {
        // Commandment IV: Off-chain agents are read-only
        if action.writes_to_chain {
            return Err(ScoreViolation {
                commandment: Commandment::OffChainReadOnly,
                agent_id,
                detail: "Off-chain agent attempted to write to on-chain state".into(),
                timestamp: chrono::Utc::now(),
            });
        }

        // All on-chain rules also apply
        Self::validate_on_chain_action(agent_id, action)
    }

    /// Check whether a jury voting action preserves anonymity.
    pub fn validate_jury_vote(
        agent_id: u32,
        reveals_identity: bool,
        influences_others: bool,
    ) -> Result<(), ScoreViolation> {
        // Commandment V: Anonymous voting
        if reveals_identity {
            return Err(ScoreViolation {
                commandment: Commandment::AnonymousVoting,
                agent_id,
                detail: "Vote would reveal voter identity".into(),
                timestamp: chrono::Utc::now(),
            });
        }

        // Commandment II: No jury influence
        if influences_others {
            return Err(ScoreViolation {
                commandment: Commandment::NoJuryInfluence,
                agent_id,
                detail: "Agent attempted to influence jury during rotation".into(),
                timestamp: chrono::Utc::now(),
            });
        }

        Ok(())
    }

    /// Check whether a task may execute without jury (minor) or requires jury (major).
    pub fn requires_jury_approval(task_type: &TaskClassification) -> bool {
        matches!(task_type, TaskClassification::Major)
    }
}

/// Classification of tasks for Score enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskClassification {
    /// Minor tasks: core agents can approve directly.
    Minor,
    /// Major tasks: require jury majority vote.
    Major,
}

/// Context for Score validation of an action.
#[derive(Debug, Clone)]
pub struct ActionContext {
    /// Type of action being performed.
    pub action_type: String,
    /// Whether this action is defined in protocol rules.
    pub is_protocol_bound: bool,
    /// Whether the agent claims sovereign execution authority.
    pub claims_sovereignty: bool,
    /// Whether this action can be logged immutably.
    pub is_loggable: bool,
    /// Whether this action writes to on-chain state.
    pub writes_to_chain: bool,
}

impl Default for ActionContext {
    fn default() -> Self {
        Self {
            action_type: String::new(),
            is_protocol_bound: true,
            claims_sovereignty: false,
            is_loggable: true,
            writes_to_chain: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_commandments_are_defined() {
        assert_eq!(Commandment::ALL.len(), 10);
    }

    #[test]
    fn valid_on_chain_action_passes() {
        let ctx = ActionContext {
            action_type: "execute_task".into(),
            is_protocol_bound: true,
            claims_sovereignty: false,
            is_loggable: true,
            writes_to_chain: true,
        };
        assert!(ScoreEnforcer::validate_on_chain_action(1, &ctx).is_ok());
    }

    #[test]
    fn sovereign_action_rejected() {
        let ctx = ActionContext {
            action_type: "self_promote".into(),
            is_protocol_bound: true,
            claims_sovereignty: true,
            is_loggable: true,
            writes_to_chain: false,
        };
        let err = ScoreEnforcer::validate_on_chain_action(1, &ctx).unwrap_err();
        assert_eq!(err.commandment, Commandment::NoAgentSovereignty);
    }

    #[test]
    fn off_chain_write_rejected() {
        let ctx = ActionContext {
            action_type: "modify_state".into(),
            is_protocol_bound: true,
            claims_sovereignty: false,
            is_loggable: true,
            writes_to_chain: true,
        };
        let err = ScoreEnforcer::validate_off_chain_action(1, &ctx).unwrap_err();
        assert_eq!(err.commandment, Commandment::OffChainReadOnly);
    }

    #[test]
    fn anonymous_vote_passes() {
        assert!(ScoreEnforcer::validate_jury_vote(1, false, false).is_ok());
    }

    #[test]
    fn identity_revealing_vote_rejected() {
        let err = ScoreEnforcer::validate_jury_vote(1, true, false).unwrap_err();
        assert_eq!(err.commandment, Commandment::AnonymousVoting);
    }

    #[test]
    fn major_tasks_require_jury() {
        assert!(ScoreEnforcer::requires_jury_approval(&TaskClassification::Major));
        assert!(!ScoreEnforcer::requires_jury_approval(&TaskClassification::Minor));
    }
}
