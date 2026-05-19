//! Tests for liquidity provision functionality

use super::*;
use crate::mock::*;
use x3_dex::amm_pools::TokenId;

#[test]
fn test_liquidity_addition() {
    new_test_ext().execute_with(|| {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };

        // Create pool
        assert_ok!(DEX::create_pool(
            RuntimeOrigin::signed(1),
            token_a.clone(),
            token_b.clone(),
            30
        ));

        // Add liquidity
        assert_ok!(DEX::add_liquidity(
            RuntimeOrigin::signed(1),
            0,    // pool_id
            1000, // amount_a_desired
            1000, // amount_b_desired
            900,  // amount_a_min
            900,  // amount_b_min
        ));

        // Verify pool state updated
        let pool = DEX::pools(0);
        assert!(pool.is_some());
        let pool = pool.unwrap();
        assert_eq!(pool.reserve_a, 1000);
        assert_eq!(pool.reserve_b, 1000);
        assert!(pool.total_lp_supply > 0);
    });
}

#[test]
fn test_lp_token_minting() {
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

        // Check LP position was created
        let positions = DEX::lp_positions(0);
        assert!(positions.is_some());
    });
}

#[test]
fn test_proportional_calculations() {
    new_test_ext().execute_with(|| {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };

        // Create pool with initial liquidity
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

        // Add more liquidity - should be proportional
        assert_ok!(DEX::add_liquidity(
            RuntimeOrigin::signed(2),
            0,
            500, // half the amount
            500, // half the amount
            450,
            450,
        ));

        // Verify reserves increased proportionally
        let pool = DEX::pools(0).unwrap();
        assert_eq!(pool.reserve_a, 1500);
        assert_eq!(pool.reserve_b, 1500);
    });
}
