//! Agent identity — core identity, alignment scoring, and section assignment.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique agent identifier — maps to `AgentId` (u32) in the agent-accounts pallet.
pub type AgentId = u32;

/// Orchestra section — agents are assigned to instrument sections.
/// Each section maps to a compute lane and responsibility domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrchestraSection {
    /// Strings — core execution agents (highest throughput, primary workload).
    Strings,
    /// Brass — security and enforcement agents (guard, validate, protect).
    Brass,
    /// Percussion — timing, synchronization, and coordination agents.
    Percussion,
    /// Woodwinds — auxiliary agents (analytics, monitoring, adaptation).
    Woodwinds,
}

impl OrchestraSection {
    /// All sections.
    pub const ALL: [OrchestraSection; 4] = [
        OrchestraSection::Strings,
        OrchestraSection::Brass,
        OrchestraSection::Percussion,
        OrchestraSection::Woodwinds,
    ];

    /// Maximum proportion of jury members from one section (prevents bias).
    pub const MAX_JURY_PROPORTION: f64 = 0.4;
}

impl fmt::Display for OrchestraSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Strings => write!(f, "Strings"),
            Self::Brass => write!(f, "Brass"),
            Self::Percussion => write!(f, "Percussion"),
            Self::Woodwinds => write!(f, "Woodwinds"),
        }
    }
}

/// Alignment score — measures how well an agent adheres to the Score.
/// Range: 0 (fully misaligned) to 200 (perfectly aligned). 100 = neutral.
/// Maps to the `reputation` field in agent-accounts pallet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AlignmentScore(pub u16);

impl AlignmentScore {
    pub const MIN: Self = Self(0);
    pub const NEUTRAL: Self = Self(100);
    pub const MAX: Self = Self(200);

    /// Threshold below which an agent is considered misaligned and sent to scrap yard.
    pub const MISALIGNMENT_THRESHOLD: Self = Self(40);
    /// Threshold above which an agent is eligible for jury duty.
    pub const JURY_ELIGIBLE_THRESHOLD: Self = Self(120);
    /// Threshold for critical misalignment — immediate retirement.
    pub const CRITICAL_MISALIGNMENT: Self = Self(20);

    pub fn new(value: u16) -> Self {
        Self(value.min(200))
    }

    pub fn adjust(&mut self, delta: i32) {
        let new_val = (self.0 as i32 + delta).clamp(0, 200) as u16;
        self.0 = new_val;
    }

    pub fn is_misaligned(&self) -> bool {
        *self < Self::MISALIGNMENT_THRESHOLD
    }

    pub fn is_critically_misaligned(&self) -> bool {
        *self < Self::CRITICAL_MISALIGNMENT
    }

    pub fn is_jury_eligible(&self) -> bool {
        *self >= Self::JURY_ELIGIBLE_THRESHOLD
    }
}

impl fmt::Display for AlignmentScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/200", self.0)
    }
}

/// Full agent identity within the Orchestra.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    /// Unique agent ID (maps to pallet agent-accounts `AgentId`).
    pub id: AgentId,
    /// Human-readable display name.
    pub display_name: String,
    /// Orchestra section assignment.
    pub section: OrchestraSection,
    /// Current alignment score.
    pub alignment: AlignmentScore,
    /// Whether this agent is currently on-chain or off-chain.
    pub domain: AgentDomain,
    /// When this agent was spawned.
    pub spawned_at: DateTime<Utc>,
    /// Total tasks completed.
    pub tasks_completed: u64,
    /// Total jury sessions served.
    pub jury_sessions_served: u64,
    /// Total violations committed.
    pub violations: u64,
    /// SHA-256 hash of the agent's public key (for anonymous voting commitments).
    pub identity_hash: [u8; 32],
}

/// Whether an agent is operating on-chain or off-chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentDomain {
    /// On-chain — persistent, executes tasks, logs to blockchain.
    OnChain,
    /// Off-chain — jury duty, auditing, stress-testing.
    OffChain,
    /// Transitioning — rotating between domains.
    Rotating,
    /// Retired — in scrap yard.
    Retired,
}

impl fmt::Display for AgentDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OnChain => write!(f, "on-chain"),
            Self::OffChain => write!(f, "off-chain"),
            Self::Rotating => write!(f, "rotating"),
            Self::Retired => write!(f, "retired"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_score_bounds() {
        let mut score = AlignmentScore::new(100);
        score.adjust(150);
        assert_eq!(score.0, 200); // clamped at max

        score.adjust(-300);
        assert_eq!(score.0, 0); // clamped at min
    }

    #[test]
    fn misalignment_threshold() {
        assert!(AlignmentScore::new(39).is_misaligned());
        assert!(!AlignmentScore::new(40).is_misaligned());
        assert!(AlignmentScore::new(19).is_critically_misaligned());
    }

    #[test]
    fn jury_eligibility() {
        assert!(!AlignmentScore::new(119).is_jury_eligible());
        assert!(AlignmentScore::new(120).is_jury_eligible());
    }
}
