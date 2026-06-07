//! Fee curve implementation — the mathematical heart of X3 economics.
//!
//! The fee curve ensures:
//! 1. Simple operations are cheap but never free
//! 2. Complex operations scale appropriately
//! 3. Large capital executions pay more (but sub-linearly via log)
//! 4. Good agents pay less (but always pay something)
//! 5. External bots face a structural cost disadvantage

use crate::types::*;

/// The fee curve engine.
pub struct FeeCurve {
    config: FeeConfig,
}

impl FeeCurve {
    /// Create a new fee curve with the given configuration.
    pub fn new(config: FeeConfig) -> Self {
        Self { config }
    }

    /// Calculate the base fee.
    /// The base fee is the absolute minimum cost of any execution.
    pub fn base_fee(&self) -> Amount {
        self.config.base_fee_floor
    }

    /// Calculate the complexity fee.
    ///
    /// ```text
    /// ComplexityFee = multiplier × legs × state_touches
    /// ```
    ///
    /// More legs and more state touches = more fee. This naturally
    /// penalizes complicated arbitrage paths.
    pub fn complexity_fee(&self, legs: u32, state_touches: u32) -> Amount {
        let product = legs as u128 * state_touches as u128;
        self.config.complexity_multiplier.saturating_mul(product)
    }

    /// Calculate the capital fee using log₂ scaling.
    ///
    /// ```text
    /// CapitalFee = coefficient × log₂(capital + 1)
    /// ```
    ///
    /// The +1 prevents log(0). Large capital pays more, but sub-linearly.
    pub fn capital_fee(&self, capital: Amount) -> Amount {
        if capital == 0 {
            return 0;
        }
        // log₂(capital) ≈ 128 - leading_zeros for u128
        let log2 = 127u32.saturating_sub(capital.leading_zeros());
        self.config.capital_coefficient.saturating_mul(log2 as u128)
    }

    /// Calculate the reputation discount.
    ///
    /// ```text
    /// Discount = min(success_rate × age_factor, max_discount)
    /// ```
    ///
    /// Reputation reduces cost but is capped. New agents pay full price.
    /// Slashed agents lose their discount.
    pub fn reputation_discount(&self, gross_fee: Amount, reputation: &AgentReputation) -> Amount {
        let success_bps = reputation.success_rate_bps();

        // No discount if success rate is below 50%
        if success_bps < 5000 {
            return 0;
        }

        // Slash penalty: each slash halves the discount
        let slash_penalty = 2u64.saturating_pow(reputation.slashes as u32);

        // Age factor: agents need at least 100 blocks of history
        let age_factor = if reputation.age_blocks >= 1000 {
            100u64 // Full credit
        } else if reputation.age_blocks >= 100 {
            50u64 // Half credit
        } else {
            0u64 // No credit — too new
        };

        if age_factor == 0 {
            return 0;
        }

        // Raw discount = success_rate (as fraction) × age_factor / 100
        let raw_discount_bps = (success_bps as u64 * age_factor) / (100 * slash_penalty);
        let capped_bps = std::cmp::min(
            raw_discount_bps as u32,
            self.config.max_reputation_discount_bps,
        );

        (gross_fee * capped_bps as u128) / 10000
    }

    /// Apply the X3 optimization discount for native X3-lang code.
    pub fn x3_optimization_discount(&self, gross_fee: Amount) -> Amount {
        (gross_fee * self.config.x3_optimization_discount_bps as u128) / 10000
    }

    /// Apply the external bot penalty for non-X3 code.
    pub fn external_bot_penalty(&self, gross_fee: Amount) -> Amount {
        let penalty_bps = self.config.external_bot_penalty_bps.saturating_sub(10000);
        (gross_fee * penalty_bps as u128) / 10000
    }

    /// Apply flashloan premium.
    pub fn flashloan_premium(&self, gross_fee: Amount) -> Amount {
        let premium_bps = self.config.flashloan_premium_bps.saturating_sub(10000);
        (gross_fee * premium_bps as u128) / 10000
    }

    /// Calculate cross-chain surcharge.
    pub fn cross_chain_surcharge(&self, hops: u32) -> Amount {
        self.config
            .cross_chain_surcharge
            .saturating_mul(hops as u128)
    }

    /// Get the fee configuration.
    pub fn config(&self) -> &FeeConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_fee() {
        let curve = FeeCurve::new(FeeConfig::default());
        assert_eq!(curve.base_fee(), 10_000);
    }

    #[test]
    fn test_complexity_fee_scales() {
        let curve = FeeCurve::new(FeeConfig::default());
        let f1 = curve.complexity_fee(1, 1);
        let f2 = curve.complexity_fee(3, 5);
        assert!(f2 > f1);
        assert_eq!(f1, 500);
        assert_eq!(f2, 7500);
    }

    #[test]
    fn test_capital_fee_log_scale() {
        let curve = FeeCurve::new(FeeConfig::default());
        let f_small = curve.capital_fee(100);
        let f_large = curve.capital_fee(1_000_000);
        // f_large should be bigger but not proportionally
        assert!(f_large > f_small);
        assert!(f_large < f_small * 100); // Sub-linear growth
    }

    #[test]
    fn test_zero_capital_zero_fee() {
        let curve = FeeCurve::new(FeeConfig::default());
        assert_eq!(curve.capital_fee(0), 0);
    }

    #[test]
    fn test_new_agent_no_discount() {
        let curve = FeeCurve::new(FeeConfig::default());
        let rep = AgentReputation {
            successes: 10,
            failures: 0,
            slashes: 0,
            total_volume: 0,
            age_blocks: 50, // Too new
        };
        assert_eq!(curve.reputation_discount(100_000, &rep), 0);
    }

    #[test]
    fn test_good_agent_gets_discount() {
        let curve = FeeCurve::new(FeeConfig::default());
        let rep = AgentReputation {
            successes: 100,
            failures: 5,
            slashes: 0,
            total_volume: 1_000_000,
            age_blocks: 1000,
        };
        let discount = curve.reputation_discount(100_000, &rep);
        assert!(discount > 0);
        assert!(discount <= 30_000); // Max 30%
    }

    #[test]
    fn test_slashed_agent_reduced_discount() {
        let curve = FeeCurve::new(FeeConfig::default());
        let clean = AgentReputation {
            successes: 100,
            failures: 5,
            slashes: 0,
            total_volume: 1_000_000,
            age_blocks: 1000,
        };
        let slashed = AgentReputation {
            slashes: 2,
            ..clean.clone()
        };
        let clean_discount = curve.reputation_discount(100_000, &clean);
        let slashed_discount = curve.reputation_discount(100_000, &slashed);
        assert!(slashed_discount < clean_discount);
    }
}
