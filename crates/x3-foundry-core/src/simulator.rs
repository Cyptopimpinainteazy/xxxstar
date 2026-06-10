use crate::error::FoundryError;
use crate::types::{
    BreakEvenModel, DAppType, MonthlyProjection, RevenueConfig, SimulationResult,
};
use chrono::Utc;
use tracing::info;

/// Revenue projection for a single period.
#[derive(Debug, Clone)]
pub struct RevenueProjection {
    pub daily_volume: u128,
    pub daily_fees: u128,
    pub monthly_volume: u128,
    pub monthly_fees: u128,
    pub annual_volume: u128,
    pub annual_fees: u128,
}

/// Gas cost estimate.
#[derive(Debug, Clone)]
pub struct GasEstimate {
    pub deploy_gas: u64,
    pub daily_tx_gas: u64,
    pub gas_price_gwei: u64,
    pub daily_gas_cost: u128,
    pub monthly_gas_cost: u128,
}

/// Break-even analysis result.
#[derive(Debug, Clone)]
pub struct BreakEvenAnalysis {
    pub days_to_break_even: u64,
    pub total_dev_cost: u128,
    pub monthly_op_cost: u128,
    pub daily_volume_required: u128,
    pub is_profitable: bool,
    pub monthly_profit: i128,
}

/// Simulator runs financial simulations on generated dApps.
pub struct Simulator;

impl Simulator {
    pub fn new() -> Self {
        Self
    }

    /// Runs the full simulation pipeline for a project.
    pub fn simulate_project(
        &self,
        dapp_type: &DAppType,
        config: &RevenueConfig,
        user_base_estimate: u64,
    ) -> Result<SimulationResult, FoundryError> {
        info!("Simulator: simulating project for {:?} with {} users", dapp_type, user_base_estimate);

        let volume = self.simulate_volume(dapp_type, user_base_estimate);
        let fees = self.simulate_fees(&volume, config);
        let gas = self.simulate_gas(dapp_type, user_base_estimate);
        let break_even = self.simulate_break_even(dapp_type, &fees, &gas);

        let treasury_contribution = self.calculate_treasury_share(&fees, config);
        let creator_earnings = self.calculate_creator_share(&fees, config);

        let monthly_projections = self.generate_monthly_projections(dapp_type, config, user_base_estimate);

        let confidence_score = self.calculate_confidence(dapp_type, user_base_estimate);

        let break_even_model = BreakEvenModel {
            days_to_break_even: break_even.days_to_break_even,
            total_dev_cost: break_even.total_dev_cost,
            monthly_op_cost: break_even.monthly_op_cost,
            daily_volume_required: break_even.daily_volume_required,
            is_profitable: break_even.is_profitable,
        };

        Ok(SimulationResult {
            expected_volume: volume.daily_volume,
            fee_revenue: fees.daily_fees,
            gas_cost: gas.daily_gas_cost,
            break_even_model,
            treasury_contribution,
            creator_earnings,
            monthly_revenue_projection: monthly_projections,
            confidence_score,
        })
    }

