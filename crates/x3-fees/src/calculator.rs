//! Fee calculator — the primary interface for computing execution fees.

use crate::curve::FeeCurve;
use crate::error::FeeError;
use crate::types::*;

/// Complete fee calculator. Computes the full fee vector for any execution.
///
/// Fee calculation is DETERMINISTIC and must happen BEFORE execution.
/// The computed fee is locked at submission time — no surprises.
pub struct FeeCalculator {
    curve: FeeCurve,
}

impl FeeCalculator {
    /// Create a new fee calculator.
    pub fn new(config: FeeConfig) -> Self {
        Self {
            curve: FeeCurve::new(config),
        }
    }

    /// Calculate the complete fee vector for an execution.
    ///
    /// This is the primary entry point. Call this BEFORE execution to
    /// determine the fee, which is then locked.
    pub fn calculate(
        &self,
        params: &ExecutionParams,
        reputation: &AgentReputation,
    ) -> Result<FeeVector, FeeError> {
        // Validate parameters
        if params.legs == 0 {
            return Err(FeeError::ZeroLegs);
        }

        // 1. Base fee
        let base_fee = self.curve.base_fee();

        // 2. Complexity fee
        let complexity_fee = self.curve.complexity_fee(params.legs, params.state_touches);

        // 3. Capital fee
        let capital_fee = self.curve.capital_fee(params.capital);

        // 4. Cross-chain surcharge
        let cross_chain = self.curve.cross_chain_surcharge(params.cross_chain_hops);

        // 5. Gross fee before discounts/penalties
        let mut gross = base_fee
            .saturating_add(complexity_fee)
            .saturating_add(capital_fee)
            .saturating_add(cross_chain);

        // 6. X3 optimization discount OR external bot penalty
        if params.x3_optimized {
            let discount = self.curve.x3_optimization_discount(gross);
            gross = gross.saturating_sub(discount);
        } else {
            let penalty = self.curve.external_bot_penalty(gross);
            gross = gross.saturating_add(penalty);
        }

        // 7. Flashloan premium
        if params.flashloan {
            let premium = self.curve.flashloan_premium(gross);
            gross = gross.saturating_add(premium);
        }

        // 8. Reputation discount (applied last, capped)
        let rep_discount = self.curve.reputation_discount(gross, reputation);

        let total = gross.saturating_sub(rep_discount);

        Ok(FeeVector {
            base_fee,
            complexity_fee: complexity_fee
                .saturating_add(capital_fee)
                .saturating_add(cross_chain),
            capital_fee,
            reputation_discount: rep_discount,
            total,
        })
    }

    /// Estimate whether an arbitrage would be profitable after fees.
    ///
    /// Returns (profitable: bool, net_profit: i128)
    pub fn estimate_profitability(
        &self,
        params: &ExecutionParams,
        reputation: &AgentReputation,
        expected_profit: Amount,
    ) -> Result<(bool, i128), FeeError> {
        let fees = self.calculate(params, reputation)?;
        let net = expected_profit as i128 - fees.total as i128;
        Ok((net > 0, net))
    }

    /// Calculate the minimum profit needed for an execution to break even.
    pub fn break_even_profit(
        &self,
        params: &ExecutionParams,
        reputation: &AgentReputation,
    ) -> Result<Amount, FeeError> {
        let fees = self.calculate(params, reputation)?;
        Ok(fees.total)
    }

    /// Compare the fee between an X3-optimized and external bot execution.
    /// Returns (x3_fee, external_fee, savings).
    pub fn compare_x3_vs_external(
        &self,
        params: &ExecutionParams,
        reputation: &AgentReputation,
    ) -> Result<(Amount, Amount, Amount), FeeError> {
        let mut x3_params = params.clone();
        x3_params.x3_optimized = true;
        let x3_fees = self.calculate(&x3_params, reputation)?;

        let mut ext_params = params.clone();
        ext_params.x3_optimized = false;
        let ext_fees = self.calculate(&ext_params, reputation)?;

        let savings = ext_fees.total.saturating_sub(x3_fees.total);

        Ok((x3_fees.total, ext_fees.total, savings))
    }

