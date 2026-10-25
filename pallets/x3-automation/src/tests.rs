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

#[test]
fn price_threshold_task_triggers_when_oracle_crosses_threshold() {
    let asset_id = 1u32;
    // Set oracle price to 1500 (above threshold of 1000, direction: above=true).
    with_test_oracle(&[(asset_id, 1500)], || {
        new_test_ext().execute_with(|| {
            let condition = Condition::PriceThreshold {
                asset_id,
                threshold: 1000,
                above: true,
            };
            let action = Action::Custom({
                let mut a = [0u8; 64];
                a[..3].copy_from_slice(&[9, 8, 7]);
                a
            });

            assert_ok!(Automation::register_task(
                RuntimeOrigin::signed(1),
                condition,
                action,
                1000
            ));

            let task_id = Automation::account_tasks(1)[0];

            // Price is above threshold → condition should be satisfied.
            assert_ok!(Automation::execute_task(RuntimeOrigin::signed(2), task_id),);
        });
    });
}

#[test]
fn price_threshold_task_blocked_when_oracle_below_threshold() {
    let asset_id = 2u32;
    // Set oracle price to 500 (below threshold of 1000, direction: above=true).
    with_test_oracle(&[(asset_id, 500)], || {
        new_test_ext().execute_with(|| {
            let condition = Condition::PriceThreshold {
                asset_id,
                threshold: 1000,
                above: true,
            };
            let action = Action::Custom({
                let mut a = [0u8; 64];
                a[..3].copy_from_slice(&[5, 5, 5]);
                a
            });

            assert_ok!(Automation::register_task(
                RuntimeOrigin::signed(1),
                condition,
                action,
                1000
            ));

            let task_id = Automation::account_tasks(1)[0];

            // Price is below threshold → condition not met.
            assert_noop!(
                Automation::execute_task(RuntimeOrigin::signed(2), task_id),
                Error::<Test>::ConditionNotMet
            );
        });
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Regressions for the four audited execution defects.
//
// Each test below fails on the pre-hardening pallet, which reported
// `success: true` without dispatching, read a missing price as zero, and paid
// the keeper unconditionally.
// ─────────────────────────────────────────────────────────────────────────────

use crate::mock::{with_dispatch_mode, DispatchMode, MOCK_NOW_MS};
use frame_support::traits::{Currency, ReservableCurrency};
use x3_automation::{ActionOutcome, TaskStatus};

fn custom_action(tag: u8) -> Action {
    let mut a = [0u8; 64];
    a[0] = tag;
    Action::Custom(a)
}

fn free(who: u64) -> u64 {
    <Balances as Currency<u64>>::free_balance(&who)
}

fn reserved(who: u64) -> u64 {
    <Balances as ReservableCurrency<u64>>::reserved_balance(&who)
}

#[test]
fn a_refused_dispatch_is_not_recorded_as_executed_and_pays_no_execution_fee() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::Manual,
            custom_action(1),
            1000
        ));
        let task_id = Automation::account_tasks(1)[0];
        let keeper_before = free(2);
        let owner_before = free(1);

        // The attempt happened, so the call succeeds and records it.
        with_dispatch_mode(DispatchMode::Refuse, || {
            assert_ok!(Automation::execute_task(RuntimeOrigin::signed(2), task_id));
        });

        // But nothing was marked executed.
        let events = System::events();
        assert!(
            !events.iter().any(|r| matches!(
                r.event,
                RuntimeEvent::Automation(Event::TaskExecuted { .. })
            )),
            "a refused dispatch must not emit TaskExecuted"
        );
        assert!(
            events.iter().any(|r| matches!(
                r.event,
                RuntimeEvent::Automation(Event::TaskExecutionFailed { .. })
            )),
            "a refused dispatch must be recorded as a failed attempt"
        );

        // Attempt 1 of 3: the task is still Active, not Executed.
        let task = Automation::tasks(task_id).expect("task survives a refused attempt");
        assert_eq!(task.status, TaskStatus::Active);
        assert_eq!(task.attempts, 1);

        // No receipt: the trail records effects, not intentions.
        assert!(Automation::receipt(task_id, 1).is_none());

        // The keeper was paid the attempt fee, never the execution fee, and the
        // owner's registration reservation is still held.
        assert_eq!(free(2), keeper_before + ExecutionFee::get());
        assert_eq!(free(1), owner_before - ExecutionFee::get());
        assert_eq!(reserved(1), BaseRegistrationFee::get());
    });
}

