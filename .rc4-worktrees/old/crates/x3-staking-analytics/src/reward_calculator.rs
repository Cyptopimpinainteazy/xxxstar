//! Reward Calculator — APY and reward estimation engine

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use crate::Result;

/// APY calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APYCalculation {
    pub current_apy: f64,
    pub historical_apy: Vec<(DateTime<Utc>, f64)>,
    pub estimated_monthly_reward: u128,
    pub estimated_annual_reward: u128,
    pub compounded_24m_balance: u128,
}

/// Era reward information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EraRewardInfo {
    pub era: u32,
    pub total_reward: u128,
    pub total_staked: u128,
    pub individual_reward: Option<u128>,
    pub timestamp: DateTime<Utc>,
}

/// Reward Calculator — Computes APY and reward estimates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardCalculator {
    era_rewards: HashMap<u32, EraRewardInfo>,
    apy_history: Vec<(DateTime<Utc>, f64)>,
    current_era: u32,
}

impl RewardCalculator {
    pub fn new() -> Self {
        RewardCalculator {
            era_rewards: HashMap::new(),
            apy_history: Vec::new(),
            current_era: 0,
        }
    }

    /// Register era reward info
    pub fn record_era_reward(&mut self, era: u32, total_reward: u128, total_staked: u128) {
        let info = EraRewardInfo {
            era,
            total_reward,
            total_staked,
            individual_reward: None,
            timestamp: Utc::now(),
        };

        self.era_rewards.insert(era, info);
        self.current_era = era;
    }

    /// Calculate APY from recent eras
    pub fn calculate_apy(&mut self, position_balance: u128) -> f64 {
        if self.era_rewards.is_empty() || position_balance == 0 {
            return 0.0;
        }

        let mut total_rewards = 0u128;
        let mut era_count = 0u32;

        // Look at last 10 eras for APY
        for era in (self.current_era.saturating_sub(10))..=self.current_era {
            if let Some(reward_info) = self.era_rewards.get(&era) {
                if reward_info.total_staked > 0 {
                    let era_individual_reward =
                        (position_balance as f64 / reward_info.total_staked as f64)
                            * reward_info.total_reward as f64;
                    total_rewards += era_individual_reward as u128;
                    era_count += 1;
                }
            }
        }

        if era_count == 0 {
            return 0.0;
        }

        let avg_era_reward = total_rewards as f64 / era_count as f64;
        // Annualized (365 days / 6 days per era ≈ 60.83 eras per year)
        (avg_era_reward / position_balance as f64) * 60.83 * 100.0
    }

    /// Calculate reward for specific balance and timeframe
    pub fn estimate_reward(
        &self,
        balance: u128,
        apy: f64,
        days: u32,
    ) -> u128 {
        if balance == 0 || apy == 0.0 {
            return 0;
        }

        let reward = balance as f64 * (apy / 100.0) * (days as f64 / 365.0);
        reward as u128
    }

    /// Calculate compounded balance over time
    pub fn compound_balance(
        &self,
        initial_balance: u128,
        apy: f64,
        months: u32,
    ) -> u128 {
        if initial_balance == 0 || apy == 0.0 {
            return initial_balance;
        }

        let monthly_rate = (apy / 100.0) / 12.0;
        let months_f64 = months as f64;
        let result = initial_balance as f64 * (1.0 + monthly_rate).powf(months_f64);
        result as u128
    }

    /// Get APY calculation with projections
    pub fn get_apy_calculation(&mut self, balance: u128) -> APYCalculation {
        let current_apy = self.calculate_apy(balance);

        // Track APY history
        self.apy_history.push((Utc::now(), current_apy));
        if self.apy_history.len() > 365 {
            self.apy_history.remove(0);
        }

        let monthly_reward = self.estimate_reward(balance, current_apy, 30);
        let annual_reward = self.estimate_reward(balance, current_apy, 365);
        let compounded_24m = self.compound_balance(balance, current_apy, 24);

        APYCalculation {
            current_apy,
            historical_apy: self.apy_history.clone(),
            estimated_monthly_reward: monthly_reward,
            estimated_annual_reward: annual_reward,
            compounded_24m_balance: compounded_24m,
        }
    }

    /// Calculate compound rewards with monthly compounding
    pub fn compound_rewards(
        &self,
        initial_balance: u128,
        apy: f64,
        months: u32,
        claim_and_restake_interval: u32, // claim every N months
    ) -> u128 {
        let mut balance = initial_balance as f64;
        let monthly_rate = apy / 100.0 / 12.0;

        for month in 1..=months {
            balance *= 1.0 + monthly_rate;

            if month % claim_and_restake_interval == 0 {
                // Rebalance position (in real system, this would claim rewards)
                // Just continuous compounding here
            }
        }

        balance as u128
    }

    /// Historical APY average
    pub fn average_apy(&self, days: u32) -> f64 {
        if self.apy_history.is_empty() {
            return 0.0;
        }

        let cutoff = Utc::now() - Duration::days(days as i64);
        let recent: Vec<f64> = self
            .apy_history
            .iter()
            .filter(|(ts, _)| ts > &cutoff)
            .map(|(_, apy)| apy)
            .copied()
            .collect();

        if recent.is_empty() {
            return 0.0;
        }

        recent.iter().sum::<f64>() / recent.len() as f64
    }

