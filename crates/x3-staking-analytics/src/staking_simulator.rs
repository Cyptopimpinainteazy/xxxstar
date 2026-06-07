//! Staking Simulator — Projection engine for staking scenarios
//! 
//! Models staking returns under various conditions and timeframes,
//! with support for fee impact analysis and delegation scenarios.

use serde::{Deserialize, Serialize};
use crate::Result;

/// Projection timeframe
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ProjectionPeriod {
    Months(u32),
    Years(u32),
}

/// Single projection point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionPoint {
    pub period: u32, // months elapsed
    pub projected_balance: u128,
    pub gross_rewards: u128,
    pub net_rewards: u128,
    pub roi_percentage: f64,
}

/// Full projection scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionScenario {
    pub name: String,
    pub initial_balance: u128,
    pub timeframe: ProjectionPeriod,
    pub apy: f64,
    pub validator_commission: f64,
    pub unstake_fee: f64,
    pub claim_frequency_months: u32,
    pub monthly_deposits: u128,
    pub projection_points: Vec<ProjectionPoint>,
}

impl ProjectionScenario {
    /// Final projected balance
    pub fn final_balance(&self) -> Option<u128> {
        self.projection_points.last().map(|p| p.projected_balance)
    }

    /// Total projected rewards (net of fees)
    pub fn total_net_rewards(&self) -> Option<u128> {
        self.projection_points.last().map(|p| p.net_rewards)
    }

    /// Final ROI percentage
    pub fn final_roi(&self) -> Option<f64> {
        self.projection_points.last().map(|p| p.roi_percentage)
    }
}

/// Staking Simulator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingSimulator;

impl StakingSimulator {
    /// One-year projection
    pub fn project_one_year(
        initial_balance: u128,
        apy: f64,
        validator_commission: f64,
        unstake_fee: f64,
    ) -> ProjectionScenario {
        Self::project(
            "1-Year Projection".to_string(),
            initial_balance,
            ProjectionPeriod::Years(1),
            apy,
            validator_commission,
            unstake_fee,
            1, // monthly claim
            0, // no additional deposits
        )
    }

    /// Five-year projection
    pub fn project_five_years(
        initial_balance: u128,
        apy: f64,
        validator_commission: f64,
        unstake_fee: f64,
    ) -> ProjectionScenario {
        Self::project(
            "5-Year Projection".to_string(),
            initial_balance,
            ProjectionPeriod::Years(5),
            apy,
            validator_commission,
            unstake_fee,
            1, // monthly claim
            0, // no additional deposits
        )
    }

    /// Custom projection
    pub fn project(
        name: String,
        initial_balance: u128,
        timeframe: ProjectionPeriod,
        apy: f64,
        validator_commission: f64,
        unstake_fee: f64,
        claim_frequency_months: u32,
        monthly_deposits: u128,
    ) -> ProjectionScenario {
        let total_months = match timeframe {
            ProjectionPeriod::Months(m) => m,
            ProjectionPeriod::Years(y) => y * 12,
        };

        let mut points = vec![];
        let mut current_balance = initial_balance as f64;
        let monthly_rate = (apy / 100.0) / 12.0;

        for month in 1..=total_months {
            // Compound monthly
            current_balance *= 1.0 + monthly_rate;

            // Add monthly deposit
            current_balance += monthly_deposits as f64;

            let gross_rewards = (current_balance as u128).saturating_sub(
                initial_balance + (monthly_deposits as u128 * month)
            );
            let commission_cost =
                (gross_rewards as f64 * (validator_commission / 100.0)) as u128;
            let net_rewards = gross_rewards.saturating_sub(commission_cost);
            let total_invested = initial_balance + (monthly_deposits as u128 * month);
            let roi = if total_invested > 0 {
                (net_rewards as f64 / total_invested as f64) * 100.0
            } else {
                0.0
            };

            points.push(ProjectionPoint {
                period: month,
                projected_balance: current_balance as u128,
                gross_rewards,
                net_rewards,
                roi_percentage: roi,
            });
        }

        ProjectionScenario {
            name,
            initial_balance,
            timeframe,
            apy,
            validator_commission,
            unstake_fee,
            claim_frequency_months,
            monthly_deposits,
            projection_points: points,
        }
    }

