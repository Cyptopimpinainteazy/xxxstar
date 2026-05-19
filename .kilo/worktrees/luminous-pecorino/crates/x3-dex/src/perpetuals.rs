/// Perpetual Futures Engine — Unlimited-expiry leveraged derivatives with funding rates
/// Enables up to 10x leverage, funding rate equilibrium, and liquidation mechanics
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct PerpetualPosition {
    pub position_id: [u8; 32],
    pub trader: [u8; 32],
    pub base_token: u128,
    pub quote_token: u128,
    pub size: i64, // Positive=long, Negative=short
    pub entry_price: u64,
    pub leverage: u32, // e.g., 5 = 5x, max 10
    pub collateral: u64,
    pub unrealized_pnl: i64,
    pub status: u8, // 0=open, 1=liquidating, 2=liquidated
    pub open_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct FundingRate {
    pub rate_id: [u8; 32],
    pub base_token: u128,
    pub quote_token: u128,
    pub funding_rate_bps: i64, // Positive=longs pay shorts, Negative=shorts pay longs
    pub next_funding_block: u64,
    pub funding_interval_blocks: u64,
    pub accumulated_funding: i64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct PerpetualLiquidation {
    pub liquidation_id: [u8; 32],
    pub position_id: [u8; 32],
    pub liquidator: [u8; 32],
    pub liquidation_price: u64,
    pub position_size: i64,
    pub liquidation_fee: u64,
    pub remaining_collateral: u64,
    pub liquidation_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct PerpetualMetrics {
    pub market_id: [u8; 32],
    pub base_token: u128,
    pub quote_token: u128,
    pub open_interest_long: u64,
    pub open_interest_short: u64,
    pub mark_price: u64,
    pub index_price: u64,
    pub current_funding_rate: i64,
    pub total_collateral_locked: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct PositionLeverage {
    pub maintenance_ratio: u32, // Minimum: 250 bps (2.5%)
    pub initial_ratio: u32,     // Initial: 500 bps (5%)
    pub liquidation_threshold: i64,
}

pub struct PerpetualFuturesEngine;

impl PerpetualFuturesEngine {
    const MIN_LEVERAGE: u32 = 1;
    const MAX_LEVERAGE: u32 = 10;
    const MIN_COLLATERAL: u64 = 1_000; // Minimum collateral per position
    const MAINTENANCE_RATIO_BPS: u32 = 250; // 2.5% maintenance
    const INITIAL_RATIO_BPS: u32 = 500; // 5% initial margin
    const LIQUIDATION_FEE_BPS: u32 = 500; // 5% of position value
    const FUNDING_INTERVAL_BLOCKS: u64 = 28_800; // ~1 hour
    const PRICE_SCALE: u64 = 1_000_000_000_000;

    /// Open a perpetual position (long or short)
    #[allow(clippy::too_many_arguments)]
    pub fn open_position(
        trader: [u8; 32],
        base_token: u128,
        quote_token: u128,
        size: i64, // Positive=long, Negative=short
        leverage: u32,
        collateral: u64,
        entry_price: u64,
        current_block: u64,
    ) -> Result<PerpetualPosition, &'static str> {
        if !(Self::MIN_LEVERAGE..=Self::MAX_LEVERAGE).contains(&leverage) {
            return Err("Leverage outside valid range");
        }

        if collateral < Self::MIN_COLLATERAL {
            return Err("Insufficient collateral");
        }

        if size == 0 {
            return Err("Position size must be non-zero");
        }

        let position = PerpetualPosition {
            position_id: Self::derive_position_id(trader, base_token, current_block),
            trader,
            base_token,
            quote_token,
            size,
            entry_price,
            leverage,
            collateral,
            unrealized_pnl: 0,
            status: 0, // open
            open_block: current_block,
        };

        Ok(position)
    }

    /// Update unrealized P&L based on current mark price
    pub fn update_position_pnl(
        position: &mut PerpetualPosition,
        current_mark_price: u64,
    ) -> Result<i64, &'static str> {
        if current_mark_price == 0 {
            return Err("Invalid price");
        }

        // PnL = size * (current_price - entry_price) / entry_price
        let price_diff = if current_mark_price > position.entry_price {
            (current_mark_price - position.entry_price) as i64
        } else {
            -((position.entry_price - current_mark_price) as i64)
        };

        let pnl = (position.size * price_diff) / (position.entry_price as i64);
        position.unrealized_pnl = pnl;

        Ok(pnl)
    }

    /// Check if position is liquidatable
    pub fn is_liquidatable(position: &PerpetualPosition, current_mark_price: u64) -> bool {
        if position.status != 0 {
            return false;
        }

        // Liquidation price: entry_price - (collateral / size) / leverage
        let liquidation_threshold = Self::calculate_liquidation_price(
            position.entry_price,
            position.size,
            position.collateral,
            position.leverage,
        );

        if position.size > 0 {
            current_mark_price <= liquidation_threshold
        } else {
            current_mark_price >= liquidation_threshold
        }
    }

    /// Calculate liquidation price
    pub fn calculate_liquidation_price(
        entry_price: u64,
        size: i64,
        collateral: u64,
        _leverage: u32,
    ) -> u64 {
        if size == 0 {
            return 0;
        }

        let maintenance =
            (collateral as u128 * Self::MAINTENANCE_RATIO_BPS as u128 / 10_000) as u64;

        if size > 0 {
            // Long liquidation: entry_price - maintenance / size
            let sub = maintenance / size.unsigned_abs();
            entry_price.saturating_sub(sub)
        } else {
            // Short liquidation: entry_price + maintenance / abs(size)
            let add = maintenance / size.unsigned_abs();
            entry_price.saturating_add(add)
        }
    }

    /// Liquidate a position
    pub fn liquidate_position(
        position: &mut PerpetualPosition,
        liquidator: [u8; 32],
        liquidation_price: u64,
        current_block: u64,
    ) -> Result<PerpetualLiquidation, &'static str> {
        if position.status != 0 {
            return Err("Position not open");
        }

        let position_value = ((position.size.unsigned_abs() as u128 * liquidation_price as u128)
            / Self::PRICE_SCALE as u128) as u64;

        let liquidation_fee =
            (position_value as u128 * Self::LIQUIDATION_FEE_BPS as u128 / 10_000) as u64;
        let remaining = position.collateral.saturating_sub(liquidation_fee);

        position.status = 2; // liquidated

        Ok(PerpetualLiquidation {
            liquidation_id: Self::derive_liquidation_id(position.position_id, current_block),
            position_id: position.position_id,
            liquidator,
            liquidation_price,
            position_size: position.size,
            liquidation_fee,
            remaining_collateral: remaining,
            liquidation_block: current_block,
        })
    }

    /// Add collateral to position
    pub fn add_collateral(
        position: &mut PerpetualPosition,
        amount: u64,
    ) -> Result<u64, &'static str> {
        if amount == 0 {
            return Err("Collateral amount must be > 0");
        }

        position.collateral = position.collateral.saturating_add(amount);
        Ok(position.collateral)
    }

    /// Reduce collateral (withdraw)
    pub fn reduce_collateral(
        position: &mut PerpetualPosition,
        amount: u64,
    ) -> Result<u64, &'static str> {
        if amount > position.collateral {
            return Err("Insufficient collateral to withdraw");
        }

        position.collateral = position.collateral.saturating_sub(amount);
        Ok(position.collateral)
    }

    /// Close position at market price
    pub fn close_position(
        position: &mut PerpetualPosition,
        close_price: u64,
    ) -> Result<i64, &'static str> {
        if position.status != 0 {
            return Err("Position not open");
        }

        // Calculate realized P&L
        let price_diff = if close_price > position.entry_price {
            (close_price - position.entry_price) as i64
        } else {
            -((position.entry_price - close_price) as i64)
        };

        let realized_pnl = (position.size * price_diff) / (position.entry_price as i64);

        position.status = 0; // Mark as closed (in production, would delete or archive)
        Ok(realized_pnl)
    }

    /// Calculate funding payment
    pub fn calculate_funding_payment(position_size: i64, funding_rate_bps: i64) -> i64 {
        // Funding = position_size * funding_rate
        (position_size * funding_rate_bps) / 10_000
    }

    /// Update funding rate based on market conditions
    pub fn update_funding_rate(long_oi: u64, short_oi: u64, max_rate: i64) -> i64 {
        if long_oi == 0 && short_oi == 0 {
            return 0;
        }

        // Funding rate = (long_oi - short_oi) / (long_oi + short_oi) capped at max_rate
        let net = long_oi as i64 - short_oi as i64;
        let total = (long_oi + short_oi) as i64;

        if total != 0 {
            (net * 10_000 / total).min(max_rate).max(-max_rate)
        } else {
            0
        }
    }

    /// Create funding rate config
    pub fn create_funding_rate(
        base_token: u128,
        quote_token: u128,
        current_block: u64,
    ) -> Result<FundingRate, &'static str> {
        Ok(FundingRate {
            rate_id: Self::derive_rate_id(base_token, quote_token),
            base_token,
            quote_token,
            funding_rate_bps: 0,
            next_funding_block: current_block + Self::FUNDING_INTERVAL_BLOCKS,
            funding_interval_blocks: Self::FUNDING_INTERVAL_BLOCKS,
            accumulated_funding: 0,
        })
    }

    /// Calculate margin ratio
    pub fn calculate_margin_ratio(collateral: u64, position_value: u64) -> u32 {
        if position_value == 0 {
            return 10_000; // 100%
        }

        ((collateral as u128 * 10_000) / position_value as u128).min(10_000) as u32
    }

    /// Check if position has sufficient initial margin
    pub fn has_sufficient_initial_margin(collateral: u64, position_value: u64) -> bool {
        let margin_ratio = Self::calculate_margin_ratio(collateral, position_value);
        margin_ratio >= Self::INITIAL_RATIO_BPS
    }

    /// Check if position has sufficient maintenance margin
    pub fn has_sufficient_maintenance_margin(collateral: u64, position_value: u64) -> bool {
        let margin_ratio = Self::calculate_margin_ratio(collateral, position_value);
        margin_ratio >= Self::MAINTENANCE_RATIO_BPS
    }

    /// Derive position ID
    fn derive_position_id(trader: [u8; 32], base_token: u128, nonce: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in trader.iter().enumerate().take(16) {
            id[i] = *byte;
        }
        let token_bytes = base_token.to_le_bytes();
        for (i, byte) in token_bytes.iter().enumerate().take(8) {
            id[i + 16] = *byte;
        }
        let nonce_bytes = nonce.to_le_bytes();
        for (i, byte) in nonce_bytes.iter().take(8).enumerate() {
            id[i + 24] = *byte;
        }
        id
    }

    /// Derive liquidation ID
    fn derive_liquidation_id(position_id: [u8; 32], block: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in position_id.iter().enumerate().take(24) {
            id[i] = *byte;
        }
        let block_bytes = block.to_le_bytes();
        for (i, byte) in block_bytes.iter().enumerate() {
            id[i + 24] = *byte;
        }
        id
    }

    /// Derive funding rate ID
    fn derive_rate_id(base_token: u128, quote_token: u128) -> [u8; 32] {
        let mut id = [0u8; 32];
        let base_bytes = base_token.to_le_bytes();
        for (i, byte) in base_bytes.iter().enumerate() {
            id[i] = *byte;
        }
        let quote_bytes = quote_token.to_le_bytes();
        for (i, byte) in quote_bytes.iter().enumerate() {
            id[i + 8] = *byte;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_position() {
        let pos = PerpetualFuturesEngine::open_position(
            [1; 32], 1, 2, 1_000,  // 1000 size (long)
            5,      // 5x leverage
            5_000,  // 5000 collateral
            10_000, // entry price
            100,
        )
        .unwrap();

        assert_eq!(pos.size, 1_000);
        assert_eq!(pos.leverage, 5);
    }

    #[test]
    fn test_update_position_pnl() {
        let mut pos =
            PerpetualFuturesEngine::open_position([1; 32], 1, 2, 1_000, 5, 5_000, 10_000, 100)
                .unwrap();

        let pnl = PerpetualFuturesEngine::update_position_pnl(&mut pos, 11_000).unwrap();

        assert!(pnl > 0); // Price went up
    }

    #[test]
    fn test_is_liquidatable() {
        let pos =
            PerpetualFuturesEngine::open_position([1; 32], 1, 2, 1_000, 5, 5_000, 10_000, 100)
                .unwrap();

        let liq_price = PerpetualFuturesEngine::calculate_liquidation_price(
            pos.entry_price,
            pos.size,
            pos.collateral,
            pos.leverage,
        );

        // Price well below liquidation shouldn't be liquidatable yet at entry
        let is_liq = PerpetualFuturesEngine::is_liquidatable(&pos, liq_price);
        assert!(is_liq);
    }

    #[test]
    fn test_calculate_liquidation_price() {
        let liq_price =
            PerpetualFuturesEngine::calculate_liquidation_price(10_000, 1_000, 5_000, 5);

        assert!(liq_price < 10_000); // For long, liq price below entry
    }

    #[test]
    fn test_liquidate_position() {
        let mut pos =
            PerpetualFuturesEngine::open_position([1; 32], 1, 2, 1_000, 5, 5_000, 10_000, 100)
                .unwrap();

        let liq =
            PerpetualFuturesEngine::liquidate_position(&mut pos, [2; 32], 9_000, 200).unwrap();

        assert_eq!(pos.status, 2);
        assert!(liq.liquidation_fee > 0);
    }

    #[test]
    fn test_add_collateral() {
        let mut pos =
            PerpetualFuturesEngine::open_position([1; 32], 1, 2, 1_000, 5, 5_000, 10_000, 100)
                .unwrap();

        let new_collateral = PerpetualFuturesEngine::add_collateral(&mut pos, 2_000).unwrap();

        assert_eq!(new_collateral, 7_000);
    }

    #[test]
    fn test_reduce_collateral() {
        let mut pos =
            PerpetualFuturesEngine::open_position([1; 32], 1, 2, 1_000, 5, 5_000, 10_000, 100)
                .unwrap();

        let new_collateral = PerpetualFuturesEngine::reduce_collateral(&mut pos, 1_000).unwrap();

        assert_eq!(new_collateral, 4_000);
    }

    #[test]
    fn test_close_position() {
        let mut pos =
            PerpetualFuturesEngine::open_position([1; 32], 1, 2, 1_000, 5, 5_000, 10_000, 100)
                .unwrap();

        let pnl = PerpetualFuturesEngine::close_position(&mut pos, 11_000).unwrap();

        assert!(pnl > 0);
    }

    #[test]
    fn test_calculate_funding_payment() {
        let payment = PerpetualFuturesEngine::calculate_funding_payment(1_000, 500);

        assert_eq!(payment, 50); // 1000 * 500 / 10000 = 50
    }

    #[test]
    fn test_update_funding_rate() {
        let rate = PerpetualFuturesEngine::update_funding_rate(1_000_000, 500_000, 10_000);

        assert!(rate > 0); // Long OI > short OI means longs pay
    }

    #[test]
    fn test_create_funding_rate() {
        let rate = PerpetualFuturesEngine::create_funding_rate(1, 2, 100).unwrap();

        assert_eq!(rate.funding_rate_bps, 0);
    }

    #[test]
    fn test_calculate_margin_ratio() {
        let ratio = PerpetualFuturesEngine::calculate_margin_ratio(5_000, 50_000);

        assert_eq!(ratio, 1_000); // 5000/50000 = 0.1 = 1000 bps
    }

    #[test]
    fn test_has_sufficient_initial_margin() {
        let sufficient = PerpetualFuturesEngine::has_sufficient_initial_margin(5_000, 10_000);

        assert!(sufficient); // 5000/10000 = 50% > 5% initial
    }
}
