//! Tests for token swapping mechanism

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use x3_dex::amm_pools::{AMMPool, TokenId};

fn pool_id(token_a: &TokenId, token_b: &TokenId, fee_basis_points: u32) -> u64 {
    AMMPool::create_pool(token_a.clone(), token_b.clone(), fee_basis_points)
        .expect("valid test pool")
        .pool_id
}

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
            pool_id(&token_a, &token_b, 30),
            1000,
            1000,
            900,
            900,
        ));

        // Execute swap
        assert_ok!(DEX::swap(
            RuntimeOrigin::signed(2),
            pool_id(&token_a, &token_b, 30),
            token_a.clone(),
            100, // amount_in
            80,  // min_out
        ));

        // Verify pool reserves changed
        let pool = DEX::pools(pool_id(&token_a, &token_b, 30)).unwrap();
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
            pool_id(&token_a, &token_b, 30),
            1000,
            1000,
            900,
            900,
        ));

        let initial_k = 1000u128 * 1000u128; // Constant product invariant

        // Execute swap
        assert_ok!(DEX::swap(
            RuntimeOrigin::signed(2),
            pool_id(&token_a, &token_b, 30),
            token_a.clone(),
            100,
            80,
        ));

        // Verify invariant is preserved (approximately, accounting for fees)
        let pool = DEX::pools(pool_id(&token_a, &token_b, 30)).unwrap();
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
            pool_id(&token_a, &token_b, 30),
            1000,
            1000,
            900,
            900,
        ));

        // Try swap with excessive slippage requirement
        assert_noop!(
            DEX::swap(
                RuntimeOrigin::signed(2),
                pool_id(&token_a, &token_b, 30),
                token_a.clone(),
                100,
                200, // min_out too high, will exceed slippage tolerance
            ),
            Error::<Test>::SlippageExceeded
        );
    });
}

#[test]
fn two_pool_second_leg_slippage_rolls_back_first_leg_and_events() {
    use frame_support::storage::{with_transaction, TransactionOutcome};
    use sp_runtime::DispatchError;

    new_test_ext().execute_with(|| {
        let a = TokenId {
            chain_id: 1,
            asset_id: 10,
        };
        let b = TokenId {
            chain_id: 1,
            asset_id: 11,
        };
        let c = TokenId {
            chain_id: 1,
            asset_id: 12,
        };
        let first = pool_id(&a, &b, 30);
        let second = pool_id(&b, &c, 30);

        assert_ok!(DEX::create_pool(
            RuntimeOrigin::signed(1),
            a.clone(),
            b.clone(),
            30
        ));
        assert_ok!(DEX::create_pool(
            RuntimeOrigin::signed(1),
            b.clone(),
            c.clone(),
            30
        ));
        for id in [first, second] {
            assert_ok!(DEX::add_liquidity(
                RuntimeOrigin::signed(1),
                id,
                10_000,
                10_000,
                1,
                1
            ));
        }

        let before_first = DEX::pools(first).expect("first pool exists");
        let before_second = DEX::pools(second).expect("second pool exists");
        let events_before = System::events();

        let result: Result<(), DispatchError> = with_transaction(|| {
            if let Err(err) = DEX::swap(RuntimeOrigin::signed(2), first, a.clone(), 100, 1) {
                return TransactionOutcome::Rollback(Err(err));
            }
            assert_ne!(
                DEX::pools(first).expect("first pool remains"),
                before_first,
                "first leg must mutate state inside transaction"
            );
            match DEX::swap(RuntimeOrigin::signed(2), second, b.clone(), 100, u128::MAX) {
                Ok(()) => TransactionOutcome::Commit(Ok(())),
                Err(err) => TransactionOutcome::Rollback(Err(err)),
            }
        });

        assert!(result.is_err(), "second-leg impossible minimum must fail");
        assert_eq!(DEX::pools(first).unwrap().reserve_a, before_first.reserve_a);
        assert_eq!(DEX::pools(first).unwrap().reserve_b, before_first.reserve_b);
        assert_eq!(
            DEX::pools(second).unwrap().reserve_a,
            before_second.reserve_a
        );
        assert_eq!(
            DEX::pools(second).unwrap().reserve_b,
            before_second.reserve_b
        );
        assert_eq!(
            System::events(),
            events_before,
            "failed bundle must not leak swap events"
        );
    });
}
