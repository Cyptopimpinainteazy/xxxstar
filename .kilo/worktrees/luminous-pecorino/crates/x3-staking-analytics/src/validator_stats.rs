//! Validator Stats — Performance tracking and metrics
//! 
//! Maintains validator performance data including uptime, commission,
//! backing, and performance scoring.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::Result;

/// Validator performance tier
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PerformanceTier {
    Excellent, // > 95%
    Good,      // 90-95%
    Average,   // 80-90%
    Poor,      // < 80%
}

/// Validator performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorPerformance {
    pub validator: String,
    pub uptime_percentage: f64,
    pub produced_blocks: u32,
    pub missed_blocks: u32,
    pub points_earned: u32,
    pub performance_tier: PerformanceTier,
    pub last_updated: DateTime<Utc>,
}

impl ValidatorPerformance {
    /// Determine performance tier based on uptime
    pub fn determine_tier(uptime: f64) -> PerformanceTier {
        match uptime {
            x if x > 95.0 => PerformanceTier::Excellent,
            x if x >= 90.0 => PerformanceTier::Good,
            x if x >= 80.0 => PerformanceTier::Average,
            _ => PerformanceTier::Poor,
        }
    }

    /// Risk score (0-100, higher = riskier)
    pub fn risk_score(&self) -> f64 {
        let uptime_risk = (100.0 - self.uptime_percentage) * 2.0;
        let tier_risk = match self.performance_tier {
            PerformanceTier::Excellent => 0.0,
            PerformanceTier::Good => 5.0,
            PerformanceTier::Average => 15.0,
            PerformanceTier::Poor => 30.0,
        };
        ((uptime_risk + tier_risk) / 2.0).min(100.0)
    }
}

/// Validator statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorStats {
    pub address: String,
    pub name: String,
    pub commission: f64, // percentage
    pub backed_amount: u128,
    pub nominator_count: u32,
    pub performance: ValidatorPerformance,
    pub total_points: u32,
    pub historical_commission: Vec<(DateTime<Utc>, f64)>,
    pub created_at: DateTime<Utc>,
}

impl ValidatorStats {
    /// Annual backing reward (before commission)
    pub fn annual_backing_reward(&self, apy: f64) -> u128 {
        (self.backed_amount as f64 * apy / 100.0) as u128
    }

    /// Annual commission earned
    pub fn annual_commission_from_backing(&self, apy: f64) -> u128 {
        let backing_reward = self.annual_backing_reward(apy);
        (backing_reward as f64 * self.commission / 100.0) as u128
    }

    /// Nominator reward after deducting commission
    pub fn nominator_net_reward(&self, apy: f64) -> f64 {
        apy * (1.0 - self.commission / 100.0)
    }

    /// Score (0-100) based on multiple factors
    pub fn overall_score(&self) -> f64 {
        let uptime_score = self.performance.uptime_percentage;
        let commission_score = 100.0 - (self.commission.min(100.0));
        let nominator_score = (self.nominator_count as f64 / 1000.0 * 100.0).min(100.0);

        (uptime_score * 0.5 + commission_score * 0.3 + nominator_score * 0.2).min(100.0)
    }

    /// Is this validator recommended (score > 80)?
    pub fn is_recommended(&self) -> bool {
        self.overall_score() > 80.0 && self.performance.uptime_percentage > 90.0
    }
}

/// Validator Stats Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorStatsManager {
    validators: HashMap<String, ValidatorStats>,
    performance_history: HashMap<String, Vec<ValidatorPerformance>>,
}

impl ValidatorStatsManager {
    pub fn new() -> Self {
        ValidatorStatsManager {
            validators: HashMap::new(),
            performance_history: HashMap::new(),
        }
    }

    /// Register or update validator stats
    pub fn register_validator(
        &mut self,
        address: &str,
        name: &str,
        commission: f64,
        backed_amount: u128,
    ) -> &ValidatorStats {
        let performance = ValidatorPerformance {
            validator: address.to_string(),
            uptime_percentage: 100.0,
            produced_blocks: 0,
            missed_blocks: 0,
            points_earned: 0,
            performance_tier: PerformanceTier::Excellent,
            last_updated: Utc::now(),
        };

        let stats = ValidatorStats {
            address: address.to_string(),
            name: name.to_string(),
            commission,
            backed_amount,
            nominator_count: 0,
            performance,
            total_points: 0,
            historical_commission: vec![(Utc::now(), commission)],
            created_at: Utc::now(),
        };

        self.validators.insert(address.to_string(), stats);
        self.validators.get(address).unwrap()
    }