    /// APY volatility (standard deviation)
    pub fn apy_volatility(&self, days: u32) -> f64 {
        if self.apy_history.len() < 2 {
            return 0.0;
        }

        let cutoff = Utc::now() - Duration::days(days as i64);
        let recent: Vec<f64> = self
            .apy_history
            .iter()
            .filter(|(ts, _)| ts > &cutoff)
            .map(|(_, apy)| apy)
            .copied()
            .collect();

        if recent.len() < 2 {
            return 0.0;
        }

        let mean = recent.iter().sum::<f64>() / recent.len() as f64;
        let variance = recent
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / recent.len() as f64;

        variance.sqrt()
    }

    /// Get era reward info
    pub fn get_era(&self, era: u32) -> Option<EraRewardInfo> {
        self.era_rewards.get(&era).cloned()
    }

    /// Calculate impact of changing validator commission
    pub fn estimate_commission_impact(
        &self,
        balance: u128,
        apy: f64,
        current_commission: f64,
        new_commission: f64,
    ) -> (u128, u128) {
        let annual_gross = self.estimate_reward(balance, apy, 365);
        let current_fees = (annual_gross as f64 * (current_commission / 100.0)) as u128;
        let new_fees = (annual_gross as f64 * (new_commission / 100.0)) as u128;

        (current_fees, new_fees)
    }
}

impl Default for RewardCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_reward_simple() {
        let calculator = RewardCalculator::new();
        let reward = calculator.estimate_reward(1000, 10.0, 365);
        assert_eq!(reward, 100);
    }

    #[test]
    fn test_estimate_reward_monthly() {
        let calculator = RewardCalculator::new();
        let reward = calculator.estimate_reward(1000, 12.0, 30);
        assert!(reward > 0);
    }

    #[test]
    fn test_compound_balance_basic() {
        let calculator = RewardCalculator::new();
        let result = calculator.compound_balance(1000, 10.0, 12);
        assert!(result > 1000);
    }

    #[test]
    fn test_record_era_reward() {
        let mut calculator = RewardCalculator::new();
        calculator.record_era_reward(1, 1000, 10000);
        calculator.record_era_reward(2, 1050, 10500);

        let era1 = calculator.get_era(1);
        assert!(era1.is_some());
        assert_eq!(era1.unwrap().total_reward, 1000);
    }

    #[test]
    fn test_calculate_apy_with_eras() {
        let mut calculator = RewardCalculator::new();
        for era in 0..20 {
            calculator.record_era_reward(era, 1000, 10000);
        }

        let apy = calculator.calculate_apy(100);
        assert!(apy > 0.0);
    }

    #[test]
    fn test_apy_history_tracking() {
        let mut calculator = RewardCalculator::new();
        calculator.record_era_reward(1, 1000, 10000);
        calculator.get_apy_calculation(1000);

        assert_eq!(calculator.apy_history.len(), 1);
    }

    #[test]
    fn test_average_apy() {
        let mut calculator = RewardCalculator::new();
        for era in 0..10 {
            calculator.record_era_reward(era, 1000, 10000);
        }

        calculator.record_era_reward(10, 1000, 10000);
        calculator.get_apy_calculation(1000);

        let avg = calculator.average_apy(365);
        assert!(avg >= 0.0);
    }

    #[test]
    fn test_compound_with_restake_interval() {
        let calculator = RewardCalculator::new();
        let monthly = calculator.compound_rewards(1000, 12.0, 12, 1);
        assert!(monthly > 1000);
    }

    #[test]
    fn test_apy_volatility() {
        let mut calculator = RewardCalculator::new();
        for era in 0..10 {
            calculator.record_era_reward(era, 1000 + era as u128 * 100, 10000);
        }

        calculator.record_era_reward(10, 1000, 10000);
        calculator.get_apy_calculation(1000);

        let volatility = calculator.apy_volatility(365);
        assert!(volatility >= 0.0);
    }

    #[test]
    fn test_commission_impact() {
        let calculator = RewardCalculator::new();
        let (current, new) = calculator.estimate_commission_impact(1000, 10.0, 5.0, 10.0);

        assert!(new > current);
    }

    #[test]
    fn test_zero_balance_no_reward() {
        let calculator = RewardCalculator::new();
        let reward = calculator.estimate_reward(0, 10.0, 365);
        assert_eq!(reward, 0);
    }

    #[test]
    fn test_apy_calculation_object() {
        let mut calculator = RewardCalculator::new();
        for era in 0..10 {
            calculator.record_era_reward(era, 1000, 10000);
        }

        let calc = calculator.get_apy_calculation(1000);
        assert!(calc.current_apy >= 0.0);
        assert!(calc.estimated_annual_reward >= 0);
        assert!(calc.compounded_24m_balance >= 1000);
    }
}
