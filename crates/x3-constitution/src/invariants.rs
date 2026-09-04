//! Core invariants that the constitution enforces.

use crate::types::InvariantBounds;
use serde::{Deserialize, Serialize};

/// A single named constitutional invariant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoreInvariant {
    /// Total token supply must never exceed `InvariantBounds::max_supply`.
    SupplyCap,
    /// Treasury balance must never exceed `InvariantBounds::max_treasury_pct` of supply.
    TreasuryBound,
    /// Number of registered agents must not exceed `InvariantBounds::max_agent_count`.
    AgentCountLimit,
    /// Governance execution depth must not exceed `InvariantBounds::max_proposal_depth`.
    GovernanceDepthBound,
    /// A single agent may not spend more than `InvariantBounds::max_agent_epoch_budget` per epoch.
    AgentBudgetBound,
    /// All state transitions must be deterministic (no FP, no clock, no external entropy).
    ExecutionDeterminism,
    /// Every state transition must be bounded (no unbounded loops).
    ExecutionTermination,
    /// Proof of compliance is required before executing any governance call.
    GovernanceProofRequirement,
}

impl CoreInvariant {
    /// Human-readable description of this invariant.
    pub fn description(&self) -> &'static str {
        match self {
            CoreInvariant::SupplyCap =>
                "Total token supply may never exceed the constitutionally-specified maximum.",
            CoreInvariant::TreasuryBound =>
                "Treasury balance may never exceed the specified fraction of total supply.",
            CoreInvariant::AgentCountLimit =>
                "The number of registered agents may never exceed the constitutional maximum.",
            CoreInvariant::GovernanceDepthBound =>
                "Governance calls may not be nested beyond the maximum depth limit.",
            CoreInvariant::AgentBudgetBound =>
                "A single agent may not spend more than the epoch budget in one epoch.",
            CoreInvariant::ExecutionDeterminism =>
                "All state transitions must be deterministic: no floating point, no wall clock, \
                 no external entropy sources.",
            CoreInvariant::ExecutionTermination =>
                "All computations must terminate within gas bounds. Unbounded loops are forbidden.",
            CoreInvariant::GovernanceProofRequirement =>
                "Governance proposals touching invariants must carry a verified proof of compliance \
                 before execution. Voting alone is insufficient.",
        }
    }

    /// Returns all core invariants.
    pub fn all() -> [CoreInvariant; 8] {
        [
            CoreInvariant::SupplyCap,
            CoreInvariant::TreasuryBound,
            CoreInvariant::AgentCountLimit,
            CoreInvariant::GovernanceDepthBound,
            CoreInvariant::AgentBudgetBound,
            CoreInvariant::ExecutionDeterminism,
            CoreInvariant::ExecutionTermination,
            CoreInvariant::GovernanceProofRequirement,
        ]
    }
}

/// A detected invariant violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantViolation {
    /// Which invariant was violated.
    pub invariant: CoreInvariant,
    /// Human-readable context.
    pub message: String,
    /// Block height at violation detection (0 if unknown).
    pub block: u64,
}

impl InvariantViolation {
    pub fn new(invariant: CoreInvariant, message: impl Into<String>, block: u64) -> Self {
        Self {
            invariant,
            message: message.into(),
            block,
        }
    }
}

/// The live set of invariants enforced by the constitution.
#[derive(Debug, Clone)]
pub struct InvariantSet {
    bounds: InvariantBounds,
}

impl InvariantSet {
    /// Create an `InvariantSet` with the default constitutional bounds.
    pub fn new() -> Self {
        Self {
            bounds: InvariantBounds::default(),
        }
    }

    /// Create with custom bounds (e.g. after a valid amendment).
    pub fn with_bounds(bounds: InvariantBounds) -> Self {
        Self { bounds }
    }

    /// Check that `current_supply` does not violate `SupplyCap`.
    pub fn check_supply(&self, current_supply: u128, block: u64) -> Result<(), InvariantViolation> {
        if current_supply > self.bounds.max_supply {
            return Err(InvariantViolation::new(
                CoreInvariant::SupplyCap,
                format!(
                    "supply {} exceeds constitutional max {}",
                    current_supply, self.bounds.max_supply
                ),
                block,
            ));
        }
        Ok(())
    }

