// SPDX-License-Identifier: Apache-2.0
//
// Tests for pallet-x3-lp-locker.

use crate::{mock::*, Error, Event, LpLockRecord, LpLocks};
use frame_support::{assert_noop, assert_ok};

#[test]
fn lock_lp_creates_lock() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);

        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(1),
            42,     // pool_id
            1000,   // lp_amount
            200,    // unlock_at_block (duration = 199 blocks, >= 100 min)
            vec![], // description
        ));

        let record = LpLocks::<Test>::get(1, 42).unwrap();
        assert_eq!(record.owner, 1);
        assert_eq!(record.pool_id, 42);
        assert_eq!(record.lp_amount, 1000);
        assert_eq!(record.unlock_at_block, 200);
        assert_eq!(record.locked_at_block, 1);

        // Verify event was emitted
        System::assert_has_event(
            Event::LpLocked {
                owner: 1,
                pool_id: 42,
                lp_amount: 1000,
                unlock_at_block: 200,
            }
            .into(),
        );
    });
}

#[test]
fn lock_lp_rejects_zero_amount() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);

        assert_noop!(
            LpLocker::lock_lp(RuntimeOrigin::signed(1), 42, 0, 200, vec![]),
            Error::<Test>::ZeroAmount
        );
    });
}

#[test]
fn lock_lp_rejects_duplicate() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);

        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(1),
            42,
            1000,
            200,
            vec![]
        ));

        assert_noop!(
            LpLocker::lock_lp(RuntimeOrigin::signed(1), 42, 2000, 300, vec![]),
            Error::<Test>::AlreadyLocked
        );
    });
}

#[test]
fn lock_lp_rejects_short_duration() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);

        // Duration = 50 blocks < MinLockDuration (100)
        assert_noop!(
            LpLocker::lock_lp(RuntimeOrigin::signed(1), 42, 1000, 51, vec![]),
            Error::<Test>::DurationBelowMinimum
        );
    });
}

#[test]
fn lock_lp_rejects_long_duration() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);

        // Duration = 200_000 blocks > MaxLockDuration (100_000)
        assert_noop!(
            LpLocker::lock_lp(RuntimeOrigin::signed(1), 42, 1000, 200_001, vec![]),
            Error::<Test>::DurationAboveMaximum
        );
    });
}

#[test]
fn lock_lp_rejects_long_description() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);
        let long_desc = vec![0u8; 129]; // > 128 bytes

        assert_noop!(
            LpLocker::lock_lp(RuntimeOrigin::signed(1), 42, 1000, 200, long_desc),
            Error::<Test>::DescriptionTooLong
        );
    });
}

#[test]
fn unlock_lp_removes_lock_after_expiry() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);
        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(1),
            42,
            1000,
            200,
            vec![]
        ));

        // Advance past unlock block
        frame_system::Pallet::<Test>::set_block_number(200);

        assert_ok!(LpLocker::unlock_lp(RuntimeOrigin::signed(1), 42));
        assert!(LpLocks::<Test>::get(1, 42).is_none());

        System::assert_has_event(
            Event::LpUnlocked {
                owner: 1,
                pool_id: 42,
                lp_amount: 1000,
            }
            .into(),
        );
    });
}

#[test]
fn unlock_lp_rejects_before_expiry() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);
        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(1),
            42,
            1000,
            200,
            vec![]
        ));

        // At block 150, lock has not expired
        frame_system::Pallet::<Test>::set_block_number(150);

        assert_noop!(
            LpLocker::unlock_lp(RuntimeOrigin::signed(1), 42),
            Error::<Test>::LockNotExpired
        );
    });
}

#[test]
fn unlock_lp_rejects_nonexistent() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            LpLocker::unlock_lp(RuntimeOrigin::signed(1), 99),
            Error::<Test>::NotFound
        );
    });
}

#[test]
fn extend_lock_increases_duration() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);
        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(1),
            42,
            1000,
            200,
            vec![]
        ));

        assert_ok!(LpLocker::extend_lock(RuntimeOrigin::signed(1), 42, 500));

        let record = LpLocks::<Test>::get(1, 42).unwrap();
        assert_eq!(record.unlock_at_block, 500);
    });
}

#[test]
fn extend_lock_rejects_shorten() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);
        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(1),
            42,
            1000,
            200,
            vec![]
        ));

        assert_noop!(
            LpLocker::extend_lock(RuntimeOrigin::signed(1), 42, 150),
            Error::<Test>::CannotShortenLock
        );
    });
}

#[test]
fn extend_lock_rejects_nonexistent() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            LpLocker::extend_lock(RuntimeOrigin::signed(1), 99, 500),
            Error::<Test>::NotFound
        );
    });
}

#[test]
fn increase_lock_adds_amount() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);
        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(1),
            42,
            1000,
            200,
            vec![]
        ));

        assert_ok!(LpLocker::increase_lock(RuntimeOrigin::signed(1), 42, 500));

        let record = LpLocks::<Test>::get(1, 42).unwrap();
        assert_eq!(record.lp_amount, 1500);
    });
}

#[test]
fn increase_lock_rejects_nonexistent() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            LpLocker::increase_lock(RuntimeOrigin::signed(1), 99, 500),
            Error::<Test>::NotFound
        );
    });
}

#[test]
fn increase_lock_rejects_zero_amount() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);
        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(1),
            42,
            1000,
            200,
            vec![]
        ));

        assert_noop!(
            LpLocker::increase_lock(RuntimeOrigin::signed(1), 42, 0),
            Error::<Test>::ZeroAmount
        );
    });
}

#[test]
fn is_locked_returns_correct_state() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);
        assert!(!LpLocker::is_locked(&1, 42)); // no lock yet

        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(1),
            42,
            1000,
            200,
            vec![]
        ));
        assert!(LpLocker::is_locked(&1, 42)); // lock active

        frame_system::Pallet::<Test>::set_block_number(200);
        assert!(!LpLocker::is_locked(&1, 42)); // lock expired
    });
}

#[test]
fn total_locked_for_pool_aggregates() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);

        // Multiple users lock LP in same pool
        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(1),
            42,
            1000,
            200,
            vec![]
        ));
        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(2),
            42,
            500,
            300,
            vec![]
        ));
        assert_ok!(LpLocker::lock_lp(
            RuntimeOrigin::signed(1),
            7,
            2000,
            200,
            vec![] // different pool
        ));

        assert_eq!(LpLocker::total_locked_for_pool(42), 1500);
        assert_eq!(LpLocker::total_locked_for_pool(7), 2000);
        assert_eq!(LpLocker::total_locked_for_pool(99), 0); // nonexistent pool
    });
}