#[test]
fn a_successful_dispatch_moves_funds_and_writes_a_receipt() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let recipient = 3u64;
        let mut key = [0u8; 32];
        key[..8].copy_from_slice(&recipient.to_le_bytes());
        let amount = 5_000u128;

        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::Manual,
            Action::TransferNative {
                recipient: key,
                amount
            },
            1000
        ));
        let task_id = Automation::account_tasks(1)[0];
        let owner_before = free(1);
        let recipient_before = free(recipient);
        let keeper_before = free(2);

        assert_ok!(Automation::execute_task(RuntimeOrigin::signed(2), task_id));

        // The effect really happened.
        assert_eq!(free(recipient), recipient_before + amount as u64);

        let fee = ExecutionFee::get();
        assert_eq!(free(2), keeper_before + fee);
        assert_eq!(
            free(1),
            owner_before - amount as u64 - fee + BaseRegistrationFee::get(),
            "owner pays the transfer and the execution fee; the registration \
             reservation is returned on terminal status"
        );
        // Registration reservation released on terminal status.
        assert_eq!(reserved(1), 0);

        let receipt =
            Automation::receipt(task_id, 1).expect("a dispatch that happened has a receipt");
        assert_eq!(receipt.task_id, task_id);
        assert_eq!(receipt.attempt, 1);
        assert_eq!(receipt.outcome, ActionOutcome::Applied);
        assert_eq!(receipt.block, 1);
        assert_eq!(receipt.weight_ref_time, 10_000);
        let mut expected = [0u8; 8];
        expected.copy_from_slice(&receipt.output[..8]);
        assert_eq!(u64::from_le_bytes(expected), amount as u64);

        let task = Automation::tasks(task_id).expect("terminal task is retained for audit");
        assert_eq!(task.status, TaskStatus::Executed);
    });
}

#[test]
fn a_missing_oracle_price_is_a_refusal_not_a_zero_price() {
    // The pre-hardening pallet used `unwrap_or(0)`, so this "below threshold"
    // task fired on an oracle outage: 0 < 1000. It must now refuse.
    with_test_oracle(&[], || {
        new_test_ext().execute_with(|| {
            let condition = Condition::PriceThreshold {
                asset_id: 42,
                threshold: 1000,
                above: false,
            };
            assert_ok!(Automation::register_task(
                RuntimeOrigin::signed(1),
                condition,
                custom_action(2),
                1000
            ));
            let task_id = Automation::account_tasks(1)[0];
            let keeper_before = free(2);

            assert_noop!(
                Automation::execute_task(RuntimeOrigin::signed(2), task_id),
                Error::<Test>::PriceUnavailable
            );

            // Nothing ran, so nothing was paid and no attempt was consumed.
            assert_eq!(free(2), keeper_before);
            let task = Automation::tasks(task_id).expect("task survives");
            assert_eq!(task.status, TaskStatus::Active);
            assert_eq!(task.attempts, 0);
            assert_eq!(Automation::last_attempt(task_id), 0);
            assert!(Automation::receipt(task_id, 1).is_none());
        });
    });
}

#[test]
fn a_missing_price_in_a_band_is_also_refused() {
    with_test_oracle(&[], || {
        new_test_ext().execute_with(|| {
            let condition = Condition::PriceBand {
                asset_id: 7,
                lower: 100,
                upper: 200,
            };
            assert_ok!(Automation::register_task(
                RuntimeOrigin::signed(1),
                condition,
                custom_action(3),
                1000
            ));
            let task_id = Automation::account_tasks(1)[0];

            assert_noop!(
                Automation::execute_task(RuntimeOrigin::signed(2), task_id),
                Error::<Test>::PriceUnavailable
            );
        });
    });
}