    /// Check that `treasury_balance` does not exceed the treasury fraction invariant.
    pub fn check_treasury(
        &self,
        treasury_balance: u128,
        total_supply: u128,
        block: u64,
    ) -> Result<(), InvariantViolation> {
        if total_supply == 0 {
            return Ok(());
        }
        let max_allowed = total_supply.saturating_mul(self.bounds.max_treasury_pct as u128) / 100;
        if treasury_balance > max_allowed {
            return Err(InvariantViolation::new(
                CoreInvariant::TreasuryBound,
                format!(
                    "treasury {} exceeds {}% of supply ({} max)",
                    treasury_balance, self.bounds.max_treasury_pct, max_allowed
                ),
                block,
            ));
        }
        Ok(())
    }

    /// Check that `agent_count` does not exceed the constitutional limit.
    pub fn check_agent_count(
        &self,
        agent_count: u64,
        block: u64,
    ) -> Result<(), InvariantViolation> {
        if agent_count > self.bounds.max_agent_count {
            return Err(InvariantViolation::new(
                CoreInvariant::AgentCountLimit,
                format!(
                    "agent count {} exceeds constitutional max {}",
                    agent_count, self.bounds.max_agent_count
                ),
                block,
            ));
        }
        Ok(())
    }

    /// Check that governance execution depth does not exceed the limit.
    pub fn check_proposal_depth(&self, depth: u8, block: u64) -> Result<(), InvariantViolation> {
        if depth > self.bounds.max_proposal_depth {
            return Err(InvariantViolation::new(
                CoreInvariant::GovernanceDepthBound,
                format!(
                    "proposal execution depth {} exceeds constitutional max {}",
                    depth, self.bounds.max_proposal_depth
                ),
                block,
            ));
        }
        Ok(())
    }

    /// Check that an agent's epoch spend does not exceed the per-agent budget.
    pub fn check_agent_budget(
        &self,
        epoch_spend: u128,
        block: u64,
    ) -> Result<(), InvariantViolation> {
        if epoch_spend > self.bounds.max_agent_epoch_budget {
            return Err(InvariantViolation::new(
                CoreInvariant::AgentBudgetBound,
                format!(
                    "agent epoch spend {} exceeds constitutional max {}",
                    epoch_spend, self.bounds.max_agent_epoch_budget
                ),
                block,
            ));
        }
        Ok(())
    }

    /// Returns a reference to the current invariant bounds.
    pub fn bounds(&self) -> &InvariantBounds {
        &self.bounds
    }
}

impl Default for InvariantSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supply_cap_enforcement() {
        let inv = InvariantSet::new();
        let max = inv.bounds().max_supply;
        assert!(inv.check_supply(max, 0).is_ok());
        assert!(inv.check_supply(max + 1, 1).is_err());
    }

    #[test]
    fn treasury_bound_enforcement() {
        let inv = InvariantSet::new();
        let supply = 1_000_000u128;
        let max_treasury = supply * 30 / 100;
        assert!(inv.check_treasury(max_treasury, supply, 0).is_ok());
        assert!(inv.check_treasury(max_treasury + 1, supply, 1).is_err());
    }

    #[test]
    fn agent_count_limit_enforcement() {
        let inv = InvariantSet::new();
        let max = inv.bounds().max_agent_count;
        assert!(inv.check_agent_count(max, 0).is_ok());
        assert!(inv.check_agent_count(max + 1, 1).is_err());
    }

    #[test]
    fn proposal_depth_enforcement() {
        let inv = InvariantSet::new();
        assert!(inv.check_proposal_depth(0, 0).is_ok());
        assert!(inv.check_proposal_depth(1, 0).is_ok());
        assert!(inv.check_proposal_depth(2, 1).is_err());
    }

    #[test]
    fn agent_budget_enforcement() {
        let inv = InvariantSet::new();
        let max = inv.bounds().max_agent_epoch_budget;
        assert!(inv.check_agent_budget(max, 0).is_ok());
        assert!(inv.check_agent_budget(max + 1, 1).is_err());
    }

    #[test]
    fn all_invariants_have_descriptions() {
        for inv in CoreInvariant::all() {
            assert!(!inv.description().is_empty());
        }
    }
}