    /// Compare two scenarios
    pub fn compare_scenarios(
        scenario1: &ProjectionScenario,
        scenario2: &ProjectionScenario,
    ) -> ScenarioComparison {
        let final1 = scenario1.final_balance().unwrap_or(0);
        let final2 = scenario2.final_balance().unwrap_or(0);
        let difference = if final1 > final2 {
            final1 - final2
        } else {
            final2 - final1
        };

        ScenarioComparison {
            scenario1_name: scenario1.name.clone(),
            scenario2_name: scenario2.name.clone(),
            scenario1_final_balance: final1,
            scenario2_final_balance: final2,
            difference,
            better_scenario: if final1 > final2 { 0 } else { 1 },
            percentage_better: if final1 > 0 {
                ((final1 as f64 - final2 as f64) / final1 as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    /// Sensitivity analysis: APY variance
    pub fn apy_sensitivity(
        initial_balance: u128,
        base_apy: f64,
        variance: f64, // e.g. ±5%
        validator_commission: f64,
        months: u32,
    ) -> SensitivityAnalysis {
        let low_apy = (base_apy - variance).max(0.0);
        let high_apy = base_apy + variance;

        let low_scenario = StakingSimulator::project(
            "Low APY".to_string(),
            initial_balance,
            ProjectionPeriod::Months(months),
            low_apy,
            validator_commission,
            0.0,
            1,
            0,
        );

        let base_scenario = StakingSimulator::project(
            "Base APY".to_string(),
            initial_balance,
            ProjectionPeriod::Months(months),
            base_apy,
            validator_commission,
            0.0,
            1,
            0,
        );

        let high_scenario = StakingSimulator::project(
            "High APY".to_string(),
            initial_balance,
            ProjectionPeriod::Months(months),
            high_apy,
            validator_commission,
            0.0,
            1,
            0,
        );

        SensitivityAnalysis {
            base_apy,
            variance,
            low_apy_scenario: low_scenario,
            base_apy_scenario: base_scenario,
            high_apy_scenario: high_scenario,
        }
    }

    /// Commission impact analysis
    pub fn commission_impact(
        initial_balance: u128,
        apy: f64,
        commissions: &[f64],
        months: u32,
    ) -> Vec<ProjectionScenario> {
        commissions
            .iter()
            .enumerate()
            .map(|(i, &commission)| {
                StakingSimulator::project(
                    format!("{}% Commission", commission as u32),
                    initial_balance,
                    ProjectionPeriod::Months(months),
                    apy,
                    commission,
                    0.0,
                    1,
                    0,
                )
            })
            .collect()
    }
}

/// Scenario comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioComparison {
    pub scenario1_name: String,
    pub scenario2_name: String,
    pub scenario1_final_balance: u128,
    pub scenario2_final_balance: u128,
    pub difference: u128,
    pub better_scenario: u32, // 0 or 1
    pub percentage_better: f64,
}

/// Sensitivity analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityAnalysis {
    pub base_apy: f64,
    pub variance: f64,
    pub low_apy_scenario: ProjectionScenario,
    pub base_apy_scenario: ProjectionScenario,
    pub high_apy_scenario: ProjectionScenario,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_year_projection() {
        let scenario = StakingSimulator::project_one_year(1000, 10.0, 5.0, 0.5);
        assert_eq!(scenario.name, "1-Year Projection");
        assert!(scenario.final_balance().unwrap() > 1000);
    }

    #[test]
    fn test_five_year_projection() {
        let scenario = StakingSimulator::project_five_years(1000, 10.0, 5.0, 0.5);
        assert_eq!(scenario.name, "5-Year Projection");
        assert!(scenario.projection_points.len() == 60);
    }

    #[test]
    fn test_projection_with_deposits() {
        let scenario = StakingSimulator::project(
            "Test".to_string(),
            1000,
            ProjectionPeriod::Months(12),
            10.0,
            5.0,
            0.0,
            1,
            100,
        );

        assert!(scenario.final_balance().unwrap() > 2200); // 1000 + (100 * 12) + interest
    }

    #[test]
    fn test_compare_scenarios() {
        let scenario1 = StakingSimulator::project_one_year(1000, 12.0, 5.0, 0.5);
        let scenario2 = StakingSimulator::project_one_year(1000, 8.0, 5.0, 0.5);

        let comparison = StakingSimulator::compare_scenarios(&scenario1, &scenario2);
        assert_eq!(comparison.better_scenario, 0);
        assert!(comparison.percentage_better > 0.0);
    }

    #[test]
    fn test_apy_sensitivity() {
        let analysis = StakingSimulator::apy_sensitivity(1000, 10.0, 5.0, 5.0, 12);

        assert_eq!(analysis.base_apy, 10.0);
        assert_eq!(analysis.variance, 5.0);
        assert!(analysis.base_apy_scenario.final_balance().unwrap() > 1000);
        assert!(
            analysis.high_apy_scenario.final_balance().unwrap()
                > analysis.low_apy_scenario.final_balance().unwrap()
        );
    }

    #[test]
    fn test_commission_impact() {
        let scenarios = StakingSimulator::commission_impact(1000, 10.0, &[0.0, 5.0, 10.0], 12);

        assert_eq!(scenarios.len(), 3);
        assert!(scenarios[0].final_balance().unwrap() > scenarios[2].final_balance().unwrap());
    }

    #[test]
    fn test_projection_roi() {
        let scenario = StakingSimulator::project_one_year(10000, 10.0, 0.0, 0.0);
        let final_roi = scenario.final_roi().unwrap();
        assert!(final_roi < 15.0 && final_roi > 5.0);
    }

    #[test]
    fn test_zero_initial_balance() {
        let scenario = StakingSimulator::project_one_year(0, 10.0, 5.0, 0.5);
        assert_eq!(scenario.final_balance().unwrap(), 0);
    }
}