    /// Simulates expected transaction volume.
    pub fn simulate_volume(&self, dapp_type: &DAppType, user_base: u64) -> RevenueProjection {
        let (tx_per_user_per_day, avg_tx_value) = match dapp_type {
            DAppType::TokenLaunchpad => (2, 100_000_000_000_000_000_000u128),    // 100 tokens
            DAppType::NFTMarketplace => (0.5, 500_000_000_000_000_000_000u128),  // 500 tokens
            DAppType::StakingPool => (0.2, 1_000_000_000_000_000_000_000u128),   // 1000 tokens
            DAppType::SubscriptionApp => (0.03, 50_000_000_000_000_000_000u128), // 50 tokens/month
            DAppType::EscrowApp => (0.1, 1_000_000_000_000_000_000_000u128),     // 1000 tokens
            DAppType::AiImageSaaS => (5, 10_000_000_000_000_000_000u128),        // 10 tokens
            DAppType::TradingBotVault => (10, 100_000_000_000_000_000_000u128),  // 100 tokens
            DAppType::YieldOptimizer => (1, 500_000_000_000_000_000_000u128),    // 500 tokens
            DAppType::CrossChainPayout => (0.5, 2_000_000_000_000_000_000_000u128), // 2000 tokens
            DAppType::DomainRegistry => (0.1, 50_000_000_000_000_000_000u128),   // 50 tokens
            DAppType::PredictionMarket => (3, 200_000_000_000_000_000_000u128),  // 200 tokens
            DAppType::AffiliateApp => (0.5, 100_000_000_000_000_000_000u128),    // 100 tokens
            DAppType::DataMarketplace => (1, 500_000_000_000_000_000_000u128),   // 500 tokens
            DAppType::Custom(_) => (1, 100_000_000_000_000_000_000u128),         // 100 tokens
        };

        let daily_tx_count = (user_base as f64 * tx_per_user_per_day) as u64;
        let daily_volume = (daily_tx_count as u128).saturating_mul(avg_tx_value);
        let daily_fees = daily_volume.saturating_mul(200) / 10000; // Default 2% fee

        RevenueProjection {
            daily_volume,
            daily_fees,
            monthly_volume: daily_volume.saturating_mul(30),
            monthly_fees: daily_fees.saturating_mul(30),
            annual_volume: daily_volume.saturating_mul(365),
            annual_fees: daily_fees.saturating_mul(365),
        }
    }

    /// Simulates fee revenue based on volume and config.
    pub fn simulate_fees(&self, volume: &RevenueProjection, config: &RevenueConfig) -> RevenueProjection {
        let platform_fee_pct = config.platform_fee_bps as u128;
        let daily_fees = volume.daily_volume.saturating_mul(platform_fee_pct) / 10000;

        RevenueProjection {
            daily_volume: volume.daily_volume,
            daily_fees,
            monthly_volume: volume.monthly_volume,
            monthly_fees: daily_fees.saturating_mul(30),
            annual_volume: volume.annual_volume,
            annual_fees: daily_fees.saturating_mul(365),
        }
    }

    /// Simulates gas costs.
    pub fn simulate_gas(&self, dapp_type: &DAppType, user_base: u64) -> GasEstimate {
        let (deploy_gas, gas_per_tx) = match dapp_type {
            DAppType::TokenLaunchpad => (3_000_000, 150_000),
            DAppType::NFTMarketplace => (5_000_000, 200_000),
            DAppType::StakingPool => (2_500_000, 120_000),
            DAppType::SubscriptionApp => (2_000_000, 100_000),
            DAppType::EscrowApp => (3_500_000, 180_000),
            DAppType::AiImageSaaS => (4_000_000, 250_000),
            DAppType::TradingBotVault => (6_000_000, 300_000),
            DAppType::YieldOptimizer => (4_500_000, 220_000),
            DAppType::CrossChainPayout => (5_500_000, 350_000),
            DAppType::DomainRegistry => (3_000_000, 160_000),
            DAppType::PredictionMarket => (4_000_000, 200_000),
            DAppType::AffiliateApp => (2_500_000, 130_000),
            DAppType::DataMarketplace => (4_000_000, 200_000),
            DAppType::Custom(_) => (3_000_000, 150_000),
        };

        let gas_price_gwei = 50; // 50 gwei
        let daily_tx_count = match dapp_type {
            DAppType::SubscriptionApp => (user_base as f64 * 0.03) as u64,
            DAppType::AiImageSaaS => (user_base as f64 * 5.0) as u64,
            DAppType::TradingBotVault => (user_base as f64 * 10.0) as u64,
            DAppType::PredictionMarket => (user_base as f64 * 3.0) as u64,
            _ => (user_base as f64 * 1.0) as u64,
        };

        let daily_tx_gas = (daily_tx_count as u64).saturating_mul(gas_per_tx);
        let daily_gas_cost_wei = (daily_tx_gas as u128).saturating_mul(gas_price_gwei as u128);
        let daily_gas_cost = daily_gas_cost_wei / 1_000_000_000_000_000_000; // Convert to token units

        GasEstimate {
            deploy_gas,
            daily_tx_gas,
            gas_price_gwei,
            daily_gas_cost,
            monthly_gas_cost: daily_gas_cost.saturating_mul(30),
        }
    }

