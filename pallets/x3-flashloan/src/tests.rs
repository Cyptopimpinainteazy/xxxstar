use crate::mock::*;
use crate::pallet::{ActiveLoan, PoolBalance};
use frame_support::{assert_noop, assert_ok};

#[test]
fn test_borrow_and_repay_succeeds() {
    new_test_ext().execute_with(|| {
        // Borrow 10_000
        assert_ok!(FlashLoan::open_native_loan(1, 10_000));
        assert!(ActiveLoan::<Test>::get().is_some());

        // Repay: 10_000 + fee (10_000 * 9 / 10_000 = 9)
        assert_ok!(FlashLoan::repay(RuntimeOrigin::signed(1), 10_009));
        assert!(ActiveLoan::<Test>::get().is_none());
    });
}

#[test]
fn test_borrow_zero_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            FlashLoan::open_native_loan(1, 0),
            crate::pallet::Error::<Test>::ZeroAmount
        );
    });
}

#[test]
fn test_double_borrow_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(FlashLoan::open_native_loan(1, 1_000));
        assert_noop!(
            FlashLoan::open_native_loan(2, 1_000),
            crate::pallet::Error::<Test>::LoanAlreadyActive
        );
    });
}

#[test]
fn test_repay_insufficient_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(FlashLoan::open_native_loan(1, 10_000));
        // Fee = 9, so need 10_009; provide 10_000 (less than required)
        assert_noop!(
            FlashLoan::repay(RuntimeOrigin::signed(1), 10_000),
            crate::pallet::Error::<Test>::RepaymentInsufficient
        );
    });
}

#[test]
fn test_repay_with_no_active_loan_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            FlashLoan::repay(RuntimeOrigin::signed(1), 1_000),
            crate::pallet::Error::<Test>::NoActiveLoan
        );
    });
}

#[test]
fn test_repay_by_wrong_account_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(FlashLoan::open_native_loan(1, 1_000));
        assert_noop!(
            FlashLoan::repay(RuntimeOrigin::signed(2), 1_001),
            crate::pallet::Error::<Test>::NoActiveLoan
        );
    });
}

#[test]
fn test_add_liquidity() {
    new_test_ext().execute_with(|| {
        let initial = PoolBalance::<Test>::get();
        assert_ok!(FlashLoan::add_liquidity(RuntimeOrigin::signed(3), 50_000));
        assert_eq!(PoolBalance::<Test>::get(), initial + 50_000);
    });
}

#[test]
fn test_check_no_pending_loans_passes_when_clean() {
    new_test_ext().execute_with(|| {
        assert!(FlashLoan::check_no_pending_loans().is_ok());
    });
}

#[test]
fn test_check_no_pending_loans_fails_with_active_loan() {
    new_test_ext().execute_with(|| {
        assert_ok!(FlashLoan::open_native_loan(1, 1_000));
        assert!(FlashLoan::check_no_pending_loans().is_err());
    });
}

#[test]
fn test_pool_balance_restored_after_repay() {
    new_test_ext().execute_with(|| {
        let initial_pool = PoolBalance::<Test>::get();
        assert_ok!(FlashLoan::open_native_loan(1, 10_000));
        assert_eq!(PoolBalance::<Test>::get(), initial_pool - 10_000);
        assert_ok!(FlashLoan::repay(RuntimeOrigin::signed(1), 10_009));
        // Pool has initial - 10_000 + 10_009 = initial + 9 (fee)
        assert_eq!(PoolBalance::<Test>::get(), initial_pool + 9);
    });
}

// ---------------------------------------------------------------- the borrow refusal

#[test]
fn the_native_borrow_is_refused_and_moves_nothing() {
    new_test_ext().execute_with(|| {
        let pool_before = Balances::free_balance(FlashLoan::pool_account());
        assert_noop!(
            FlashLoan::borrow(RuntimeOrigin::signed(1), 10_000),
            crate::pallet::Error::<Test>::FlashLoansRequireAtomicTrade
        );
        assert!(ActiveLoan::<Test>::get().is_none());
        assert_eq!(
            Balances::free_balance(FlashLoan::pool_account()),
            pool_before
        );
    });
}

// ---------------------------------------------------------------- asset pools

use crate::pallet::{AssetPools, AssetShares};
use x3_asset_kernel_types::traits::CanonicalLedgerTransfer;

const ASSET: u32 = 7;

#[test]
fn only_the_pool_origin_opens_an_asset_pool() {
    new_test_ext().execute_with(|| {
        assert!(FlashLoan::create_asset_pool(RuntimeOrigin::signed(1), ASSET, 5).is_err());
        assert_ok!(FlashLoan::create_asset_pool(
            RuntimeOrigin::root(),
            ASSET,
            5
        ));
        assert_noop!(
            FlashLoan::create_asset_pool(RuntimeOrigin::root(), ASSET, 5),
            crate::pallet::Error::<Test>::AssetPoolExists
        );
        assert_noop!(
            FlashLoan::create_asset_pool(RuntimeOrigin::root(), ASSET + 1, 10_001),
            crate::pallet::Error::<Test>::FeeOutOfRange
        );
    });
}

