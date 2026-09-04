/// Batch Swap Router — Execute multiple swaps atomically with MEV protection
/// Enables batch execution, path optimization, and sandwich attack prevention
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct SwapInstruction {
    pub pool_id: [u8; 32],
    pub token_in: u128,
    pub token_out: u128,
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub sequence: u32, // Order in batch
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct BatchSwap {
    pub batch_id: [u8; 32],
    pub initiator: [u8; 32],
    pub swaps: Vec<SwapInstruction>,
    pub total_input: u64,
    pub total_output: u64,
    pub status: u8, // 0=pending, 1=executed, 2=failed
    pub created_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct SwapRoute {
    pub route_id: [u8; 32],
    pub token_in: u128,
    pub token_out: u128,
    pub hops: Vec<[u8; 32]>, // Pool IDs in sequence
    pub input_amount: u64,
    pub expected_output: u64,
    pub price_impact_bps: u32,
    pub route_valid_until_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct AtomicSwapExecution {
    pub execution_id: [u8; 32],
    pub router_address: [u8; 32],
    pub input_amount: u64,
    pub output_amount: u64,
    pub actual_price_impact: u32,
    pub timestamp: u64,
    pub execution_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct MEVProtection {
    pub protection_id: [u8; 32],
    pub min_output: u64,
    pub deadline_block: u64,
    pub max_slippage_bps: u32,
    pub protected: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct RouteMetrics {
    pub route_id: [u8; 32],
    pub efficiency_score: u32, // 0-10000 (higher = better)
    pub fee_cost_bps: u32,
    pub slippage_estimated_bps: u32,
    pub number_hops: u32,
}

pub struct BatchSwapRouter;

impl BatchSwapRouter {
    const MAX_SWAPS_PER_BATCH: u32 = 10;
    const ROUTE_VALIDITY_BLOCKS: u64 = 10;

    /// Create a batch swap
    pub fn create_batch_swap(
        initiator: [u8; 32],
        swaps: Vec<SwapInstruction>,
        current_block: u64,
    ) -> Result<BatchSwap, &'static str> {
        if swaps.is_empty() || swaps.len() > Self::MAX_SWAPS_PER_BATCH as usize {
            return Err("Invalid number of swaps in batch");
        }

        let mut total_input = 0u64;

        for swap in &swaps {
            if swap.amount_in == 0 || swap.min_amount_out == 0 {
                return Err("Invalid swap amounts");
            }
            total_input = total_input.saturating_add(swap.amount_in);
        }

        let batch = BatchSwap {
            batch_id: Self::derive_batch_id(initiator, current_block),
            initiator,
            swaps,
            total_input,
            total_output: 0,
            status: 0, // pending
            created_block: current_block,
        };

        Ok(batch)
    }

    /// Execute batch swap atomically
    pub fn execute_batch_swap(
        batch: &mut BatchSwap,
        actual_outputs: Vec<u64>,
    ) -> Result<u64, &'static str> {
        if batch.status != 0 {
            return Err("Batch not pending");
        }

        if actual_outputs.len() != batch.swaps.len() {
            return Err("Output count mismatch");
        }

        // Verify all outputs meet minimum
        for (i, &output) in actual_outputs.iter().enumerate() {
            if output < batch.swaps[i].min_amount_out {
                return Err("Output below minimum for one or more swaps");
            }
        }

        // Sum outputs
        let total = actual_outputs
            .iter()
            .fold(0u64, |acc, &x| acc.saturating_add(x));

        batch.total_output = total;
        batch.status = 1; // executed

        Ok(total)
    }

    /// Calculate optimal route (return single best route)
    pub fn calculate_optimal_route(
        token_in: u128,
        token_out: u128,
        amount_in: u64,
        available_pools: Vec<([u8; 32], u32)>, // (pool_id, fee_tier)
        current_block: u64,
    ) -> Result<SwapRoute, &'static str> {
        if available_pools.is_empty() {
            return Err("No pools available");
        }

        // Simplified: assume direct pool exists, take lowest fee
        let best_pool = available_pools
            .iter()
            .min_by_key(|(_, fee)| fee)
            .ok_or("No pool found")?;

        // Estimate output (simplified)
        let expected_output = (amount_in as u128 * 99 / 100) as u64; // Assume ~1% impact

        let route = SwapRoute {
            route_id: Self::derive_route_id(token_in, token_out, current_block),
            token_in,
            token_out,
            hops: vec![best_pool.0],
            input_amount: amount_in,
            expected_output,
            price_impact_bps: 100, // 1% estimated
            route_valid_until_block: current_block + Self::ROUTE_VALIDITY_BLOCKS,
        };

        Ok(route)
    }

    /// Split swap into multiple routes (for large orders)
    pub fn split_swap_across_pools(
        token_in: u128,
        token_out: u128,
        total_amount: u64,
        num_splits: u32,
    ) -> Result<Vec<SwapInstruction>, &'static str> {
        if num_splits == 0 || num_splits > 5 {
            return Err("Invalid split count");
        }

        let amount_per_split = total_amount / num_splits as u64;
        let remainder = total_amount % num_splits as u64;

        let mut swaps = Vec::new();

        for i in 0..num_splits {
            let amount = if i == num_splits - 1 {
                amount_per_split + remainder // Last split gets remainder
            } else {
                amount_per_split
            };

            if amount > 0 {
                swaps.push(SwapInstruction {
                    pool_id: [0; 32], // Will be filled in by router
                    token_in,
                    token_out,
                    amount_in: amount,
                    min_amount_out: (amount as u128 * 99 / 100) as u64, // 1% slippage tolerance
                    sequence: i,
                });
            }
        }

        Ok(swaps)
    }

    /// Apply MEV protection to batch
    pub fn apply_mev_protection(
        min_output: u64,
        deadline_blocks: u64,
        max_slippage: u32,
        current_block: u64,
    ) -> MEVProtection {
        MEVProtection {
            protection_id: Self::derive_protection_id(current_block),
            min_output,
            deadline_block: current_block + deadline_blocks,
            max_slippage_bps: max_slippage,
            protected: true,
        }
    }

    /// Validate batch execution against MEV protection
    pub fn validate_mev_protection(
        actual_output: u64,
        protection: &MEVProtection,
        current_block: u64,
    ) -> Result<bool, &'static str> {
        if !protection.protected {
            return Ok(true); // No protection, pass
        }

        if current_block > protection.deadline_block {
            return Err("Deadline exceeded");
        }

        if actual_output < protection.min_output {
            return Err("Output below MEV protection minimum");
        }

        Ok(true)
    }

    /// Calculate route efficiency score
    pub fn calculate_route_efficiency(hops: u32, price_impact_bps: u32, fee_cost_bps: u32) -> u32 {
        // Efficiency = (10000 - impact - fee) / hops (prefer fewer hops, less impact)
        let total_cost = price_impact_bps.saturating_add(fee_cost_bps);
        let net = 10_000u32.saturating_sub(total_cost);

        if hops == 0 {
            return 0;
        }

        (net / hops).min(10_000)
    }

    /// Get route metrics
    pub fn get_route_metrics(
        route_id: [u8; 32],
        efficiency: u32,
        fee_bps: u32,
        slippage_bps: u32,
        hops: u32,
    ) -> RouteMetrics {
        RouteMetrics {
            route_id,
            efficiency_score: efficiency,
            fee_cost_bps: fee_bps,
            slippage_estimated_bps: slippage_bps,
            number_hops: hops,
        }
    }

    /// Mark batch as failed
    pub fn fail_batch_swap(batch: &mut BatchSwap) -> bool {
        if batch.status == 0 {
            batch.status = 2; // failed
            true
        } else {
            false
        }
    }

    /// Estimate total cost (fees + slippage) for batch
    pub fn estimate_total_cost(
        swaps: &[SwapInstruction],
        fee_bps_per_swap: u32,
        price_impact_bps: u32,
    ) -> u64 {
        let mut total_cost = 0u64;

        for swap in swaps {
            let fee_amount = (swap.amount_in as u128 * fee_bps_per_swap as u128 / 10_000) as u64;
            let impact = (swap.amount_in as u128 * price_impact_bps as u128 / 10_000) as u64;

            total_cost = total_cost.saturating_add(fee_amount).saturating_add(impact);
        }

        total_cost
    }

    /// Derive batch ID
    fn derive_batch_id(initiator: [u8; 32], nonce: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in initiator.iter().enumerate().take(24) {
            id[i] = *byte;
        }
        let nonce_bytes = nonce.to_le_bytes();
        for (i, byte) in nonce_bytes.iter().enumerate() {
            id[i + 24] = *byte;
        }
        id
    }

    /// Derive route ID
    fn derive_route_id(token_in: u128, token_out: u128, nonce: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        let in_bytes = token_in.to_le_bytes();
        for (i, byte) in in_bytes.iter().enumerate() {
            id[i] = *byte;
        }
        let out_bytes = token_out.to_le_bytes();
        for (i, byte) in out_bytes.iter().enumerate() {
            id[i + 8] = *byte;
        }
        let nonce_bytes = nonce.to_le_bytes();
        for (i, byte) in nonce_bytes.iter().enumerate() {
            id[i + 16] = *byte;
        }
        id
    }

    /// Derive protection ID
    fn derive_protection_id(nonce: u64) -> [u8; 32] {
        let mut id = [0; 32];
        let nonce_bytes = nonce.to_le_bytes();
        for (i, byte) in nonce_bytes.iter().enumerate() {
            id[i] = *byte;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_batch_swap() {
        let swaps = vec![SwapInstruction {
            pool_id: [1; 32],
            token_in: 1,
            token_out: 2,
            amount_in: 100,
            min_amount_out: 95,
            sequence: 0,
        }];

        let batch = BatchSwapRouter::create_batch_swap([1; 32], swaps, 100).unwrap();

        assert_eq!(batch.total_input, 100);
        assert_eq!(batch.status, 0);
    }

    #[test]
    fn test_execute_batch_swap() {
        let swaps = vec![SwapInstruction {
            pool_id: [1; 32],
            token_in: 1,
            token_out: 2,
            amount_in: 100,
            min_amount_out: 95,
            sequence: 0,
        }];

        let mut batch = BatchSwapRouter::create_batch_swap([1; 32], swaps, 100).unwrap();

        let output = BatchSwapRouter::execute_batch_swap(&mut batch, vec![98]).unwrap();

        assert_eq!(output, 98);
        assert_eq!(batch.status, 1);
    }

    #[test]
    fn test_calculate_optimal_route() {
        let pools = vec![([1; 32], 3000), ([2; 32], 500)];

        let route = BatchSwapRouter::calculate_optimal_route(1, 2, 1000, pools, 100).unwrap();

        assert_eq!(route.expected_output, 990);
    }

    #[test]
    fn test_split_swap_across_pools() {
        let swaps = BatchSwapRouter::split_swap_across_pools(1, 2, 1000, 5).unwrap();

        assert_eq!(swaps.len(), 5);
        let total: u64 = swaps.iter().map(|s| s.amount_in).sum();
        assert_eq!(total, 1000);
    }

    #[test]
    fn test_apply_mev_protection() {
        let protection = BatchSwapRouter::apply_mev_protection(950, 100, 50, 100);

        assert!(protection.protected);
        assert_eq!(protection.min_output, 950);
    }

    #[test]
    fn test_validate_mev_protection() {
        let protection = BatchSwapRouter::apply_mev_protection(950, 100, 50, 100);

        let valid = BatchSwapRouter::validate_mev_protection(975, &protection, 150).unwrap();

        assert!(valid);
    }

    #[test]
    fn test_calculate_route_efficiency() {
        let efficiency = BatchSwapRouter::calculate_route_efficiency(2, 100, 30);

        assert!(efficiency > 0);
    }

    #[test]
    fn test_get_route_metrics() {
        let metrics = BatchSwapRouter::get_route_metrics([1; 32], 8_500, 30, 100, 2);

        assert_eq!(metrics.fee_cost_bps, 30);
    }

    #[test]
    fn test_fail_batch_swap() {
        let swaps = vec![SwapInstruction {
            pool_id: [1; 32],
            token_in: 1,
            token_out: 2,
            amount_in: 100,
            min_amount_out: 95,
            sequence: 0,
        }];

        let mut batch = BatchSwapRouter::create_batch_swap([1; 32], swaps, 100).unwrap();

        let failed = BatchSwapRouter::fail_batch_swap(&mut batch);

        assert!(failed);
        assert_eq!(batch.status, 2);
    }

    #[test]
    fn test_estimate_total_cost() {
        let swaps = vec![SwapInstruction {
            pool_id: [1; 32],
            token_in: 1,
            token_out: 2,
            amount_in: 1000,
            min_amount_out: 950,
            sequence: 0,
        }];

        let cost = BatchSwapRouter::estimate_total_cost(&swaps, 30, 100);

        assert!(cost > 0);
    }
}
