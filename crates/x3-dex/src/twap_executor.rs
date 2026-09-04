/// TWAP Executor — Time-Weighted Average Price order execution
/// Splits large orders across time to minimize price impact
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TWAPOrder {
    pub id: [u8; 32],
    pub user: [u8; 32],
    pub token_in: u128,
    pub token_out: u128,
    pub total_amount: u64,
    pub time_window_blocks: u64, // How long to spread execution
    pub slice_count: u32,        // Number of slices
    pub executed_slices: u32,
    pub total_executed: u64,
    pub start_block: u64,
    pub status: u8, // 0=active, 1=completed, 2=cancelled
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TWAPSliceExecution {
    pub slice_index: u32,
    pub slice_amount: u64,
    pub execution_price: u64,
    pub execution_fee: u64,
    pub executed_block: u64,
    pub timestamp: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TWAPSlice {
    pub index: u32,
    pub amount: u64,
    pub target_block: u64,
    pub is_executed: bool,
    pub execution_price: Option<u64>,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TWAPStatistics {
    pub order_id: [u8; 32],
    pub total_amount: u64,
    pub average_price: u64,
    pub total_fees: u64,
    pub price_improvement: i64, // vs market price at order creation
    pub execution_time: u64,
}

pub struct TWAPExecutor;

impl TWAPExecutor {
    const MIN_SLICE_SIZE: u64 = 1;
    const MAX_SLICES: u32 = 1_000;
    const MIN_TIME_WINDOW: u64 = 1; // 1 block minimum
    const MAX_TIME_WINDOW: u64 = 1_036_800; // ~12 days at 1 block/sec
    const EXECUTION_FEE_BPS: u64 = 25; // 0.25%

    /// Create a TWAP order
    pub fn create_twap_order(
        user: [u8; 32],
        token_in: u128,
        token_out: u128,
        total_amount: u64,
        time_window_blocks: u64,
        slice_count: u32,
        current_block: u64,
    ) -> Result<TWAPOrder, &'static str> {
        if total_amount == 0 {
            return Err("Amount cannot be zero");
        }

        if slice_count == 0 || slice_count > Self::MAX_SLICES {
            return Err("Slice count out of range");
        }

        if !(Self::MIN_TIME_WINDOW..=Self::MAX_TIME_WINDOW).contains(&time_window_blocks) {
            return Err("Time window out of range");
        }

        let slice_size = total_amount / (slice_count as u64);
        if slice_size < Self::MIN_SLICE_SIZE {
            return Err("Slice size too small");
        }

        let order = TWAPOrder {
            id: Self::derive_order_id(user, token_in, token_out, slice_count, current_block),
            user,
            token_in,
            token_out,
            total_amount,
            time_window_blocks,
            slice_count,
            executed_slices: 0,
            total_executed: 0,
            start_block: current_block,
            status: 0, // active
        };

        Ok(order)
    }

    /// Get the next slice to execute
    pub fn get_next_slice(
        order: &TWAPOrder,
        current_block: u64,
    ) -> Result<Option<TWAPSlice>, &'static str> {
        if order.status != 0 {
            return Err("Order is not active");
        }

        if order.executed_slices >= order.slice_count {
            return Err("All slices already executed");
        }

        let elapsed = current_block - order.start_block;
        let time_per_slice = order.time_window_blocks / (order.slice_count as u64);
        let slices_due =
            sp_std::cmp::min(((elapsed / time_per_slice) + 1) as u32, order.slice_count);

        if slices_due <= order.executed_slices {
            return Ok(None); // Not yet time for next slice
        }

        let next_index = order.executed_slices;
        let slice_amount = if next_index == order.slice_count - 1 {
            // Last slice: execute remaining
            order.total_amount - order.total_executed
        } else {
            order.total_amount / (order.slice_count as u64)
        };

        let target_block = order.start_block + (time_per_slice * (next_index + 1) as u64);

        let slice = TWAPSlice {
            index: next_index,
            amount: slice_amount,
            target_block,
            is_executed: false,
            execution_price: None,
        };

        Ok(Some(slice))
    }

    /// Execute a TWAP slice
    pub fn execute_slice(
        order: &mut TWAPOrder,
        slice: &mut TWAPSlice,
        execution_price: u64,
        current_block: u64,
    ) -> Result<TWAPSliceExecution, &'static str> {
        if order.status != 0 {
            return Err("Order is not active");
        }

        if slice.is_executed {
            return Err("Slice already executed");
        }

        let fee = Self::calculate_fee(slice.amount);

        slice.is_executed = true;
        slice.execution_price = Some(execution_price);
        order.executed_slices += 1;
        order.total_executed += slice.amount;

        if order.executed_slices >= order.slice_count {
            order.status = 1; // completed
        }

        let execution = TWAPSliceExecution {
            slice_index: slice.index,
            slice_amount: slice.amount,
            execution_price,
            execution_fee: fee,
            executed_block: current_block,
            timestamp: current_block,
        };

        Ok(execution)
    }

    /// Cancel a TWAP order
    pub fn cancel_twap_order(order: &mut TWAPOrder) -> Result<u64, &'static str> {
        if order.status == 1 {
            return Err("Cannot cancel completed order");
        }

        let remaining = order.total_amount - order.total_executed;
        order.status = 2; // cancelled

        Ok(remaining)
    }

    /// Calculate TWAP statistics
    pub fn calculate_statistics(
        order: &TWAPOrder,
        executions: &[TWAPSliceExecution],
        market_price_at_creation: u64,
    ) -> Result<TWAPStatistics, &'static str> {
        if executions.is_empty() {
            return Err("No executions to calculate");
        }

        let mut total_price_volume: u128 = 0;
        let mut total_volume: u64 = 0;
        let mut total_fees: u64 = 0;

        for exec in executions {
            total_price_volume = total_price_volume.saturating_add(
                (exec.execution_price as u128).saturating_mul(exec.slice_amount as u128),
            );
            total_volume = total_volume.saturating_add(exec.slice_amount);
            total_fees = total_fees.saturating_add(exec.execution_fee);
        }

        let avg_price = if total_volume > 0 {
            (total_price_volume / total_volume as u128) as u64
        } else {
            0
        };

        let price_improvement = (avg_price as i64) - (market_price_at_creation as i64);

        let execution_time = if executions.len() > 1 {
            executions.last().unwrap().executed_block - executions.first().unwrap().executed_block
        } else {
            0
        };

        Ok(TWAPStatistics {
            order_id: order.id,
            total_amount: order.total_amount,
            average_price: avg_price,
            total_fees,
            price_improvement,
            execution_time,
        })
    }

    /// Get execution schedule (when each slice should execute)
    pub fn get_execution_schedule(order: &TWAPOrder) -> Vec<(u32, u64)> {
        let mut schedule = Vec::new();
        let time_per_slice = order.time_window_blocks / (order.slice_count as u64);

        for i in 0..order.slice_count {
            let target_block = order.start_block + (time_per_slice * (i + 1) as u64);
            schedule.push((i, target_block));
        }

        schedule
    }

    /// Estimate price impact across slices
    pub fn estimate_price_impact(
        base_price: u64,
        total_volume: u64,
        slice_count: u32,
        _liquidity_depth: u64,
    ) -> Result<Vec<u64>, &'static str> {
        if slice_count == 0 {
            return Err("Slice count must be > 0");
        }

        let _slice_volume = total_volume / (slice_count as u64);
        let mut prices = Vec::new();

        for i in 0..slice_count {
            // Simple linear impact model: price = base * (1 + volume/liquidity * i/slices)
            let impact_factor = 10_000 + (((i as u64) * 100) / slice_count as u64);
            let price = (base_price as u128)
                .saturating_mul(impact_factor as u128)
                .saturating_div(10_000) as u64;
            prices.push(price);
        }

        Ok(prices)
    }

    /// Check if order should be auto-cancelled (market conditions changed drastically)
    pub fn should_cancel_due_to_volatility(
        current_price: u64,
        initial_price: u64,
        max_slippage_bps: u64,
    ) -> bool {
        let price_diff = current_price.abs_diff(initial_price);

        let slippage = (price_diff * 10_000) / initial_price;
        slippage > max_slippage_bps
    }

    /// Calculate execution fee
    fn calculate_fee(amount: u64) -> u64 {
        (amount * Self::EXECUTION_FEE_BPS) / 10_000
    }

    /// Derive deterministic order ID
    fn derive_order_id(
        user: [u8; 32],
        _token_in: u128,
        _token_out: u128,
        slices: u32,
        nonce: u64,
    ) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in user.iter().enumerate().take(16) {
            id[i] = *byte;
        }
        let slices_bytes = (slices as u64).to_le_bytes();
        for (i, byte) in slices_bytes.iter().enumerate() {
            id[i + 8] = *byte;
        }
        let nonce_bytes = nonce.to_le_bytes();
        for (i, byte) in nonce_bytes.iter().enumerate().take(8) {
            id[i + 16] = *byte;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_twap_order() {
        let order = TWAPExecutor::create_twap_order([1; 32], 1, 2, 10_000_000, 100, 10, 0).unwrap();

        assert_eq!(order.slice_count, 10);
        assert_eq!(order.total_amount, 10_000_000);
        assert_eq!(order.status, 0);
    }

    #[test]
    fn test_create_twap_invalid_amount() {
        let result = TWAPExecutor::create_twap_order([1; 32], 1, 2, 0, 100, 10, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_next_slice() {
        let order = TWAPExecutor::create_twap_order([1; 32], 1, 2, 10_000_000, 100, 10, 0).unwrap();

        let slice = TWAPExecutor::get_next_slice(&order, 0).unwrap();
        assert!(slice.is_some());
        assert_eq!(slice.unwrap().index, 0);
    }

    #[test]
    fn test_execute_slice() {
        let mut order =
            TWAPExecutor::create_twap_order([1; 32], 1, 2, 10_000_000, 100, 10, 0).unwrap();

        let mut slice = TWAPSlice {
            index: 0,
            amount: 1_000_000,
            target_block: 10,
            is_executed: false,
            execution_price: None,
        };

        let exec = TWAPExecutor::execute_slice(&mut order, &mut slice, 5_000, 10).unwrap();

        assert_eq!(exec.execution_price, 5_000);
        assert_eq!(order.executed_slices, 1);
        assert_eq!(order.total_executed, 1_000_000);
    }

    #[test]
    fn test_cancel_twap_order() {
        let mut order =
            TWAPExecutor::create_twap_order([1; 32], 1, 2, 10_000_000, 100, 10, 0).unwrap();

        let remaining = TWAPExecutor::cancel_twap_order(&mut order).unwrap();

        assert_eq!(remaining, 10_000_000);
        assert_eq!(order.status, 2);
    }

    #[test]
    fn test_get_execution_schedule() {
        let order = TWAPExecutor::create_twap_order([1; 32], 1, 2, 10_000_000, 100, 10, 0).unwrap();

        let schedule = TWAPExecutor::get_execution_schedule(&order);

        assert_eq!(schedule.len(), 10);
        assert_eq!(schedule[0].1, 10); // First slice at block 10
        assert_eq!(schedule[9].1, 100); // Last slice at block 100
    }

    #[test]
    fn test_calculate_statistics() {
        let order = TWAPOrder {
            id: [0; 32],
            user: [1; 32],
            token_in: 1,
            token_out: 2,
            total_amount: 10_000_000,
            time_window_blocks: 100,
            slice_count: 10,
            executed_slices: 2,
            total_executed: 2_000_000,
            start_block: 0,
            status: 0,
        };

        let executions = vec![
            TWAPSliceExecution {
                slice_index: 0,
                slice_amount: 1_000_000,
                execution_price: 5_000,
                execution_fee: 2_500,
                executed_block: 10,
                timestamp: 10,
            },
            TWAPSliceExecution {
                slice_index: 1,
                slice_amount: 1_000_000,
                execution_price: 5_100,
                execution_fee: 2_500,
                executed_block: 20,
                timestamp: 20,
            },
        ];

        let stats = TWAPExecutor::calculate_statistics(&order, &executions, 5_050).unwrap();

        assert_eq!(stats.total_amount, 10_000_000);
        assert_eq!(stats.average_price, 5_050);
    }

    #[test]
    fn test_estimate_price_impact() {
        let prices =
            TWAPExecutor::estimate_price_impact(5_000, 10_000_000, 10, 100_000_000).unwrap();

        assert_eq!(prices.len(), 10);
        assert!(prices[0] <= prices[9]); // Later slices have more impact
    }

    #[test]
    fn test_should_cancel_due_to_volatility_no() {
        let should_cancel = TWAPExecutor::should_cancel_due_to_volatility(
            5_100, 5_000, 500, // 5% max slippage
        );

        assert!(!should_cancel);
    }

    #[test]
    fn test_should_cancel_due_to_volatility_yes() {
        let should_cancel = TWAPExecutor::should_cancel_due_to_volatility(
            6_000, 5_000, 500, // 5% max slippage
        );

        assert!(should_cancel);
    }
}
