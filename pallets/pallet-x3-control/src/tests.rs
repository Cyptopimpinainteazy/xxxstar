//! Control-plane tests.
//!
//! The first test is the reason this pallet is in the workspace at all: it
//! failed before the authority check was handed the real origin.

use crate::mock::*;
use crate::{ControlAction, ControlAuthority, ControlDomain, ControlRecords, ControlState, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn a_stranger_cannot_claim_governance_authority() {
    new_test_ext().execute_with(|| {
        // Before the fix this call succeeded: the Governance arm checked a
        // manufactured root origin instead of the origin that signed the call,
        // so any account could freeze the chain by asking for governance.
        assert_noop!(
            crate::Pallet::<Test>::execute_control_action(
                RuntimeOrigin::signed(Stranger::get()),
                ControlDomain::Chain,
                ControlAction::Freeze,
                ControlAuthority::Governance,
            ),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn a_stranger_cannot_claim_root_authority() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            crate::Pallet::<Test>::execute_control_action(
                RuntimeOrigin::signed(Stranger::get()),
                ControlDomain::HTLC,
                ControlAction::Freeze,
                ControlAuthority::Root,
            ),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn root_may_freeze_and_thaw() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::execute_control_action(
            RuntimeOrigin::root(),
            ControlDomain::Chain,
            ControlAction::Freeze,
            ControlAuthority::Root,
        ));
        assert_eq!(
            ControlRecords::<Test>::get(ControlDomain::Chain)
                .unwrap()
                .state,
            ControlState::Frozen
        );

        frame_system::Pallet::<Test>::set_block_number(50);
        assert_ok!(crate::Pallet::<Test>::execute_control_action(
            RuntimeOrigin::root(),
            ControlDomain::Chain,
            ControlAction::Thaw,
            ControlAuthority::Root,
        ));
        assert_eq!(
            ControlRecords::<Test>::get(ControlDomain::Chain)
                .unwrap()
                .state,
            ControlState::Active
        );
    });
}

#[test]
fn an_operator_may_pause_but_not_freeze() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::execute_control_action(
            RuntimeOrigin::signed(Stranger::get()),
            ControlDomain::ProofRelay,
            ControlAction::Pause,
            ControlAuthority::Operator,
        ));
        assert_eq!(
            ControlRecords::<Test>::get(ControlDomain::ProofRelay)
                .unwrap()
                .state,
            ControlState::Paused
        );

        frame_system::Pallet::<Test>::set_block_number(100);
        // A pause is reversible; a freeze is not something an operator may ask
        // for, however it is worded.
        assert_noop!(
            crate::Pallet::<Test>::execute_control_action(
                RuntimeOrigin::signed(Stranger::get()),
                ControlDomain::Chain,
                ControlAction::Freeze,
                ControlAuthority::Operator,
            ),
            Error::<Test>::InsufficientAuthority
        );
    });
}

#[test]
fn only_committee_members_may_act_as_the_committee() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            crate::Pallet::<Test>::execute_control_action(
                RuntimeOrigin::signed(Stranger::get()),
                ControlDomain::Swarm,
                ControlAction::Quarantine,
                ControlAuthority::EmergencyCommittee,
            ),
            Error::<Test>::InsufficientAuthority
        );

        assert_ok!(crate::Pallet::<Test>::execute_control_action(
            RuntimeOrigin::signed(CommitteeMember::get()),
            ControlDomain::Swarm,
            ControlAction::Quarantine,
            ControlAuthority::EmergencyCommittee,
        ));
        assert_eq!(
            ControlRecords::<Test>::get(ControlDomain::Swarm)
                .unwrap()
                .state,
            ControlState::Quarantined
        );
    });
}

#[test]
fn the_cooldown_is_enforced_per_domain() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::execute_control_action(
            RuntimeOrigin::root(),
            ControlDomain::Swarm,
            ControlAction::Pause,
            ControlAuthority::Root,
        ));

        // Root's cooldown is zero, so the committee's is what has to bite.
        assert_ok!(crate::Pallet::<Test>::execute_control_action(
            RuntimeOrigin::signed(CommitteeMember::get()),
            ControlDomain::Chain,
            ControlAction::Pause,
            ControlAuthority::EmergencyCommittee,
        ));

        assert_noop!(
            crate::Pallet::<Test>::execute_control_action(
                RuntimeOrigin::signed(CommitteeMember::get()),
                ControlDomain::Chain,
                ControlAction::Resume,
                ControlAuthority::EmergencyCommittee,
            ),
            Error::<Test>::ActionRateLimited
        );

        // The cooldown is a delay, not a permanent block: once the committee's
        // one-block cooldown has passed, the same account may act again.
        frame_system::Pallet::<Test>::set_block_number(10);
        assert_ok!(crate::Pallet::<Test>::execute_control_action(
            RuntimeOrigin::signed(CommitteeMember::get()),
            ControlDomain::Chain,
            ControlAction::Resume,
            ControlAuthority::EmergencyCommittee,
        ));
        assert_eq!(
            ControlRecords::<Test>::get(ControlDomain::Chain)
                .unwrap()
                .state,
            ControlState::Active
        );
    });
}

#[test]
fn a_transition_that_changes_nothing_is_refused() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            crate::Pallet::<Test>::execute_control_action(
                RuntimeOrigin::root(),
                ControlDomain::Indexer,
                ControlAction::Resume,
                ControlAuthority::Root,
            ),
            Error::<Test>::InvalidStateTransition
        );
    });
}

#[test]
fn only_root_may_set_the_committee() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            crate::Pallet::<Test>::set_emergency_committee(
                RuntimeOrigin::signed(Stranger::get()),
                vec![Stranger::get()].try_into().unwrap(),
            ),
            frame_support::error::BadOrigin
        );
    });
}
