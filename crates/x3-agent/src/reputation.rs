//! Reputation tracking — public, immutable performance history.
//!
//! Reputation affects COST, not ACCESS. Anyone can execute.
//! Good agents pay less. Bad agents pay more. Everyone can see the record.

use crate::types::AgentStats;
use x3_fees::reputation::ReputationScore;
use x3_fees::types::AgentReputation;

/// Reputation tracker — computes and caches reputation scores.
pub struct ReputationTracker;

impl ReputationTracker {
    /// Compute the current reputation score from stats.
    pub fn compute_score(
        stats: &AgentStats,
        registered_at: u64,
        current_block: u64,
    ) -> ReputationScore {
        let reputation = AgentReputation {
            successes: stats.intents_succeeded,
            failures: stats.intents_failed,
            slashes: stats.slash_count,
            total_volume: stats.total_volume,
            age_blocks: current_block.saturating_sub(registered_at),
        };
        ReputationScore::compute(&reputation)
    }

    /// Get the fee reputation data from agent stats.
    pub fn to_fee_reputation(
        stats: &AgentStats,
        registered_at: u64,
        current_block: u64,
    ) -> AgentReputation {
        AgentReputation {
            successes: stats.intents_succeeded,
            failures: stats.intents_failed,
            slashes: stats.slash_count,
            total_volume: stats.total_volume,
            age_blocks: current_block.saturating_sub(registered_at),
        }
    }

    /// Compute the effective fee discount for an agent (in basis points).
    pub fn fee_discount_bps(
        stats: &AgentStats,
        registered_at: u64,
        current_block: u64,
        max_discount_bps: u32,
    ) -> u32 {
        let score = Self::compute_score(stats, registered_at, current_block);
        score.discount_bps(max_discount_bps)
    }

    /// Determine if an agent's reputation qualifies for reduced monitoring.
    pub fn is_trusted(stats: &AgentStats, registered_at: u64, current_block: u64) -> bool {
        let score = Self::compute_score(stats, registered_at, current_block);
        score.composite_score >= 8000 && stats.slash_count == 0
    }
}
