//! Core fee types.

use serde::{Deserialize, Serialize};

/// Amount in base units.
pub type Amount = u128;

/// The complete fee vector for an execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeeVector {
    /// Base fee — minimum cost to touch the system.
    pub base_fee: Amount,
    /// Complexity fee — scales with legs × state touches.
    pub complexity_fee: Amount,
    /// Capital fee — log-scaled based on capital involved.
    pub capital_fee: Amount,
    /// Reputation discount — earned through successful execution.
    pub reputation_discount: Amount,
    /// Total fee after all components.
    pub total: Amount,
}

impl FeeVector {
    /// Compute the total fee (base + complexity + capital - reputation).
    pub fn compute_total(&mut self) {
        let gross = self
            .base_fee
            .saturating_add(self.complexity_fee)
            .saturating_add(self.capital_fee);
        self.total = gross.saturating_sub(self.reputation_discount);
    }
}

/// Parameters describing the execution for fee calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionParams {
    /// Number of execution legs (e.g., swap hops).
    pub legs: u32,
    /// Number of storage slots touched.
    pub state_touches: u32,
    /// Total capital involved in the execution (in base units).
    pub capital: Amount,
    /// Whether the execution uses X3-lang optimized code.
    pub x3_optimized: bool,
    /// Whether this is a flashloan execution.
    pub flashloan: bool,
    /// Number of cross-chain hops.
    pub cross_chain_hops: u32,
}

/// Fee curve configuration parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeConfig {
    /// Base fee per execution (minimum floor).
    pub base_fee_floor: Amount,
    /// Complexity multiplier per (leg × state_touch).
    pub complexity_multiplier: Amount,
    /// Capital fee coefficient (applied to log₂(capital)).
    pub capital_coefficient: Amount,
    /// Maximum reputation discount in basis points (e.g., 3000 = 30%).
    pub max_reputation_discount_bps: u32,
    /// X3-optimized code discount in basis points (e.g., 2000 = 20%).
    pub x3_optimization_discount_bps: u32,
    /// Flashloan premium multiplier (in basis points; 10000 = 1x, 15000 = 1.5x).
    pub flashloan_premium_bps: u32,
    /// Cross-chain hop surcharge per hop.
    pub cross_chain_surcharge: Amount,
    /// External bot penalty multiplier (basis points above 10000).
    pub external_bot_penalty_bps: u32,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            base_fee_floor: 10_000,             // 10k base units
            complexity_multiplier: 500,         // 500 per (leg × touch)
            capital_coefficient: 1_000,         // 1k per log₂ unit
            max_reputation_discount_bps: 3000,  // 30% max discount
            x3_optimization_discount_bps: 2000, // 20% discount for X3 code
            flashloan_premium_bps: 15000,       // 1.5x for flashloans
            cross_chain_surcharge: 5_000,       // 5k per cross-chain hop
            external_bot_penalty_bps: 13000,    // 1.3x for non-X3 code
        }
    }
}

/// Agent's reputation data for fee calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReputation {
    /// Total successful executions.
    pub successes: u64,
    /// Total failed executions.
    pub failures: u64,
    /// Total slash events.
    pub slashes: u64,
    /// Total volume executed (for weighted scoring).
    pub total_volume: Amount,
    /// Agent age in blocks.
    pub age_blocks: u64,
}

impl AgentReputation {
    /// Calculate success rate as basis points (10000 = 100%).
    pub fn success_rate_bps(&self) -> u32 {
        let total = self.successes + self.failures;
        if total == 0 {
            return 0;
        }
        ((self.successes as u128 * 10000) / total as u128) as u32
    }
}
