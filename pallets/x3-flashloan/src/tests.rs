use crate::mock::*;
use crate::pallet::{ActiveLoan, PoolBalance};
use frame_support::{assert_noop, assert_ok};

#[test]
fn test_borrow_and_repay_succeeds() {
    new_test_ext().execute_with(|| {
        // Borrow 10_000
        assert_ok!(FlashLoan::borrow(RuntimeOrigin::signed(1), 10_000));
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
            FlashLoan::borrow(RuntimeOrigin::signed(1), 0),
            crate::pallet::Error::<Test>::ZeroAmount
        );
    });
}

#[test]
fn test_double_borrow_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(FlashLoan::borrow(RuntimeOrigin::signed(1), 1_000));
        assert_noop!(
            FlashLoan::borrow(RuntimeOrigin::signed(2), 1_000),
            crate::pallet::Error::<Test>::LoanAlreadyActive
        );
    });
}

#[test]
fn test_repay_insufficient_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(FlashLoan::borrow(RuntimeOrigin::signed(1), 10_000));
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
        assert_ok!(FlashLoan::borrow(RuntimeOrigin::signed(1), 1_000));
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
        assert_ok!(FlashLoan::borrow(RuntimeOrigin::signed(1), 1_000));
        assert!(FlashLoan::check_no_pending_loans().is_err());
    });
}

#[test]
fn test_pool_balance_restored_after_repay() {
    new_test_ext().execute_with(|| {
        let initial_pool = PoolBalance::<Test>::get();
        assert_ok!(FlashLoan::borrow(RuntimeOrigin::signed(1), 10_000));
        assert_eq!(PoolBalance::<Test>::get(), initial_pool - 10_000);
        assert_ok!(FlashLoan::repay(RuntimeOrigin::signed(1), 10_009));
        // Pool has initial - 10_000 + 10_009 = initial + 9 (fee)
        assert_eq!(PoolBalance::<Test>::get(), initial_pool + 9);
    });
}
