//! Unit tests for pallet-x3-launchpad (20 tests).

use crate::mock::*;
use crate::{Contributions, Error, Event, LaunchStatus, Launches};
use frame_support::{assert_noop, assert_ok};

// ── helpers ────────────────────────────────────────────────────────────────────

/// Create a standard launch via root governance. Creator is account 1.
fn create_default_launch() -> u64 {
    assert_ok!(Launchpad::create_launch(
        RuntimeOrigin::root(),
        1u64,   // creator account
        1,      // token_asset_id
        500,    // soft_cap
        1000,   // hard_cap
        10,     // price_per_token
        2u64,   // start_block
        12u64,  // end_block  (duration = 10 blocks, within bounds 5..=1000)
    ));
    crate::NextLaunchId::<Test>::get() - 1
}

fn advance_to(block: u64) {
    for b in (System::block_number() + 1)..=block {
        System::set_block_number(b);
        Launchpad::on_initialize(b);
    }
}

// ── create_launch ─────────────────────────────────────────────────────────────

#[test]
fn create_launch_stores_state() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        let state = Launches::<Test>::get(id).expect("launch stored");
        assert_eq!(state.soft_cap, 500);
        assert_eq!(state.hard_cap, 1000);
        assert_eq!(state.status, LaunchStatus::Active);
        assert_eq!(state.total_raised, 0);
    });
}

#[test]
fn create_launch_emits_event() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        System::assert_has_event(RuntimeEvent::Launchpad(Event::LaunchCreated {
            launch_id: id,
            creator: 1u64, // creator passed explicitly in create_default_launch
            token_asset_id: 1,
            soft_cap: 500,
            hard_cap: 1000,
            start_block: 2,
            end_block: 12,
        }));
    });
}

#[test]
fn create_launch_increments_active_count() {
    new_test_ext().execute_with(|| {
        assert_eq!(Launchpad::active_launch_count(), 0);
        create_default_launch();
        assert_eq!(Launchpad::active_launch_count(), 1);
    });
}

#[test]
fn create_launch_fails_invalid_caps() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Launchpad::create_launch(
                RuntimeOrigin::root(),
                1u64, 1, 0, 1000, 10, 2u64, 12u64  // soft_cap == 0
            ),
            Error::<Test>::InvalidLaunchParams
        );
        assert_noop!(
            Launchpad::create_launch(
                RuntimeOrigin::root(),
                1u64, 1, 1000, 500, 10, 2u64, 12u64  // hard_cap < soft_cap
            ),
            Error::<Test>::InvalidLaunchParams
        );
    });
}

#[test]
fn create_launch_fails_duration_out_of_bounds() {
    new_test_ext().execute_with(|| {
        // Duration = 3 < MinLaunchDurationBlocks (5)
        assert_noop!(
            Launchpad::create_launch(
                RuntimeOrigin::root(),
                1u64, 1, 500, 1000, 10, 2u64, 5u64
            ),
            Error::<Test>::DurationOutOfBounds
        );
    });
}

#[test]
fn create_launch_fails_when_cap_reached() {
    new_test_ext().execute_with(|| {
        for i in 0..20u64 {
            assert_ok!(Launchpad::create_launch(
                RuntimeOrigin::root(),
                i, 1, 500, 1000, 10, 2u64, 12u64
            ));
        }
        assert_noop!(
            Launchpad::create_launch(RuntimeOrigin::root(), 99u64, 1, 500, 1000, 10, 2u64, 12u64),
            Error::<Test>::MaxLaunchesReached
        );
    });
}

#[test]
fn create_launch_requires_governance_origin() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Launchpad::create_launch(
                RuntimeOrigin::signed(1),
                1u64, 1, 500, 1000, 10, 2u64, 12u64
            ),
            frame_support::error::BadOrigin
        );
    });
}

// ── contribute ────────────────────────────────────────────────────────────────