    /// Get the underlying fee curve.
    pub fn curve(&self) -> &FeeCurve {
        &self.curve
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_agent() -> AgentReputation {
        AgentReputation {
            successes: 0,
            failures: 0,
            slashes: 0,
            total_volume: 0,
            age_blocks: 0,
        }
    }

    fn veteran_agent() -> AgentReputation {
        AgentReputation {
            successes: 500,
            failures: 10,
            slashes: 0,
            total_volume: 100_000_000,
            age_blocks: 5000,
        }
    }

    fn simple_params() -> ExecutionParams {
        ExecutionParams {
            legs: 2,
            state_touches: 3,
            capital: 100_000,
            x3_optimized: true,
            flashloan: false,
            cross_chain_hops: 0,
        }
    }

    #[test]
    fn test_basic_fee_calculation() {
        let calc = FeeCalculator::new(FeeConfig::default());
        let fees = calc.calculate(&simple_params(), &new_agent()).unwrap();
        assert!(fees.total > 0);
    }

    #[test]
    fn test_x3_cheaper_than_external() {
        let calc = FeeCalculator::new(FeeConfig::default());
        let p = simple_params();
        let (x3, ext, savings) = calc.compare_x3_vs_external(&p, &new_agent()).unwrap();
        assert!(
            x3 < ext,
            "X3 fee {} should be less than external {}",
            x3,
            ext
        );
        assert!(savings > 0);
    }

    #[test]
    fn test_veteran_pays_less() {
        let calc = FeeCalculator::new(FeeConfig::default());
        let p = simple_params();
        let new_fee = calc.calculate(&p, &new_agent()).unwrap();
        let vet_fee = calc.calculate(&p, &veteran_agent()).unwrap();
        assert!(vet_fee.total <= new_fee.total);
    }

    #[test]
    fn test_flashloan_premium() {
        let calc = FeeCalculator::new(FeeConfig::default());
        let p = simple_params();
        let mut flash_p = p.clone();
        flash_p.flashloan = true;
        let normal = calc.calculate(&p, &new_agent()).unwrap();
        let flash = calc.calculate(&flash_p, &new_agent()).unwrap();
        assert!(flash.total > normal.total);
    }

    #[test]
    fn test_cross_chain_surcharge() {
        let calc = FeeCalculator::new(FeeConfig::default());
        let p = simple_params();
        let mut cross = p.clone();
        cross.cross_chain_hops = 3;
        let local = calc.calculate(&p, &new_agent()).unwrap();
        let remote = calc.calculate(&cross, &new_agent()).unwrap();
        assert!(remote.total > local.total);
    }

    #[test]
    fn test_more_complex_more_expensive() {
        let calc = FeeCalculator::new(FeeConfig::default());
        let simple = ExecutionParams {
            legs: 1,
            state_touches: 1,
            capital: 1000,
            x3_optimized: true,
            flashloan: false,
            cross_chain_hops: 0,
        };
        let complex = ExecutionParams {
            legs: 5,
            state_touches: 10,
            capital: 10_000_000,
            x3_optimized: true,
            flashloan: true,
            cross_chain_hops: 2,
        };
        let s_fee = calc.calculate(&simple, &new_agent()).unwrap();
        let c_fee = calc.calculate(&complex, &new_agent()).unwrap();
        assert!(c_fee.total > s_fee.total * 2); // Significantly more expensive
    }

    #[test]
    fn test_zero_legs_error() {
        let calc = FeeCalculator::new(FeeConfig::default());
        let mut p = simple_params();
        p.legs = 0;
        assert!(calc.calculate(&p, &new_agent()).is_err());
    }

    #[test]
    fn test_profitability() {
        let calc = FeeCalculator::new(FeeConfig::default());
        let p = simple_params();
        let fees = calc.calculate(&p, &new_agent()).unwrap();

        // Profitable when profit > fees
        let (profitable, _) = calc
            .estimate_profitability(&p, &new_agent(), fees.total + 1000)
            .unwrap();
        assert!(profitable);

        // Not profitable when profit < fees
        let (profitable, _) = calc
            .estimate_profitability(&p, &new_agent(), fees.total.saturating_sub(1))
            .unwrap();
        assert!(!profitable);
    }
}