    /// Simulates break-even analysis.
    pub fn simulate_break_even(&self, dapp_type: &DAppType, fees: &RevenueProjection, gas: &GasEstimate) -> BreakEvenAnalysis {
        let (dev_cost, monthly_op) = match dapp_type {
            DAppType::TokenLaunchpad => (5_000_000_000_000_000_000_000u128, 500_000_000_000_000_000_000u128),
            DAppType::NFTMarketplace => (10_000_000_000_000_000_000_000u128, 1_000_000_000_000_000_000_000u128),
            DAppType::StakingPool => (3_000_000_000_000_000_000_000u128, 300_000_000_000_000_000_000u128),
            DAppType::SubscriptionApp => (4_000_000_000_000_000_000_000u128, 400_000_000_000_000_000_000u128),
            DAppType::EscrowApp => (5_000_000_000_000_000_000_000u128, 500_000_000_000_000_000_000u128),
            DAppType::AiImageSaaS => (20_000_000_000_000_000_000_000u128, 5_000_000_000_000_000_000_000u128),
            DAppType::TradingBotVault => (15_000_000_000_000_000_000_000u128, 2_000_000_000_000_000_000_000u128),
            DAppType::YieldOptimizer => (8_000_000_000_000_000_000_000u128, 800_000_000_000_000_000_000u128),
            DAppType::CrossChainPayout => (12_000_000_000_000_000_000_000u128, 1_500_000_000_000_000_000_000u128),
            DAppType::DomainRegistry => (5_000_000_000_000_000_000_000u128, 500_000_000_000_000_000_000u128),
            DAppType::PredictionMarket => (8_000_000_000_000_000_000_000u128, 1_000_000_000_000_000_000_000u128),
            DAppType::AffiliateApp => (3_000_000_000_000_000_000_000u128, 300_000_000_000_000_000_000u128),
            DAppType::DataMarketplace => (10_000_000_000_000_000_000_000u128, 1_000_000_000_000_000_000_000u128),
            DAppType::Custom(_) => (5_000_000_000_000_000_000_000u128, 500_000_000_000_000_000_000u128),
        };

        let daily_net = fees.daily_fees.saturating_sub(gas.daily_gas_cost);
        let monthly_net = daily_net.saturating_mul(30);
        let monthly_profit = (monthly_net as i128).saturating_sub(monthly_op as i128);

        let days_to_break_even = if daily_net > 0 {
            let total_cost = dev_cost.saturating_add(monthly_op);
            (total_cost as f64 / daily_net as f64).ceil() as u64
        } else {
            u64::MAX
        };

        let daily_volume_required = if config.platform_fee_bps > 0 {
            let needed_daily_revenue = monthly_op / 30 + gas.daily_gas_cost;
            needed_daily_revenue.saturating_mul(10000) / config.platform_fee_bps as u128
        } else {
            0
        };

        BreakEvenAnalysis {
            days_to_break_even,
            total_dev_cost: dev_cost,
            monthly_op_cost: monthly_op,
            daily_volume_required,
            is_profitable: monthly_profit > 0,
            monthly_profit,
        }
    }

    /// Calculates treasury share of fees.
    fn calculate_treasury_share(&self, fees: &RevenueProjection, config: &RevenueConfig) -> u128 {
        fees.daily_fees.saturating_mul(config.platform_fee_bps as u128) / 10000
    }

