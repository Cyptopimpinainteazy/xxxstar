//! Tests for real stake-deduction slashing via `report_misbehavior`.
//!
//! Each test populates [`ValidatorStake`] directly, calls the extrinsic, then
//! asserts on both the resulting storage state and the emitted events.

use crate::{
    mock::*,
    pallet::{Error, Event, ValidatorInfo, ValidatorStake},
    SlashReason,
};
use frame_support::{assert_noop, assert_ok};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Insert a `ValidatorInfo` with the given stake and `is_active = true`.
fn register(who: u64, stake: u128) {
    ValidatorStake::<Test>::insert(who, ValidatorInfo { stake, is_active: true });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// `report_misbehavior` must reduce the validator's stake by exactly
/// `SlashFraction` basis points (10 % in the mock, i.e. 1 000 bps).
///
/// Starting stake: 10_000_000.
/// Expected slash:  1_000_000  (10 %).
/// Expected remaining: 9_000_000 — above `MinStakeAfterSlash` (1_000_000),
///   so `is_active` stays `true`.
#[test]
fn slash_reduces_stake_by_fraction() {
    new_test_ext().execute_with(|| {
        // Events are suppressed at block 0 by FRAME; advance to block 1.
        System::set_block_number(1);

        let validator: u64 = 1;
        let initial_stake: u128 = 10_000_000;
        register(validator, initial_stake);

        assert_ok!(Consensus::report_misbehavior(
            RuntimeOrigin::signed(42),
            validator,
            SlashReason::DoubleSign,
        ));

        let info = ValidatorStake::<Test>::get(validator).expect("validator must still exist");
        assert_eq!(info.stake, 9_000_000, "stake should be reduced by 10 %");
        assert!(info.is_active, "validator should remain active above the floor");

        // Confirm SlashApplied event carries correct amounts.
        System::assert_has_event(
            Event::SlashApplied {
                validator,
                slash_amount: 1_000_000,
                new_stake: 9_000_000,
            }
            .into(),
        );
    });
}

/// When the computed post-slash stake would fall below `MinStakeAfterSlash`,
/// the stored stake is clamped to the minimum rather than going lower.
///
/// Starting stake: 1_050_000.
/// Computed slash: 105_000 (10 %).
/// Naïve result:   945_000 — below `MinStakeAfterSlash` (1_000_000).
/// Expected stored stake: 1_000_000 (the floor).
/// Actual slash deducted: 50_000 (only down to the floor).
#[test]
fn slash_floors_at_min_stake() {
    new_test_ext().execute_with(|| {
        // Events are suppressed at block 0 by FRAME; advance to block 1.
        System::set_block_number(1);

        let validator: u64 = 2;
        register(validator, 1_050_000);

        assert_ok!(Consensus::report_misbehavior(
            RuntimeOrigin::signed(99),
            validator,
            SlashReason::MissingBlocks,
        ));

        let info = ValidatorStake::<Test>::get(validator).unwrap();
        assert_eq!(info.stake, 1_000_000, "stake must not drop below MinStakeAfterSlash");
        // `is_active` should be false because we landed exactly at the floor.
        assert!(!info.is_active, "validator at the floor must be deactivated");

        System::assert_has_event(
            Event::SlashApplied {
                validator,
                slash_amount: 50_000,  // 1_050_000 - 1_000_000
                new_stake: 1_000_000,
            }
            .into(),
        );
    });
}

/// A validator that is slashed exactly to `MinStakeAfterSlash` must be
/// marked `is_active = false`.
///
/// Starting stake: 10_000_000.
/// After 10 % slash: 9_000_000 — well above the floor, active = true.
/// After a second slash from 1_100_000: floor clamped, active = false.
#[test]
fn slash_marks_validator_inactive_at_min_stake() {
    new_test_ext().execute_with(|| {
        let validator: u64 = 3;
        // Start just above the floor so one slash lands us exactly on it.
        // MinStakeAfterSlash = 1_000_000, SlashFraction = 10 %.
        // need:  stake - (stake * 0.10)  <  1_000_000
        // i.e.   0.90 * stake < 1_000_000  →  stake < 1_111_111.
        register(validator, 1_100_000);

        assert_ok!(Consensus::report_misbehavior(
            RuntimeOrigin::signed(7),
            validator,
            SlashReason::Equivocation,
        ));

        let info = ValidatorStake::<Test>::get(validator).unwrap();
        // 1_100_000 * 10% = 110_000 slash → 990_000 naïve → clamped to 1_000_000.
        assert_eq!(info.stake, 1_000_000);
        assert!(!info.is_active, "validator should be deactivated when stake hits the floor");
    });
}

/// Reporting a misbehavior for an account that has no entry in
/// [`ValidatorStake`] must return `Error::ValidatorNotFound` without
/// mutating any storage.
#[test]
fn slash_on_nonexistent_validator_returns_error() {
    new_test_ext().execute_with(|| {
        let ghost: u64 = 999;
        // No `register(ghost, ...)` — the map entry is absent.

        assert_noop!(
            Consensus::report_misbehavior(
                RuntimeOrigin::signed(1),
                ghost,
                SlashReason::InvalidFinality,
            ),
            Error::<Test>::ValidatorNotFound,
        );

        // Nothing should have been written.
        assert!(ValidatorStake::<Test>::get(ghost).is_none());
    });
}
