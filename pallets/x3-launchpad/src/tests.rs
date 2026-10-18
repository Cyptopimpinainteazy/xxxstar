//! Unit tests for pallet-x3-launchpad.

use crate::mock::*;
use crate::{Contributions, Error, Event, LaunchStatus, Launches};
use frame_support::{assert_noop, assert_ok, traits::Hooks};

// ── helpers ────────────────────────────────────────────────────────────────────

/// Create a standard launch via root governance. Creator is account 1.
fn create_default_launch() -> u32 {
    assert_ok!(Launchpad::create_launch(
        RuntimeOrigin::signed(1),
        1,     // token_asset_id
        500,   // soft_cap
        1000,  // hard_cap
        10,    // price_per_token
        1u64,  // start_block
        12u64, // end_block  (duration = 11 blocks, within bounds 5..=1000)
        5u64,  // lp_lock_duration_blocks
    ));
    crate::NextLaunchId::<Test>::get().saturating_sub(1)
}

fn advance_to(block: u64) {
    for b in (System::block_number() + 1)..=block {
        System::set_block_number(b);
        Launchpad::on_initialize(b);
    }
}

fn jump_to(block: u64) {
    System::set_block_number(block);
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
            start_block: 1,
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
                RuntimeOrigin::signed(1),
                1,
                0,
                1000,
                10,
                2u64,
                12u64, // soft_cap == 0
                5u64,
            ),
            Error::<Test>::SoftCapExceeded
        );
        assert_noop!(
            Launchpad::create_launch(
                RuntimeOrigin::signed(1),
                1,
                1000,
                500,
                10,
                2u64,
                12u64, // hard_cap < soft_cap
                5u64,
            ),
            Error::<Test>::SoftCapExceeded
        );
    });
}

#[test]
fn create_launch_fails_duration_out_of_bounds() {
    new_test_ext().execute_with(|| {
        // Duration = 3 < MinLaunchDurationBlocks (5)
        assert_noop!(
            Launchpad::create_launch(RuntimeOrigin::signed(1), 1, 500, 1000, 10, 2u64, 5u64, 5u64,),
            Error::<Test>::BadDuration
        );
    });
}

#[test]
fn create_launch_fails_when_cap_reached() {
    new_test_ext().execute_with(|| {
        for _ in 0..10u32 {
            assert_ok!(Launchpad::create_launch(
                RuntimeOrigin::signed(1),
                1,
                500,
                1000,
                10,
                2u64,
                12u64,
                5u64,
            ));
        }
        assert_noop!(
            Launchpad::create_launch(
                RuntimeOrigin::signed(1),
                1,
                500,
                1000,
                10,
                2u64,
                12u64,
                5u64,
            ),
            Error::<Test>::MaxActiveLaunchesReached
        );
    });
}

#[test]
fn create_launch_requires_signed_origin() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Launchpad::create_launch(RuntimeOrigin::none(), 1, 500, 1000, 10, 2u64, 12u64, 5u64,),
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
        jump_to(13); // past end_block 12 without auto-finalizing
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
        jump_to(13);
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
        jump_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_ok!(Launchpad::claim_refund(RuntimeOrigin::signed(10), id));
    });
}

#[test]
fn claim_refund_double_claim_fails() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 400));
        jump_to(13);
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
        jump_to(13);
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
        jump_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_noop!(
            Launchpad::claim_allocation(RuntimeOrigin::signed(10), id),
            Error::<Test>::NotGraduated
        );
        assert_ok!(Launchpad::graduate_launch(RuntimeOrigin::signed(1), id));
        assert_ok!(Launchpad::claim_allocation(RuntimeOrigin::signed(10), id));
        // 600 raised at 10 per token: the sole contributor's 60 tokens, paid from custody.
        assert_eq!(TestLedger::balance(TOKEN, 10), 60);
    });
}

