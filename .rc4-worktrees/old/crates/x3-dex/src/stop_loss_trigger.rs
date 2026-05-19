/// Stop-Loss / Take-Profit Triggers — Price-based automatic order execution
/// Monitors price feeds and executes orders when thresholds are reached
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct StopLossTrigger {
    pub id: [u8; 32],
    pub user: [u8; 32],
    pub token_in: u128,
    pub token_out: u128,
    pub amount: u64,
    pub stop_price: u64,  // Trigger price (bps)
    pub trigger_type: u8, // 0=stop_loss, 1=take_profit
    pub status: u8,       // 0=active, 1=triggered, 2=cancelled
    pub created_at: u64,
    pub triggered_at: Option<u64>,
    pub execution_price: Option<u64>,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TakeProfitTrigger {
    pub id: [u8; 32],
    pub user: [u8; 32],
    pub token_in: u128,
    pub token_out: u128,
    pub amount: u64,
    pub profit_target_price: u64,
    pub profit_percentage: u32, // 100 = 1%
    pub status: u8,             // 0=active, 1=triggered, 2=cancelled
    pub created_at: u64,
    pub entry_price: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TrailingStopTrigger {
    pub id: [u8; 32],
    pub user: [u8; 32],
    pub token_in: u128,
    pub token_out: u128,
    pub amount: u64,
    pub trail_percentage: u32, // 100 = 1%, maintains distance from peak
    pub peak_price: u64,
    pub current_price: u64,
    pub status: u8,
    pub created_at: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct GridTradingConfig {
    pub id: [u8; 32],
    pub user: [u8; 32],
    pub token_in: u128,
    pub token_out: u128,
    pub grid_levels: u32,
    pub upper_price: u64,
    pub lower_price: u64,
    pub amount_per_grid: u64,
    pub active_orders: u32,
    pub status: u8,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TriggerExecution {
    pub trigger_id: [u8; 32],
    pub executed_at_price: u64,
    pub execution_amount: u64,
    pub execution_fee: u64,
    pub timestamp: u64,
    pub profit_realized: Option<i64>,
}

pub struct StopLossTakeProfitEngine;

impl StopLossTakeProfitEngine {
    const MIN_TRIGGER_AMOUNT: u64 = 1;
    const EXECUTION_FEE_BPS: u64 = 25; // 0.25%

    /// Create a stop-loss trigger (sell if price falls below threshold)
    pub fn create_stop_loss(
        user: [u8; 32],
        token_in: u128,
        token_out: u128,
        amount: u64,
        stop_price: u64,
        current_block: u64,
    ) -> Result<StopLossTrigger, &'static str> {
        if amount < Self::MIN_TRIGGER_AMOUNT {
            return Err("Amount too small");
        }

        if stop_price == 0 {
            return Err("Stop price cannot be zero");
        }

        let trigger = StopLossTrigger {
            id: Self::derive_trigger_id(user, token_in, token_out, stop_price, current_block),
            user,
            token_in,
            token_out,
            amount,
            stop_price,
            trigger_type: 0, // stop_loss
            status: 0,       // active
            created_at: current_block,
            triggered_at: None,
            execution_price: None,
        };

        Ok(trigger)
    }

    /// Create a take-profit trigger (sell if price rises above target)
    pub fn create_take_profit(
        user: [u8; 32],
        token_in: u128,
        token_out: u128,
        amount: u64,
        entry_price: u64,
        profit_percentage: u32,
        current_block: u64,
    ) -> Result<TakeProfitTrigger, &'static str> {
        if amount < Self::MIN_TRIGGER_AMOUNT {
            return Err("Amount too small");
        }

        if entry_price == 0 {
            return Err("Entry price cannot be zero");
        }

        if profit_percentage == 0 {
            return Err("Profit percentage must be > 0");
        }

        // Calculate target price
        let profit_factor = 10_000 + (profit_percentage as u64);
        let target_price = (entry_price as u128)
            .saturating_mul(profit_factor as u128)
            .saturating_div(10_000) as u64;

        let trigger = TakeProfitTrigger {
            id: Self::derive_trigger_id(user, token_in, token_out, target_price, current_block),
            user,
            token_in,
            token_out,
            amount,
            profit_target_price: target_price,
            profit_percentage,
            status: 0, // active
            created_at: current_block,
            entry_price,
        };

        Ok(trigger)
    }

    /// Create a trailing stop (follows price up, executes on down reversal)
    pub fn create_trailing_stop(
        user: [u8; 32],
        token_in: u128,
        token_out: u128,
        amount: u64,
        trail_percentage: u32,
        current_price: u64,
        current_block: u64,
    ) -> Result<TrailingStopTrigger, &'static str> {
        if amount < Self::MIN_TRIGGER_AMOUNT {
            return Err("Amount too small");
        }

        if current_price == 0 {
            return Err("Current price cannot be zero");
        }

        if trail_percentage == 0 {
            return Err("Trail percentage must be > 0");
        }

        let trigger = TrailingStopTrigger {
            id: Self::derive_trigger_id(user, token_in, token_out, current_price, current_block),
            user,
            token_in,
            token_out,
            amount,
            trail_percentage,
            peak_price: current_price,
            current_price,
            status: 0, // active
            created_at: current_block,
        };

        Ok(trigger)
    }

    /// Check if stop-loss should trigger
    pub fn check_stop_loss_trigger(
        trigger: &mut StopLossTrigger,
        current_price: u64,
        timestamp: u64,
    ) -> Result<Option<TriggerExecution>, &'static str> {
        if trigger.status != 0 {
            return Err("Trigger is not active");
        }

        if current_price <= trigger.stop_price {
            trigger.status = 1; // triggered
            trigger.triggered_at = Some(timestamp);
            trigger.execution_price = Some(current_price);

            let fee = Self::calculate_fee(trigger.amount);
            let execution = TriggerExecution {
                trigger_id: trigger.id,
                executed_at_price: current_price,
                execution_amount: trigger.amount,
                execution_fee: fee,
                timestamp,
                profit_realized: None,
            };

            return Ok(Some(execution));
        }

        Ok(None)
    }

    /// Check if take-profit should trigger
    pub fn check_take_profit_trigger(
        trigger: &mut TakeProfitTrigger,
        current_price: u64,
        timestamp: u64,
    ) -> Result<Option<TriggerExecution>, &'static str> {
        if trigger.status != 0 {
            return Err("Trigger is not active");
        }

        if current_price >= trigger.profit_target_price {
            trigger.status = 1; // triggered

            let profit = Self::calculate_profit(trigger.entry_price, current_price, trigger.amount);

            let fee = Self::calculate_fee(trigger.amount);
            let execution = TriggerExecution {
                trigger_id: trigger.id,
                executed_at_price: current_price,
                execution_amount: trigger.amount,
                execution_fee: fee,
                timestamp,
                profit_realized: Some(profit),
            };

            return Ok(Some(execution));
        }

        Ok(None)
    }

    /// Update trailing stop and check if triggered
    pub fn update_trailing_stop(
        trigger: &mut TrailingStopTrigger,
        current_price: u64,
        timestamp: u64,
    ) -> Result<Option<TriggerExecution>, &'static str> {
        if trigger.status != 0 {
            return Err("Trigger is not active");
        }

        // Update peak price if current price is higher
        if current_price > trigger.peak_price {
            trigger.peak_price = current_price;
            trigger.current_price = current_price;
            return Ok(None);
        }

        trigger.current_price = current_price;

        // Calculate stop price based on trail
        let trail_factor = 10_000 - (trigger.trail_percentage as u64);
        let stop_price = (trigger.peak_price as u128)
            .saturating_mul(trail_factor as u128)
            .saturating_div(10_000) as u64;

        // Check if triggered
        if current_price <= stop_price {
            trigger.status = 1; // triggered

            let fee = Self::calculate_fee(trigger.amount);
            let execution = TriggerExecution {
                trigger_id: trigger.id,
                executed_at_price: current_price,
                execution_amount: trigger.amount,
                execution_fee: fee,
                timestamp,
                profit_realized: None,
            };

            return Ok(Some(execution));
        }

        Ok(None)
    }

    /// Create grid trading configuration
    #[allow(clippy::too_many_arguments)]
    pub fn create_grid_config(
        user: [u8; 32],
        token_in: u128,
        token_out: u128,
        grid_levels: u32,
        lower_price: u64,
        upper_price: u64,
        total_amount: u64,
        current_block: u64,
    ) -> Result<GridTradingConfig, &'static str> {
        if !(2..=100).contains(&grid_levels) {
            return Err("Grid levels must be between 2 and 100");
        }

        if lower_price >= upper_price {
            return Err("Lower price must be < upper price");
        }

        let amount_per_grid = total_amount / (grid_levels as u64);

        let config = GridTradingConfig {
            id: Self::derive_trigger_id(user, token_in, token_out, lower_price, current_block),
            user,
            token_in,
            token_out,
            grid_levels,
            upper_price,
            lower_price,
            amount_per_grid,
            active_orders: 0,
            status: 0, // active
        };

        Ok(config)
    }

    /// Cancel a trigger
    pub fn cancel_stop_loss(trigger: &mut StopLossTrigger) -> Result<bool, &'static str> {
        if trigger.status == 1 {
            return Err("Cannot cancel already triggered order");
        }

        trigger.status = 2; // cancelled
        Ok(true)
    }

    /// Cancel take-profit trigger
    pub fn cancel_take_profit(trigger: &mut TakeProfitTrigger) -> Result<bool, &'static str> {
        if trigger.status == 1 {
            return Err("Cannot cancel already triggered order");
        }

        trigger.status = 2; // cancelled
        Ok(true)
    }

    /// Calculate execution fee
    pub fn calculate_fee(amount: u64) -> u64 {
        (amount * Self::EXECUTION_FEE_BPS) / 10_000
    }

    /// Calculate realized profit
    fn calculate_profit(entry: u64, exit: u64, amount: u64) -> i64 {
        if exit > entry {
            let gain = ((exit as i128 - entry as i128) * amount as i128) / entry as i128;
            gain as i64
        } else {
            let loss = ((entry as i128 - exit as i128) * amount as i128) / entry as i128;
            -(loss as i64)
        }
    }

    /// Get grid price levels
    pub fn get_grid_prices(config: &GridTradingConfig) -> Vec<u64> {
        let mut prices = Vec::new();
        let price_range = config.upper_price - config.lower_price;
        let step = price_range / (config.grid_levels as u64);

        for i in 0..=config.grid_levels {
            let price = config.lower_price + (step * i as u64);
            prices.push(price);
        }

        prices
    }

    /// Derive deterministic trigger ID
    fn derive_trigger_id(
        user: [u8; 32],
        token_in: u128,
        _token_out: u128,
        price: u64,
        nonce: u64,
    ) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in user.iter().enumerate().take(16) {
            id[i] = *byte;
        }
        let price_bytes = price.to_le_bytes();
        for (i, byte) in price_bytes.iter().enumerate() {
            id[i + 8] = *byte;
        }
        let token_in_bytes = token_in.to_le_bytes();
        for (i, byte) in token_in_bytes.iter().take(8).enumerate() {
            id[i + 16] = *byte;
        }
        let nonce_bytes = nonce.to_le_bytes();
        for (i, byte) in nonce_bytes.iter().take(8).enumerate() {
            id[i + 24] = *byte;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_stop_loss() {
        let trigger =
            StopLossTakeProfitEngine::create_stop_loss([1; 32], 1, 2, 1_000_000, 4_500, 100)
                .unwrap();

        assert_eq!(trigger.user, [1; 32]);
        assert_eq!(trigger.stop_price, 4_500);
        assert_eq!(trigger.trigger_type, 0);
    }

    #[test]
    fn test_create_take_profit() {
        let trigger = StopLossTakeProfitEngine::create_take_profit(
            [1; 32], 1, 2, 1_000_000, 5_000, 500, // 5% profit
            100,
        )
        .unwrap();

        assert_eq!(trigger.user, [1; 32]);
        assert_eq!(trigger.entry_price, 5_000);
        // Target = 5000 * 1.05 = 5250
        assert_eq!(trigger.profit_target_price, 5_250);
    }

    #[test]
    fn test_create_trailing_stop() {
        let trigger = StopLossTakeProfitEngine::create_trailing_stop(
            [1; 32], 1, 2, 1_000_000, 200, // 2% trail
            5_000, 100,
        )
        .unwrap();

        assert_eq!(trigger.peak_price, 5_000);
        assert_eq!(trigger.trail_percentage, 200);
    }

    #[test]
    fn test_check_stop_loss_trigger() {
        let mut trigger =
            StopLossTakeProfitEngine::create_stop_loss([1; 32], 1, 2, 1_000_000, 4_500, 100)
                .unwrap();

        let result =
            StopLossTakeProfitEngine::check_stop_loss_trigger(&mut trigger, 4_200, 200).unwrap();

        assert!(result.is_some());
        assert_eq!(trigger.status, 1); // triggered
    }

    #[test]
    fn test_check_stop_loss_not_triggered() {
        let mut trigger =
            StopLossTakeProfitEngine::create_stop_loss([1; 32], 1, 2, 1_000_000, 4_500, 100)
                .unwrap();

        let result =
            StopLossTakeProfitEngine::check_stop_loss_trigger(&mut trigger, 5_000, 200).unwrap();

        assert!(result.is_none());
        assert_eq!(trigger.status, 0); // still active
    }

    #[test]
    fn test_check_take_profit_trigger() {
        let mut trigger =
            StopLossTakeProfitEngine::create_take_profit([1; 32], 1, 2, 1_000_000, 5_000, 500, 100)
                .unwrap();

        let result =
            StopLossTakeProfitEngine::check_take_profit_trigger(&mut trigger, 5_300, 200).unwrap();

        assert!(result.is_some());
        assert_eq!(trigger.status, 1);
        let exec = result.unwrap();
        assert!(exec.profit_realized.is_some());
    }

    #[test]
    fn test_update_trailing_stop_peak() {
        let mut trigger = StopLossTakeProfitEngine::create_trailing_stop(
            [1; 32], 1, 2, 1_000_000, 200, 5_000, 100,
        )
        .unwrap();

        let result =
            StopLossTakeProfitEngine::update_trailing_stop(&mut trigger, 5_500, 200).unwrap();

        assert!(result.is_none());
        assert_eq!(trigger.peak_price, 5_500);
    }

    #[test]
    fn test_update_trailing_stop_trigger() {
        let mut trigger = StopLossTakeProfitEngine::create_trailing_stop(
            [1; 32], 1, 2, 1_000_000, 200, 5_000, 100,
        )
        .unwrap();

        // Peak goes to 6000
        StopLossTakeProfitEngine::update_trailing_stop(&mut trigger, 6_000, 200).unwrap();

        // Price falls to 5800, below 2% trail of 6000
        let result =
            StopLossTakeProfitEngine::update_trailing_stop(&mut trigger, 5_799, 201).unwrap();

        assert!(result.is_some());
        assert_eq!(trigger.status, 1);
    }

    #[test]
    fn test_create_grid_config() {
        let config = StopLossTakeProfitEngine::create_grid_config(
            [1; 32], 1, 2, 10, 4_000, 6_000, 10_000_000, 100,
        )
        .unwrap();

        assert_eq!(config.grid_levels, 10);
        assert_eq!(config.amount_per_grid, 1_000_000);
    }

    #[test]
    fn test_get_grid_prices() {
        let config = GridTradingConfig {
            id: [0; 32],
            user: [1; 32],
            token_in: 1,
            token_out: 2,
            grid_levels: 5,
            upper_price: 5_000,
            lower_price: 4_000,
            amount_per_grid: 1_000_000,
            active_orders: 0,
            status: 0,
        };

        let prices = StopLossTakeProfitEngine::get_grid_prices(&config);
        assert_eq!(prices.len(), 6);
        assert_eq!(prices[0], 4_000);
        assert_eq!(prices[5], 5_000);
    }

    #[test]
    fn test_cancel_stop_loss() {
        let mut trigger =
            StopLossTakeProfitEngine::create_stop_loss([1; 32], 1, 2, 1_000_000, 4_500, 100)
                .unwrap();

        StopLossTakeProfitEngine::cancel_stop_loss(&mut trigger).unwrap();
        assert_eq!(trigger.status, 2);
    }

    #[test]
    fn test_calculate_fee() {
        let fee = StopLossTakeProfitEngine::calculate_fee(1_000_000);
        assert_eq!(fee, 2_500); // 0.25%
    }
}
