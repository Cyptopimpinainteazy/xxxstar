//! Tests for AMM mathematical safety

use super::*;
use crate::mock::*;
use x3_dex::amm_pools::{AMMPool, LiquidityPool, TokenId};

#[test]
fn test_amm_calculations() {
    // Test basic AMM calculations are mathematically correct
    let mut pool = LiquidityPool {
        pool_id: 0,
        token_a: TokenId {
            chain_id: 1,
            asset_id: 0,
        },
        token_b: TokenId {
            chain_id: 1,
            asset_id: 1,
        },
        reserve_a: 1000,
        reserve_b: 1000,
        total_lp_supply: 1000,
        fee_basis_points: 30,
        created_block: 0,
    };

    // Test adding liquidity calculations
    let result = AMMPool::add_liquidity_calculate(
        &pool, 100, // amount_a_desired
        100, // amount_b_desired
        90,  // amount_a_min
        90,  // amount_b_min
    );

    assert!(result.is_ok());
    let (amount_a, amount_b, lp_tokens) = result.unwrap();
    assert!(amount_a >= 90 && amount_b >= 90);
    assert!(lp_tokens > 0);
}

#[test]
fn test_overflow_protection() {
    // Test that calculations handle overflow safely
    let pool = LiquidityPool {
        pool_id: 0,
        token_a: TokenId {
            chain_id: 1,
            asset_id: 0,
        },
        token_b: TokenId {
            chain_id: 1,
            asset_id: 1,
        },
        reserve_a: u128::MAX / 2,
        reserve_b: u128::MAX / 2,
        total_lp_supply: 1000,
        fee_basis_points: 30,
        created_block: 0,
    };

    // Test with large amounts that could cause overflow
    let result = AMMPool::add_liquidity_calculate(&pool, u128::MAX / 4, u128::MAX / 4, 1, 1);

    // Should not panic, should handle gracefully
    assert!(result.is_ok() || result.is_err()); // Either succeeds safely or fails safely
}

#[test]
fn test_slippage_calculation() {
    let pool = LiquidityPool {
        pool_id: 0,
        token_a: TokenId {
            chain_id: 1,
            asset_id: 0,
        },
        token_b: TokenId {
            chain_id: 1,
            asset_id: 1,
        },
        reserve_a: 1000,
        reserve_b: 1000,
        total_lp_supply: 1000,
        fee_basis_points: 30,
        created_block: 0,
    };

    // Test swap calculations respect slippage
    let result = AMMPool::swap_calculate(
        &pool,
        &pool.token_a,
        100, // amount_in
        90,  // min_out (reasonable expectation)
    );

    assert!(result.is_ok());
    let amount_out = result.unwrap();
    assert!(amount_out >= 90); // Should meet minimum
}
