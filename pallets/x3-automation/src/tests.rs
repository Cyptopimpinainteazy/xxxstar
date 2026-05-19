//! Unit tests for pallet-x3-automation

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;
use x3_automation::{Action, Condition};

#[test]
fn register_task_works() {
    new_test_ext().execute_with(|| {
        let condition = Condition::BlockNumber(100);
        let action = Action::Custom({
            let mut a = [0u8; 64];
            a[..3].copy_from_slice(&[1, 2, 3]);
            a
        });
        let max_fee = 1000;

        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            condition,
            action,
            max_fee
        ));

        let tasks = Automation::account_tasks(1);
        assert_eq!(tasks.len(), 1);
    });
}

#[test]
fn register_task_requires_signed_origin() {
    new_test_ext().execute_with(|| {
        let condition = Condition::BlockNumber(100);
        let action = Action::Custom({
            let mut a = [0u8; 64];
            a[..3].copy_from_slice(&[1, 2, 3]);
            a
        });

        assert_noop!(
            Automation::register_task(RuntimeOrigin::none(), condition, action, 1000),
            BadOrigin
        );
    });
}

#[test]
fn register_task_insufficient_balance() {
    new_test_ext().execute_with(|| {
        let condition = Condition::BlockNumber(100);
        let action = Action::Custom({
            let mut a = [0u8; 64];
            a[..3].copy_from_slice(&[1, 2, 3]);
            a
        });

        assert_noop!(
            Automation::register_task(RuntimeOrigin::signed(999), condition, action, 50),
            Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn cancel_task_works() {
    new_test_ext().execute_with(|| {
        // Register a task
        let condition = Condition::BlockNumber(100);
        let action = Action::Custom({
            let mut a = [0u8; 64];
            a[..3].copy_from_slice(&[1, 2, 3]);
            a
        });
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            condition,
            action,
            1000
        ));

        let task_id = Automation::account_tasks(1)[0];

        // Cancel it
        assert_ok!(Automation::cancel_task(RuntimeOrigin::signed(1), task_id));

        // Check it's removed
        assert_eq!(Automation::account_tasks(1).len(), 0);
        assert!(Automation::tasks(task_id).is_none());
    });
}

#[test]
fn cancel_task_not_owner() {
    new_test_ext().execute_with(|| {
        // Register a task with account 1
        let condition = Condition::BlockNumber(100);
        let action = Action::Custom({
            let mut a = [0u8; 64];
            a[..3].copy_from_slice(&[1, 2, 3]);
            a
        });
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            condition,
            action,
            1000
        ));

        let task_id = Automation::account_tasks(1)[0];

        // Try to cancel with account 2
        assert_noop!(
            Automation::cancel_task(RuntimeOrigin::signed(2), task_id),
            Error::<Test>::NotTaskOwner
        );
    });
}

#[test]
fn execute_task_condition_not_met() {
    new_test_ext().execute_with(|| {
        // Register a task for block 100
        let condition = Condition::BlockNumber(100);
        let action = Action::Custom({
            let mut a = [0u8; 64];
            a[..3].copy_from_slice(&[1, 2, 3]);
            a
        });
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            condition,
            action,
            1000
        ));

        let task_id = Automation::account_tasks(1)[0];

        // Try to execute at block 50 (condition not met)
        assert_noop!(
            Automation::execute_task(RuntimeOrigin::signed(2), task_id),
            Error::<Test>::ConditionNotMet
        );
    });
}
