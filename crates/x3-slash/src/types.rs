//! Core slashing types.

use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use x3_proof::types::{AgentIdentity, BlockHeight, Hash256, IntentId};

/// Amount in base units (e.g., smallest denomination).
pub type Amount = u128;

/// Bond identifier.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BondId(pub u64);

/// Reason for slashing — exhaustive, deterministic.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlashReason {
    /// Execution failed within a slashable scope.
    ExecutionFailure {
        /// Hash of the failed execution proof.
        proof_hash: Hash256,
        /// Instruction index where failure occurred.
        failure_at: u64,
    },
    /// State divergence detected during court replay.
    StateDivergence {
        /// Original proof hash.
        original_proof: Hash256,
        /// Replay proof hash.
        replay_proof: Hash256,
        /// Proof index where divergence occurred.
        divergence_at: usize,
    },
    /// Bond expired without settlement — agent abandoned execution.
    BondExpiry {
        /// The expired bond.
        bond_id: BondId,
        /// Intent that was abandoned.
        intent_id: IntentId,
    },
    /// Invalid proof submitted (tampered or malformed).
    InvalidProof {
        /// Hash of the invalid proof.
        proof_hash: Hash256,
    },
    /// Timeout — execution did not complete within the finality window.
    ExecutionTimeout {
        /// Intent that timed out.
        intent_id: IntentId,
        /// Block at which timeout occurred.
        timeout_block: BlockHeight,
    },
    /// Double execution — agent attempted to execute same intent twice.
    DoubleExecution {
        /// Intent that was double-executed.
        intent_id: IntentId,
        /// First execution proof.
        first_proof: Hash256,
        /// Second execution proof.
        second_proof: Hash256,
    },
}

/// Severity of the slash — determines the percentage of bond forfeited.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlashSeverity {
    /// Minor infraction — 10% of bond.
    Minor,
    /// Moderate infraction — 50% of bond.
    Moderate,
    /// Major infraction — 100% of bond.
    Major,
    /// Critical — 100% of bond + permanent reputation damage.
    Critical,
}

impl SlashSeverity {
    /// Get the fraction of bond to slash (in basis points, 10000 = 100%).
    pub fn slash_bps(&self) -> u64 {
        match self {
            SlashSeverity::Minor => 1000,     // 10%
            SlashSeverity::Moderate => 5000,  // 50%
            SlashSeverity::Major => 10000,    // 100%
            SlashSeverity::Critical => 10000, // 100% + reputation
        }
    }
}

/// A bond posted by an agent to participate in execution.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bond {
    /// Unique bond identifier.
    pub id: BondId,
    /// Agent who posted the bond.
    pub agent_id: AgentIdentity,
    /// Amount bonded.
    pub amount: Amount,
    /// Block at which the bond was posted.
    pub posted_at: BlockHeight,
    /// Block at which the bond expires (must settle before this).
    pub expires_at: BlockHeight,
    /// Intent this bond is associated with (if any).
    pub intent_id: Option<IntentId>,
    /// Current status.
    pub status: BondStatus,
}

/// Bond lifecycle status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BondStatus {
    /// Bond is active and locked.
    Active,
    /// Bond has been partially slashed.
    PartiallySlashed { remaining_bps: u64 },
    /// Bond has been fully slashed.
    FullySlashed,
    /// Bond has been released back to the agent.
    Released,
    /// Bond expired without settlement.
    Expired,
}

/// A slash event — the immutable record of punishment.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlashEvent {
    /// Unique slash identifier.
    pub id: u64,
    /// Agent being slashed.
    pub agent_id: AgentIdentity,
    /// Bond being slashed.
    pub bond_id: BondId,
    /// Reason for slashing.
    pub reason: SlashReason,
    /// Severity of the slash.
    pub severity: SlashSeverity,
    /// Amount slashed.
    pub amount_slashed: Amount,
    /// Block at which slashing was executed.
    pub slashed_at: BlockHeight,
    /// Hash of the slash event (for verification).
    pub event_hash: Hash256,
}

/// Configuration for the slashing engine.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashConfig {
    /// Minimum bond amount required for execution.
    pub min_bond: Amount,
    /// Maximum finality window in blocks.
    pub finality_window: u64,
    /// Whether to apply reputation damage on critical slashes.
    pub reputation_damage_enabled: bool,
    /// Grace period in blocks before bond expiry triggers slash.
    pub expiry_grace_period: u64,
}

#[cfg(feature = "std")]
impl Default for SlashConfig {
    fn default() -> Self {
        Self {
            min_bond: 1_000_000,
            finality_window: 100,
            reputation_damage_enabled: true,
            expiry_grace_period: 10,
        }
    }
}
