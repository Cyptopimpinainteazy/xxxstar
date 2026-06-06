/// Arbitrage Bot Event System — WebSocket-based event streaming for MEV opportunities
/// Enables bots to subscribe to detected arbitrage opportunities and capture discrepancies
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ArbOpportunity {
    pub opportunity_id: [u8; 32],
    pub pool_a: [u8; 32],
    pub pool_b: [u8; 32],
    pub token_in: [u8; 32],
    pub token_out: [u8; 32],
    pub price_a: u128,
    pub price_b: u128,
    pub spread_bps: u32,
    pub profit_amount: u128,
    pub profit_token: [u8; 32],
    pub min_input: u128,
    pub max_input: u128,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct BotSubscription {
    pub subscription_id: [u8; 32],
    pub bot_id: [u8; 32],
    pub min_spread_bps: u32,
    pub min_profit: u128,
    pub token_whitelist: Vec<[u8; 32]>,
    pub is_active: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ExecutedArb {
    pub execution_id: [u8; 32],
    pub opportunity_id: [u8; 32],
    pub executor: [u8; 32],
    pub input_amount: u128,
    pub output_amount: u128,
    pub actual_profit: u128,
    pub gas_cost: u128,
    pub net_profit: u128,
    pub tx_hash: [u8; 32],
    pub timestamp: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct BotPerformance {
    pub bot_id: [u8; 32],
    pub total_executions: u64,
    pub total_profit: u128,
    pub average_profit_per_execution: u128,
    pub win_rate_bps: u32,
    pub total_gas_spent: u128,
    pub last_execution: u64,
}

pub struct ArbBotEventSystem;

impl ArbBotEventSystem {
    /// Detect arbitrage opportunity between two pools
    #[allow(clippy::too_many_arguments)]
    pub fn detect_opportunity(
        pool_a_id: [u8; 32],
        pool_b_id: [u8; 32],
        token_in: [u8; 32],
        token_out: [u8; 32],
        price_a: u128,
        price_b: u128,
        min_profit_tokens: u128,
        current_timestamp: u64,
        opportunity_ttl_seconds: u64,
    ) -> Result<ArbOpportunity, &'static str> {
        if price_a == 0 || price_b == 0 {
            return Err("Prices cannot be zero");
        }
        if pool_a_id == pool_b_id {
            return Err("Cannot arb between same pool");
        }

        // Calculate spread in basis points (100 bps = 1%)
        let (spread, profit_token, min_input, max_input) = if price_a > price_b {
            let spread = ((price_a - price_b) as f64) / (price_b as f64) * 10000.0;
            (spread as u32, token_out, 0, u128::MAX)
        } else {
            let spread = ((price_b - price_a) as f64) / (price_a as f64) * 10000.0;
            (spread as u32, token_in, 0, u128::MAX)
        };

        if spread < 10 {
            return Err("Spread too small (minimum 10 bps)");
        }

        let profit_amount = min_profit_tokens;

        Ok(ArbOpportunity {
            opportunity_id: Self::generate_opportunity_id(&pool_a_id, &pool_b_id),
            pool_a: pool_a_id,
            pool_b: pool_b_id,
            token_in,
            token_out,
            price_a,
            price_b,
            spread_bps: spread,
            profit_amount,
            profit_token,
            min_input,
            max_input,
            created_at: current_timestamp,
            expires_at: current_timestamp.saturating_add(opportunity_ttl_seconds),
        })
    }

    /// Create bot subscription with filters
    pub fn create_subscription(
        bot_id: [u8; 32],
        min_spread_bps: u32,
        min_profit: u128,
        token_whitelist: Vec<[u8; 32]>,
    ) -> Result<BotSubscription, &'static str> {
        if bot_id == [0; 32] {
            return Err("Invalid bot ID");
        }
        if min_spread_bps == 0 {
            return Err("Minimum spread must be positive");
        }

        let subscription_id = Self::generate_subscription_id(&bot_id);

        Ok(BotSubscription {
            subscription_id,
            bot_id,
            min_spread_bps,
            min_profit,
            token_whitelist,
            is_active: true,
        })
    }

    /// Check if opportunity matches bot's subscription filters
    pub fn matches_subscription(
        opportunity: &ArbOpportunity,
        subscription: &BotSubscription,
        current_timestamp: u64,
    ) -> Result<bool, &'static str> {
        if !subscription.is_active {
            return Err("Subscription is not active");
        }

        // Check expiry
        if current_timestamp > opportunity.expires_at {
            return Err("Opportunity has expired");
        }

        // Check spread
        if opportunity.spread_bps < subscription.min_spread_bps {
            return Ok(false);
        }

        // Check profit
        if opportunity.profit_amount < subscription.min_profit {
            return Ok(false);
        }

        // Check whitelist (if specified)
        if !subscription.token_whitelist.is_empty() {
            let token_in_whitelisted = subscription.token_whitelist.contains(&opportunity.token_in);
            let token_out_whitelisted = subscription
                .token_whitelist
                .contains(&opportunity.token_out);

            if !token_in_whitelisted && !token_out_whitelisted {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Record executed arbitrage trade
    pub fn record_execution(
        opportunity_id: [u8; 32],
        executor: [u8; 32],
        input_amount: u128,
        output_amount: u128,
        gas_cost: u128,
        tx_hash: [u8; 32],
        current_timestamp: u64,
    ) -> Result<ExecutedArb, &'static str> {
        if executor == [0; 32] {
            return Err("Invalid executor");
        }
        if input_amount == 0 || output_amount == 0 {
            return Err("Amounts must be positive");
        }

        let actual_profit = output_amount.saturating_sub(input_amount);
        let net_profit = actual_profit.saturating_sub(gas_cost);

        let execution_id = Self::generate_execution_id(&opportunity_id, &executor);

        Ok(ExecutedArb {
            execution_id,
            opportunity_id,
            executor,
            input_amount,
            output_amount,
            actual_profit,
            gas_cost,
            net_profit,
            tx_hash,
            timestamp: current_timestamp,
        })
    }

    /// Update bot performance metrics after execution
    pub fn update_performance(
        performance: &mut BotPerformance,
        execution: &ExecutedArb,
    ) -> Result<(), &'static str> {
        performance.total_executions = performance.total_executions.saturating_add(1);
        performance.total_profit = performance
            .total_profit
            .saturating_add(execution.net_profit);
        performance.average_profit_per_execution = performance.total_profit
            / if performance.total_executions > 0 {
                performance.total_executions as u128
            } else {
                1u128
            };
        performance.total_gas_spent = performance
            .total_gas_spent
            .saturating_add(execution.gas_cost);
        performance.last_execution = execution.timestamp;

        // Update win rate (profitable executions / total)
        if execution.net_profit > 0 {
            let new_win_rate = (performance.total_executions as f64
                * (performance.win_rate_bps as f64 / 10000.0))
                + 1.0;
            performance.win_rate_bps =
                ((new_win_rate / performance.total_executions as f64) * 10000.0) as u32;
        }

        Ok(())
    }

    /// Get bot performance summary
    pub fn get_performance_summary(performance: &BotPerformance) -> (u64, u128, u128, u32) {
        (
            performance.total_executions,
            performance.total_profit,
            performance.average_profit_per_execution,
            performance.win_rate_bps,
        )
    }

    /// Check if opportunity is still valid (not expired)
    pub fn is_opportunity_valid(opportunity: &ArbOpportunity, current_timestamp: u64) -> bool {
        current_timestamp < opportunity.expires_at
    }

    /// Calculate minimum input amount for profitable execution
    pub fn calculate_min_input(
        opportunity: &ArbOpportunity,
        max_gas_cost: u128,
    ) -> Result<u128, &'static str> {
        if opportunity.profit_amount <= max_gas_cost {
            return Err("Profit too small to cover gas");
        }

        // Min input = (max_gas_cost / spread%) + slippage buffer (5%)
        let min_profitable = max_gas_cost
            .saturating_mul(10000u128)
            .saturating_div(opportunity.spread_bps as u128);

        let min_with_buffer = min_profitable
            .saturating_mul(105u128)
            .saturating_div(100u128);

        Ok(min_with_buffer)
    }

    /// Enable/disable subscription
    pub fn set_subscription_active(
        subscription: &mut BotSubscription,
        active: bool,
    ) -> Result<(), &'static str> {
        subscription.is_active = active;
        Ok(())
    }

    /// Add token to whitelist
    pub fn add_whitelisted_token(
        subscription: &mut BotSubscription,
        token: [u8; 32],
    ) -> Result<(), &'static str> {
        if subscription.token_whitelist.contains(&token) {
            return Err("Token already whitelisted");
        }
        subscription.token_whitelist.push(token);
        Ok(())
    }

    /// Generate deterministic opportunity ID
    fn generate_opportunity_id(pool_a: &[u8; 32], pool_b: &[u8; 32]) -> [u8; 32] {
        let mut id = [0u8; 32];
        for i in 0..32 {
            id[i] = pool_a[i] ^ pool_b[i];
        }
        id
    }

    /// Generate deterministic subscription ID
    fn generate_subscription_id(bot_id: &[u8; 32]) -> [u8; 32] {
        let mut id = [0u8; 32];
        let mut hash = 0u64;
        for byte in bot_id {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        id[0..8].copy_from_slice(&hash.to_le_bytes());
        id
    }

    /// Generate deterministic execution ID
    fn generate_execution_id(opportunity_id: &[u8; 32], executor: &[u8; 32]) -> [u8; 32] {
        let mut id = [0u8; 32];
        for i in 0..32 {
            id[i] = opportunity_id[i].wrapping_add(executor[i]);
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_opportunity() {
        let opp = ArbBotEventSystem::detect_opportunity(
            [1; 32], [2; 32], [3; 32], [4; 32], 1000000000, 1100000000, // 10% spread
            100000, 1000, 3600,
        )
        .unwrap();

        assert!(opp.spread_bps >= 1000); // At least 10%
        assert_eq!(opp.profit_amount, 100000);
    }

    #[test]
    fn test_detect_opportunity_zero_price() {
        let result = ArbBotEventSystem::detect_opportunity(
            [1; 32], [2; 32], [3; 32], [4; 32], 0, 1000000000, 100000, 1000, 3600,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_create_subscription() {
        let sub =
            ArbBotEventSystem::create_subscription([1; 32], 100, 50000, vec![[2; 32]]).unwrap();

        assert_eq!(sub.bot_id, [1; 32]);
        assert!(sub.is_active);
    }

    #[test]
    fn test_matches_subscription_valid() {
        let opp = ArbBotEventSystem::detect_opportunity(
            [1; 32], [2; 32], [3; 32], [4; 32], 1000000000, 1100000000, 100000, 1000, 3600,
        )
        .unwrap();

        let sub = ArbBotEventSystem::create_subscription(
            [5; 32],
            100, // 1% min spread
            50000,
            vec![[3; 32], [4; 32]],
        )
        .unwrap();

        assert!(ArbBotEventSystem::matches_subscription(&opp, &sub, 1000).unwrap());
    }

    #[test]
    fn test_matches_subscription_spread_too_small() {
        let opp = ArbBotEventSystem::detect_opportunity(
            [1; 32], [2; 32], [3; 32], [4; 32], 1000000000, 1001000000, // Only 0.1% spread
            100000, 1000, 3600,
        )
        .unwrap();

        let sub = ArbBotEventSystem::create_subscription(
            [5; 32],
            1000, // Require 10% spread
            50000,
            vec![],
        )
        .unwrap();

        assert!(!ArbBotEventSystem::matches_subscription(&opp, &sub, 1000).unwrap());
    }

    #[test]
    fn test_record_execution() {
        let exec = ArbBotEventSystem::record_execution(
            [1; 32], [2; 32], 1000000, 1090000, 5000, [3; 32], 1000,
        )
        .unwrap();

        assert_eq!(exec.executor, [2; 32]);
        assert_eq!(exec.actual_profit, 90000);
        assert_eq!(exec.net_profit, 85000); // 90000 - 5000 gas
    }

    #[test]
    fn test_update_performance() {
        let mut perf = BotPerformance {
            bot_id: [1; 32],
            total_executions: 0,
            total_profit: 0,
            average_profit_per_execution: 0,
            win_rate_bps: 0,
            total_gas_spent: 0,
            last_execution: 0,
        };

        let exec = ArbBotEventSystem::record_execution(
            [1; 32], [2; 32], 1000000, 1090000, 5000, [3; 32], 1000,
        )
        .unwrap();

        ArbBotEventSystem::update_performance(&mut perf, &exec).unwrap();

        assert_eq!(perf.total_executions, 1);
        assert_eq!(perf.total_profit, 85000);
    }

    #[test]
    fn test_is_opportunity_valid() {
        let opp = ArbBotEventSystem::detect_opportunity(
            [1; 32], [2; 32], [3; 32], [4; 32], 1000000000, 1100000000, 100000, 1000, 3600,
        )
        .unwrap();

        assert!(ArbBotEventSystem::is_opportunity_valid(&opp, 1000));
        assert!(!ArbBotEventSystem::is_opportunity_valid(&opp, 5000));
    }

    #[test]
    fn test_calculate_min_input() {
        let opp = ArbBotEventSystem::detect_opportunity(
            [1; 32], [2; 32], [3; 32], [4; 32], 1000000000, 1100000000, 100000, 1000, 3600,
        )
        .unwrap();

        let min_input = ArbBotEventSystem::calculate_min_input(&opp, 5000).unwrap();
        assert!(min_input > 0);
    }

    #[test]
    fn test_set_subscription_active() {
        let mut sub = ArbBotEventSystem::create_subscription([1; 32], 100, 50000, vec![]).unwrap();

        ArbBotEventSystem::set_subscription_active(&mut sub, false).unwrap();
        assert!(!sub.is_active);
    }

    #[test]
    fn test_add_whitelisted_token() {
        let mut sub =
            ArbBotEventSystem::create_subscription([1; 32], 100, 50000, vec![[2; 32]]).unwrap();

        ArbBotEventSystem::add_whitelisted_token(&mut sub, [3; 32]).unwrap();
        assert_eq!(sub.token_whitelist.len(), 2);
    }

    #[test]
    fn test_performance_summary() {
        let perf = BotPerformance {
            bot_id: [1; 32],
            total_executions: 100,
            total_profit: 5000000,
            average_profit_per_execution: 50000,
            win_rate_bps: 7500,
            total_gas_spent: 100000,
            last_execution: 2000,
        };

        let (execs, profit, _avg, wr) = ArbBotEventSystem::get_performance_summary(&perf);
        assert_eq!(execs, 100);
        assert_eq!(profit, 5000000);
        assert_eq!(wr, 7500);
    }
}