#[test]
fn contribute_stores_amount() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 200));
        assert_eq!(Contributions::<Test>::get(id, 10u64), 200);
        let state = Launches::<Test>::get(id).unwrap();
        assert_eq!(state.total_raised, 200);
        assert_eq!(state.contributor_count, 1);
    });
}

#[test]
fn contribute_emits_event() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 300));
        System::assert_has_event(RuntimeEvent::Launchpad(Event::ContributionMade {
            launch_id: id,
            contributor: 10,
            amount: 300,
        }));
    });
}

#[test]
fn contribute_fails_when_exceeding_hard_cap() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 800));
        assert_noop!(
            Launchpad::contribute(RuntimeOrigin::signed(11), id, 300),
            Error::<Test>::HardCapExceeded
        );
    });
}

// ── finalize_launch ───────────────────────────────────────────────────────────

#[test]
fn finalize_launch_successful_when_soft_cap_met() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 600));
        advance_to(13); // past end_block 12
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        let state = Launches::<Test>::get(id).unwrap();
        assert_eq!(state.status, LaunchStatus::Successful);
    });
}

#[test]
fn finalize_launch_failed_when_soft_cap_not_met() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 400));
        advance_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        let state = Launches::<Test>::get(id).unwrap();
        assert_eq!(state.status, LaunchStatus::Failed);
    });
}

#[test]
fn finalize_launch_fails_before_end_block() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_noop!(
            Launchpad::finalize_launch(RuntimeOrigin::signed(99), id),
            Error::<Test>::LaunchNotEnded
        );
    });
}

// ── claim_refund ──────────────────────────────────────────────────────────────

#[test]
fn claim_refund_on_failed_launch() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 400));
        advance_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_ok!(Launchpad::claim_refund(RuntimeOrigin::signed(10), id));
    });
}

#[test]
fn claim_refund_double_claim_fails() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 400));
        advance_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_ok!(Launchpad::claim_refund(RuntimeOrigin::signed(10), id));
        assert_noop!(
            Launchpad::claim_refund(RuntimeOrigin::signed(10), id),
            Error::<Test>::AlreadyClaimed
        );
    });
}

#[test]
fn claim_refund_non_contributor_fails() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 400));
        advance_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_noop!(
            Launchpad::claim_refund(RuntimeOrigin::signed(99), id),
            Error::<Test>::NotContributor
        );
    });
}

// ── claim_allocation ──────────────────────────────────────────────────────────

#[test]
fn claim_allocation_on_successful_launch() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 600));
        advance_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_ok!(Launchpad::claim_allocation(RuntimeOrigin::signed(10), id));
    });
}

#[test]
fn claim_allocation_double_claim_fails() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 600));
        advance_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_ok!(Launchpad::claim_allocation(RuntimeOrigin::signed(10), id));
        assert_noop!(
            Launchpad::claim_allocation(RuntimeOrigin::signed(10), id),
            Error::<Test>::AlreadyClaimed
        );
    });
}

// ── cancel_launch ─────────────────────────────────────────────────────────────

#[test]
fn cancel_launch_by_governance_transitions_to_failed() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::cancel_launch(RuntimeOrigin::root(), id));
        let state = Launches::<Test>::get(id).unwrap();
        assert_eq!(state.status, LaunchStatus::Failed);
        assert_eq!(Launchpad::active_launch_count(), 0);
    });
}

// ── on_initialize ─────────────────────────────────────────────────────────────

#[test]
fn on_initialize_auto_finalizes_launch() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 600));
        advance_to(12); // exactly end_block triggers on_initialize auto-finalize
        let state = Launches::<Test>::get(id).unwrap();
        assert_eq!(state.status, LaunchStatus::Successful);
    });
}

// ── withdraw_raised_funds ─────────────────────────────────────────────────────

#[test]
fn withdraw_raised_funds_by_creator() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 600));
        advance_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        // creator was set to account 1 in create_default_launch.
        assert_ok!(Launchpad::withdraw_raised_funds(RuntimeOrigin::signed(1), id));
        let state = Launches::<Test>::get(id).unwrap();
        assert_eq!(state.status, LaunchStatus::Completed);
    });
}
