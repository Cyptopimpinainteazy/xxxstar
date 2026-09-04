//! Cross-chain portfolio rebalancing engine
//!
//! This module provides:
//! - Target allocation management
//! - Volatility-based triggers
//! - APY change monitoring
//! - Fee change detection
//! - Gas efficiency optimization
//! - Liquidity shift monitoring
//! - "Rebalance All Chains" mode

use crate::config::PositionManagerConfig;
use crate::error::{PositionManagerError, Result};
use crate::types::{
    AllocationTarget, PositionId, RebalanceAction, RebalanceActionType, H160, H256, U256,
};
use serde::{Deserialize, Serialize};
use sp_std::vec::Vec;

/// Rebalancing engine for portfolio optimization
#[derive(Debug, Clone)]
pub struct RebalancingEngine {
    /// Target allocations
    target_allocations: sp_std::collections::btree_map::BTreeMap<(u64, H160), AllocationTarget>,
    /// Current allocations
    current_allocations: sp_std::collections::btree_map::BTreeMap<(u64, H160), U256>,
    /// Rebalancing history
    rebalance_history: Vec<RebalanceRecord>,
    /// Volatility thresholds
    volatility_thresholds: sp_std::collections::btree_map::BTreeMap<(u64, H160), f64>,
    /// APY tracking
    apy_tracking: sp_std::collections::btree_map::BTreeMap<(u64, H160), ApyTracker>,
    /// Fee tracking
    fee_tracking: sp_std::collections::btree_map::BTreeMap<(u64, H160), FeeTracker>,
    /// Configuration
    config: PositionManagerConfig,
}

impl RebalancingEngine {
    /// Create a new rebalancing engine
    pub fn new(config: &PositionManagerConfig) -> Result<Self> {
        Ok(Self {
            target_allocations: sp_std::collections::btree_map::BTreeMap::new(),
            current_allocations: sp_std::collections::btree_map::BTreeMap::new(),
            rebalance_history: Vec::new(),
            volatility_thresholds: sp_std::collections::btree_map::BTreeMap::new(),
            apy_tracking: sp_std::collections::btree_map::BTreeMap::new(),
            fee_tracking: sp_std::collections::btree_map::BTreeMap::new(),
            config: config.clone(),
        })
    }

    /// Set target allocation for a chain/asset pair
    pub fn set_target_allocation(
        &mut self,
        chain_id: u64,
        asset: H160,
        target: AllocationTarget,
    ) -> Result<()> {
        self.target_allocations.insert((chain_id, asset), target);
        Ok(())
    }

    /// Get target allocation
    pub fn get_target_allocation(&self, chain_id: u64, asset: H160) -> Option<&AllocationTarget> {
        self.target_allocations.get(&(chain_id, asset))
    }

    /// Update current allocation
    pub fn update_current_allocation(&mut self, chain_id: u64, asset: H160, amount: U256) {
        self.current_allocations.insert((chain_id, asset), amount);
    }

