//! Unit tests for pallet-x3-dex

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use x3_dex::amm_pools::TokenId;

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
        let pools = DEX::pools();
        assert!(!pools.is_empty());
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
            0,    // pool_id
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
            0,
            100000, // amount_a_desired
            100000, // amount_b_desired
            90000,  // amount_a_min
            90000,  // amount_b_min
        ));

        // Perform swap
        assert_ok!(DEX::swap(
            RuntimeOrigin::signed(2),
            0, // pool_id
            token_a,
            1000, // amount_in
            900,  // min_out
        ));

        System::assert_has_event(RuntimeEvent::DEX(Event::SwapExecuted {
            pool_id: 0,
            amount_in: 1000,
            amount_out: 975, // Approximately 1000 * 0.997 * 100000 / (100000 + 997) ≈ 975
            user: 2,
        }));
    });
}
