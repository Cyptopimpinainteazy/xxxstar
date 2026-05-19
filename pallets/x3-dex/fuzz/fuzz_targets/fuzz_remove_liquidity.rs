//! Fuzz target: Remove liquidity calculation
//!
//! Feeds arbitrary inputs to remove_liquidity_calculate to find edge cases,
//! division by zero, or incorrect proportional calculations.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::Decode;
use x3_dex::amm_pools::{AMMPool, LiquidityPool};

fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }

    // Create a pool from the first part of data
    let pool_data = &data[..std::cmp::min(data.len(), 32)];
    if let Ok(pool) = LiquidityPool::decode(&mut &pool_data[..]) {
        // Only test pools with liquidity
        if pool.total_lp_supply > 0 && pool.reserve_a > 0 && pool.reserve_b > 0 {
            // Extract calculation parameters from remaining data
            let remaining = &data[pool_data.len()..];
            if remaining.len() >= 32 {
                let lp_amount = u128::from_le_bytes(remaining[0..16].try_into().unwrap_or([0; 16]));

                let remaining2 = &remaining[16..];
                let (amount_a_min, amount_b_min) = if remaining2.len() >= 32 {
                    (
                        u128::from_le_bytes(remaining2[0..16].try_into().unwrap_or([0; 16])),
                        u128::from_le_bytes(remaining2[16..32].try_into().unwrap_or([0; 16]))
                    )
                } else {
                    (0, 0)
                };

                // Test remove_liquidity_calculate - should not panic
                let result = AMMPool::remove_liquidity_calculate(
                    &pool,
                    lp_amount,
                    amount_a_min,
                    amount_b_min,
                );

                if let Ok((amount_a, amount_b)) = result {
                    // INVARIANT: Output amounts should meet minimums
                    assert!(amount_a >= amount_a_min, "Calculated amount_a below minimum");
                    assert!(amount_b >= amount_b_min, "Calculated amount_b below minimum");

                    // INVARIANT: Proportional calculation should be correct
                    let expected_a = (lp_amount as f64) * (pool.reserve_a as f64) / (pool.total_lp_supply as f64);
                    let expected_b = (lp_amount as f64) * (pool.reserve_b as f64) / (pool.total_lp_supply as f64);

                    // Allow small floating point differences
                    let tolerance = 1u128;
                    assert!(
                        (amount_a as f64 - expected_a).abs() < tolerance as f64,
                        "Amount A calculation incorrect: got {}, expected {}",
                        amount_a, expected_a as u128
                    );
                    assert!(
                        (amount_b as f64 - expected_b).abs() < tolerance as f64,
                        "Amount B calculation incorrect: got {}, expected {}",
                        amount_b, expected_b as u128
                    );

                    // INVARIANT: Total value should not exceed pool reserves
                    assert!(amount_a <= pool.reserve_a, "Cannot remove more than reserve A");
                    assert!(amount_b <= pool.reserve_b, "Cannot remove more than reserve B");

                    // INVARIANT: LP amount should not exceed total supply
                    assert!(lp_amount <= pool.total_lp_supply, "Cannot burn more LP than exists");
                }
            }
        }
    }
});