    /// Update performance metrics
    pub fn update_performance(
        &mut self,
        validator: &str,
        produced: u32,
        missed: u32,
        points: u32,
    ) -> Result<()> {
        if let Some(stats) = self.validators.get_mut(validator) {
            let total_blocks = produced + missed;
            let uptime = if total_blocks > 0 {
                (produced as f64 / total_blocks as f64) * 100.0
            } else {
                100.0
            };

            stats.performance.produced_blocks = produced;
            stats.performance.missed_blocks = missed;
            stats.performance.uptime_percentage = uptime;
            stats.performance.performance_tier = ValidatorPerformance::determine_tier(uptime);
            stats.performance.points_earned = points;
            stats.performance.last_updated = Utc::now();
            stats.total_points = stats.total_points.saturating_add(points);

            // Track history
            self.performance_history
                .entry(validator.to_string())
                .or_insert_with(Vec::new)
                .push(stats.performance.clone());

            Ok(())
        } else {
            Err(crate::StakingError::ValidatorNotFound)
        }
    }

    /// Update commission
    pub fn update_commission(&mut self, validator: &str, new_commission: f64) -> Result<()> {
        if let Some(stats) = self.validators.get_mut(validator) {
            stats.commission = new_commission;
            stats
                .historical_commission
                .push((Utc::now(), new_commission));
            Ok(())
        } else {
            Err(crate::StakingError::ValidatorNotFound)
        }
    }

    /// Update backed amount
    pub fn update_backed_amount(&mut self, validator: &str, amount: u128) -> Result<()> {
        if let Some(stats) = self.validators.get_mut(validator) {
            stats.backed_amount = amount;
            Ok(())
        } else {
            Err(crate::StakingError::ValidatorNotFound)
        }
    }

    /// Update nominator count
    pub fn update_nominator_count(&mut self, validator: &str, count: u32) -> Result<()> {
        if let Some(stats) = self.validators.get_mut(validator) {
            stats.nominator_count = count;
            Ok(())
        } else {
            Err(crate::StakingError::ValidatorNotFound)
        }
    }

    /// Get validator by address
    pub fn get_validator(&self, validator: &str) -> Option<ValidatorStats> {
        self.validators.get(validator).cloned()
    }

    /// Get all validators
    pub fn all_validators(&self) -> Vec<ValidatorStats> {
        self.validators.values().cloned().collect()
    }

    /// Get recommended validators (score > 80)
    pub fn recommended_validators(&self) -> Vec<ValidatorStats> {
        self.validators
            .values()
            .filter(|v| v.is_recommended())
            .cloned()
            .collect()
    }

