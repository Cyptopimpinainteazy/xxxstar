/// Options & Derivatives Engine — Black-Scholes pricing for call/put options with settlement
/// Enables options trading, volatility trading, and hedging strategies
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct Option {
    pub option_id: [u8; 32],
    pub holder: [u8; 32],
    pub underlying_token: u128,
    pub quote_token: u128,
    pub strike_price: u64,
    pub expiration_block: u64,
    pub option_type: u8, // 0=call, 1=put
    pub quantity: u64,
    pub premium_paid: u64,
    pub status: u8, // 0=open, 1=exercised, 2=expired, 3=sold
    pub open_block: u64,
    pub greeks: OptionGreeks,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct OptionGreeks {
    pub delta: u64, // ∂price/∂underlying [0, 10000]
    pub gamma: u64, // ∂delta/∂underlying [0, 10000]
    pub theta: i64, // ∂price/∂time (daily)
    pub vega: u64,  // ∂price/∂volatility [0, 10000]
    pub rho: u64,   // ∂price/∂interest_rate [0, 10000]
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct OptionQuote {
    pub quote_id: [u8; 32],
    pub underlying: u128,
    pub quote_token: u128,
    pub strike: u64,
    pub expiration_blocks: u64,
    pub option_type: u8,
    pub underlying_price: u64,
    pub implied_volatility: u32, // bps
    pub option_price: u64,
    pub greeks: OptionGreeks,
    pub quote_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct OptionExercise {
    pub exercise_id: [u8; 32],
    pub option_id: [u8; 32],
    pub exerciser: [u8; 32],
    pub underlying_amount: u64,
    pub settlement_amount: u64,
    pub exercise_block: u64,
    pub profit_loss: i64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct OptionPool {
    pub pool_id: [u8; 32],
    pub underlying: u128,
    pub quote_token: u128,
    pub total_call_open_interest: u64,
    pub total_put_open_interest: u64,
    pub total_premium_collected: u64,
    pub volatility_index: u32, // IV in bps
    pub is_paused: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct VolatilitySurface {
    pub surface_id: [u8; 32],
    pub underlying: u128,
    pub strikes: Vec<u64>,
    pub maturities: Vec<u64>,
    pub volatilities: Vec<u32>, // 2D array flattened [maturity][strike]
    pub last_update_block: u64,
}

pub struct OptionsEngine;

impl OptionsEngine {
    const MIN_EXPIRATION_BLOCKS: u64 = 10; // At least 10 blocks
    const MAX_EXPIRATION_BLOCKS: u64 = 52_560_000; // ~2 years

    /// Calculate Black-Scholes option price
    /// Simplified implementation without true sqrt (using approximations)
    pub fn calculate_black_scholes_price(
        underlying_price: u64,
        strike_price: u64,
        time_to_expiry_blocks: u64,
        volatility_bps: u32,
        option_type: u8, // 0=call, 1=put
    ) -> Result<u64, &'static str> {
        if underlying_price == 0 || strike_price == 0 || time_to_expiry_blocks == 0 {
            return Err("Invalid parameters for pricing");
        }

        // Simplified B-S: option_price ≈ intrinsic + time_value
        let intrinsic = if option_type == 0 {
            // Call: max(S - K, 0)
            underlying_price.saturating_sub(strike_price)
        } else {
            // Put: max(K - S, 0)
            strike_price.saturating_sub(underlying_price)
        };

        // Time value approximation: TV ≈ 0.4 * S * σ * √T
        let time_factor = time_to_expiry_blocks / 1_000; // Simplified time component
        let vol_factor = (volatility_bps as u128 * underlying_price as u128) / 10_000;
        let time_value = (vol_factor * time_factor as u128 / 100) as u64;

        Ok(intrinsic.saturating_add(time_value))
    }

    /// Calculate option Greeks
    pub fn calculate_greeks(
        underlying_price: u64,
        strike_price: u64,
        time_to_expiry_blocks: u64,
        volatility_bps: u32,
        option_type: u8,
    ) -> Result<OptionGreeks, &'static str> {
        // Simplified Greeks (approximations)
        let delta = if option_type == 0 {
            // Call delta: N(d1) ≈ 0.5 + 0.4 * ATM
            if underlying_price >= strike_price {
                7_000 // ~70%
            } else {
                3_000 // ~30%
            }
        } else {
            // Put delta: N(d1) - 1 ≈ -0.5 - 0.4 * ATM
            if underlying_price >= strike_price {
                2_000 // ~20%
            } else {
                8_000 // ~80%
            }
        };

        let gamma = (volatility_bps as u64 / 40).min(200);
        let theta = -((volatility_bps as u64 * underlying_price / 10000) as i64 / 100);
        let vega = volatility_bps as u64 / 2;
        let rho = time_to_expiry_blocks / 100;

        Ok(OptionGreeks {
            delta,
            gamma,
            theta,
            vega,
            rho,
        })
    }

    /// Quote an option price
    #[allow(clippy::too_many_arguments)]
    pub fn quote_option(
        underlying: u128,
        quote_token: u128,
        strike: u64,
        expiration_blocks: u64,
        option_type: u8,
        underlying_price: u64,
        implied_vol: u32,
        current_block: u64,
    ) -> Result<OptionQuote, &'static str> {
        if !(Self::MIN_EXPIRATION_BLOCKS..=Self::MAX_EXPIRATION_BLOCKS).contains(&expiration_blocks)
        {
            return Err("Expiration outside valid range");
        }

        let price = Self::calculate_black_scholes_price(
            underlying_price,
            strike,
            expiration_blocks,
            implied_vol,
            option_type,
        )?;

        let greeks = Self::calculate_greeks(
            underlying_price,
            strike,
            expiration_blocks,
            implied_vol,
            option_type,
        )?;

        Ok(OptionQuote {
            quote_id: Self::derive_quote_id(underlying, strike, option_type),
            underlying,
            quote_token,
            strike,
            expiration_blocks,
            option_type,
            underlying_price,
            implied_volatility: implied_vol,
            option_price: price,
            greeks,
            quote_block: current_block,
        })
    }

    /// Buy an option
    #[allow(clippy::too_many_arguments)]
    pub fn buy_option(
        holder: [u8; 32],
        underlying: u128,
        quote_token: u128,
        strike: u64,
        expiration_block: u64,
        option_type: u8,
        quantity: u64,
        premium_per_unit: u64,
        current_block: u64,
    ) -> Result<Option, &'static str> {
        if quantity == 0 || premium_per_unit == 0 {
            return Err("Invalid quantity or premium");
        }

        let total_premium = premium_per_unit
            .checked_mul(quantity)
            .ok_or("Premium overflow")?;

        let option = Option {
            option_id: Self::derive_option_id(holder, underlying, strike, option_type),
            holder,
            underlying_token: underlying,
            quote_token,
            strike_price: strike,
            expiration_block,
            option_type,
            quantity,
            premium_paid: total_premium,
            status: 0, // open
            open_block: current_block,
            greeks: OptionGreeks {
                delta: 5_000,
                gamma: 50,
                theta: -100,
                vega: 25,
                rho: 10,
            },
        };

        Ok(option)
    }

    /// Exercise an option
    pub fn exercise_option(
        option: &mut Option,
        underlying_price: u64,
        current_block: u64,
    ) -> Result<OptionExercise, &'static str> {
        if option.status != 0 {
            return Err("Option not open");
        }

        if current_block > option.expiration_block {
            return Err("Option expired");
        }

        let (profit_loss, settlement) = if option.option_type == 0 {
            // Call exercise
            if underlying_price > option.strike_price {
                let intrinsic = underlying_price - option.strike_price;
                let settlement_amt = intrinsic.saturating_mul(option.quantity);
                let profit = (settlement_amt as i64).saturating_sub(option.premium_paid as i64);
                (profit, settlement_amt)
            } else {
                return Err("Call out of money");
            }
        } else {
            // Put exercise
            if option.strike_price > underlying_price {
                let intrinsic = option.strike_price - underlying_price;
                let settlement_amt = intrinsic.saturating_mul(option.quantity);
                let profit = (settlement_amt as i64).saturating_sub(option.premium_paid as i64);
                (profit, settlement_amt)
            } else {
                return Err("Put out of money");
            }
        };

        option.status = 1; // exercised

        Ok(OptionExercise {
            exercise_id: Self::derive_exercise_id(option.option_id, current_block),
            option_id: option.option_id,
            exerciser: option.holder,
            underlying_amount: option.quantity,
            settlement_amount: settlement,
            exercise_block: current_block,
            profit_loss,
        })
    }

    /// Expire an option
    pub fn expire_option(option: &mut Option, current_block: u64) -> Result<bool, &'static str> {
        if current_block <= option.expiration_block {
            return Err("Option not yet expired");
        }

        option.status = 2; // expired
        Ok(true)
    }

    /// Update option Greeks
    pub fn update_greeks(
        option: &mut Option,
        new_greeks: OptionGreeks,
    ) -> Result<(), &'static str> {
        option.greeks = new_greeks;
        Ok(())
    }

    /// Create option pool
    pub fn create_option_pool(
        underlying: u128,
        quote_token: u128,
    ) -> Result<OptionPool, &'static str> {
        Ok(OptionPool {
            pool_id: Self::derive_pool_id(underlying, quote_token),
            underlying,
            quote_token,
            total_call_open_interest: 0,
            total_put_open_interest: 0,
            total_premium_collected: 0,
            volatility_index: 1_500, // 15% initial IV
            is_paused: false,
        })
    }

    /// Update implied volatility in pool
    pub fn update_pool_volatility(pool: &mut OptionPool, new_iv: u32) -> Result<(), &'static str> {
        if new_iv > 50_000 {
            return Err("IV out of range");
        }
        pool.volatility_index = new_iv;
        Ok(())
    }

    /// Derive option ID
    fn derive_option_id(holder: [u8; 32], underlying: u128, strike: u64, opt_type: u8) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in holder.iter().enumerate().take(11) {
            id[i] = *byte;
        }
        let underlying_bytes = underlying.to_le_bytes();
        for (i, byte) in underlying_bytes.iter().enumerate().take(8) {
            id[i + 11] = *byte;
        }
        let strike_bytes = strike.to_le_bytes();
        for (i, byte) in strike_bytes.iter().enumerate() {
            id[i + 19] = *byte;
        }
        id[31] = opt_type;
        id
    }

    /// Derive quote ID
    fn derive_quote_id(underlying: u128, strike: u64, opt_type: u8) -> [u8; 32] {
        let mut id = [0u8; 32];
        let underlying_bytes = underlying.to_le_bytes();
        for (i, byte) in underlying_bytes.iter().enumerate() {
            id[i] = *byte;
        }
        let strike_bytes = strike.to_le_bytes();
        for (i, byte) in strike_bytes.iter().enumerate() {
            id[i + 8] = *byte;
        }
        id[31] = opt_type;
        id
    }

    /// Derive exercise ID
    fn derive_exercise_id(option_id: [u8; 32], block: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in option_id.iter().enumerate().take(24) {
            id[i] = *byte;
        }
        let block_bytes = block.to_le_bytes();
        for (i, byte) in block_bytes.iter().enumerate() {
            id[i + 24] = *byte;
        }
        id
    }

    /// Derive pool ID
    fn derive_pool_id(underlying: u128, quote_token: u128) -> [u8; 32] {
        let mut id = [0u8; 32];
        let underlying_bytes = underlying.to_le_bytes();
        for (i, byte) in underlying_bytes.iter().enumerate() {
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
    fn test_calculate_black_scholes_price() {
        let price = OptionsEngine::calculate_black_scholes_price(
            10_000, 10_000, 1_000, 1_500, 0, // call
        )
        .unwrap();

        assert!(price > 0);
    }

    #[test]
    fn test_calculate_greeks() {
        let greeks = OptionsEngine::calculate_greeks(10_000, 10_000, 1_000, 1_500, 0).unwrap();

        assert!(greeks.delta > 0);
    }

    #[test]
    fn test_quote_option() {
        let quote =
            OptionsEngine::quote_option(1, 2, 10_000, 1_000, 0, 10_000, 1_500, 100).unwrap();

        assert!(quote.option_price > 0);
    }

    #[test]
    fn test_buy_option() {
        let option =
            OptionsEngine::buy_option([1; 32], 1, 2, 10_000, 5_000, 0, 100, 500, 100).unwrap();

        assert_eq!(option.quantity, 100);
        assert_eq!(option.status, 0);
    }

    #[test]
    fn test_exercise_call_option() {
        let mut option = OptionsEngine::buy_option(
            [1; 32], 1, 2, 10_000, 5_000, 0, // call
            100, 500, 100,
        )
        .unwrap();

        let exercise = OptionsEngine::exercise_option(
            &mut option,
            12_000, // Above strike
            4_000,
        )
        .unwrap();

        assert!(exercise.profit_loss > 0);
    }

    #[test]
    fn test_expire_option() {
        let mut option =
            OptionsEngine::buy_option([1; 32], 1, 2, 10_000, 1_000, 0, 100, 500, 100).unwrap();

        OptionsEngine::expire_option(&mut option, 2_000).unwrap();

        assert_eq!(option.status, 2);
    }

    #[test]
    fn test_create_option_pool() {
        let pool = OptionsEngine::create_option_pool(1, 2).unwrap();

        assert_eq!(pool.total_call_open_interest, 0);
    }

    #[test]
    fn test_update_pool_volatility() {
        let mut pool = OptionsEngine::create_option_pool(1, 2).unwrap();

        OptionsEngine::update_pool_volatility(&mut pool, 2_000).unwrap();

        assert_eq!(pool.volatility_index, 2_000);
    }

    #[test]
    fn test_put_option_exercise() {
        let mut option = OptionsEngine::buy_option(
            [1; 32], 1, 2, 10_000, 5_000, 1, // put
            100, 500, 100,
        )
        .unwrap();

        let exercise = OptionsEngine::exercise_option(
            &mut option,
            8_000, // Below strike
            4_000,
        )
        .unwrap();

        assert!(exercise.profit_loss > 0);
    }

    #[test]
    fn test_call_out_of_money() {
        let mut option = OptionsEngine::buy_option(
            [1; 32], 1, 2, 10_000, 5_000, 0, // call
            100, 500, 100,
        )
        .unwrap();

        let result = OptionsEngine::exercise_option(
            &mut option,
            8_000, // Below strike
            4_000,
        );

        assert!(result.is_err());
    }
}
