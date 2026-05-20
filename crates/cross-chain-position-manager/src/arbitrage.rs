//! Arbitrage detection and execution engine
//!
//! This module provides:
//! - Cross-chain arbitrage opportunity detection
//! - Atomic execution of arbitrage bundles
//! - Profit calculation and risk assessment

use crate::config::PositionManagerConfig;
use crate::error::{PositionManagerError, Result};
use crate::types::{ArbitrageOpportunity as ArbitrageOpportunityType, SwapRoute, H160, H256, U256};
use serde::{Deserialize, Serialize};
use sp_std::vec::Vec;

/// Arbitrage detector for finding opportunities
#[derive(Debug, Clone)]
pub struct ArbitrageDetector {
    /// Price feeds for assets
    price_feeds: sp_std::collections::btree_map::BTreeMap<(u64, H160), U256>,
    /// Minimum profit threshold
    min_profit_usd: U256,
    /// Maximum slippage tolerance
    max_slippage: f64,
    /// Supported chains
    supported_chains: Vec<u64>,
    /// Configuration
    config: PositionManagerConfig,
}

impl ArbitrageDetector {
    /// Create a new arbitrage detector
    pub fn new(config: &PositionManagerConfig) -> Result<Self> {
        Ok(Self {
            price_feeds: sp_std::collections::btree_map::BTreeMap::new(),
            min_profit_usd: U256::from(10_000_000_000_000_000_000u128), // 10 USD
            max_slippage: 0.005,                                        // 0.5%
            supported_chains: config.chain_configs.keys().cloned().collect(),
            config: config.clone(),
        })
    }

    /// Update price for an asset on a specific chain
    pub fn update_price(&mut self, chain_id: u64, asset: H160, price: U256) {
        self.price_feeds.insert((chain_id, asset), price);
    }

    /// Get price for an asset on a specific chain
    pub fn get_price(&self, chain_id: u64, asset: H160) -> Option<&U256> {
        self.price_feeds.get(&(chain_id, asset))
    }

    /// Find arbitrage opportunities across chains
    pub async fn find_opportunities(&self) -> Result<Vec<ArbitrageOpportunityType>> {
        let mut opportunities = Vec::new();

        // Check each pair of chains for price discrepancies
        for &chain1 in &self.supported_chains {
            for &chain2 in &self.supported_chains {
                if chain1 >= chain2 {
                    continue;
                }

                // Check each asset for price differences
                for ((c1, asset1), price1) in &self.price_feeds {
                    if *c1 != chain1 {
                        continue;
                    }

                    // Look for same asset on chain2
                    if let Some(price2) = self.price_feeds.get(&(chain2, *asset1)) {
                        // Calculate price difference
                        let price_diff = if price1 > price2 {
                            price1.checked_sub(*price2).unwrap_or(U256::zero())
                        } else {
                            price2.checked_sub(*price1).unwrap_or(U256::zero())
                        };

                        // Calculate percentage difference
                        let avg_price = price1
                            .checked_add(*price2)
                            .unwrap_or(U256::zero())
                            .checked_div(U256::from(2))
                            .unwrap_or(U256::zero());

                        if avg_price == U256::zero() {
                            continue;
                        }

                        let percentage_diff =
                            price_diff.as_u128() as f64 / avg_price.as_u128() as f64;

                        // Check if profitable after gas costs
                        if percentage_diff > 0.01 {
                            // 1% difference
                            let opportunity = self
                                .calculate_opportunity(
                                    chain1,
                                    chain2,
                                    *asset1,
                                    *price1,
                                    *price2,
                                    percentage_diff,
                                )
                                .await?;

                            if let Some(opp) = opportunity {
                                opportunities.push(opp);
                            }
                        }
                    }
                }
            }
        }

        // Sort by profit (highest first)
        opportunities.sort_by(|a, b| b.profit_usd.cmp(&a.profit_usd));

        Ok(opportunities)
    }

    /// Calculate arbitrage opportunity details
    async fn calculate_opportunity(
        &self,
        chain1: u64,
        chain2: u64,
        asset: H160,
        price1: U256,
        price2: U256,
        percentage_diff: f64,
    ) -> Result<Option<ArbitrageOpportunityType>> {
        // Determine buy and sell chains
        let (buy_chain, sell_chain, buy_price, sell_price) = if price1 < price2 {
            (chain1, chain2, price1, price2)
        } else {
            (chain2, chain1, price2, price1)
        };

        // Estimate gas costs
        let gas_cost_buy = self.estimate_gas_cost(buy_chain).await?;
        let gas_cost_sell = self.estimate_gas_cost(sell_chain).await?;
        let bridge_fee = self.estimate_bridge_fee(buy_chain, sell_chain).await?;

        let total_cost = gas_cost_buy
            .checked_add(gas_cost_sell)
            .unwrap_or(U256::zero())
            .checked_add(bridge_fee)
            .unwrap_or(U256::zero());

        // Calculate potential profit for different amounts
        let amounts = vec![
            U256::from(1_000_000_000_000_000_000u128),   // 1 ETH
            U256::from(10_000_000_000_000_000_000u128),  // 10 ETH
            U256::from(100_000_000_000_000_000_000u128), // 100 ETH
        ];

        let mut best_opportunity = None;
        let mut best_profit = U256::zero();

        for amount in amounts {
            let profit = self.calculate_profit(amount, buy_price, sell_price, total_cost)?;

            if profit > best_profit && profit >= self.min_profit_usd {
                best_profit = profit;
                best_opportunity = Some(self.create_opportunity(
                    buy_chain,
                    sell_chain,
                    asset,
                    amount,
                    profit,
                    percentage_diff,
                )?);
            }
        }

        Ok(best_opportunity)
    }

