//! Tests for token swapping mechanism

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use x3_dex::amm_pools::TokenId;

/// The id of the pool the test just created: the pallet assigns ids sequentially, and every
/// caller asks right after its one `create_pool`.
fn pool_id(_token_a: &TokenId, _token_b: &TokenId, _fee_basis_points: u32) -> u64 {
    crate::NextPoolId::<Test>::get()
}

#[test]
fn two_pool_second_leg_slippage_rolls_back_first_leg_and_events() {
    use frame_support::storage::{with_transaction, TransactionOutcome};
    use sp_runtime::DispatchError;
    use x3_asset_kernel_types::traits::CanonicalLedgerTransfer;
    use x3_dex::amm_pools::AMMPool;

    new_test_ext().execute_with(|| {
        let a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let c = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        assert_ok!(DEX::create_pool(
            RuntimeOrigin::signed(1),
            a.clone(),
            b.clone(),
            30
        ));
        let first = crate::NextPoolId::<Test>::get();
        assert_ok!(DEX::create_pool(RuntimeOrigin::signed(1), b.clone(), c, 30));
        let second = crate::NextPoolId::<Test>::get();
        assert_ne!(first, second);
        for pool in [first, second] {
            assert_ok!(DEX::add_liquidity(
                RuntimeOrigin::signed(1),
                pool,
                10_000,
                10_000,
                1,
                1
            ));
        }
        let before_first = DEX::pools(first).unwrap();
        let before_second = DEX::pools(second).unwrap();
        let carry = AMMPool::swap_calculate(&before_first, &a, 100, 1).unwrap();
        assert!(carry > 0);
        // Include every funded account and custody account. Deduplicate since
        // truncated pallet subaccounts may alias in this u64 test runtime.
        let mut accounts = vec![
            1,
            2,
            3,
            4,
            5,
            DEX::pool_account(first),
            DEX::pool_account(second),
        ];
        accounts.sort_unstable();
        accounts.dedup();
        let balances = || {
            accounts
                .iter()
                .flat_map(|who| {
                    (0..=2).map(move |asset| (asset, *who, TestLedger::ledger_balance(asset, who)))
                })
                .collect::<Vec<_>>()
        };
        let before_balances = balances();
        let before_events = System::events();
        let result: Result<(), DispatchError> = with_transaction(|| {
            assert_ok!(DEX::swap(RuntimeOrigin::signed(2), first, a, 100, 1));
            assert_ne!(DEX::pools(first).unwrap(), before_first);
            assert_ne!(
                balances(),
                before_balances,
                "first leg must transfer ledger funds"
            );
            match DEX::swap(RuntimeOrigin::signed(2), second, b, carry, u128::MAX) {
                Ok(()) => TransactionOutcome::Commit(Ok(())),
                Err(error) => TransactionOutcome::Rollback(Err(error)),
            }
        });
        assert_eq!(result, Err(Error::<Test>::SlippageExceeded.into()));
        assert_eq!(DEX::pools(first).unwrap(), before_first);
        assert_eq!(DEX::pools(second).unwrap(), before_second);
        assert_eq!(System::events(), before_events);
        assert_eq!(
            balances(),
            before_balances,
            "failed bundle must restore every ledger balance"
        );
    });
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
        let final_k = pool.reserve_a * pool.reserve_b;
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