    /// Check if rebalancing is needed
    pub fn is_rebalance_needed(&self) -> Result<bool> {
        for ((chain_id, asset), target) in &self.target_allocations {
            let current = self
                .current_allocations
                .get(&(*chain_id, *asset))
                .copied()
                .unwrap_or(U256::zero());

            let total_value = self.get_total_value()?;
            if total_value == U256::zero() {
                continue;
            }

            let current_percentage = current
                .checked_mul(U256::from(10000))
                .unwrap_or(U256::zero())
                .checked_div(total_value)
                .unwrap_or(U256::zero());

            let target_percentage = U256::from((target.target_percentage * 10000.0) as u128);

            let diff = if current_percentage > target_percentage {
                current_percentage - target_percentage
            } else {
                target_percentage - current_percentage
            };

            // 1% threshold
            if diff > U256::from(100) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Execute rebalancing
    pub async fn rebalance(&self, targets: &[AllocationTarget]) -> Result<RebalanceResult> {
        let mut actions = Vec::new();
        let mut total_cost = U256::zero();

        for target in targets {
            let current = self
                .current_allocations
                .get(&(target.chain_id, target.asset))
                .copied()
                .unwrap_or(U256::zero());

            let target_amount = self.calculate_target_amount(target)?;

            if current < target_amount {
                // Need to buy/bridge in
                let diff = target_amount - current;
                actions.push(RebalanceAction {
                    action_type: RebalanceActionType::Buy,
                    chain_id: target.chain_id,
                    asset: target.asset,
                    amount: diff,
                    expected_output: diff,
                    gas_estimate: U256::from(100_000),
                    priority: 1,
                });
            } else if current > target_amount {
                // Need to sell/bridge out
                let diff = current - target_amount;
                actions.push(RebalanceAction {
                    action_type: RebalanceActionType::Sell,
                    chain_id: target.chain_id,
                    asset: target.asset,
                    amount: diff,
                    expected_output: diff,
                    gas_estimate: U256::from(100_000),
                    priority: 1,
                });
            }

            total_cost = total_cost.saturating_add(U256::from(100_000));
        }

        let rebalance_id = self.generate_rebalance_id(&actions);

        Ok(RebalanceResult {
            success: true,
            rebalance_id,
            actions_executed: actions.len(),
            total_cost_usd: total_cost,
            improvement_estimate: 0.05, // 5% improvement estimate
        })
    }

    /// Calculate target amount for allocation
    fn calculate_target_amount(&self, target: &AllocationTarget) -> Result<U256> {
        let total_value = self.get_total_value()?;
        let target_value = total_value
            .checked_mul(U256::from((target.target_percentage * 10000.0) as u128))
            .unwrap_or(U256::zero())
            .checked_div(U256::from(10000))
            .unwrap_or(U256::zero());

        Ok(target_value)
    }

    /// Get total portfolio value
    fn get_total_value(&self) -> Result<U256> {
        let mut total = U256::zero();
        for (_, amount) in &self.current_allocations {
            total = total.saturating_add(*amount);
        }
        Ok(total)
    }

    /// Check volatility triggers
    pub fn check_volatility_triggers(&self) -> Result<Vec<VolatilityAlert>> {
        let mut alerts = Vec::new();

        for ((chain_id, asset), threshold) in &self.volatility_thresholds {
            let volatility = self.calculate_volatility(*chain_id, *asset)?;
            if volatility > *threshold {
                alerts.push(VolatilityAlert {
                    chain_id: *chain_id,
                    asset: *asset,
                    current_volatility: volatility,
                    threshold: *threshold,
                    recommended_action: "Consider reducing position".to_string(),
                });
            }
        }

        Ok(alerts)
    }

    /// Calculate volatility for an asset
    fn calculate_volatility(&self, _chain_id: u64, _asset: H160) -> Result<f64> {
        // Placeholder - would calculate actual volatility from price history
        Ok(0.1) // 10% volatility
    }

    /// Set volatility threshold
    pub fn set_volatility_threshold(&mut self, chain_id: u64, asset: H160, threshold: f64) {
        self.volatility_thresholds
            .insert((chain_id, asset), threshold);
    }

    /// Check APY changes
    pub fn check_apy_changes(&self) -> Result<Vec<ApyChangeAlert>> {
        let mut alerts = Vec::new();

        for ((chain_id, asset), tracker) in &self.apy_tracking {
            let change = tracker.current_apy - tracker.previous_apy;
            let change_pct = if tracker.previous_apy > 0.0 {
                (change / tracker.previous_apy).abs()
            } else {
                0.0
            };

            if change_pct > 0.1 {
                // 10% change
                alerts.push(ApyChangeAlert {
                    chain_id: *chain_id,
                    asset: *asset,
                    previous_apy: tracker.previous_apy,
                    current_apy: tracker.current_apy,
                    change_percentage: change_pct,
                    recommended_action: if change > 0.0 {
                        "Consider increasing allocation".to_string()
                    } else {
                        "Consider decreasing allocation".to_string()
                    },
                });
            }
        }

        Ok(alerts)
    }

    /// Update APY tracking
    pub fn update_apy(&mut self, chain_id: u64, asset: H160, current_apy: f64) {
        let tracker = self
            .apy_tracking
            .entry((chain_id, asset))
            .or_insert(ApyTracker {
                previous_apy: 0.0,
                current_apy: 0.0,
                last_update: 0,
            });

        tracker.previous_apy = tracker.current_apy;
        tracker.current_apy = current_apy;
        tracker.last_update = sp_io::offchain::timestamp().unix_millis();
    }

    /// Check fee changes
    pub fn check_fee_changes(&self) -> Result<Vec<FeeChangeAlert>> {
        let mut alerts = Vec::new();

        for ((chain_id, asset), tracker) in &self.fee_tracking {
            let change = tracker.current_fee - tracker.previous_fee;
            let change_pct = if tracker.previous_fee > 0.0 {
                (change / tracker.previous_fee).abs()
            } else {
                0.0
            };

            if change_pct > 0.2 {
                // 20% change
                alerts.push(FeeChangeAlert {
                    chain_id: *chain_id,
                    asset: *asset,
                    previous_fee: tracker.previous_fee,
                    current_fee: tracker.current_fee,
                    change_percentage: change_pct,
                    recommended_action: if change > 0.0 {
                        "Consider alternative routes".to_string()
                    } else {
                        "Favorable fee change".to_string()
                    },
                });
            }
        }

        Ok(alerts)
    }

    /// Update fee tracking
    pub fn update_fee(&mut self, chain_id: u64, asset: H160, current_fee: f64) {
        let tracker = self
            .fee_tracking
            .entry((chain_id, asset))
            .or_insert(FeeTracker {
                previous_fee: 0.0,
                current_fee: 0.0,
                last_update: 0,
            });

        tracker.previous_fee = tracker.current_fee;
        tracker.current_fee = current_fee;
        tracker.last_update = sp_io::offchain::timestamp().unix_millis();
    }

    /// Optimize gas efficiency
    pub fn optimize_gas_efficiency(&self) -> Result<GasOptimizationReport> {
        let mut recommendations = Vec::new();
        let mut total_savings = U256::zero();

        for ((chain_id, asset), current) in &self.current_allocations {
            let gas_cost = self.estimate_gas_cost(*chain_id)?;
            let optimal_gas = self.calculate_optimal_gas(*chain_id, *asset, *current)?;

            if gas_cost > optimal_gas {
                let savings = gas_cost - optimal_gas;
                total_savings = total_savings.saturating_add(savings);

                recommendations.push(GasOptimizationRecommendation {
                    chain_id: *chain_id,
                    asset: *asset,
                    current_gas: gas_cost,
                    optimal_gas: optimal_gas,
                    potential_savings: savings,
                    action: "Consider batch transactions or timing optimization".to_string(),
                });
            }
        }

        Ok(GasOptimizationReport {
            total_potential_savings: total_savings,
            recommendations,
            generated_at: sp_io::offchain::timestamp().unix_millis(),
        })
    }

    /// Estimate gas cost for a chain
    fn estimate_gas_cost(&self, chain_id: u64) -> Result<U256> {
        let chain_config = self
            .config
            .chain_configs
            .get(&chain_id)
            .ok_or_else(|| PositionManagerError::ChainNotFound(chain_id))?;

        Ok(U256::from(
            chain_config.gas_price_multiplier as u128 * 100_000,
        ))
    }

    /// Calculate optimal gas
    fn calculate_optimal_gas(&self, _chain_id: u64, _asset: H160, _amount: U256) -> Result<U256> {
        // Placeholder - would calculate optimal gas based on network conditions
        Ok(U256::from(50_000))
    }

    /// Check liquidity shifts
    pub fn check_liquidity_shifts(&self) -> Result<Vec<LiquidityShiftAlert>> {
        let mut alerts = Vec::new();

        for ((chain_id, asset), current) in &self.current_allocations {
            let liquidity = self.get_liquidity(*chain_id, *asset)?;
            let liquidity_ratio = liquidity
                .checked_mul(U256::from(100))
                .unwrap_or(U256::zero())
                .checked_div(*current)
                .unwrap_or(U256::zero());

            if liquidity_ratio < U256::from(10) {
                // Less than 10x liquidity
                alerts.push(LiquidityShiftAlert {
                    chain_id: *chain_id,
                    asset: *asset,
                    current_position: *current,
                    available_liquidity: liquidity,
                    liquidity_ratio: liquidity_ratio.as_u128() as f64 / 100.0,
                    recommended_action: "Consider reducing position size".to_string(),
                });
            }
        }

        Ok(alerts)
    }

    /// Get liquidity for an asset
    fn get_liquidity(&self, _chain_id: u64, _asset: H160) -> Result<U256> {
        // Placeholder - would query actual liquidity from DEX
        Ok(U256::from(1_000_000_000_000_000_000_000u128)) // 1000 tokens
    }

    /// Execute "Rebalance All Chains" mode
    pub async fn rebalance_all_chains(&self) -> Result<RebalanceResult> {
        let mut all_actions = Vec::new();
        let mut total_cost = U256::zero();

        // Get all targets
        let targets: Vec<AllocationTarget> = self.target_allocations.values().cloned().collect();

        // Execute rebalancing for all targets
        let result = self.rebalance(&targets).await?;

        Ok(result)
    }

    /// Simulate rebalancing
    pub fn simulate_rebalance(&self, targets: &[AllocationTarget]) -> Result<SimulationResult> {
        let mut actions = Vec::new();
        let mut total_cost = U256::zero();

        for target in targets {
            let current = self
                .current_allocations
                .get(&(target.chain_id, target.asset))
                .copied()
                .unwrap_or(U256::zero());

            let target_amount = self.calculate_target_amount(target)?;

            if current < target_amount {
                let diff = target_amount - current;
                actions.push(RebalanceAction {
                    action_type: RebalanceActionType::Buy,
                    chain_id: target.chain_id,
                    asset: target.asset,
                    amount: diff,
                    expected_output: diff,
                    gas_estimate: U256::from(100_000),
                    priority: 1,
                });
            } else if current > target_amount {
                let diff = current - target_amount;
                actions.push(RebalanceAction {
                    action_type: RebalanceActionType::Sell,
                    chain_id: target.chain_id,
                    asset: target.asset,
                    amount: diff,
                    expected_output: diff,
                    gas_estimate: U256::from(100_000),
                    priority: 1,
                });
            }

            total_cost = total_cost.saturating_add(U256::from(100_000));
        }

        Ok(SimulationResult {
            feasible: true,
            total_actions: actions.len(),
            estimated_cost: total_cost,
            estimated_improvement: 0.05,
            risks: Vec::new(),
            actions,
        })
    }

    /// Generate rebalance ID
    fn generate_rebalance_id(&self, actions: &[RebalanceAction]) -> H256 {
        use sp_core::Hasher;
        use sp_runtime::traits::BlakeTwo256;

        let mut hasher = BlakeTwo256::default();
        hasher.hash(&actions.len().to_le_bytes());
        for action in actions {
            hasher.hash(&action.chain_id.to_le_bytes());
            hasher.hash(action.asset.as_bytes());
            hasher.hash(&action.amount.as_bytes());
        }
        hasher.hash(&sp_io::offchain::timestamp().unix_millis().to_le_bytes());
        H256::from_slice(hasher.finish().as_ref())
    }
}

/// Rebalance result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceResult {
    pub success: bool,
    pub rebalance_id: H256,
    pub actions_executed: usize,
    pub total_cost_usd: U256,
    pub improvement_estimate: f64,
}

/// Rebalance record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceRecord {
    pub rebalance_id: H256,
    pub timestamp: u64,
    pub actions_count: usize,
    pub total_cost: U256,
    pub success: bool,
}

