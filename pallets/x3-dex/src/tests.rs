//! Unit tests for pallet-x3-dex

use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use x3_dex::amm_pools::{AMMPool, TokenId};

fn pool_id(token_a: &TokenId, token_b: &TokenId, fee_basis_points: u32) -> u64 {
    AMMPool::create_pool(token_a.clone(), token_b.clone(), fee_basis_points)
        .expect("valid test pool")
        .pool_id
}

#[test]
fn create_pool_works() {
    new_test_ext().execute_with(|| {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };

        assert_ok!(DEX::create_pool(
            RuntimeOrigin::signed(1),
            token_a.clone(),
            token_b.clone(),
            30
        ));

        // Check pool was created
        let pool = DEX::pools(pool_id(&token_a, &token_b, 30));
        assert!(pool.is_some());
    });
}

#[test]
fn add_liquidity_works() {
    new_test_ext().execute_with(|| {
        // Create pool first
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        assert_ok!(DEX::create_pool(
            RuntimeOrigin::signed(1),
            token_a.clone(),
            token_b.clone(),
            30
        ));

        // Add liquidity
        assert_ok!(DEX::add_liquidity(
            RuntimeOrigin::signed(1),
            pool_id(&token_a, &token_b, 30),
            1000, // amount_a_desired
            1000, // amount_b_desired
            900,  // amount_a_min
            900,  // amount_b_min
        ));
    });
}

#[test]
fn swap_works() {
    new_test_ext().execute_with(|| {
        // Create pool and add liquidity first
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        assert_ok!(DEX::create_pool(
            RuntimeOrigin::signed(1),
            token_a.clone(),
            token_b.clone(),
            30
        ));
        assert_ok!(DEX::add_liquidity(
            RuntimeOrigin::signed(1),
            pool_id(&token_a, &token_b, 30),
            100000, // amount_a_desired
            100000, // amount_b_desired
            90000,  // amount_a_min
            90000,  // amount_b_min
        ));

        // Perform swap
        assert_ok!(DEX::swap(
            RuntimeOrigin::signed(2),
            pool_id(&token_a, &token_b, 30),
            token_a.clone(),
            1000, // amount_in
            900,  // min_out
        ));

        System::assert_has_event(RuntimeEvent::DEX(Event::SwapExecuted {
            pool_id: pool_id(&token_a, &token_b, 30),
            amount_in: 1000,
            amount_out: 987,
            user: 2,
        }));
    });
}
