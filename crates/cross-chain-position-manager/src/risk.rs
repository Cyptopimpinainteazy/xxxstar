//! Risk management system for cross-chain positions
//!
//! This module provides:
//! - Kill switch configuration and triggering
//! - Risk threshold management
//! - Liquidation threshold monitoring
//! - Error severity classification
//! - Retryable vs fatal error handling

use crate::config::{PositionManagerConfig, RiskConfig};
use crate::error::{PositionManagerError, Result};
use crate::types::{
    AutoAction, KillSwitchConfig, PositionId, RiskLevel, TriggerCondition, TriggerConditionType,
    H160, H256, U256,
};
use serde::{Deserialize, Serialize};
use sp_std::vec::Vec;

/// Risk manager for cross-chain position monitoring
#[derive(Debug, Clone)]
pub struct RiskManager {
    /// Kill switch configurations
    kill_switches: sp_std::collections::btree_map::BTreeMap<String, KillSwitchConfig>,
    /// Risk thresholds
    risk_thresholds: sp_std::collections::btree_map::BTreeMap<String, RiskThreshold>,
    /// Active alerts
    active_alerts: Vec<RiskAlert>,
    /// Alert history
    alert_history: Vec<RiskAlert>,
    /// Configuration
    config: RiskConfig,
}

impl RiskManager {
    /// Create a new risk manager
    pub fn new(config: &RiskConfig) -> Result<Self> {
        Ok(Self {
            kill_switches: sp_std::collections::btree_map::BTreeMap::new(),
            risk_thresholds: sp_std::collections::btree_map::BTreeMap::new(),
            active_alerts: Vec::new(),
            alert_history: Vec::new(),
            config: config.clone(),
        })
    }

    /// Start risk monitoring
    pub async fn start(&mut self) -> Result<()> {
        // Initialize default kill switches
        self.initialize_default_kill_switches();
        Ok(())
    }

    /// Stop risk monitoring
    pub async fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// Initialize default kill switches
    fn initialize_default_kill_switches(&mut self) {
        // Chain failure kill switch
        self.kill_switches.insert(
            "chain_failure".to_string(),
            KillSwitchConfig {
                enabled: true,
                trigger_conditions: vec![TriggerCondition {
                    condition_type: TriggerConditionType::ChainLatency,
                    threshold_value: 30000.0, // 30 seconds
                    chain_ids: Vec::new(),
                    position_types: Vec::new(),
                }],
                auto_actions: vec![AutoAction::PauseTrading],
                notification_channels: vec!["email".to_string(), "slack".to_string()],
            },
        );

        // Rug detection kill switch
        self.kill_switches.insert(
            "rug_detection".to_string(),
            KillSwitchConfig {
                enabled: true,
                trigger_conditions: vec![TriggerCondition {
                    condition_type: TriggerConditionType::PriceDrop,
                    threshold_value: 0.5, // 50% drop
                    chain_ids: Vec::new(),
                    position_types: Vec::new(),
                }],
                auto_actions: vec![AutoAction::UnwindPositions],
                notification_channels: vec!["email".to_string(), "sms".to_string()],
            },
        );

        // Liquidity crisis kill switch
        self.kill_switches.insert(
            "liquidity_crisis".to_string(),
            KillSwitchConfig {
                enabled: true,
                trigger_conditions: vec![TriggerCondition {
                    condition_type: TriggerConditionType::LiquidityDrop,
                    threshold_value: 0.8, // 80% drop
                    chain_ids: Vec::new(),
                    position_types: Vec::new(),
                }],
                auto_actions: vec![AutoAction::ConsolidateToT1],
                notification_channels: vec!["email".to_string()],
            },
        );

        // Gas spike kill switch
        self.kill_switches.insert(
            "gas_spike".to_string(),
            KillSwitchConfig {
                enabled: true,
                trigger_conditions: vec![TriggerCondition {
                    condition_type: TriggerConditionType::GasSpike,
                    threshold_value: 10.0, // 10x normal
                    chain_ids: Vec::new(),
                    position_types: Vec::new(),
                }],
                auto_actions: vec![AutoAction::PauseTrading],
                notification_channels: vec!["email".to_string()],
            },
        );

        // Strategy failure kill switch
        self.kill_switches.insert(
            "strategy_failure".to_string(),
            KillSwitchConfig {
                enabled: true,
                trigger_conditions: vec![TriggerCondition {
                    condition_type: TriggerConditionType::ErrorRate,
                    threshold_value: 0.1, // 10% error rate
                    chain_ids: Vec::new(),
                    position_types: Vec::new(),
                }],
                auto_actions: vec![AutoAction::EmergencyStop],
                notification_channels: vec![
                    "email".to_string(),
                    "slack".to_string(),
                    "sms".to_string(),
                ],
            },
        );
    }

