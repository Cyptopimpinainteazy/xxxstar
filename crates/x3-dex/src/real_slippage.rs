/// Real Slippage Calculator — Deterministic slippage calculation using constant-product AMM formula
/// Provides users with transparent real-time slippage estimates before trade execution
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct PoolReserves {
    pub pool_id: [u8; 32],
    pub token_a: u128,
    pub token_b: u128,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub total_liquidity: u64,
    pub fee_bps: u32, // e.g., 3000 = 0.3%
    pub last_update_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct SlippageQuote {
    pub pool_id: [u8; 32],
    pub input_amount: u64,
    pub output_amount: u64,
    pub price_impact_bps: u32,
    pub execution_price: u64,
    pub spot_price: u64,
    pub fee_amount: u64,
    pub quote_block: u64,
    pub is_valid: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct PriceImpact {
    pub impact_bps: u32,
    pub description: Vec<u8>, // e.g., "0.5% price impact"
    pub impact_level: u8,     // 0=low, 1=medium, 2=high, 3=very_high
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct SlippageProtection {
    pub quote_id: [u8; 32],
    pub min_output_amount: u64,
    pub max_slippage_bps: u32,
    pub deadline_block: u64,
    pub protected_address: [u8; 32],
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct MarketPrice {
    pub token_pair: (u128, u128),
    pub price_a_to_b: u64, // How many B for 1 A (scaled)
    pub price_b_to_a: u64, // How many A for 1 B (scaled)
    pub update_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct RoutePath {
    pub route_id: [u8; 32],
    pub hops: Vec<[u8; 32]>, // Pool IDs in sequence
    pub input_token: u128,
    pub output_token: u128,
    pub total_price_impact: u32, // Aggregated across all hops
}

pub struct RealSlippageCalculator;

impl RealSlippageCalculator {
    const PRICE_SCALE: u64 = 10_000; // Scale for fixed-point price calculations
    const LOW_IMPACT_THRESHOLD: u32 = 25; // < 0.25% = low
    const MEDIUM_IMPACT_THRESHOLD: u32 = 100; // < 1% = medium
    const HIGH_IMPACT_THRESHOLD: u32 = 500; // < 5% = high
    const QUOTE_VALIDITY_BLOCKS: u64 = 10; // Quote valid for 10 blocks

    /// Calculate output amount using constant-product formula: x*y = k
    /// Formula: output = (input * reserve_out * (10000 - fee)) / (reserve_in * 10000 + input * (10000 - fee))
    pub fn calculate_output_amount(
        input_amount: u64,
        reserve_in: u64,
        reserve_out: u64,
        fee_bps: u32,
    ) -> Result<(u64, u64), &'static str> {
        if input_amount == 0 || reserve_in == 0 || reserve_out == 0 {
            return Err("Invalid amount or reserves");
        }

        // Calculate fee
        let fee_amount = (input_amount as u128 * fee_bps as u128 / 10_000) as u64;
        let input_after_fee = input_amount.saturating_sub(fee_amount);

        // Constant product formula
        let numerator = (input_after_fee as u128)
            .checked_mul(reserve_out as u128)
            .ok_or("Overflow in output calculation")?;

        let denominator = (reserve_in as u128)
            .saturating_add(input_after_fee as u128)
            .checked_mul(1)
            .ok_or("Invalid denominator")?;

        let output = (numerator / denominator) as u64;

        Ok((output, fee_amount))
    }

    /// Calculate spot price (no trade impact)
    pub fn calculate_spot_price(reserve_a: u64, reserve_b: u64) -> Result<u64, &'static str> {
        if reserve_a == 0 {
            return Err("Reserve A is zero");
        }

        let price = ((reserve_b as u128 * Self::PRICE_SCALE as u128) / reserve_a as u128) as u64;
        Ok(price)
    }

    /// Calculate execution price (actual price after slippage)
    pub fn calculate_execution_price(
        input_amount: u64,
        output_amount: u64,
    ) -> Result<u64, &'static str> {
        if input_amount == 0 {
            return Err("Input amount is zero");
        }

        let price =
            ((output_amount as u128 * Self::PRICE_SCALE as u128) / input_amount as u128) as u64;
        Ok(price)
    }

    /// Calculate price impact in basis points
    pub fn calculate_price_impact_bps(spot_price: u64, execution_price: u64) -> u32 {
        if spot_price == 0 {
            return 0;
        }

        if execution_price < spot_price {
            // Buying: execution price < spot price (unfavorable for buyer)
            ((spot_price as u128 - execution_price as u128) * 10_000 / spot_price as u128) as u32
        } else {
            // Selling: execution price > spot price (favorable for seller)
            ((spot_price as u128 - execution_price as u128) * 10_000 / spot_price as u128) as u32
        }
    }

    /// Generate slippage quote for a trade
    pub fn generate_quote(
        pool: &PoolReserves,
        input_amount: u64,
        current_block: u64,
    ) -> Result<SlippageQuote, &'static str> {
        let (output_amount, fee) = Self::calculate_output_amount(
            input_amount,
            pool.reserve_a,
            pool.reserve_b,
            pool.fee_bps,
        )?;

        let spot_price = Self::calculate_spot_price(pool.reserve_a, pool.reserve_b)?;
        let execution_price = Self::calculate_execution_price(input_amount, output_amount)?;
        let impact_bps = Self::calculate_price_impact_bps(spot_price, execution_price);

        let quote = SlippageQuote {
            pool_id: pool.pool_id,
            input_amount,
            output_amount,
            price_impact_bps: impact_bps,
            execution_price,
            spot_price,
            fee_amount: fee,
            quote_block: current_block,
            is_valid: true,
        };

        Ok(quote)
    }

    /// Analyze price impact level (low/medium/high/very_high)
    pub fn analyze_price_impact(impact_bps: u32) -> PriceImpact {
        let (level, description) = if impact_bps < Self::LOW_IMPACT_THRESHOLD {
            (0, b"Low impact (<0.25%)".to_vec())
        } else if impact_bps < Self::MEDIUM_IMPACT_THRESHOLD {
            (1, b"Medium impact (0.25-1%)".to_vec())
        } else if impact_bps < Self::HIGH_IMPACT_THRESHOLD {
            (2, b"High impact (1-5%)".to_vec())
        } else {
            (3, b"Very high impact (>5%)".to_vec())
        };

        PriceImpact {
            impact_bps,
            description,
            impact_level: level,
        }
    }

    /// Create slippage protection for quote
    pub fn create_slippage_protection(
        quote: &SlippageQuote,
        max_slippage_bps: u32,
        deadline_blocks: u64,
        current_block: u64,
        user: [u8; 32],
    ) -> Result<SlippageProtection, &'static str> {
        if max_slippage_bps < quote.price_impact_bps {
            return Err("Slippage protection too strict for current impact");
        }

        // Calculate minimum output with protection
        let slippage_amount =
            (quote.output_amount as u128 * max_slippage_bps as u128 / 10_000) as u64;
        let min_output = quote.output_amount.saturating_sub(slippage_amount);

        let protection = SlippageProtection {
            quote_id: Self::derive_quote_id(quote.pool_id, quote.input_amount),
            min_output_amount: min_output,
            max_slippage_bps,
            deadline_block: current_block + deadline_blocks,
            protected_address: user,
        };

        Ok(protection)
    }

    /// Validate quote is still within protection parameters
    pub fn validate_execution(
        quote: &SlippageQuote,
        protection: &SlippageProtection,
        actual_output: u64,
        current_block: u64,
    ) -> Result<bool, &'static str> {
        // Check deadline
        if current_block > protection.deadline_block {
            return Err("Quote deadline expired");
        }

        // Check minimum output
        if actual_output < protection.min_output_amount {
            return Err("Actual output below minimum protection");
        }

        // Check quote is still valid
        if current_block > quote.quote_block + Self::QUOTE_VALIDITY_BLOCKS {
            return Err("Quote expired (older than 10 blocks)");
        }

        Ok(true)
    }

    /// Calculate multi-hop route price impact
    pub fn calculate_route_impact(impacts: Vec<u32>) -> u32 {
        // Aggregate impacts: total_impact = 1 - (1 - impact1) * (1 - impact2) * ...
        let mut aggregated: u128 = 10_000; // Start with 100%

        for impact in impacts {
            let impact_multiplier = 10_000_u128 - impact as u128;
            aggregated = (aggregated * impact_multiplier) / 10_000;
        }

        10_000 - aggregated as u32
    }

    /// Get suggested minimum output with default 0.5% slippage allowance
    pub fn get_minimum_output_with_default_slippage(quote: &SlippageQuote) -> u64 {
        let default_slippage = 50; // 0.5%
        let slippage_amount =
            (quote.output_amount as u128 * default_slippage as u128 / 10_000) as u64;
        quote.output_amount.saturating_sub(slippage_amount)
    }

    /// Calculate effective fee including price impact
    pub fn calculate_effective_fee(quote: &SlippageQuote) -> u64 {
        let price_impact_fee =
            (quote.input_amount as u128 * quote.price_impact_bps as u128 / 10_000) as u64;
        quote.fee_amount.saturating_add(price_impact_fee)
    }

    /// Derive deterministic quote ID
    fn derive_quote_id(pool_id: [u8; 32], input: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in pool_id.iter().enumerate().take(24) {
            id[i] = *byte;
        }
        let input_bytes = input.to_le_bytes();
        for (i, byte) in input_bytes.iter().enumerate().take(8) {
            id[i + 24] = *byte;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_output_amount() {
        // Pool: 1000 A, 1000 B, 0.3% fee
        let (output, fee) =
            RealSlippageCalculator::calculate_output_amount(100, 1_000, 1_000, 30).unwrap();

        assert!(output > 0);
        assert_eq!(fee, 0); // 0.3% of 100 = 0.3, rounds to 0
    }

    #[test]
    fn test_calculate_spot_price() {
        let price = RealSlippageCalculator::calculate_spot_price(1_000, 1_000).unwrap();

        assert_eq!(price, 10_000); // 1:1 price
    }

    #[test]
    fn test_calculate_execution_price() {
        let price = RealSlippageCalculator::calculate_execution_price(100, 95).unwrap();

        assert!(price > 0);
    }

    #[test]
    fn test_calculate_price_impact_bps() {
        let impact = RealSlippageCalculator::calculate_price_impact_bps(10_000, 9_950);

        assert!(impact > 0);
    }

    #[test]
    fn test_generate_quote() {
        let pool = PoolReserves {
            pool_id: [1; 32],
            token_a: 1,
            token_b: 2,
            reserve_a: 1_000_000,
            reserve_b: 1_000_000,
            total_liquidity: 1_000_000,
            fee_bps: 30,
            last_update_block: 100,
        };

        let quote = RealSlippageCalculator::generate_quote(&pool, 100_000, 200).unwrap();

        assert!(quote.output_amount > 0);
        assert!(quote.is_valid);
    }

    #[test]
    fn test_analyze_price_impact_low() {
        let impact = RealSlippageCalculator::analyze_price_impact(10);

        assert_eq!(impact.impact_level, 0); // low
    }

    #[test]
    fn test_analyze_price_impact_medium() {
        let impact = RealSlippageCalculator::analyze_price_impact(75);

        assert_eq!(impact.impact_level, 1); // medium
    }

    #[test]
    fn test_analyze_price_impact_high() {
        let impact = RealSlippageCalculator::analyze_price_impact(250);

        assert_eq!(impact.impact_level, 2); // high
    }

    #[test]
    fn test_analyze_price_impact_very_high() {
        let impact = RealSlippageCalculator::analyze_price_impact(1_000);

        assert_eq!(impact.impact_level, 3); // very high
    }

    #[test]
    fn test_create_slippage_protection() {
        let quote = SlippageQuote {
            pool_id: [1; 32],
            input_amount: 100_000,
            output_amount: 98_000,
            price_impact_bps: 50,
            execution_price: 9_800,
            spot_price: 10_000,
            fee_amount: 30,
            quote_block: 200,
            is_valid: true,
        };

        let protection = RealSlippageCalculator::create_slippage_protection(
            &quote, 100, // 1% max slippage
            20,  // 20 block deadline
            200, [3; 32],
        )
        .unwrap();

        assert!(protection.min_output_amount > 0);
        assert_eq!(protection.deadline_block, 220);
    }

    #[test]
    fn test_validate_execution() {
        let quote = SlippageQuote {
            pool_id: [1; 32],
            input_amount: 100_000,
            output_amount: 98_000,
            price_impact_bps: 50,
            execution_price: 9_800,
            spot_price: 10_000,
            fee_amount: 30,
            quote_block: 200,
            is_valid: true,
        };

        let protection = SlippageProtection {
            quote_id: [2; 32],
            min_output_amount: 97_000,
            max_slippage_bps: 100,
            deadline_block: 220,
            protected_address: [3; 32],
        };

        let valid =
            RealSlippageCalculator::validate_execution(&quote, &protection, 97_500, 205).unwrap();

        assert!(valid);
    }

    #[test]
    fn test_calculate_route_impact() {
        let impacts = vec![25, 30, 20]; // Three hops with impacts

        let total = RealSlippageCalculator::calculate_route_impact(impacts);

        assert!(total > 0);
    }

    #[test]
    fn test_get_minimum_output_with_default_slippage() {
        let quote = SlippageQuote {
            pool_id: [1; 32],
            input_amount: 100_000,
            output_amount: 98_000,
            price_impact_bps: 50,
            execution_price: 9_800,
            spot_price: 10_000,
            fee_amount: 30,
            quote_block: 200,
            is_valid: true,
        };

        let min_output = RealSlippageCalculator::get_minimum_output_with_default_slippage(&quote);

        assert!(min_output < quote.output_amount);
    }

    #[test]
    fn test_calculate_effective_fee() {
        let quote = SlippageQuote {
            pool_id: [1; 32],
            input_amount: 100_000,
            output_amount: 98_000,
            price_impact_bps: 50,
            execution_price: 9_800,
            spot_price: 10_000,
            fee_amount: 30,
            quote_block: 200,
            is_valid: true,
        };

        let effective_fee = RealSlippageCalculator::calculate_effective_fee(&quote);

        assert!(effective_fee > quote.fee_amount);
    }
}
