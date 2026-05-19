/// Concentrated Liquidity — Uniswap V3 model with capital efficiency
/// LPs set custom price ranges for 10-100x capital efficiency vs basic AMM
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ConcentratedPosition {
    pub id: [u8; 32],
    pub lp: [u8; 32],
    pub token0: u128,
    pub token1: u128,
    pub lower_tick: i32,
    pub upper_tick: i32,
    pub liquidity: u128,
    pub amount0_deposited: u64,
    pub amount1_deposited: u64,
    pub fee_tier: u32, // 100 (0.01%), 500 (0.05%), 3000 (0.3%), 10000 (1%)
    pub is_active: bool,
    pub created_at: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TickLiquidity {
    pub tick: i32,
    pub liquidity_gross: u128,
    pub liquidity_net: i128,
    pub fee_growth_outside0: u128,
    pub fee_growth_outside1: u128,
    pub initialized: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ConcentratedPool {
    pub pool_id: [u8; 32],
    pub token0: u128,
    pub token1: u128,
    pub current_tick: i32,
    pub current_price: u64,
    pub liquidity: u128,
    pub fee_tier: u32,
    pub fee_growth_global0: u128,
    pub fee_growth_global1: u128,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct FeeAccrual {
    pub position_id: [u8; 32],
    pub fee0_unclaimed: u64,
    pub fee1_unclaimed: u64,
    pub last_fee_growth0: u128,
    pub last_fee_growth1: u128,
}

pub struct ConcentratedLiquidityEngine;

impl ConcentratedLiquidityEngine {
    const MIN_LIQUIDITY: u128 = 1;
    const MAX_TICK: i32 = 887_272;
    const MIN_TICK: i32 = -887_272;

    /// Create a concentrated liquidity position
    #[allow(clippy::too_many_arguments)]
    pub fn create_position(
        lp: [u8; 32],
        token0: u128,
        token1: u128,
        lower_tick: i32,
        upper_tick: i32,
        amount0: u64,
        amount1: u64,
        fee_tier: u32,
        current_block: u64,
    ) -> Result<ConcentratedPosition, &'static str> {
        if lower_tick >= upper_tick {
            return Err("Lower tick must be < upper tick");
        }

        if lower_tick < Self::MIN_TICK || upper_tick > Self::MAX_TICK {
            return Err("Tick out of range");
        }

        if amount0 == 0 && amount1 == 0 {
            return Err("Must deposit at least one token");
        }

        if !Self::is_valid_fee_tier(fee_tier) {
            return Err("Invalid fee tier");
        }

        // Calculate liquidity (simplified)
        let liquidity = Self::calculate_liquidity(amount0, amount1, lower_tick, upper_tick)?;

        let position = ConcentratedPosition {
            id: Self::derive_position_id(lp, token0, token1, lower_tick, upper_tick),
            lp,
            token0,
            token1,
            lower_tick,
            upper_tick,
            liquidity,
            amount0_deposited: amount0,
            amount1_deposited: amount1,
            fee_tier,
            is_active: true,
            created_at: current_block,
        };

        Ok(position)
    }

    /// Increase liquidity in an existing position
    pub fn increase_liquidity(
        position: &mut ConcentratedPosition,
        amount0: u64,
        amount1: u64,
    ) -> Result<u128, &'static str> {
        if !position.is_active {
            return Err("Position is inactive");
        }

        if amount0 == 0 && amount1 == 0 {
            return Err("Must deposit at least one token");
        }

        let additional_liquidity =
            Self::calculate_liquidity(amount0, amount1, position.lower_tick, position.upper_tick)?;

        position.liquidity = position.liquidity.saturating_add(additional_liquidity);
        position.amount0_deposited = position.amount0_deposited.saturating_add(amount0);
        position.amount1_deposited = position.amount1_deposited.saturating_add(amount1);

        Ok(position.liquidity)
    }

    /// Decrease liquidity in a position
    pub fn decrease_liquidity(
        position: &mut ConcentratedPosition,
        liquidity_to_remove: u128,
    ) -> Result<(u64, u64), &'static str> {
        if !position.is_active {
            return Err("Position is inactive");
        }

        if liquidity_to_remove > position.liquidity {
            return Err("Cannot remove more liquidity than deposited");
        }

        let ratio = liquidity_to_remove * 1_000_000 / position.liquidity;

        let amount0_out = ((position.amount0_deposited as u128) * ratio / 1_000_000) as u64;
        let amount1_out = ((position.amount1_deposited as u128) * ratio / 1_000_000) as u64;

        position.liquidity -= liquidity_to_remove;
        position.amount0_deposited -= amount0_out;
        position.amount1_deposited -= amount1_out;

        Ok((amount0_out, amount1_out))
    }

    /// Close a position (remove all liquidity)
    pub fn close_position(position: &mut ConcentratedPosition) -> Result<(u64, u64), &'static str> {
        let (amount0, amount1) = Self::decrease_liquidity(position, position.liquidity)?;
        position.is_active = false;

        Ok((amount0, amount1))
    }

    /// Calculate claimable fees for a position
    pub fn calculate_claimable_fees(
        position: &ConcentratedPosition,
        current_fee_growth0: u128,
        current_fee_growth1: u128,
        last_fee_growth0: u128,
        last_fee_growth1: u128,
    ) -> Result<(u64, u64), &'static str> {
        let uncollected_fee0_growth = current_fee_growth0.saturating_sub(last_fee_growth0);
        let uncollected_fee1_growth = current_fee_growth1.saturating_sub(last_fee_growth1);

        // Q128.128 fixed-point: approximate (growth * liquidity) / 2^128 using u128 bit-split.
        // Exact calc needs 256-bit arithmetic; (a >> 64) * (b >> 64) gives (a*b) >> 128 approximation.
        let fee0 = ((uncollected_fee0_growth >> 64) as u64)
            .saturating_mul((position.liquidity >> 64) as u64);
        let fee1 = ((uncollected_fee1_growth >> 64) as u64)
            .saturating_mul((position.liquidity >> 64) as u64);

        Ok((fee0, fee1))
    }

    /// Collect accumulated fees
    pub fn collect_fees(
        accrue: &mut FeeAccrual,
        current_fee_growth0: u128,
        current_fee_growth1: u128,
    ) -> Result<(u64, u64), &'static str> {
        let fee0_delta = (current_fee_growth0.saturating_sub(accrue.last_fee_growth0)) as u64;
        let fee1_delta = (current_fee_growth1.saturating_sub(accrue.last_fee_growth1)) as u64;

        let claimed0 = accrue.fee0_unclaimed + fee0_delta;
        let claimed1 = accrue.fee1_unclaimed + fee1_delta;

        accrue.fee0_unclaimed = 0;
        accrue.fee1_unclaimed = 0;
        accrue.last_fee_growth0 = current_fee_growth0;
        accrue.last_fee_growth1 = current_fee_growth1;

        Ok((claimed0, claimed1))
    }

    /// Calculate capital efficiency ratio
    pub fn calculate_capital_efficiency(
        liquidity_concentrated: u128,
        liquidity_full_range: u128,
    ) -> u32 {
        if liquidity_full_range == 0 {
            return 1;
        }

        ((liquidity_concentrated / liquidity_full_range) as u32).min(10_000)
    }

    /// Rebalance position when price moves out of range
    pub fn rebalance_position(
        position: &mut ConcentratedPosition,
        current_tick: i32,
        new_lower_tick: i32,
        new_upper_tick: i32,
    ) -> Result<bool, &'static str> {
        if new_lower_tick >= new_upper_tick {
            return Err("Invalid new tick range");
        }

        // Check if current position is out of range
        if current_tick >= position.lower_tick && current_tick < position.upper_tick {
            return Err("Position is still in range");
        }

        position.lower_tick = new_lower_tick;
        position.upper_tick = new_upper_tick;

        Ok(true)
    }

    /// Get position's fee tier
    pub fn get_position_fee_tier(position: &ConcentratedPosition) -> u32 {
        position.fee_tier
    }

    /// Determine if fee tier is valid
    fn is_valid_fee_tier(fee_tier: u32) -> bool {
        matches!(fee_tier, 100 | 500 | 3000 | 10000)
    }

    /// Calculate liquidity from amounts and tick range
    fn calculate_liquidity(
        amount0: u64,
        amount1: u64,
        lower_tick: i32,
        upper_tick: i32,
    ) -> Result<u128, &'static str> {
        if amount0 == 0 && amount1 == 0 {
            return Err("Both amounts cannot be zero");
        }

        let tick_range = (upper_tick - lower_tick) as u128;
        if tick_range == 0 {
            return Err("Tick range must be positive");
        }

        // Simplified liquidity calculation
        let combined = (amount0 as u128).saturating_add(amount1 as u128);
        let liquidity = combined / tick_range;

        if liquidity < Self::MIN_LIQUIDITY {
            return Err("Liquidity below minimum");
        }

        Ok(liquidity)
    }

    /// Derive deterministic position ID
    fn derive_position_id(
        lp: [u8; 32],
        token0: u128,
        _token1: u128,
        lower_tick: i32,
        upper_tick: i32,
    ) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in lp.iter().enumerate().take(16) {
            id[i] = *byte;
        }
        let token0_bytes = token0.to_le_bytes();
        for (i, byte) in token0_bytes.iter().enumerate().take(8) {
            id[i + 8] = *byte;
        }
        let lower_bytes = (lower_tick as u32).to_le_bytes();
        for (i, byte) in lower_bytes.iter().enumerate() {
            id[i + 16] = *byte;
        }
        let upper_bytes = (upper_tick as u32).to_le_bytes();
        for (i, byte) in upper_bytes.iter().enumerate().take(4) {
            id[i + 20] = *byte;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_position() {
        let pos = ConcentratedLiquidityEngine::create_position(
            [1; 32], 1, 2, -1000, 1000, 1_000_000, 1_000_000, 500, 0,
        )
        .unwrap();

        assert_eq!(pos.lower_tick, -1000);
        assert_eq!(pos.upper_tick, 1000);
        assert!(pos.is_active);
    }

    #[test]
    fn test_create_position_invalid_ticks() {
        let result = ConcentratedLiquidityEngine::create_position(
            [1; 32], 1, 2, 1000, -1000, 1_000_000, 1_000_000, 500, 0,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_create_position_invalid_fee_tier() {
        let result = ConcentratedLiquidityEngine::create_position(
            [1; 32], 1, 2, -1000, 1000, 1_000_000, 1_000_000, 999, // Invalid
            0,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_increase_liquidity() {
        let mut pos = ConcentratedLiquidityEngine::create_position(
            [1; 32], 1, 2, -1000, 1000, 1_000_000, 1_000_000, 500, 0,
        )
        .unwrap();

        let initial_liquidity = pos.liquidity;

        ConcentratedLiquidityEngine::increase_liquidity(&mut pos, 500_000, 500_000).unwrap();

        assert!(pos.liquidity > initial_liquidity);
    }

    #[test]
    fn test_decrease_liquidity() {
        let mut pos = ConcentratedLiquidityEngine::create_position(
            [1; 32], 1, 2, -1000, 1000, 1_000_000, 1_000_000, 500, 0,
        )
        .unwrap();

        let initial_liquidity = pos.liquidity;
        let (amount0, amount1) =
            ConcentratedLiquidityEngine::decrease_liquidity(&mut pos, initial_liquidity / 2)
                .unwrap();

        assert!(amount0 > 0);
        assert!(amount1 > 0);
        assert_eq!(pos.liquidity, initial_liquidity / 2);
    }

    #[test]
    fn test_close_position() {
        let mut pos = ConcentratedLiquidityEngine::create_position(
            [1; 32], 1, 2, -1000, 1000, 1_000_000, 1_000_000, 500, 0,
        )
        .unwrap();

        ConcentratedLiquidityEngine::close_position(&mut pos).unwrap();

        assert!(!pos.is_active);
        assert_eq!(pos.liquidity, 0);
    }

    #[test]
    fn test_calculate_claimable_fees() {
        let pos = ConcentratedPosition {
            id: [0; 32],
            lp: [1; 32],
            token0: 1,
            token1: 2,
            lower_tick: -1000,
            upper_tick: 1000,
            liquidity: 1_000_000_000_000_000_000,
            amount0_deposited: 1_000_000,
            amount1_deposited: 1_000_000,
            fee_tier: 500,
            is_active: true,
            created_at: 0,
        };

        let (fee0, fee1) =
            ConcentratedLiquidityEngine::calculate_claimable_fees(&pos, 1_000_000, 1_000_000, 0, 0)
                .unwrap();

        assert!(fee0 > 0 || fee1 > 0);
    }

    #[test]
    fn test_collect_fees() {
        let mut accrue = FeeAccrual {
            position_id: [0; 32],
            fee0_unclaimed: 100_000,
            fee1_unclaimed: 100_000,
            last_fee_growth0: 0,
            last_fee_growth1: 0,
        };

        let (claimed0, claimed1) =
            ConcentratedLiquidityEngine::collect_fees(&mut accrue, 500_000, 500_000).unwrap();

        assert!(claimed0 > 100_000);
        assert!(claimed1 > 100_000);
        assert_eq!(accrue.fee0_unclaimed, 0);
    }

    #[test]
    fn test_calculate_capital_efficiency() {
        let efficiency = ConcentratedLiquidityEngine::calculate_capital_efficiency(
            10_000_000_000,
            1_000_000_000,
        );

        assert_eq!(efficiency, 10); // 10x efficiency
    }

    #[test]
    fn test_rebalance_position() {
        let mut pos = ConcentratedLiquidityEngine::create_position(
            [1; 32], 1, 2, -1000, 1000, 1_000_000, 1_000_000, 500, 0,
        )
        .unwrap();

        let result = ConcentratedLiquidityEngine::rebalance_position(
            &mut pos, 2000, // Out of initial range
            -2000, 2000,
        );

        assert!(result.is_ok());
        assert_eq!(pos.lower_tick, -2000);
    }
}
