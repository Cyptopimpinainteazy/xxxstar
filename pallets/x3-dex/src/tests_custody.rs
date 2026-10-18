//! Custody: pool reserves are ledger balances held by the pool's account, positions have owners,
//! and pools from before custody are frozen.

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use x3_asset_kernel_types::traits::CanonicalLedgerTransfer;
use x3_dex::amm_pools::{AMMPool, LiquidityPool, TokenId};

fn token(asset_id: u128) -> TokenId {
    TokenId {
        chain_id: 1,
        asset_id,
    }
}

fn funded_pool() -> u64 {
    let (a, b) = (token(0), token(1));
    assert_ok!(DEX::create_pool(
        RuntimeOrigin::signed(1),
        a.clone(),
        b.clone(),
        30
    ));
    let pool_id = AMMPool::create_pool(a, b, 30).expect("pool").pool_id;
    assert_ok!(DEX::add_liquidity(
        RuntimeOrigin::signed(1),
        pool_id,
        100_000,
        50_000,
        0,
        0
    ));
    pool_id
}

fn balance(asset: u32, who: u64) -> u128 {
    TestLedger::ledger_balance(asset, &who)
}

#[test]
fn liquidity_moves_the_providers_funds_into_the_pool() {
    new_test_ext().execute_with(|| {
        let pool_id = funded_pool();
        let account = DEX::pool_account(pool_id);
        assert_eq!(balance(0, account), 100_000);
        assert_eq!(balance(1, account), 50_000);
        assert_eq!(balance(0, 1), STARTING_BALANCE - 100_000);
        assert_eq!(balance(1, 1), STARTING_BALANCE - 50_000);
    });
}

#[test]
fn a_swap_pays_the_input_in_and_the_output_out() {
    new_test_ext().execute_with(|| {
        let pool_id = funded_pool();
        let account = DEX::pool_account(pool_id);
        let pool: LiquidityPool = DEX::pools(pool_id).expect("pool");
        let out = AMMPool::swap_calculate(&pool, &token(0), 1_000, 1).expect("prices");
        assert_ok!(DEX::swap(
            RuntimeOrigin::signed(2),
            pool_id,
            token(0),
            1_000,
            1
        ));
        assert_eq!(balance(0, 2), STARTING_BALANCE - 1_000);
        assert_eq!(balance(1, 2), STARTING_BALANCE + out);
        assert_eq!(balance(0, account), 101_000);
        assert_eq!(balance(1, account), 50_000 - out);
        let after = DEX::pools(pool_id).expect("pool");
        assert_eq!((after.reserve_a, after.reserve_b), (101_000, 50_000 - out));
    });
}

#[test]
fn a_swap_the_trader_cannot_fund_moves_nothing() {
    new_test_ext().execute_with(|| {
        let pool_id = funded_pool();
        let before = DEX::pools(pool_id).expect("pool");
        // Account 9 holds nothing.
        assert!(DEX::swap(RuntimeOrigin::signed(9), pool_id, token(0), 1_000, 1).is_err());
        assert_eq!(DEX::pools(pool_id).expect("pool"), before);
    });
}

#[test]
fn only_the_owner_withdraws_a_position_and_is_paid_out() {
    new_test_ext().execute_with(|| {
        let pool_id = funded_pool();
        let position = 0;
        assert_noop!(
            DEX::remove_liquidity(RuntimeOrigin::signed(2), position, 1_000, 0, 0),
            Error::<Test>::NotPositionOwner
        );
        let lp = DEX::lp_positions(position).expect("position").lp_balance;
        assert_ok!(DEX::remove_liquidity(
            RuntimeOrigin::signed(1),
            position,
            lp,
            0,
            0
        ));
        assert_eq!(balance(0, 1), STARTING_BALANCE);
        assert_eq!(balance(1, 1), STARTING_BALANCE);
        assert_eq!(balance(0, DEX::pool_account(pool_id)), 0);
    });
}

#[test]
fn a_position_cannot_withdraw_more_than_it_holds() {
    new_test_ext().execute_with(|| {
        funded_pool();
        let lp = DEX::lp_positions(0).expect("position").lp_balance;
        assert_noop!(
            DEX::remove_liquidity(RuntimeOrigin::signed(1), 0, lp + 1, 0, 0),
            Error::<Test>::InsufficientLiquidity
        );
    });
}

#[test]
fn a_pool_from_before_custody_is_frozen() {
    new_test_ext().execute_with(|| {
        let pool = AMMPool::create_pool(token(2), token(3), 30).expect("pool");
        let pool_id = pool.pool_id;
        // Written as the old pallet wrote it: reserves, no custody.
        crate::Pools::<Test>::insert(
            pool_id,
            LiquidityPool {
                reserve_a: 1_000,
                reserve_b: 1_000,
                total_lp_supply: 1_000,
                ..pool
            },
        );
        assert_noop!(
            DEX::swap(RuntimeOrigin::signed(2), pool_id, token(2), 10, 1),
            Error::<Test>::LegacyPoolWithoutCustody
        );
        assert_noop!(
            DEX::add_liquidity(RuntimeOrigin::signed(2), pool_id, 10, 10, 0, 0),
            Error::<Test>::LegacyPoolWithoutCustody
        );
        assert!(DEX::trading_pool(pool_id).is_none());
    });
}

#[test]
fn a_token_off_the_ledger_cannot_be_pooled() {
    new_test_ext().execute_with(|| {
        let off_ledger = TokenId {
            chain_id: 2,
            asset_id: 0,
        };
        assert_noop!(
            DEX::create_pool(RuntimeOrigin::signed(1), off_ledger, token(1), 30),
            Error::<Test>::TokenNotOnLedger
        );
        let too_wide = token(u128::from(u32::MAX) + 1);
        assert_noop!(
            DEX::create_pool(RuntimeOrigin::signed(1), too_wide, token(1), 30),
            Error::<Test>::TokenNotOnLedger
        );
    });
}

#[test]
fn trading_reserves_are_refused_beyond_custody() {
    new_test_ext().execute_with(|| {
        let pool_id = funded_pool();
        assert_noop!(
            DEX::apply_trading_reserves(pool_id, 100_001, 50_000),
            Error::<Test>::ReserveExceedsCustody
        );
        assert_ok!(DEX::apply_trading_reserves(pool_id, 99_000, 50_000));
        let view = DEX::trading_pool(pool_id).expect("custodied");
        assert_eq!((view.asset_a, view.asset_b, view.reserve_a), (0, 1, 99_000));
    });
}
