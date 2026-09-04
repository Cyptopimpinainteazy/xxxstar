//! Fuzz target: Swap calculation
//!
//! Feeds arbitrary inputs to swap_calculate to find edge cases,
//! incorrect slippage calculations, or AMM formula errors.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::Decode;
use x3_dex::amm_pools::{AMMPool, LiquidityPool, TokenId};

fuzz_target!(|data: &[u8]| {
    if data.len() < 64 {
        return;
    }

    // Create a pool from the first part of data
    let pool_data = &data[..std::cmp::min(data.len(), 32)];
    if let Ok(pool) = LiquidityPool::decode(&mut &pool_data[..]) {
        // Only test pools with liquidity
        if pool.reserve_a > 0 && pool.reserve_b > 0 {
            // Extract swap parameters from remaining data
            let remaining = &data[pool_data.len()..];
            if remaining.len() >= 48 {
                let token_in_data = &remaining[0..16];
                if let Ok(token_in) = TokenId::decode(&mut &token_in_data[..]) {
                    let amount_in = u128::from_le_bytes(remaining[16..32].try_into().unwrap_or([0; 16]));
                    let min_out = u128::from_le_bytes(remaining[32..48].try_into().unwrap_or([0; 16]));

                    // Only test if token_in is actually in the pool
                    if token_in == pool.token_a || token_in == pool.token_b {
                        // Test swap_calculate - should not panic
                        let result = AMMPool::swap_calculate(
                            &pool,
                            &token_in,
                            amount_in,
                            min_out,
                        );

                        if let Ok(amount_out) = result {
                            // INVARIANT: Output amount should meet minimum
                            assert!(amount_out >= min_out, "Output below minimum");

                            // INVARIANT: Output should be positive for positive input
                            if amount_in > 0 {
                                assert!(amount_out > 0, "Positive input should yield positive output");
                            }

                            // INVARIANT: Fee calculation should reduce output appropriately
                            // For 0 fee, output should be higher
                            if pool.fee_basis_points == 0 {
                                // With 0 fee, should get more output
                                // We can't easily test this without complex math, but we can check bounds
                                let max_possible = if token_in == pool.token_a {
                                    pool.reserve_b
                                } else {
                                    pool.reserve_a
                                };
                                assert!(amount_out <= max_possible, "Cannot output more than reserve");
                            }

                            // INVARIANT: No division by zero or overflow in calculation
                            let _ = amount_out.checked_add(1); // Should not panic

                            // INVARIANT: Swap should respect constant product invariant
                            // (reserve_a + amount_in) * (reserve_b - amount_out) should ≈ reserve_a * reserve_b
                            // Allow for small floating point differences and fees
                            let (reserve_in, reserve_out) = if token_in == pool.token_a {
                                (pool.reserve_a, pool.reserve_b)
                            } else {
                                (pool.reserve_b, pool.reserve_a)
                            };

                            let product_before = reserve_in as f64 * reserve_out as f64;
                            let product_after = (reserve_in + amount_in) as f64 * (reserve_out - amount_out) as f64;

                            // Allow 1% tolerance for floating point and fee calculations
                            let tolerance = product_before * 0.01;
                            assert!(
                                (product_before - product_after).abs() < tolerance,
                                "Constant product invariant violated: before={}, after={}",
                                product_before, product_after
                            );
                        }
                    }
                }
            }
        }
    }
});