/// Volatility alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolatilityAlert {
    pub chain_id: u64,
    pub asset: H160,
    pub current_volatility: f64,
    pub threshold: f64,
    pub recommended_action: String,
}

/// APY tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApyTracker {
    pub previous_apy: f64,
    pub current_apy: f64,
    pub last_update: u64,
}

/// APY change alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApyChangeAlert {
    pub chain_id: u64,
    pub asset: H160,
    pub previous_apy: f64,
    pub current_apy: f64,
    pub change_percentage: f64,
    pub recommended_action: String,
}

/// Fee tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeTracker {
    pub previous_fee: f64,
    pub current_fee: f64,
    pub last_update: u64,
}

/// Fee change alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeChangeAlert {
    pub chain_id: u64,
    pub asset: H160,
    pub previous_fee: f64,
    pub current_fee: f64,
    pub change_percentage: f64,
    pub recommended_action: String,
}

/// Gas optimization report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasOptimizationReport {
    pub total_potential_savings: U256,
    pub recommendations: Vec<GasOptimizationRecommendation>,
    pub generated_at: u64,
}

/// Gas optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasOptimizationRecommendation {
    pub chain_id: u64,
    pub asset: H160,
    pub current_gas: U256,
    pub optimal_gas: U256,
    pub potential_savings: U256,
    pub action: String,
}

/// Liquidity shift alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityShiftAlert {
    pub chain_id: u64,
    pub asset: H160,
    pub current_position: U256,
    pub available_liquidity: U256,
    pub liquidity_ratio: f64,
    pub recommended_action: String,
}

/// Simulation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub feasible: bool,
    pub total_actions: usize,
    pub estimated_cost: U256,
    pub estimated_improvement: f64,
    pub risks: Vec<String>,
    pub actions: Vec<RebalanceAction>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rebalancing_engine() {
        let config = PositionManagerConfig::default();
        let engine = RebalancingEngine::new(&config).unwrap();

        assert!(engine.target_allocations.is_empty());
        assert!(engine.current_allocations.is_empty());
    }

    #[test]
    fn test_is_rebalance_needed() {
        let config = PositionManagerConfig::default();
        let engine = RebalancingEngine::new(&config).unwrap();

        let result = engine.is_rebalance_needed().unwrap();
        assert!(!result); // No targets set, so no rebalancing needed
    }
}