#[test]
fn providers_deposit_for_shares_and_redeem_fees_pro_rata() {
    new_test_ext().execute_with(|| {
        assert_ok!(FlashLoan::create_asset_pool(
            RuntimeOrigin::root(),
            ASSET,
            5
        ));
        assert_ok!(FlashLoan::provide_asset_liquidity(
            RuntimeOrigin::signed(1),
            ASSET,
            100_000,
            0
        ));
        assert_ok!(FlashLoan::provide_asset_liquidity(
            RuntimeOrigin::signed(2),
            ASSET,
            50_000,
            50_000
        ));
        let account = FlashLoan::asset_pool_account(ASSET);
        assert_eq!(TestLedger::ledger_balance(ASSET, &account), 150_000);
        // The first deposit locks `MINIMUM_LOCKED_SHARES` of its shares for good.
        assert_eq!(
            AssetShares::<Test>::get(ASSET, 1),
            100_000 - crate::MINIMUM_LOCKED_SHARES
        );
        assert_eq!(AssetShares::<Test>::get(ASSET, 2), 50_000);
        assert_eq!(FlashLoan::asset_pool(ASSET), Some((5, 150_000)));

        // Fees paid into the pool (as a repaid trade does) accrue to shares.
        TestLedger::fund(ASSET, account, 3_000);
        assert_ok!(FlashLoan::withdraw_asset_liquidity(
            RuntimeOrigin::signed(2),
            ASSET,
            50_000
        ));
        assert_eq!(
            TestLedger::ledger_balance(ASSET, &2),
            1_000_000 - 50_000 + 51_000
        );
        assert_eq!(
            AssetPools::<Test>::get(ASSET).expect("pool").total_shares,
            100_000
        );
        assert_noop!(
            FlashLoan::withdraw_asset_liquidity(RuntimeOrigin::signed(2), ASSET, 1),
            crate::pallet::Error::<Test>::InsufficientShares
        );
    });
}

#[test]
fn a_deposit_the_provider_cannot_fund_mints_nothing() {
    new_test_ext().execute_with(|| {
        assert_ok!(FlashLoan::create_asset_pool(
            RuntimeOrigin::root(),
            ASSET,
            5
        ));
        assert!(
            FlashLoan::provide_asset_liquidity(RuntimeOrigin::signed(9), ASSET, 10_000, 0).is_err()
        );
        assert_eq!(AssetShares::<Test>::get(ASSET, 9), 0);
        assert_eq!(
            AssetPools::<Test>::get(ASSET).expect("pool").total_shares,
            0
        );
    });
}

/// The first-depositor inflation attack: a dust first deposit, then fees paid straight into the
/// pool to raise the share price, so the next deposit rounds down to a fraction of its value.
/// With shares locked by the first deposit the attacker pays for nearly all of that inflation, and
/// the victim loses less than one share to rounding; `min_shares` refuses anything worse.
#[test]
fn a_dust_first_deposit_cannot_inflate_the_share_price_against_the_next_depositor() {
    new_test_ext().execute_with(|| {
        assert_ok!(FlashLoan::create_asset_pool(
            RuntimeOrigin::root(),
            ASSET,
            5
        ));
        let account = FlashLoan::asset_pool_account(ASSET);
        let (attacker, victim) = (1u64, 2u64);
        assert_noop!(
            FlashLoan::provide_asset_liquidity(
                RuntimeOrigin::signed(attacker),
                ASSET,
                crate::MINIMUM_LOCKED_SHARES,
                0
            ),
            crate::pallet::Error::<Test>::FirstDepositTooSmall
        );
        assert_ok!(FlashLoan::provide_asset_liquidity(
            RuntimeOrigin::signed(attacker),
            ASSET,
            crate::MINIMUM_LOCKED_SHARES + 1,
            0
        ));
        assert_eq!(AssetShares::<Test>::get(ASSET, attacker), 1);
        // The attacker inflates the pool, as repaid flash fees would.
        let donation = 400_000u128;
        assert_ok!(TestLedger::ledger_transfer(
            ASSET, &attacker, &account, donation
        ));

        let deposit = 500_000u128;
        let total = AssetPools::<Test>::get(ASSET).expect("pool").total_shares;
        let liquidity = TestLedger::ledger_balance(ASSET, &account);
        let fair = deposit * total / liquidity;
        // Asking for one share more than the price allows is refused, not silently rounded.
        assert_noop!(
            FlashLoan::provide_asset_liquidity(
                RuntimeOrigin::signed(victim),
                ASSET,
                deposit,
                fair + 1
            ),
            crate::pallet::Error::<Test>::SharesBelowMinimum
        );
        assert_ok!(FlashLoan::provide_asset_liquidity(
            RuntimeOrigin::signed(victim),
            ASSET,
            deposit,
            fair
        ));
        let victim_shares = AssetShares::<Test>::get(ASSET, victim);
        let before = TestLedger::ledger_balance(ASSET, &victim);
        assert_ok!(FlashLoan::withdraw_asset_liquidity(
            RuntimeOrigin::signed(victim),
            ASSET,
            victim_shares
        ));
        let redeemed = TestLedger::ledger_balance(ASSET, &victim) - before;
        let share_price = liquidity / total + 1;
        assert!(
            redeemed + share_price >= deposit,
            "the victim lost {} of {deposit}: more than one share",
            deposit - redeemed
        );
        // And the attacker cannot get the donation back: its one share is worth a sliver of it.
        let attacker_before = TestLedger::ledger_balance(ASSET, &attacker);
        assert_ok!(FlashLoan::withdraw_asset_liquidity(
            RuntimeOrigin::signed(attacker),
            ASSET,
            1
        ));
        let recovered = TestLedger::ledger_balance(ASSET, &attacker) - attacker_before;
        assert!(
            recovered < donation / 100,
            "recovered {recovered} of {donation}"
        );
    });
}