#[test]
fn claim_allocation_double_claim_fails() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 600));
        jump_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_ok!(Launchpad::graduate_launch(RuntimeOrigin::signed(1), id));
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
        assert_ok!(Launchpad::cancel_launch(RuntimeOrigin::signed(1), id));
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
        jump_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        // The pool's share is set aside at graduation, so there is nothing to withdraw before it.
        assert_noop!(
            Launchpad::withdraw_raised_funds(RuntimeOrigin::signed(1), id),
            Error::<Test>::NotGraduated
        );
        assert_ok!(Launchpad::graduate_launch(RuntimeOrigin::signed(1), id));
        // creator was set to account 1 in create_default_launch.
        assert_ok!(Launchpad::withdraw_raised_funds(
            RuntimeOrigin::signed(1),
            id
        ));
        let state = Launches::<Test>::get(id).unwrap();
        assert_eq!(state.status, LaunchStatus::Completed);
        // 600 raised, 50% to the pool: the creator receives the other 300.
        assert_eq!(TestLedger::balance(QUOTE, 1), STARTING_QUOTE + 300);
    });
}

// ── graduate_launch ───────────────────────────────────────────────────────────
//
// The mock's token factory mints into the test ledger and its DEX takes real deposits and tracks
// position owners, so these tests see where every unit went.

#[test]
fn graduate_launch_happy_path() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        // Fund past soft cap to make it Successful after finalize.
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 800));
        jump_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));

        // Creator graduates the launch.
        assert_ok!(Launchpad::graduate_launch(RuntimeOrigin::signed(1), id));

        // Graduation recorded, and the pool seeded from custody: 800 raised at 10 per token,
        // 50% (400) of the quote asset and 40 tokens into the pool; 80 tokens kept for the
        // contributors; custody owns the LP position.
        assert!(crate::GraduatedLaunches::<Test>::contains_key(id));
        let custody = Launchpad::launch_account(id);
        assert_eq!(TestLedger::balance(QUOTE, POOL_ACCOUNT), 400);
        assert_eq!(TestLedger::balance(TOKEN, POOL_ACCOUNT), 40);
        assert_eq!(TestLedger::balance(TOKEN, custody), 80);
        assert_eq!(TestLedger::balance(QUOTE, custody), 400);
        assert_eq!(TestDex::owner(POSITION), Some(custody));
    });
}

#[test]
fn graduate_launch_fails_for_non_creator() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 800));
        jump_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));

        // Non-creator (account 2) cannot graduate.
        assert_noop!(
            Launchpad::graduate_launch(RuntimeOrigin::signed(2), id),
            Error::<Test>::NotCreator
        );
    });
}

#[test]
fn graduate_launch_fails_before_finalize() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 800));
        // Do NOT finalize or withdraw — launch is still Active.
        assert_noop!(
            Launchpad::graduate_launch(RuntimeOrigin::signed(1), id),
            Error::<Test>::LaunchNotSuccessful
        );
    });
}

#[test]
fn graduate_launch_fails_when_already_graduated() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 800));
        jump_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));

        // First graduation succeeds.
        assert_ok!(Launchpad::graduate_launch(RuntimeOrigin::signed(1), id));
        // Second graduation must fail with AlreadyGraduated.
        assert_noop!(
            Launchpad::graduate_launch(RuntimeOrigin::signed(1), id),
            Error::<Test>::AlreadyGraduated
        );
    });
}

// ── funds ─────────────────────────────────────────────────────────────────────

#[test]
fn a_contribution_is_paid_into_the_launchs_custody() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 600));
        assert_eq!(TestLedger::balance(QUOTE, 10), STARTING_QUOTE - 600);
        assert_eq!(
            TestLedger::balance(QUOTE, Launchpad::launch_account(id)),
            600
        );
    });
}

#[test]
fn a_contribution_the_contributor_cannot_pay_records_nothing() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        // Account 50 holds no quote asset.
        assert!(Launchpad::contribute(RuntimeOrigin::signed(50), id, 600).is_err());
        assert_eq!(Contributions::<Test>::get(id, 50), 0);
        assert_eq!(Launches::<Test>::get(id).unwrap().total_raised, 0);
    });
}

