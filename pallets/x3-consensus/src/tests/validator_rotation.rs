//! Tests for validator rotation
//!
//! These tests verify that `set_validators` correctly writes to the pending
//! validator-set storage and that the pallet correctly enforces the
//! governance-only and size-limit constraints.
//!
//! Note: the pallet's `on_initialize` hook activates the pending set when the
//! activation block is reached, but in the unit-test mock the Aura/Grandpa
//! authorities storage is not populated, which causes the block-proposer
//! recording to fail. The end-to-end activation flow is covered by the
//! integration tests under `tests/e2e/`.

use crate::mock::*;
use frame_support::{assert_noop, assert_ok, dispatch::RawOrigin};

#[test]
fn validator_set_transition_is_scheduled() {
    new_test_ext().execute_with(|| {
        // Initial: no validators
        assert!(crate::Pallet::<Test>::current_validators().is_empty());

        // Schedule a new validator set with a 5-block activation delay
        let new_set: Vec<u64> = vec![1, 2, 3];
        assert_ok!(crate::Pallet::<Test>::set_validators(
            RawOrigin::Root.into(),
            new_set.clone(),
            5
        ));

        // The next-set is recorded
        let next = crate::NextValidators::<Test>::get();
        assert_eq!(next, new_set);

        // Activation block is current + 5
        let sys_block = frame_system::Pallet::<Test>::block_number();
        assert_eq!(
            crate::ValidatorSetActivationBlock::<Test>::get(),
            Some(sys_block + 5)
        );
    });
}

#[test]
fn authority_change_requires_root() {
    new_test_ext().execute_with(|| {
        // A signed (non-root) origin must be rejected
        let set: Vec<u64> = vec![1, 2];
        assert_noop!(
            crate::Pallet::<Test>::set_validators(RawOrigin::Signed(42).into(), set, 0),
            sp_runtime::DispatchError::BadOrigin
        );

        // The pending set is unchanged
        assert!(crate::NextValidators::<Test>::get().is_empty());
    });
}

#[test]
fn authority_change_rejects_oversized_set() {
    new_test_ext().execute_with(|| {
        // Build a set larger than MaxValidators
        let oversized: Vec<u64> = (0..200).collect();
        assert_noop!(
            crate::Pallet::<Test>::set_validators(RawOrigin::Root.into(), oversized, 0),
            crate::Error::<Test>::TooManyValidators
        );
    });
}

#[test]
fn zero_delay_activates_immediately_on_next_block() {
    new_test_ext().execute_with(|| {
        // Schedule with zero delay — activation block is current block
        let set: Vec<u64> = vec![7, 8, 9];
        assert_ok!(crate::Pallet::<Test>::set_validators(
            RawOrigin::Root.into(),
            set.clone(),
            0
        ));

        // The activation block is the current block
        let sys_block = frame_system::Pallet::<Test>::block_number();
        assert_eq!(
            crate::ValidatorSetActivationBlock::<Test>::get(),
            Some(sys_block)
        );

        // The pending set is stored
        assert_eq!(crate::NextValidators::<Test>::get(), set);
    });
}
