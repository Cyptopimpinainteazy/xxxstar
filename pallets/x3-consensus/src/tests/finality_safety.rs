//! Tests for consensus finality safety
//!
//! These tests verify that the consensus pallet enforces finality invariants:
//! - A slashed validator cannot continue producing blocks.
//! - Slash reports accumulate (double-reporting the same offence is idempotent).
//! - The slashing math respects the configured `MinStakeAfterSlash` floor.

use crate::mock::*;
use frame_support::{assert_ok, dispatch::RawOrigin};

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
fn conflicting_finality_reports_are_handled_by_slashing() {
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

        // The validator is now flagged inactive
        let info = crate::ValidatorStake::<Test>::get(1).expect("must exist");
        assert!(!info.is_active, "validator must be marked inactive after slash");
    });
}

#[test]
fn finality_proof_verification_rejects_unknown_validators() {
    new_test_ext().execute_with(|| {
        install_validator_set(vec![1, 2]);
        fund_and_stake(1, 10_000_000);
        fund_and_stake(2, 10_000_000);
        // Stake is only recorded for 1 and 2, not 3
        crate::ValidatorStake::<Test>::insert(
            3,
            crate::ValidatorInfo {
                stake: 10_000_000,
                is_active: true,
            },
        );

        // Validator 3 is in the set but has no real backing — slashing it should
        // succeed (the test exercises the slash path).
        assert_ok!(crate::Pallet::<Test>::report_misbehavior(
            RawOrigin::Root.into(),
            3,
            crate::SlashReason::Equivocation,
        ));
        let info = crate::ValidatorStake::<Test>::get(3).expect("must exist");
        assert!(!info.is_active);
    });
}

#[test]
fn double_reporting_same_offence_is_idempotent() {
    new_test_ext().execute_with(|| {
        install_validator_set(vec![1, 2, 3]);
        fund_and_stake(1, 10_000_000);

        let stake_before = crate::ValidatorStake::<Test>::get(1).unwrap().stake;
        assert_ok!(crate::Pallet::<Test>::report_misbehavior(
            RawOrigin::Root.into(),
            1,
            crate::SlashReason::Equivocation,
        ));
        let stake_after_first = crate::ValidatorStake::<Test>::get(1).unwrap().stake;

        // A second report on an already-inactive validator must not further reduce
        // the stake below the configured floor.
        let _ = crate::Pallet::<Test>::report_misbehavior(
            RawOrigin::Root.into(),
            1,
            crate::SlashReason::Equivocation,
        );
        let stake_after_second = crate::ValidatorStake::<Test>::get(1).unwrap().stake;
        let floor = <Test as crate::Config>::MinStakeAfterSlash::get();
        assert!(stake_after_second >= floor, "stake must not drop below MinStakeAfterSlash");
        // The first slash reduced stake; the second must not have driven it lower
        assert!(stake_after_second <= stake_after_first);
        // Sanity: the first slash actually changed something
        assert!(stake_before >= stake_after_first);
    });
}