#[test]
fn price_band_triggers_only_inside_the_band() {
    let asset_id = 11u32;
    with_test_oracle(&[(asset_id, 150)], || {
        new_test_ext().execute_with(|| {
            assert_ok!(Automation::register_task(
                RuntimeOrigin::signed(1),
                Condition::PriceBand {
                    asset_id,
                    lower: 100,
                    upper: 200
                },
                Action::NoChange,
                1000
            ));
            let task_id = Automation::account_tasks(1)[0];
            assert_ok!(Automation::execute_task(RuntimeOrigin::signed(2), task_id));
            let task = Automation::tasks(task_id).unwrap();
            assert_eq!(task.status, TaskStatus::Executed);
        });
    });
    with_test_oracle(&[(asset_id, 250)], || {
        new_test_ext().execute_with(|| {
            assert_ok!(Automation::register_task(
                RuntimeOrigin::signed(1),
                Condition::PriceBand {
                    asset_id,
                    lower: 100,
                    upper: 200
                },
                Action::NoChange,
                1000
            ));
            let task_id = Automation::account_tasks(1)[0];
            assert_noop!(
                Automation::execute_task(RuntimeOrigin::signed(2), task_id),
                Error::<Test>::ConditionNotMet
            );
        });
    });
}

#[test]
fn an_inverted_price_band_cannot_be_registered() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Automation::register_task(
                RuntimeOrigin::signed(1),
                Condition::PriceBand {
                    asset_id: 1,
                    lower: 200,
                    upper: 100
                },
                Action::NoChange,
                1000
            ),
            Error::<Test>::InvalidTask
        );
    });
}

#[test]
fn repeated_failures_exhaust_the_task_after_max_attempts() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::Manual,
            custom_action(4),
            1000
        ));
        let task_id = Automation::account_tasks(1)[0];

        for expected_attempt in 1..=MaxAttempts::get() {
            with_dispatch_mode(DispatchMode::Refuse, || {
                assert_ok!(Automation::execute_task(RuntimeOrigin::signed(2), task_id));
            });
            let task = Automation::tasks(task_id).unwrap();
            assert_eq!(task.attempts, expected_attempt);
            let terminal = expected_attempt == MaxAttempts::get();
            assert_eq!(
                task.status,
                if terminal {
                    TaskStatus::Failed
                } else {
                    TaskStatus::Active
                }
            );
            if terminal {
                assert_eq!(reserved(1), 0, "terminal status releases the reservation");
            }
        }

        // Every attempt number was consumed exactly once and wrote no receipt.
        for attempt in 1..=MaxAttempts::get() {
            assert!(Automation::receipt(task_id, attempt).is_none());
            assert_eq!(Automation::last_attempt(task_id), MaxAttempts::get());
        }

        // A terminal task refuses further execution.
        assert_noop!(
            Automation::execute_task(RuntimeOrigin::signed(2), task_id),
            Error::<Test>::TaskNotActive
        );
    });
}

#[test]
fn attempt_numbers_are_unique_so_a_replay_cannot_double_pay() {
    // Two keepers racing for the same task: the first dispatch succeeds and the
    // task becomes terminal, so the second is refused before any fee moves.
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::Manual,
            Action::NoChange,
            1000
        ));
        let task_id = Automation::account_tasks(1)[0];
        let keeper_before = free(2);

        assert_ok!(Automation::execute_task(RuntimeOrigin::signed(2), task_id));
        assert_eq!(free(2), keeper_before + ExecutionFee::get());

        assert_noop!(
            Automation::execute_task(RuntimeOrigin::signed(2), task_id),
            Error::<Test>::TaskNotActive
        );
        assert_eq!(
            free(2),
            keeper_before + ExecutionFee::get(),
            "a refused replay pays nothing"
        );
        // Exactly one receipt exists.
        assert!(Automation::receipt(task_id, 1).is_some());
        assert!(Automation::receipt(task_id, 2).is_none());
    });
}

#[test]
fn a_noop_action_is_recorded_as_noop_not_as_a_transfer() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::Manual,
            Action::NoChange,
            1000
        ));
        let task_id = Automation::account_tasks(1)[0];
        let owner_before = free(1);

        assert_ok!(Automation::execute_task(RuntimeOrigin::signed(2), task_id));

        let receipt = Automation::receipt(task_id, 1).unwrap();
        assert_eq!(receipt.outcome, ActionOutcome::NoChange);
        // No funds were moved: the owner only paid the execution fee, net of
        // the registration reservation returned on terminal status.
        assert_eq!(
            free(1),
            owner_before - ExecutionFee::get() + BaseRegistrationFee::get()
        );
    });
}

