use std::collections::HashMap;

pub type AssetId = [u8; 32];
pub type FundId = [u8; 32];
pub type RouteId = [u8; 32];
pub type IncidentId = [u8; 32];
pub type Balance = u128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsuranceFundStatus {
    Active,
    Frozen,
    Depleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsuranceFund {
    pub fund_id: FundId,
    pub asset_id: AssetId,
    pub balance: Balance,
    pub coverage_limit: Balance,
    pub status: InsuranceFundStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCoverage {
    pub route_id: RouteId,
    pub fund_id: FundId,
    pub max_covered_amount: Balance,
    pub premium_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeBreakdown {
    pub route_id: RouteId,
    pub gross_amount: Balance,
    pub premium_amount: Balance,
    pub net_amount: Balance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsuranceError {
    MissingFund,
    MissingCoverage,
    FundFrozen,
    InsufficientFundBalance,
    CoverageExceeded,
    ArithmeticOverflow,
}

#[derive(Debug, Default)]
pub struct GatewayInsuranceEngine {
    funds: HashMap<FundId, InsuranceFund>,
    coverage: HashMap<RouteId, RouteCoverage>,
}

impl GatewayInsuranceEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_fund(&mut self, fund_id: FundId, asset_id: AssetId, coverage_limit: Balance) {
        self.funds.insert(
            fund_id,
            InsuranceFund {
                fund_id,
                asset_id,
                balance: 0,
                coverage_limit,
                status: InsuranceFundStatus::Active,
            },
        );
    }

    pub fn fund_insurance(
        &mut self,
        fund_id: FundId,
        amount: Balance,
    ) -> Result<InsuranceFund, InsuranceError> {
        let fund = self
            .funds
            .get_mut(&fund_id)
            .ok_or(InsuranceError::MissingFund)?;
        if fund.status != InsuranceFundStatus::Active {
            return Err(InsuranceError::FundFrozen);
        }
        fund.balance = fund
            .balance
            .checked_add(amount)
            .ok_or(InsuranceError::ArithmeticOverflow)?;
        Ok(fund.clone())
    }

    pub fn set_route_coverage(&mut self, coverage: RouteCoverage) {
        self.coverage.insert(coverage.route_id, coverage);
    }

    pub fn charge_route_premium(
        &mut self,
        route_id: RouteId,
        amount: Balance,
    ) -> Result<FeeBreakdown, InsuranceError> {
        let coverage = self
            .coverage
            .get(&route_id)
            .cloned()
            .ok_or(InsuranceError::MissingCoverage)?;
        let premium_amount = amount
            .checked_mul(coverage.premium_bps as Balance)
            .ok_or(InsuranceError::ArithmeticOverflow)?
            / 10_000;
        let net_amount = amount
            .checked_sub(premium_amount)
            .ok_or(InsuranceError::ArithmeticOverflow)?;
        self.fund_insurance(coverage.fund_id, premium_amount)?;
        Ok(FeeBreakdown {
            route_id,
            gross_amount: amount,
            premium_amount,
            net_amount,
        })
    }

    pub fn cover_gateway_loss(
        &mut self,
        route_id: RouteId,
        _incident_id: IncidentId,
        amount: Balance,
    ) -> Result<InsuranceFund, InsuranceError> {
        let coverage = self
            .coverage
            .get(&route_id)
            .cloned()
            .ok_or(InsuranceError::MissingCoverage)?;
        if amount > coverage.max_covered_amount {
            return Err(InsuranceError::CoverageExceeded);
        }
        let fund = self
            .funds
            .get_mut(&coverage.fund_id)
            .ok_or(InsuranceError::MissingFund)?;
        if fund.status != InsuranceFundStatus::Active {
            return Err(InsuranceError::FundFrozen);
        }
        if amount > fund.balance {
            return Err(InsuranceError::InsufficientFundBalance);
        }
        fund.balance -= amount;
        if fund.balance == 0 {
            fund.status = InsuranceFundStatus::Depleted;
        }
        Ok(fund.clone())
    }

    pub fn get_route_coverage(&self, route_id: RouteId) -> Option<&RouteCoverage> {
        self.coverage.get(&route_id)
    }

    pub fn get_fund(&self, fund_id: FundId) -> Option<&InsuranceFund> {
        self.funds.get(&fund_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> GatewayInsuranceEngine {
        let mut engine = GatewayInsuranceEngine::new();
        engine.create_fund([1; 32], [9; 32], 10_000);
        engine.set_route_coverage(RouteCoverage {
            route_id: [2; 32],
            fund_id: [1; 32],
            max_covered_amount: 5_000,
            premium_bps: 100,
        });
        engine
    }

    #[test]
    fn insurance_fund_receives_deposits() {
        let mut engine = engine();
        let fund = engine.fund_insurance([1; 32], 1_000).unwrap();

        assert_eq!(fund.balance, 1_000);
    }

    #[test]
    fn route_premium_is_charged_and_visible() {
        let mut engine = engine();
        let fee = engine.charge_route_premium([2; 32], 10_000).unwrap();

        assert_eq!(fee.premium_amount, 100);
        assert_eq!(fee.net_amount, 9_900);
        assert_eq!(engine.get_fund([1; 32]).unwrap().balance, 100);
    }

    #[test]
    fn covered_payout_debits_fund() {
        let mut engine = engine();
        engine.fund_insurance([1; 32], 1_000).unwrap();
        let fund = engine.cover_gateway_loss([2; 32], [7; 32], 400).unwrap();

        assert_eq!(fund.balance, 600);
    }

    #[test]
    fn payout_cannot_exceed_fund_balance_or_coverage() {
        let mut engine = engine();
        engine.fund_insurance([1; 32], 1_000).unwrap();

        assert!(matches!(
            engine.cover_gateway_loss([2; 32], [7; 32], 6_000),
            Err(InsuranceError::CoverageExceeded)
        ));
        assert!(matches!(
            engine.cover_gateway_loss([2; 32], [7; 32], 2_000),
            Err(InsuranceError::InsufficientFundBalance)
        ));
    }

    #[test]
    fn insurance_accounting_preserves_total_value_model() {
        let mut engine = engine();
        engine.fund_insurance([1; 32], 1_000).unwrap();
        let before = engine.get_fund([1; 32]).unwrap().balance;
        engine.cover_gateway_loss([2; 32], [7; 32], 250).unwrap();
        let after = engine.get_fund([1; 32]).unwrap().balance;

        assert_eq!(before - after, 250);
    }
}