#[test]
fn a_failed_launch_refunds_real_funds() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 200));
        jump_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_ok!(Launchpad::claim_refund(RuntimeOrigin::signed(10), id));
        assert_eq!(TestLedger::balance(QUOTE, 10), STARTING_QUOTE);
        assert_eq!(TestLedger::balance(QUOTE, Launchpad::launch_account(id)), 0);
    });
}

/// Two contributors, graduation, both claims, the creator's withdrawal and the LP release: every
/// unit of the quote asset and of the token is accounted for, and custody keeps only rounding
/// dust of the contributors' tokens.
#[test]
fn a_full_launch_moves_every_unit_where_it_belongs() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        let quote_supply = TestLedger::supply(QUOTE);
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 333));
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(9), id, 667));
        jump_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_ok!(Launchpad::graduate_launch(RuntimeOrigin::signed(1), id));
        assert_ok!(Launchpad::claim_allocation(RuntimeOrigin::signed(10), id));
        assert_ok!(Launchpad::claim_allocation(RuntimeOrigin::signed(9), id));
        assert_ok!(Launchpad::withdraw_raised_funds(
            RuntimeOrigin::signed(1),
            id
        ));

        let custody = Launchpad::launch_account(id);
        // 1 000 raised at 10: 100 tokens for contributors, 500 quote + 50 tokens to the pool.
        assert_eq!(TestLedger::balance(TOKEN, 10), 33);
        assert_eq!(TestLedger::balance(TOKEN, 9), 66);
        assert_eq!(TestLedger::balance(TOKEN, POOL_ACCOUNT), 50);
        assert_eq!(TestLedger::balance(TOKEN, custody), 1, "rounding dust only");
        assert_eq!(TestLedger::balance(QUOTE, POOL_ACCOUNT), 500);
        assert_eq!(TestLedger::balance(QUOTE, 1), STARTING_QUOTE + 500);
        assert_eq!(TestLedger::balance(QUOTE, custody), 0);
        assert_eq!(
            TestLedger::supply(QUOTE),
            quote_supply,
            "no quote asset created or lost"
        );
        assert_eq!(
            TestLedger::supply(TOKEN),
            150,
            "exactly what graduation minted"
        );
    });
}

#[test]
fn the_lp_position_stays_locked_in_custody_until_the_lock_expires() {
    new_test_ext().execute_with(|| {
        let id = create_default_launch();
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 600));
        jump_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_ok!(Launchpad::graduate_launch(RuntimeOrigin::signed(1), id));
        // Graduated at block 13 with a 5-block lock.
        assert_noop!(
            Launchpad::claim_lp(RuntimeOrigin::signed(1), id),
            Error::<Test>::LpStillLocked
        );
        jump_to(18);
        assert_noop!(
            Launchpad::claim_lp(RuntimeOrigin::signed(2), id),
            Error::<Test>::NotCreator
        );
        assert_ok!(Launchpad::claim_lp(RuntimeOrigin::signed(1), id));
        assert_eq!(TestDex::owner(POSITION), Some(1));
        assert_noop!(
            Launchpad::claim_lp(RuntimeOrigin::signed(1), id),
            Error::<Test>::AlreadyClaimed
        );
    });
}

#[test]
fn a_raise_too_small_to_seed_a_pool_does_not_graduate() {
    new_test_ext().execute_with(|| {
        assert_ok!(Launchpad::create_launch(
            RuntimeOrigin::signed(1),
            1,
            5,
            1_000,
            10,
            1u64,
            12u64,
            5u64
        ));
        let id = crate::NextLaunchId::<Test>::get() - 1;
        // 15 raised at 10 per token: 1 token for contributors, but 50% (7) buys no pool token.
        assert_ok!(Launchpad::contribute(RuntimeOrigin::signed(10), id, 15));
        jump_to(13);
        assert_ok!(Launchpad::finalize_launch(RuntimeOrigin::signed(99), id));
        assert_noop!(
            Launchpad::graduate_launch(RuntimeOrigin::signed(1), id),
            Error::<Test>::RaiseTooSmallForPool
        );
    });
}