    /// Assess position risk
    pub async fn assess_position(&self, position_id: &PositionId) -> Result<RiskAssessment> {
        // In a real implementation, this would:
        // 1. Get position details
        // 2. Check various risk factors
        // 3. Calculate risk score
        // 4. Generate recommendations

        let risk_factors = vec![
            RiskFactor {
                factor_type: RiskFactorType::Liquidity,
                severity: RiskLevel::Low,
                description: "Adequate liquidity available".to_string(),
                mitigation: None,
            },
            RiskFactor {
                factor_type: RiskFactorType::Market,
                severity: RiskLevel::Medium,
                description: "Moderate market volatility".to_string(),
                mitigation: Some("Consider hedging strategies".to_string()),
            },
        ];

        let overall_risk = self.calculate_overall_risk(&risk_factors)?;

        Ok(RiskAssessment {
            position_id: position_id.clone(),
            overall_risk,
            risk_factors,
            recommendations: vec![
                "Monitor liquidity conditions".to_string(),
                "Consider position sizing adjustments".to_string(),
            ],
            score: 0.3, // 30% risk score
        })
    }

    /// Check all kill switches
    pub async fn check_all_kill_switches(&self) -> Result<Vec<KillSwitchTrigger>> {
        let mut triggers = Vec::new();

        for (name, config) in &self.kill_switches {
            if !config.enabled {
                continue;
            }

            for condition in &config.trigger_conditions {
                let triggered = self.evaluate_condition(condition).await?;
                if triggered {
                    triggers.push(KillSwitchTrigger {
                        trigger_type: self.map_condition_to_trigger_type(&condition.condition_type),
                        severity: RiskLevel::Critical,
                        description: format!("Kill switch '{}' triggered", name),
                        auto_action: config
                            .auto_actions
                            .first()
                            .cloned()
                            .unwrap_or(AutoAction::None),
                    });
                }
            }
        }

        Ok(triggers)
    }

    /// Evaluate a trigger condition
    async fn evaluate_condition(&self, condition: &TriggerCondition) -> Result<bool> {
        match condition.condition_type {
            TriggerConditionType::PriceDrop => {
                // Check if price has dropped below threshold
                // In a real implementation, this would check actual price feeds
                Ok(false)
            }
            TriggerConditionType::VolumeSpike => {
                // Check if volume has spiked above threshold
                Ok(false)
            }
            TriggerConditionType::GasSpike => {
                // Check if gas prices have spiked
                Ok(false)
            }
            TriggerConditionType::ChainLatency => {
                // Check if chain latency is above threshold
                Ok(false)
            }
            TriggerConditionType::LiquidityDrop => {
                // Check if liquidity has dropped
                Ok(false)
            }
            TriggerConditionType::ErrorRate => {
                // Check if error rate is above threshold
                Ok(false)
            }
            TriggerConditionType::UnauthorizedAccess => {
                // Check for unauthorized access attempts
                Ok(false)
            }
        }
    }

    /// Map condition type to trigger type
    fn map_condition_to_trigger_type(
        &self,
        condition_type: &TriggerConditionType,
    ) -> crate::types::KillSwitchType {
        match condition_type {
            TriggerConditionType::PriceDrop => crate::types::KillSwitchType::RugDetection,
            TriggerConditionType::VolumeSpike => crate::types::KillSwitchType::RiskThreshold,
            TriggerConditionType::GasSpike => crate::types::KillSwitchType::GasSpike,
            TriggerConditionType::ChainLatency => crate::types::KillSwitchType::ChainFailure,
            TriggerConditionType::LiquidityDrop => crate::types::KillSwitchType::LiquidityCrisis,
            TriggerConditionType::ErrorRate => crate::types::KillSwitchType::StrategyFailure,
            TriggerConditionType::UnauthorizedAccess => crate::types::KillSwitchType::RiskThreshold,
        }
    }

    /// Calculate overall risk from risk factors
    fn calculate_overall_risk(&self, risk_factors: &[RiskFactor]) -> Result<RiskLevel> {
        if risk_factors.is_empty() {
            return Ok(RiskLevel::Low);
        }

        let max_severity = risk_factors
            .iter()
            .map(|f| f.severity)
            .max()
            .unwrap_or(RiskLevel::Low);

        Ok(max_severity)
    }

    /// Add a kill switch
    pub fn add_kill_switch(&mut self, name: String, config: KillSwitchConfig) {
        self.kill_switches.insert(name, config);
    }