    /// Calculate profit for a given amount
    fn calculate_profit(
        &self,
        amount: U256,
        buy_price: U256,
        sell_price: U256,
        gas_cost: U256,
    ) -> Result<U256> {
        // Calculate buy cost: amount * buy_price / 10^18
        let buy_cost = amount
            .checked_mul(buy_price)
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?
            .checked_div(U256::from(10).pow(U256::from(18)))
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?;

        // Calculate sell revenue: amount * sell_price / 10^18
        let sell_revenue = amount
            .checked_mul(sell_price)
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?
            .checked_div(U256::from(10).pow(U256::from(18)))
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?;

        // Calculate profit: sell_revenue - buy_cost - gas_cost
        let profit = sell_revenue
            .checked_sub(buy_cost)
            .unwrap_or(U256::zero())
            .checked_sub(gas_cost)
            .unwrap_or(U256::zero());

        Ok(profit)
    }

    /// Estimate gas cost for a chain
    async fn estimate_gas_cost(&self, chain_id: u64) -> Result<U256> {
        // Base gas cost for swap
        let base_gas = U256::from(200_000);

        // Get gas price multiplier from config
        let multiplier = self
            .config
            .chain_configs
            .get(&chain_id)
            .map(|c| c.gas_price_multiplier)
            .unwrap_or(1.0);

        let gas_cost = base_gas
            .checked_mul(U256::from((multiplier * 100.0) as u64))
            .unwrap_or(base_gas)
            .checked_div(U256::from(100))
            .unwrap_or(base_gas);

        Ok(gas_cost)
    }

    /// Estimate bridge fee between chains
    async fn estimate_bridge_fee(&self, from_chain: u64, to_chain: u64) -> Result<U256> {
        // Base bridge fee
        let base_fee = U256::from(1_000_000_000_000_000u128); // 0.001 ETH

        // Adjust based on chain distance
        let distance = if from_chain < to_chain {
            to_chain - from_chain
        } else {
            from_chain - to_chain
        };

        let fee = base_fee
            .checked_mul(U256::from(distance))
            .unwrap_or(base_fee)
            .checked_div(U256::from(10))
            .unwrap_or(base_fee);

        Ok(fee)
    }

    /// Create arbitrage opportunity
    fn create_opportunity(
        &self,
        buy_chain: u64,
        sell_chain: u64,
        asset: H160,
        amount: U256,
        profit: U256,
        percentage_diff: f64,
    ) -> Result<ArbitrageOpportunityType> {
        let opportunity_id = self.generate_opportunity_id(buy_chain, sell_chain, asset, amount);

        let route = SwapRoute {
            source_chain: buy_chain,
            target_chain: sell_chain,
            source_asset: asset,
            target_asset: asset,
            amount_in: amount,
            amount_out: amount,
            hops: vec![buy_chain, sell_chain],
            gas_estimate: U256::from(500_000),
            price_impact: percentage_diff,
        };

        Ok(ArbitrageOpportunityType {
            id: opportunity_id,
            profit_usd: profit,
            confidence: 0.8, // 80% confidence
            routes: vec![route],
            deadline: sp_io::offchain::timestamp().unix_millis() + 60000, // 1 minute
            min_capital: amount,
            max_capital: amount.checked_mul(U256::from(10)).unwrap_or(amount),
        })
    }

    /// Generate opportunity ID
    fn generate_opportunity_id(
        &self,
        buy_chain: u64,
        sell_chain: u64,
        asset: H160,
        amount: U256,
    ) -> H256 {
        use sp_core::Hasher;
        use sp_runtime::traits::BlakeTwo256;

        let mut hasher = BlakeTwo256::default();
        hasher.hash(&buy_chain.to_le_bytes());
        hasher.hash(&sell_chain.to_le_bytes());
        hasher.hash(asset.as_bytes());
        hasher.hash(&amount.as_bytes());
        hasher.hash(&sp_io::offchain::timestamp().unix_millis().to_le_bytes());
        H256::from_slice(hasher.finish().as_ref())
    }

    /// Set minimum profit threshold
    pub fn set_min_profit(&mut self, min_profit_usd: U256) {
        self.min_profit_usd = min_profit_usd;
    }

    /// Set maximum slippage
    pub fn set_max_slippage(&mut self, max_slippage: f64) {
        self.max_slippage = max_slippage;
    }
}

