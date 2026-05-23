/// Pool Analytics Engine — Real-time TVL, volume, APY, and performance tracking
/// Aggregates pool metrics for liquidity provider decision-making and protocol analytics
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct PoolAnalytics {
    pub pool_id: [u8; 32],
    pub token_a: u128,
    pub token_b: u128,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub tvl_usd: u64,
    pub accumulated_fees: u64,
    pub accumulated_swaps: u32,
    pub last_update_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct PoolMetrics {
    pub pool_id: [u8; 32],
    pub current_tvl: u64,
    pub volume_24h: u64,
    pub volume_7d: u64,
    pub volume_30d: u64,
    pub apy_24h: u32, // bps
    pub apy_7d: u32,
    pub apy_30d: u32,
    pub fee_tier: u32,
    pub tx_count_24h: u32,
    pub update_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct LiquidityProviderStats {
    pub provider: [u8; 32],
    pub pool_id: [u8; 32],
    pub liquidity_provided: u64,
    pub fees_earned_24h: u64,
    pub fees_earned_7d: u64,
    pub fees_earned_30d: u64,
    pub impermanent_loss_estimated: i64,
    pub share_percentage: u32, // bps
    pub positions_count: u32,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TokenMetrics {
    pub token_id: u128,
    pub token_name: Vec<u8>,
    pub total_liquidity: u64,
    pub price_usd: u64,
    pub price_change_24h: i32, // bps
    pub market_cap: u64,
    pub holders_count: u32,
    pub last_update_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct VolumeBucket {
    pub bucket_id: [u8; 32],
    pub pool_id: [u8; 32],
    pub period_start_block: u64,
    pub period_end_block: u64,
    pub volume: u64,
    pub num_trades: u32,
    pub avg_trade_size: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ProtocolMetrics {
    pub total_tvl: u64,
    pub total_volume_24h: u64,
    pub total_volume_all_time: u64,
    pub active_pools: u32,
    pub active_lps: u32,
    pub generated_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct PoolSnapshot {
    pub snapshot_id: [u8; 32],
    pub pool_id: [u8; 32],
    pub tvl: u64,
    pub volume: u64,
    pub fee_growth: u64,
    pub swap_count: u32,
    pub snapshot_block: u64,
}

pub struct PoolAnalyticsEngine;

impl PoolAnalyticsEngine {
    const BLOCKS_PER_DAY: u64 = 28_800; // 3-second blocks
    const BLOCKS_PER_7D: u64 = 201_600;
    const BLOCKS_PER_30D: u64 = 864_000;

    /// Calculate pool APY from fees
    pub fn calculate_apy(fees_earned: u64, tvl: u64, time_period_blocks: u64) -> u32 {
        if tvl == 0 || time_period_blocks == 0 {
            return 0;
        }

        // APY = (fees / TVL) * (365 / period_days) * 10000
        let period_days = time_period_blocks / Self::BLOCKS_PER_DAY;

        if period_days == 0 {
            return 0;
        }

        let fee_ratio = (fees_earned as u128 * 10_000 / tvl as u128) as u32;
        let annualized = (fee_ratio as u128 * 365 / period_days as u128) as u32;

        annualized.min(100_000_000) // Cap at 1M% APY
    }

    /// Calculate impermanent loss estimate
    pub fn calculate_impermanent_loss(
        initial_price_a: u64,
        initial_price_b: u64,
        current_price_a: u64,
        current_price_b: u64,
    ) -> i64 {
        if initial_price_a == 0 || initial_price_b == 0 {
            return 0;
        }

        // IL ≈ -((√(P1/P0) - 1) / (√(P1/P0) + 1))² for 50/50 pool
        // Simplified: IL = 2 - 2 * √(Pa/Pa_0 * Pb/Pb_0)

        let price_ratio_a = (current_price_a as u128 * 100 / initial_price_a as u128) as u64;
        let price_ratio_b = (current_price_b as u128 * 100 / initial_price_b as u128) as u64;

        let product = price_ratio_a as i64 * price_ratio_b as i64;
        let il = 100 - (product / 50); // Simplified approximation

        -il // Return as negative loss
    }

    /// Get pool metrics snapshot
    #[allow(clippy::too_many_arguments)]
    pub fn get_pool_metrics(
        pool_id: [u8; 32],
        tvl: u64,
        volume_24h: u64,
        volume_7d: u64,
        volume_30d: u64,
        fees_24h: u64,
        fees_7d: u64,
        fees_30d: u64,
        fee_tier: u32,
        tx_count_24h: u32,
        current_block: u64,
    ) -> PoolMetrics {
        let apy_24h = Self::calculate_apy(fees_24h, tvl, Self::BLOCKS_PER_DAY);
        let apy_7d = Self::calculate_apy(fees_7d, tvl, Self::BLOCKS_PER_7D);
        let apy_30d = Self::calculate_apy(fees_30d, tvl, Self::BLOCKS_PER_30D);

        PoolMetrics {
            pool_id,
            current_tvl: tvl,
            volume_24h,
            volume_7d,
            volume_30d,
            apy_24h,
            apy_7d,
            apy_30d,
            fee_tier,
            tx_count_24h,
            update_block: current_block,
        }
    }

    /// Get LP statistics
    #[allow(clippy::too_many_arguments)]
    pub fn get_lp_stats(
        provider: [u8; 32],
        pool_id: [u8; 32],
        liquidity: u64,
        total_pool_liquidity: u64,
        fees_24h: u64,
        fees_7d: u64,
        fees_30d: u64,
        positions: u32,
    ) -> Result<LiquidityProviderStats, &'static str> {
        if total_pool_liquidity == 0 {
            return Err("Invalid pool liquidity");
        }

        let share_bps = ((liquidity as u128 * 10_000) / total_pool_liquidity as u128) as u32;

        let stats = LiquidityProviderStats {
            provider,
            pool_id,
            liquidity_provided: liquidity,
            fees_earned_24h: fees_24h,
            fees_earned_7d: fees_7d,
            fees_earned_30d: fees_30d,
            impermanent_loss_estimated: 0, // Should be calculated separately
            share_percentage: share_bps,
            positions_count: positions,
        };

        Ok(stats)
    }

    /// Track volume in time buckets
    pub fn track_volume_bucket(
        pool_id: [u8; 32],
        volume: u64,
        num_trades: u32,
        start_block: u64,
        end_block: u64,
    ) -> Result<VolumeBucket, &'static str> {
        if end_block <= start_block {
            return Err("Invalid block range");
        }

        let avg_size = if num_trades > 0 {
            volume / num_trades as u64
        } else {
            0
        };

        let bucket = VolumeBucket {
            bucket_id: Self::derive_bucket_id(pool_id, start_block),
            pool_id,
            period_start_block: start_block,
            period_end_block: end_block,
            volume,
            num_trades,
            avg_trade_size: avg_size,
        };

        Ok(bucket)
    }

    /// Get protocol-wide metrics
    pub fn get_protocol_metrics(
        total_tvl: u64,
        total_vol_24h: u64,
        total_vol_all: u64,
        active_pools: u32,
        active_lps: u32,
        current_block: u64,
    ) -> ProtocolMetrics {
        ProtocolMetrics {
            total_tvl,
            total_volume_24h: total_vol_24h,
            total_volume_all_time: total_vol_all,
            active_pools,
            active_lps,
            generated_block: current_block,
        }
    }

    /// Calculate token metrics
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_token_metrics(
        token_id: u128,
        name: Vec<u8>,
        total_liquidity: u64,
        price_usd: u64,
        price_24h_ago: u64,
        market_cap: u64,
        holders: u32,
        current_block: u64,
    ) -> TokenMetrics {
        let price_change = if price_24h_ago > 0 {
            ((price_usd as i64 - price_24h_ago as i64) * 10_000 / price_24h_ago as i64) as i32
        } else {
            0
        };

        TokenMetrics {
            token_id,
            token_name: name,
            total_liquidity,
            price_usd,
            price_change_24h: price_change,
            market_cap,
            holders_count: holders,
            last_update_block: current_block,
        }
    }

    /// Create pool snapshot for historical tracking
    pub fn create_snapshot(
        pool_id: [u8; 32],
        tvl: u64,
        volume: u64,
        fee_growth: u64,
        swap_count: u32,
        current_block: u64,
    ) -> PoolSnapshot {
        PoolSnapshot {
            snapshot_id: Self::derive_snapshot_id(pool_id, current_block),
            pool_id,
            tvl,
            volume,
            fee_growth,
            swap_count,
            snapshot_block: current_block,
        }
    }

    /// Estimate LP income (simple)
    pub fn estimate_lp_income(
        share_percentage: u32, // bps
        pool_apy: u32,         // bps
        tvl: u64,
    ) -> u64 {
        // Income = TVL * APY% * LP_share%
        let share_value = (tvl as u128 * share_percentage as u128 / 10_000) as u64;

        (share_value as u128 * pool_apy as u128 / 10_000) as u64
    }

    /// Calculate concentration of liquidity (Herfindahl index)
    pub fn calculate_liquidity_concentration(
        provider_shares: Vec<u32>, // All LP shares in bps
    ) -> u32 {
        // HHI = Σ(share_i)²
        let mut hhi_sum: u128 = 0;

        for share in provider_shares {
            let share_u128 = share as u128;
            hhi_sum += share_u128 * share_u128 / 10_000; // Normalize
        }

        hhi_sum.min(10_000) as u32
    }

    /// Get projected APY based on current conditions
    pub fn project_apy(
        current_apy_24h: u32,
        volatility_score: u32, // Higher = more volatile = potentially higher fees
        trend: i32,            // Positive = uptrend, more volume expected
    ) -> u32 {
        // Simple projection: adjust APY up/down based on trend
        let adjustment: i32 = if trend > 0 {
            (volatility_score / 100).min(5_000) as i32 // Up to 50% boost
        } else {
            -(((volatility_score / 100).min(5_000)) as i32)
        };

        (current_apy_24h as i32 + adjustment).max(0) as u32
    }

    /// Derive volume bucket ID
    fn derive_bucket_id(pool_id: [u8; 32], start_block: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in pool_id.iter().enumerate().take(24) {
            id[i] = *byte;
        }
        let block_bytes = start_block.to_le_bytes();
        for (i, byte) in block_bytes.iter().enumerate() {
            id[i + 24] = *byte;
        }
        id
    }

    /// Derive snapshot ID
    fn derive_snapshot_id(pool_id: [u8; 32], block: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in pool_id.iter().enumerate().take(24) {
            id[i] = *byte;
        }
        let block_bytes = block.to_le_bytes();
        for (i, byte) in block_bytes.iter().enumerate() {
            id[i + 24] = *byte;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_apy() {
        let apy = PoolAnalyticsEngine::calculate_apy(
            1_000,   // Fees earned
            100_000, // TVL
            28_800,  // 1 day
        );

        assert!(apy > 0);
    }

    #[test]
    fn test_calculate_impermanent_loss() {
        let il = PoolAnalyticsEngine::calculate_impermanent_loss(
            100, // Initial price A
            100, // Initial price B
            110, // Current price A
            91,  // Current price B (+10% / -9.1%)
        );

        assert!(il < 0); // Loss
    }

    #[test]
    fn test_get_pool_metrics() {
        let metrics = PoolAnalyticsEngine::get_pool_metrics(
            [1; 32], 100_000,   // TVL
            50_000,    // Volume 24h
            300_000,   // Volume 7d
            1_000_000, // Volume 30d
            500,       // Fees 24h
            4_000,     // Fees 7d
            12_000,    // Fees 30d
            3_000,     // Fee tier
            250,       // Tx count 24h
            100,       // Current block
        );

        assert_eq!(metrics.current_tvl, 100_000);
        assert!(metrics.apy_24h > 0);
    }

    #[test]
    fn test_get_lp_stats() {
        let stats = PoolAnalyticsEngine::get_lp_stats(
            [1; 32], [2; 32], 25_000,  // LP liquidity
            100_000, // Total pool
            200,     // Fees 24h
            1_500,   // Fees 7d
            5_000,   // Fees 30d
            3,       // Positions
        )
        .unwrap();

        assert_eq!(stats.share_percentage, 2_500); // 25%
    }

    #[test]
    fn test_track_volume_bucket() {
        let bucket = PoolAnalyticsEngine::track_volume_bucket(
            [1; 32], 50_000, 100, // 100 trades
            100, 200,
        )
        .unwrap();

        assert_eq!(bucket.avg_trade_size, 500); // 50000 / 100
    }

    #[test]
    fn test_get_protocol_metrics() {
        let metrics = PoolAnalyticsEngine::get_protocol_metrics(
            5_000_000,   // Total TVL
            1_000_000,   // Volume 24h
            100_000_000, // AUM
            50,          // Active pools
            1_200,       // Active LPs
            100,
        );

        assert_eq!(metrics.total_tvl, 5_000_000);
    }

    #[test]
    fn test_calculate_token_metrics() {
        let metrics = PoolAnalyticsEngine::calculate_token_metrics(
            1,
            b"Token A".to_vec(),
            1_000_000,
            10_000,
            9_900, // 1% higher
            50_000_000,
            5_000,
            100,
        );

        assert!(metrics.price_change_24h > 0); // Price went up
    }

    #[test]
    fn test_create_snapshot() {
        let snap = PoolAnalyticsEngine::create_snapshot([1; 32], 100_000, 50_000, 5_000, 250, 100);

        assert_eq!(snap.tvl, 100_000);
    }

    #[test]
    fn test_estimate_lp_income() {
        let income = PoolAnalyticsEngine::estimate_lp_income(
            2_500,   // 25% of pool
            10_000,  // 100% APY
            100_000, // 100k TVL
        );

        assert_eq!(income, 25_000); // 25k in expected income
    }

    #[test]
    fn test_calculate_liquidity_concentration() {
        let shares = vec![5_000, 3_000, 2_000]; // 50%, 30%, 20%

        let hhi = PoolAnalyticsEngine::calculate_liquidity_concentration(shares);

        assert!(hhi > 0);
    }

    #[test]
    fn test_project_apy() {
        let projected = PoolAnalyticsEngine::project_apy(
            5_000, // 50% current
            2_000, // 20% volatility
            1,     // Uptrend
        );

        assert!(projected >= 5_000); // Should be same or higher
    }
}
