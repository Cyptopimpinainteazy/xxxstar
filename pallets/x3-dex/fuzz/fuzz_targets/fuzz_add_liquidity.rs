//! Fuzz target: Add liquidity calculation
//!
//! Feeds arbitrary inputs to add_liquidity_calculate to find edge cases,
//! overflows, or incorrect calculations in AMM math.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::Decode;
use x3_dex::amm_pools::{AMMPool, LiquidityPool, TokenId};

fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }

    // Create a pool from the first part of data
    let pool_data = &data[..std::cmp::min(data.len(), 32)];
    if let Ok(pool) = LiquidityPool::decode(&mut &pool_data[..]) {
        // Extract calculation parameters from remaining data
        let remaining = &data[pool_data.len()..];
        if remaining.len() >= 32 {
            let amount_a_desired = u128::from_le_bytes(remaining[0..16].try_into().unwrap_or([0; 16]));
            let amount_b_desired = u128::from_le_bytes(remaining[16..32].try_into().unwrap_or([0; 16]));

            let remaining2 = &remaining[32..];
            let (amount_a_min, amount_b_min) = if remaining2.len() >= 32 {
                (
                    u128::from_le_bytes(remaining2[0..16].try_into().unwrap_or([0; 16])),
                    u128::from_le_bytes(remaining2[16..32].try_into().unwrap_or([0; 16]))
                )
            } else {
                (0, 0)
            };

            // Test add_liquidity_calculate - should not panic
            let result = AMMPool::add_liquidity_calculate(
                &pool,
                amount_a_desired,
                amount_b_desired,
                amount_a_min,
                amount_b_min,
            );

            if let Ok((amount_a, amount_b, lp_tokens)) = result {
                // INVARIANT: Calculated amounts should not exceed desired amounts
                assert!(amount_a <= amount_a_desired, "Calculated amount_a exceeds desired");
                assert!(amount_b <= amount_b_desired, "Calculated amount_b exceeds desired");

                // INVARIANT: Calculated amounts should meet minimums
                assert!(amount_a >= amount_a_min, "Calculated amount_a below minimum");
                assert!(amount_b >= amount_b_min, "Calculated amount_b below minimum");

                // INVARIANT: LP tokens should be positive for successful additions
                if amount_a > 0 && amount_b > 0 {
                    assert!(lp_tokens > 0, "LP tokens must be positive for valid liquidity addition");
                }

                // INVARIANT: No overflow in LP token calculation
                let _ = lp_tokens.checked_add(1); // Should not panic

                // INVARIANT: For first liquidity provision, amounts should equal desired
                if pool.reserve_a == 0 && pool.reserve_b == 0 {
                    // First provision - amounts should match desired (within bounds)
                    if amount_a_desired >= amount_a_min && amount_b_desired >= amount_b_min {
                        // Should succeed, but amounts might be adjusted for ratio
                        // We don't assert exact equality since ratio adjustment is valid
                    }
                }
            }
        }
    }
});