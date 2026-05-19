//! Tests for token swapping mechanism

use super::*;
use crate::mock::*;
use x3_dex::amm_pools::TokenId;

#[test]
fn test_swap_execution() {
    new_test_ext().execute_with(|| {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };

        // Create pool and add liquidity
        assert_ok!(DEX::create_pool(
            RuntimeOrigin::signed(1),
            token_a.clone(),
            token_b.clone(),
            30
        ));
        assert_ok!(DEX::add_liquidity(
            RuntimeOrigin::signed(1),
            0,
            1000,
            1000,
            900,
            900,
        ));

        // Execute swap
        assert_ok!(DEX::swap(
            RuntimeOrigin::signed(2),
            0, // pool_id
            token_a.clone(),
            100, // amount_in
            80,  // min_out
        ));

        // Verify pool reserves changed
        let pool = DEX::pools(0).unwrap();
        assert!(pool.reserve_a > 1000); // Increased by swap input
        assert!(pool.reserve_b < 1000); // Decreased by swap output
    });
}

#[test]
fn test_invariant_preservation() {
    new_test_ext().execute_with(|| {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };

        // Create pool and add liquidity
        assert_ok!(DEX::create_pool(
            RuntimeOrigin::signed(1),
            token_a.clone(),
            token_b.clone(),
            30
        ));
        assert_ok!(DEX::add_liquidity(
            RuntimeOrigin::signed(1),
            0,
            1000,
            1000,
            900,
            900,
        ));

        let initial_k = 1000u128 * 1000u128; // Constant product invariant

        // Execute swap
        assert_ok!(DEX::swap(
            RuntimeOrigin::signed(2),
            0,
            token_a.clone(),
            100,
            80,
        ));

        // Verify invariant is preserved (approximately, accounting for fees)
        let pool = DEX::pools(0).unwrap();
        let final_k = pool.reserve_a as u128 * pool.reserve_b as u128;
        assert!(final_k >= initial_k); // Should not decrease (fees make it increase slightly)
    });
}

#[test]
fn test_slippage_protection() {
    new_test_ext().execute_with(|| {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };

        // Create pool and add liquidity
        assert_ok!(DEX::create_pool(
            RuntimeOrigin::signed(1),
            token_a.clone(),
            token_b.clone(),
            30
        ));
        assert_ok!(DEX::add_liquidity(
            RuntimeOrigin::signed(1),
            0,
            1000,
            1000,
            900,
            900,
        ));

        // Try swap with excessive slippage requirement
        assert_noop!(
            DEX::swap(
                RuntimeOrigin::signed(2),
                0,
                token_a.clone(),
                100,
                200, // min_out too high, will exceed slippage tolerance
            ),
            Error::<Test>::SlippageExceeded
        );
    });
}