    /// Get validators sorted by score (descending)
    pub fn validators_by_score(&self) -> Vec<ValidatorStats> {
        let mut validators = self.all_validators();
        validators.sort_by(|a, b| {
            b.overall_score()
                .partial_cmp(&a.overall_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        validators
    }

    /// Get validators by commission (ascending)
    pub fn validators_by_commission(&self) -> Vec<ValidatorStats> {
        let mut validators = self.all_validators();
        validators.sort_by(|a, b| {
            a.commission
                .partial_cmp(&b.commission)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        validators
    }

    /// Get top N validators by score
    pub fn top_validators(&self, count: usize) -> Vec<ValidatorStats> {
        self.validators_by_score()
            .into_iter()
            .take(count)
            .collect()
    }

    /// Get performance history for validator
    pub fn get_history(&self, validator: &str) -> Vec<ValidatorPerformance> {
        self.performance_history
            .get(validator)
            .cloned()
            .unwrap_or_default()
    }

    /// Average uptime across all validators
    pub fn average_network_uptime(&self) -> f64 {
        if self.validators.is_empty() {
            return 0.0;
        }

        let sum: f64 = self
            .validators
            .values()
            .map(|v| v.performance.uptime_percentage)
            .sum();

        sum / self.validators.len() as f64
    }

    /// Risk score for portfolio
    pub fn portfolio_risk_score(&self, validators: &[&str]) -> f64 {
        if validators.is_empty() {
            return 0.0;
        }

        let scores: Vec<f64> = validators
            .iter()
            .filter_map(|v| {
                self.get_validator(v)
                    .map(|stats| stats.performance.risk_score())
            })
            .collect();

        if scores.is_empty() {
            return 0.0;
        }

        scores.iter().sum::<f64>() / scores.len() as f64
    }

    /// Validator count
    pub fn count(&self) -> u32 {
        self.validators.len() as u32
    }
}

impl Default for ValidatorStatsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_tier_determination() {
        assert_eq!(
            ValidatorPerformance::determine_tier(96.0),
            PerformanceTier::Excellent
        );
        assert_eq!(
            ValidatorPerformance::determine_tier(92.0),
            PerformanceTier::Good
        );
        assert_eq!(
            ValidatorPerformance::determine_tier(85.0),
            PerformanceTier::Average
        );
        assert_eq!(
            ValidatorPerformance::determine_tier(75.0),
            PerformanceTier::Poor
        );
    }

    #[test]
    fn test_risk_score_calculation() {
        let perf = ValidatorPerformance {
            validator: "val1".to_string(),
            uptime_percentage: 95.0,
            produced_blocks: 950,
            missed_blocks: 50,
            points_earned: 1000,
            performance_tier: PerformanceTier::Excellent,
            last_updated: Utc::now(),
        };

        let risk = perf.risk_score();
        assert!(risk < 20.0); // Good validator should have low risk
    }

    #[test]
    fn test_register_validator() {
        let mut manager = ValidatorStatsManager::new();
        manager.register_validator("val1", "Validator One", 5.0, 1000000);

        let val = manager.get_validator("val1");
        assert!(val.is_some());
        assert_eq!(val.unwrap().commission, 5.0);
    }

    #[test]
    fn test_update_performance() {
        let mut manager = ValidatorStatsManager::new();
        manager.register_validator("val1", "Validator One", 5.0, 1000000);
        manager.update_performance("val1", 950, 50, 500).unwrap();

        let val = manager.get_validator("val1").unwrap();
        assert_eq!(val.performance.produced_blocks, 950);
        assert_eq!(val.performance.uptime_percentage, 95.0);
    }

    #[test]
    fn test_annual_backing_reward() {
        let mut manager = ValidatorStatsManager::new();
        manager.register_validator("val1", "Validator One", 5.0, 1000000);

        let val = manager.get_validator("val1").unwrap();
        let reward = val.annual_backing_reward(10.0);
        assert_eq!(reward, 100000);
    }

    #[test]
    fn test_nominator_net_reward() {
        let mut manager = ValidatorStatsManager::new();
        manager.register_validator("val1", "Validator One", 10.0, 1000000);

        let val = manager.get_validator("val1").unwrap();
        let net = val.nominator_net_reward(10.0);
        assert_eq!(net, 9.0);
    }

    #[test]
    fn test_overall_score() {
        let mut manager = ValidatorStatsManager::new();
        manager.register_validator("val1", "Validator One", 5.0, 1000000);
        manager.update_performance("val1", 950, 50, 500).unwrap();

        let val = manager.get_validator("val1").unwrap();
        let score = val.overall_score();
        assert!(score > 85.0);
    }

    #[test]
    fn test_is_recommended() {
        let mut manager = ValidatorStatsManager::new();
        manager.register_validator("val1", "Validator One", 5.0, 1000000);
        manager.update_performance("val1", 950, 50, 500).unwrap();

        let val = manager.get_validator("val1").unwrap();
        assert!(val.is_recommended());
    }

    #[test]
    fn test_top_validators() {
        let mut manager = ValidatorStatsManager::new();
        for i in 0..5 {
            manager.register_validator(
                &format!("val{}", i),
                &format!("Validator {}", i),
                5.0,
                1000000,
            );
            manager
                .update_performance(&format!("val{}", i), 940 + i as u32 * 2, 60 - i as u32, 500)
                .unwrap();
        }

        let top = manager.top_validators(3);
        assert_eq!(top.len(), 3);
    }

    #[test]
    fn test_average_network_uptime() {
        let mut manager = ValidatorStatsManager::new();
        manager.register_validator("val1", "Validator One", 5.0, 1000000);
        manager.register_validator("val2", "Validator Two", 5.0, 1000000);

        manager.update_performance("val1", 950, 50, 500).unwrap();
        manager.update_performance("val2", 900, 100, 500).unwrap();

        let avg_uptime = manager.average_network_uptime();
        assert!((avg_uptime - 92.5).abs() < 0.1);
    }

    #[test]
    fn test_update_commission() {
        let mut manager = ValidatorStatsManager::new();
        manager.register_validator("val1", "Validator One", 5.0, 1000000);
        manager.update_commission("val1", 7.0).unwrap();

        let val = manager.get_validator("val1").unwrap();
        assert_eq!(val.commission, 7.0);
        assert_eq!(val.historical_commission.len(), 2);
    }

    #[test]
    fn test_portfolio_risk_score() {
        let mut manager = ValidatorStatsManager::new();
        manager.register_validator("val1", "Validator One", 5.0, 1000000);
        manager.register_validator("val2", "Validator Two", 5.0, 1000000);

        manager.update_performance("val1", 950, 50, 500).unwrap();
        manager.update_performance("val2", 950, 50, 500).unwrap();

        let risk = manager.portfolio_risk_score(&["val1", "val2"]);
        assert!(risk < 20.0);
    }

    #[test]
    fn test_validators_by_commission() {
        let mut manager = ValidatorStatsManager::new();
        manager.register_validator("val1", "Validator One", 10.0, 1000000);
        manager.register_validator("val2", "Validator Two", 3.0, 1000000);
        manager.register_validator("val3", "Validator Three", 7.0, 1000000);

        let sorted = manager.validators_by_commission();
        assert_eq!(sorted[0].commission, 3.0);
        assert_eq!(sorted[2].commission, 10.0);
    }
}
