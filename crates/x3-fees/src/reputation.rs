//! Reputation scoring for fee discounts.
//!
//! Reputation affects COST, not ACCESS. Any agent can participate
//! regardless of reputation. Good agents simply pay less.

use crate::types::AgentReputation;
use serde::{Deserialize, Serialize};

/// Computed reputation score for fee calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    /// Success rate in basis points (10000 = 100%).
    pub success_rate_bps: u32,
    /// Age factor (0-100, representing maturity).
    pub age_factor: u32,
    /// Slash penalty multiplier (powers of 2).
    pub slash_penalty: u32,
    /// Final composite score (0-10000 bps).
    pub composite_score: u32,
}

impl ReputationScore {
    /// Compute reputation score from raw agent data.
    pub fn compute(rep: &AgentReputation) -> Self {
        let success_rate_bps = rep.success_rate_bps();

        let age_factor = if rep.age_blocks >= 1000 {
            100
        } else if rep.age_blocks >= 500 {
            75
        } else if rep.age_blocks >= 100 {
            50
        } else if rep.age_blocks >= 10 {
            25
        } else {
            0
        };

        let slash_penalty = 2u32.saturating_pow(rep.slashes.min(10) as u32);

        // Composite: (success_rate × age_factor) / (100 × slash_penalty)
        let raw =
            (success_rate_bps as u64 * age_factor as u64) / (100 * slash_penalty as u64).max(1);

        let composite_score = (raw as u32).min(10000);

        ReputationScore {
            success_rate_bps,
            age_factor,
            slash_penalty,
            composite_score,
        }
    }

    /// Whether this agent qualifies for any discount.
    pub fn qualifies_for_discount(&self) -> bool {
        self.composite_score > 5000 && self.age_factor >= 50
    }

    /// Get the discount percentage in basis points.
    pub fn discount_bps(&self, max_discount_bps: u32) -> u32 {
        if !self.qualifies_for_discount() {
            return 0;
        }
        // Scale composite score to max discount
        let raw_discount = (self.composite_score as u64 * max_discount_bps as u64) / 10000;
        (raw_discount as u32).min(max_discount_bps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_agent_zero_score() {
        let rep = AgentReputation {
            successes: 0,
            failures: 0,
            slashes: 0,
            total_volume: 0,
            age_blocks: 0,
        };
        let score = ReputationScore::compute(&rep);
        assert_eq!(score.composite_score, 0);
        assert!(!score.qualifies_for_discount());
    }

    #[test]
    fn test_veteran_high_score() {
        let rep = AgentReputation {
            successes: 1000,
            failures: 10,
            slashes: 0,
            total_volume: 100_000_000,
            age_blocks: 5000,
        };
        let score = ReputationScore::compute(&rep);
        assert!(score.composite_score > 5000);
        assert!(score.qualifies_for_discount());
    }

    #[test]
    fn test_slashes_reduce_score() {
        let clean = AgentReputation {
            successes: 100,
            failures: 0,
            slashes: 0,
            total_volume: 1_000_000,
            age_blocks: 1000,
        };
        let slashed = AgentReputation {
            slashes: 3,
            ..clean.clone()
        };
        let clean_score = ReputationScore::compute(&clean);
        let slashed_score = ReputationScore::compute(&slashed);
        assert!(slashed_score.composite_score < clean_score.composite_score);
    }
}