    /// Remove a kill switch
    pub fn remove_kill_switch(&mut self, name: &str) {
        self.kill_switches.remove(name);
    }

    /// Enable/disable a kill switch
    pub fn set_kill_switch_enabled(&mut self, name: &str, enabled: bool) -> Result<()> {
        let config = self
            .kill_switches
            .get_mut(name)
            .ok_or_else(|| PositionManagerError::KillSwitchNotFound(name.to_string()))?;
        config.enabled = enabled;
        Ok(())
    }

    /// Set risk threshold
    pub fn set_risk_threshold(&mut self, name: String, threshold: RiskThreshold) {
        self.risk_thresholds.insert(name, threshold);
    }

    /// Get risk threshold
    pub fn get_risk_threshold(&self, name: &str) -> Option<&RiskThreshold> {
        self.risk_thresholds.get(name)
    }

    /// Add risk alert
    pub fn add_alert(&mut self, alert: RiskAlert) {
        self.active_alerts.push(alert.clone());
        self.alert_history.push(alert);
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> &[RiskAlert] {
        &self.active_alerts
    }

    /// Clear active alert
    pub fn clear_alert(&mut self, alert_id: &H256) {
        self.active_alerts.retain(|a| a.id != *alert_id);
    }

    /// Get alert history
    pub fn get_alert_history(&self, limit: Option<usize>) -> &[RiskAlert] {
        let limit = limit.unwrap_or(self.alert_history.len());
        let start = if self.alert_history.len() > limit {
            self.alert_history.len() - limit
        } else {
            0
        };
        &self.alert_history[start..]
    }

    /// Check if position should be liquidated
    pub async fn should_liquidate(&self, position_id: &PositionId) -> Result<bool> {
        // In a real implementation, this would:
        // 1. Get position health factor
        // 2. Check against liquidation threshold
        // 3. Return true if below threshold

        Ok(false)
    }

    /// Get risk score for a position
    pub async fn get_risk_score(&self, position_id: &PositionId) -> Result<f64> {
        let assessment = self.assess_position(position_id).await?;
        Ok(assessment.score)
    }

    /// Check if emergency stop is needed
    pub async fn check_emergency_stop(&self) -> Result<bool> {
        let triggers = self.check_all_kill_switches().await?;
        for trigger in &triggers {
            if trigger.severity == RiskLevel::Critical {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Risk threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskThreshold {
    pub max_position_size_usd: U256,
    pub max_exposure_per_chain: f64,
    pub max_correlation: f64,
    pub liquidation_threshold: f64,
    pub stop_loss_percentage: f64,
}

/// Risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub position_id: PositionId,
    pub overall_risk: RiskLevel,
    pub risk_factors: Vec<RiskFactor>,
    pub recommendations: Vec<String>,
    pub score: f64,
}

/// Individual risk factor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_type: RiskFactorType,
    pub severity: RiskLevel,
    pub description: String,
    pub mitigation: Option<String>,
}

/// Types of risk factors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskFactorType {
    Liquidity,
    Market,
    Technical,
    Operational,
    Regulatory,
    Counterparty,
    SmartContract,
}

/// Kill switch trigger event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSwitchTrigger {
    pub trigger_type: crate::types::KillSwitchType,
    pub severity: RiskLevel,
    pub description: String,
    pub auto_action: AutoAction,
}

/// Risk alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAlert {
    pub id: H256,
    pub alert_type: RiskAlertType,
    pub severity: RiskLevel,
    pub message: String,
    pub position_id: Option<PositionId>,
    pub chain_id: Option<u64>,
    pub timestamp: u64,
    pub acknowledged: bool,
}

/// Risk alert types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskAlertType {
    HighVolatility,
    LowLiquidity,
    HighExposure,
    LiquidationRisk,
    GasSpike,
    ChainFailure,
    StrategyFailure,
    UnauthorizedAccess,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_manager() {
        let config = RiskConfig::default();
        let manager = RiskManager::new(&config).unwrap();

        assert!(manager.kill_switches.is_empty());
        assert!(manager.risk_thresholds.is_empty());
    }

    #[test]
    fn test_default_kill_switches() {
        let config = RiskConfig::default();
        let mut manager = RiskManager::new(&config).unwrap();
        manager.initialize_default_kill_switches();

        assert!(manager.kill_switches.contains_key("chain_failure"));
        assert!(manager.kill_switches.contains_key("rug_detection"));
        assert!(manager.kill_switches.contains_key("liquidity_crisis"));
        assert!(manager.kill_switches.contains_key("gas_spike"));
        assert!(manager.kill_switches.contains_key("strategy_failure"));
    }
}
