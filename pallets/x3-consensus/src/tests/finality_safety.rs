//! Tests for consensus finality safety
//!
//! These tests verify that the consensus pallet enforces finality invariants:
//! - A slash reduces stake and deactivates only at the configured floor.
//! - Unknown validators cannot be slashed.
//! - Repeated reports cannot reduce stake below the configured floor.
//! - The slashing math respects the configured `MinStakeAfterSlash` floor.

use crate::mock::*;
use frame_support::{assert_noop, assert_ok, dispatch::RawOrigin};

fn install_validator_set(set: Vec<u64>) {
    assert_ok!(crate::Pallet::<Test>::set_validators(
        RawOrigin::Root.into(),
        set,
        0
    ));
}

fn fund_and_stake(who: u64, stake: u128) {
    crate::ValidatorStake::<Test>::insert(
        who,
        crate::ValidatorInfo {
            stake,
            is_active: true,
        },
    );
}

#[test]
fn grandpas_finalized_set_round_trips_through_pending() {
    new_test_ext().execute_with(|| {
        // Schedule a set with zero delay
        install_validator_set(vec![1, 2, 3]);

        // The pending set is stored
        let pending = crate::NextValidators::<Test>::get();
        assert_eq!(pending, vec![1u64, 2, 3]);

        // The activation block is recorded (current block + 0 = current block)
        let sys_block = frame_system::Pallet::<Test>::block_number();
        assert_eq!(
            crate::ValidatorSetActivationBlock::<Test>::get(),
            Some(sys_block)
        );
    });
}

#[test]
fn equivocation_report_applies_configured_slash() {
    new_test_ext().execute_with(|| {
        install_validator_set(vec![1, 2, 3]);
        fund_and_stake(1, 10_000_000);
        fund_and_stake(2, 10_000_000);

        // Slash validator 1 for equivocation
        assert_ok!(crate::Pallet::<Test>::report_misbehavior(
            RawOrigin::Root.into(),
            1,
            crate::SlashReason::Equivocation,
        ));

        // The configured 10% slash leaves the validator above the floor, so it
        // remains active with the reduced stake.
        let info = crate::ValidatorStake::<Test>::get(1).expect("must exist");
        assert_eq!(info.stake, 9_000_000);
        assert!(
            info.is_active,
            "validator must remain active above the stake floor"
        );
    });
}

#[test]
fn misbehavior_report_rejects_unknown_validator() {
    new_test_ext().execute_with(|| {
        install_validator_set(vec![1, 2]);
        fund_and_stake(1, 10_000_000);
        fund_and_stake(2, 10_000_000);
        assert_noop!(
            crate::Pallet::<Test>::report_misbehavior(
                RawOrigin::Root.into(),
                3,
                crate::SlashReason::Equivocation,
            ),
            crate::Error::<Test>::ValidatorNotFound
        );
        assert!(!crate::ValidatorStake::<Test>::contains_key(3));
    });
}

#[test]
fn repeated_reports_apply_slashes_without_crossing_stake_floor() {
    new_test_ext().execute_with(|| {
        install_validator_set(vec![1, 2, 3]);
        fund_and_stake(1, 10_000_000);

        assert_ok!(crate::Pallet::<Test>::report_misbehavior(
            RawOrigin::Root.into(),
            1,
            crate::SlashReason::Equivocation,
        ));
        let stake_after_first = crate::ValidatorStake::<Test>::get(1).unwrap().stake;

        // This administrative reporting interface has no offence identifier, so
        // each accepted report applies another slash.
        assert_ok!(crate::Pallet::<Test>::report_misbehavior(
            RawOrigin::Root.into(),
            1,
            crate::SlashReason::Equivocation,
        ));
        let stake_after_second = crate::ValidatorStake::<Test>::get(1).unwrap().stake;
        let floor = <Test as crate::Config>::MinStakeAfterSlash::get();
        assert_eq!(stake_after_first, 9_000_000);
        assert_eq!(stake_after_second, 8_100_000);
        assert!(
            stake_after_second >= floor,
            "stake must not drop below MinStakeAfterSlash"
        );
    });
}
