//! Flashloan planner — validates and prepares multi-chain flashloan plans.

use sha2::{Digest, Sha256};

use crate::error::FlashloanError;
use crate::types::{FlashloanPlan, ValidatedPlan};

/// Default flashloan premium in basis points.
const DEFAULT_PREMIUM_BPS: u128 = 5;

/// Maximum number of borrows in a single plan.
const MAX_BORROWS: usize = 8;

/// Maximum number of execution legs in a single plan.
const MAX_LEGS: usize = 16;

/// Maximum deadline (60 seconds).
const MAX_DEADLINE_MS: u64 = 60_000;

/// Flashloan planner: validates and hashes multi-chain execution plans.
pub struct FlashloanPlanner {
    premium_bps: u128,
}

impl FlashloanPlanner {
    pub fn new() -> Self {
        Self {
            premium_bps: DEFAULT_PREMIUM_BPS,
        }
    }

    /// Create a planner with custom premium.
    pub fn with_premium(premium_bps: u128) -> Self {
        Self { premium_bps }
    }

    /// Validate and prepare a flashloan plan for execution.
    pub fn plan(&self, plan: FlashloanPlan) -> Result<ValidatedPlan, FlashloanError> {
        // Validate borrows
        if plan.borrows.is_empty() {
            return Err(FlashloanError::InvalidPlan(
                "plan must contain at least one borrow".to_string(),
            ));
        }
        if plan.borrows.len() > MAX_BORROWS {
            return Err(FlashloanError::InvalidPlan(format!(
                "too many borrows: {} (max {})",
                plan.borrows.len(),
                MAX_BORROWS
            )));
        }

        // Validate legs
        if plan.legs.is_empty() {
            return Err(FlashloanError::InvalidPlan(
                "plan must contain at least one execution leg".to_string(),
            ));
        }
        if plan.legs.len() > MAX_LEGS {
            return Err(FlashloanError::InvalidPlan(format!(
                "too many legs: {} (max {})",
                plan.legs.len(),
                MAX_LEGS
            )));
        }

        // Validate deadline
        if plan.deadline_ms == 0 || plan.deadline_ms > MAX_DEADLINE_MS {
            return Err(FlashloanError::InvalidPlan(format!(
                "deadline must be between 1 and {} ms",
                MAX_DEADLINE_MS
            )));
        }

        // Validate borrow amounts
        for borrow in &plan.borrows {
            if borrow.amount == 0 {
                return Err(FlashloanError::InvalidPlan(
                    "borrow amount must be non-zero".to_string(),
                ));
            }
        }

        // Validate gas limits
        for leg in &plan.legs {
            if leg.gas_limit == 0 {
                return Err(FlashloanError::InvalidPlan(
                    "gas limit must be non-zero".to_string(),
                ));
            }
        }

        // Calculate total premium
        let total_premium: u128 = plan
            .borrows
            .iter()
            .map(|b| b.amount * self.premium_bps / 10_000)
            .sum();

        // Calculate estimated gas
        let estimated_gas: u64 = plan.legs.iter().map(|l| l.gas_limit).sum();

        // Compute plan hash
        let plan_hash = self.hash_plan(&plan);

        Ok(ValidatedPlan {
            intent_id: plan.intent_id,
            borrows: plan.borrows,
            legs: plan.legs,
            deadline_ms: plan.deadline_ms,
            total_premium,
            estimated_gas,
            plan_hash,
        })
    }

    /// Compute a deterministic hash of the plan for commitment.
    fn hash_plan(&self, plan: &FlashloanPlan) -> String {
        let mut hasher = Sha256::new();

        hasher.update(plan.intent_id.as_bytes());
        hasher.update(plan.deadline_ms.to_le_bytes());

        for borrow in &plan.borrows {
            hasher.update(borrow.id.0.as_bytes());
            hasher.update(format!("{}", borrow.chain).as_bytes());
            hasher.update(borrow.asset.0.as_bytes());
            hasher.update(borrow.amount.to_le_bytes());
        }

        for leg in &plan.legs {
            hasher.update(format!("{}", leg.chain).as_bytes());
            hasher.update(leg.gas_limit.to_le_bytes());
        }

        hex::encode(hasher.finalize())
    }
}

impl Default for FlashloanPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn sample_plan() -> FlashloanPlan {
        FlashloanPlan {
            intent_id: "test-001".to_string(),
            borrows: vec![BorrowRequest {
                id: FlashloanId::from_str("borrow-1"),
                chain: ChainKind::Evm(1),
                asset: AssetId::new("USDC"),
                amount: 100_000,
                purpose: BorrowPurpose::ArbExecution,
            }],
            legs: vec![ExecutionLeg {
                chain: ChainKind::Evm(1),
                action: LegAction::Swap {
                    from: AssetId::new("USDC"),
                    to: AssetId::new("WETH"),
                    amount: 100_000,
                },
                gas_limit: 200_000,
            }],
            deadline_ms: 30_000,
        }
    }

    #[test]
    fn test_valid_plan() {
        let planner = FlashloanPlanner::new();
        let result = planner.plan(sample_plan());
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_borrows_rejected() {
        let planner = FlashloanPlanner::new();
        let mut plan = sample_plan();
        plan.borrows.clear();
        assert!(matches!(
            planner.plan(plan),
            Err(FlashloanError::InvalidPlan(_))
        ));
    }

    #[test]
    fn test_empty_legs_rejected() {
        let planner = FlashloanPlanner::new();
        let mut plan = sample_plan();
        plan.legs.clear();
        assert!(matches!(
            planner.plan(plan),
            Err(FlashloanError::InvalidPlan(_))
        ));
    }

    #[test]
    fn test_zero_borrow_amount_rejected() {
        let planner = FlashloanPlanner::new();
        let mut plan = sample_plan();
        plan.borrows[0].amount = 0;
        assert!(matches!(
            planner.plan(plan),
            Err(FlashloanError::InvalidPlan(_))
        ));
    }

    #[test]
    fn test_zero_deadline_rejected() {
        let planner = FlashloanPlanner::new();
        let mut plan = sample_plan();
        plan.deadline_ms = 0;
        assert!(matches!(
            planner.plan(plan),
            Err(FlashloanError::InvalidPlan(_))
        ));
    }

    #[test]
    fn test_excessive_deadline_rejected() {
        let planner = FlashloanPlanner::new();
        let mut plan = sample_plan();
        plan.deadline_ms = 120_000;
        assert!(matches!(
            planner.plan(plan),
            Err(FlashloanError::InvalidPlan(_))
        ));
    }

    #[test]
    fn test_plan_hash_deterministic() {
        let planner = FlashloanPlanner::new();
        let plan1 = sample_plan();
        let plan2 = sample_plan();
        let hash1 = planner.hash_plan(&plan1);
        let hash2 = planner.hash_plan(&plan2);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_premium_calculation() {
        let planner = FlashloanPlanner::with_premium(10); // 10 bps = 0.10%
        let result = planner.plan(sample_plan()).unwrap();
        // 100_000 * 10 / 10_000 = 100 per borrow (sample_plan has 1 borrow of 100_000)
        assert_eq!(result.total_premium, 100);
    }
}