    /// Calculates creator share of fees.
    fn calculate_creator_share(&self, fees: &RevenueProjection, config: &RevenueConfig) -> u128 {
        fees.daily_fees.saturating_mul(config.creator_fee_bps as u128) / 10000
    }

    /// Generates 12-month revenue projections.
    fn generate_monthly_projections(&self, dapp_type: &DAppType, config: &RevenueConfig, user_base: u64) -> Vec<MonthlyProjection> {
        let mut projections = Vec::new();
        let growth_rate = match dapp_type {
            DAppType::AiImageSaaS | DAppType::TradingBotVault => 1.15,  // 15% monthly growth
            DAppType::NFTMarketplace | DAppType::PredictionMarket => 1.10,
            DAppType::SubscriptionApp | DAppType::StakingPool => 1.05,
            _ => 1.08,
        };

        let mut current_users = user_base as f64;
        for month in 1..=12 {
            current_users *= growth_rate;
            let volume = self.simulate_volume(dapp_type, current_users as u64);
            let fees = self.simulate_fees(&volume, config);
            let gas = self.simulate_gas(dapp_type, current_users as u64);

            projections.push(MonthlyProjection {
                month,
                volume: volume.monthly_volume,
                revenue: fees.monthly_fees,
                gas_cost: gas.monthly_gas_cost,
            });
        }
        projections
    }

    /// Calculates confidence score for the simulation.
    fn calculate_confidence(&self, dapp_type: &DAppType, user_base: u64) -> f64 {
        let base_confidence = match dapp_type {
            DAppType::TokenLaunchpad => 0.85,
            DAppType::NFTMarketplace => 0.80,
            DAppType::StakingPool => 0.90,
            DAppType::SubscriptionApp => 0.85,
            DAppType::EscrowApp => 0.75,
            DAppType::AiImageSaaS => 0.70,
            DAppType::TradingBotVault => 0.65,
            DAppType::YieldOptimizer => 0.75,
            DAppType::CrossChainPayout => 0.70,
            DAppType::DomainRegistry => 0.80,
            DAppType::PredictionMarket => 0.70,
            DAppType::AffiliateApp => 0.80,
            DAppType::DataMarketplace => 0.75,
            DAppType::Custom(_) => 0.60,
        };

        // Adjust for user base size
        let user_factor = if user_base >= 10000 {
            1.0
        } else if user_base >= 1000 {
            0.9
        } else if user_base >= 100 {
            0.8
        } else {
            0.6
        };

        (base_confidence * user_factor).clamp(0.0, 1.0)
    }
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FeeMode;

    #[test]
    fn test_simulate_volume() {
        let sim = Simulator::new();
        let vol = sim.simulate_volume(&DAppType::NFTMarketplace, 1000);
        assert!(vol.daily_volume > 0);
        assert!(vol.monthly_volume > vol.daily_volume);
    }

    #[test]
    fn test_simulate_gas() {
        let sim = Simulator::new();
        let gas = sim.simulate_gas(&DAppType::TokenLaunchpad, 1000);
        assert!(gas.deploy_gas > 0);
        assert!(gas.daily_gas_cost > 0);
    }

    #[test]
    fn test_break_even() {
        let sim = Simulator::new();
        let vol = sim.simulate_volume(&DAppType::StakingPool, 5000);
        let config = RevenueConfig::default();
        let fees = sim.simulate_fees(&vol, &config);
        let gas = sim.simulate_gas(&DAppType::StakingPool, 5000);
        let be = sim.simulate_break_even(&DAppType::StakingPool, &fees, &gas);
        assert!(be.total_dev_cost > 0);
    }

    #[test]
    fn test_full_simulation() {
        let sim = Simulator::new();
        let config = RevenueConfig::default();
        let result = sim.simulate_project(&DAppType::TokenLaunchpad, &config, 1000);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.confidence_score > 0.0);
        assert_eq!(r.monthly_revenue_projection.len(), 12);
    }
}