/// Arbitrage executor for executing opportunities
#[derive(Debug, Clone)]
pub struct ArbitrageExecutor {
    /// Arbitrage detector
    detector: ArbitrageDetector,
    /// Execution history
    execution_history: Vec<ExecutionRecord>,
    /// Configuration
    config: PositionManagerConfig,
}

impl ArbitrageExecutor {
    /// Create a new arbitrage executor
    pub fn new(config: &PositionManagerConfig) -> Result<Self> {
        let detector = ArbitrageDetector::new(config)?;

        Ok(Self {
            detector,
            execution_history: Vec::new(),
            config: config.clone(),
        })
    }

    /// Start the arbitrage executor
    pub async fn start(&mut self) -> Result<()> {
        // Initialize price feeds
        // In a real implementation, this would connect to price oracles
        Ok(())
    }

    /// Stop the arbitrage executor
    pub async fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// Find opportunities
    pub async fn find_opportunities(&self) -> Result<Vec<ArbitrageOpportunityType>> {
        self.detector.find_opportunities().await
    }

    /// Execute an arbitrage opportunity
    pub async fn execute_opportunity(
        &mut self,
        opportunity: &ArbitrageOpportunityType,
    ) -> Result<ExecutionResult> {
        // Validate opportunity
        if opportunity.profit_usd < self.detector.min_profit_usd {
            return Err(PositionManagerError::InsufficientProfit(
                opportunity.profit_usd,
            ));
        }

        // Check if deadline has passed
        let current_time = sp_io::offchain::timestamp().unix_millis();
        if current_time > opportunity.deadline {
            return Err(PositionManagerError::OpportunityExpired(opportunity.id));
        }

        // Execute the arbitrage
        let execution_id = self.generate_execution_id(opportunity);
        let start_time = sp_io::offchain::timestamp().unix_millis();

        // Simulate execution (in a real implementation, this would execute the trades)
        let success = self.simulate_execution(opportunity).await?;
        let end_time = sp_io::offchain::timestamp().unix_millis();

        let result = ExecutionResult {
            execution_id,
            opportunity_id: opportunity.id,
            success,
            actual_profit: if success {
                opportunity.profit_usd
            } else {
                U256::zero()
            },
            gas_used: U256::from(500_000),
            execution_time_ms: end_time - start_time,
            error: if success {
                None
            } else {
                Some("Simulation failed".to_string())
            },
        };

        // Record execution
        self.execution_history.push(ExecutionRecord {
            execution_id,
            opportunity_id: opportunity.id,
            timestamp: start_time,
            result: result.clone(),
        });

        Ok(result)
    }

    /// Simulate execution
    async fn simulate_execution(&self, opportunity: &ArbitrageOpportunityType) -> Result<bool> {
        // In a real implementation, this would:
        // 1. Check liquidity on both chains
        // 2. Verify prices haven't changed
        // 3. Simulate the trades
        // 4. Calculate actual profit

        // For now, return true with 80% probability
        let random = sp_io::offchain::random_seed();
        Ok(random[0] % 5 != 0) // 80% success rate
    }

    /// Generate execution ID
    fn generate_execution_id(&self, opportunity: &ArbitrageOpportunityType) -> H256 {
        use sp_core::Hasher;
        use sp_runtime::traits::BlakeTwo256;

        let mut hasher = BlakeTwo256::default();
        hasher.hash(opportunity.id.as_bytes());
        hasher.hash(&sp_io::offchain::timestamp().unix_millis().to_le_bytes());
        hasher.hash(&opportunity.profit_usd.as_bytes());
        H256::from_slice(hasher.finish().as_ref())
    }

    /// Get execution history
    pub fn execution_history(&self) -> &[ExecutionRecord] {
        &self.execution_history
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.execution_history.is_empty() {
            return 0.0;
        }

        let successful = self
            .execution_history
            .iter()
            .filter(|r| r.result.success)
            .count();

        successful as f64 / self.execution_history.len() as f64
    }

    /// Get total profit
    pub fn total_profit(&self) -> U256 {
        self.execution_history
            .iter()
            .filter(|r| r.result.success)
            .map(|r| r.result.actual_profit)
            .fold(U256::zero(), |acc, x| acc.saturating_add(x))
    }
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: H256,
    pub opportunity_id: H256,
    pub success: bool,
    pub actual_profit: U256,
    pub gas_used: U256,
    pub execution_time_ms: u64,
    pub error: Option<String>,
}

/// Execution record for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub execution_id: H256,
    pub opportunity_id: H256,
    pub timestamp: u64,
    pub result: ExecutionResult,
}

/// Opportunity wrapper for external use
pub type Opportunity = ArbitrageOpportunityType;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbitrage_detector() {
        let config = PositionManagerConfig::default();
        let detector = ArbitrageDetector::new(&config).unwrap();

        assert_eq!(detector.supported_chains.len(), config.chain_configs.len());
    }

    #[test]
    fn test_arbitrage_executor() {
        let config = PositionManagerConfig::default();
        let executor = ArbitrageExecutor::new(&config).unwrap();

        assert_eq!(executor.success_rate(), 0.0);
        assert_eq!(executor.total_profit(), U256::zero());
    }
}