#[test]
fn an_unaffordable_execution_fee_fails_the_call_instead_of_paying_zero() {
    new_test_ext().execute_with(|| {
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::Manual,
            Action::NoChange,
            0
        ));
        let task_id = Automation::account_tasks(1)[0];
        // max_fee = 0 means the owner budgeted nothing for execution.
        let keeper_before = free(2);
        assert_ok!(Automation::execute_task(RuntimeOrigin::signed(2), task_id));
        assert_eq!(free(2), keeper_before, "a zero budget pays nothing");
    });
}

#[test]
fn timestamp_trigger_uses_the_configured_clock() {
    new_test_ext().execute_with(|| {
        // Below the mock clock: fires.
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::Timestamp(MOCK_NOW_MS - 1),
            Action::NoChange,
            1000
        ));
        let due = Automation::account_tasks(1)[0];
        assert_ok!(Automation::execute_task(RuntimeOrigin::signed(2), due));

        // Above the mock clock: does not fire.
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::Timestamp(MOCK_NOW_MS + 60_000),
            Action::NoChange,
            1000
        ));
        let not_due = Automation::account_tasks(1)[0];
        assert_noop!(
            Automation::execute_task(RuntimeOrigin::signed(2), not_due),
            Error::<Test>::ConditionNotMet
        );
    });
}

#[test]
fn an_expired_task_never_dispatches_and_releases_the_reservation() {
    new_test_ext().execute_with(|| {
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::BlockNumber(50),
            Action::NoChange,
            1000
        ));
        let task_id = Automation::account_tasks(1)[0];
        assert_eq!(reserved(1), BaseRegistrationFee::get());

        System::set_block_number(100);
        let keeper_before = free(2);
        assert_ok!(Automation::execute_task(RuntimeOrigin::signed(2), task_id));

        let task = Automation::tasks(task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Expired);
        assert_eq!(reserved(1), 0);
        assert_eq!(
            free(2),
            keeper_before,
            "an expiry is not work a keeper is paid for"
        );
        assert!(Automation::receipt(task_id, 1).is_none());
    });
}

#[test]
fn cleanup_expires_stale_tasks_without_dispatching() {
    new_test_ext().execute_with(|| {
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::BlockNumber(50),
            Action::NoChange,
            1000
        ));
        let task_id = Automation::account_tasks(1)[0];
        System::set_block_number(100);
        assert_eq!(Automation::cleanup_expired_tasks(), 1);
        assert_eq!(
            Automation::tasks(task_id).unwrap().status,
            TaskStatus::Expired
        );
        assert_eq!(reserved(1), 0);
        // Idempotent.
        assert_eq!(Automation::cleanup_expired_tasks(), 0);
    });
}

#[test]
fn due_tasks_surfaces_only_active_block_triggers_that_are_ready() {
    new_test_ext().execute_with(|| {
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::BlockNumber(50),
            Action::NoChange,
            1000
        ));
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::BlockNumber(500),
            Action::NoChange,
            1000
        ));
        // A price trigger is not a time trigger and is not surfaced by a scan.
        assert_ok!(Automation::register_task(
            RuntimeOrigin::signed(1),
            Condition::PriceThreshold {
                asset_id: 1,
                threshold: 1,
                above: true
            },
            Action::NoChange,
            1000
        ));

        assert_eq!(Automation::due_tasks(1, 10).len(), 0);
        assert_eq!(Automation::due_tasks(50, 10).len(), 1);
        assert_eq!(Automation::due_tasks(500, 10).len(), 2);
        assert_eq!(
            Automation::due_tasks(500, 1).len(),
            1,
            "batch bound is honoured"
        );
    });
}

#[test]
fn event_trigger_registration_is_refused_without_a_bounded_event_index() {
    new_test_ext().execute_with(|| {
        let reserved_before = reserved(1);
        assert_noop!(
            Automation::register_task(
                RuntimeOrigin::signed(1),
                Condition::Event { topic: [0u8; 32] },
                Action::NoChange,
                1000,
            ),
            Error::<Test>::EventConditionUnavailable
        );
        assert!(Automation::account_tasks(1).is_empty());
        assert_eq!(
            reserved(1),
            reserved_before,
            "refusal must not reserve funds"
        );
    });
}
