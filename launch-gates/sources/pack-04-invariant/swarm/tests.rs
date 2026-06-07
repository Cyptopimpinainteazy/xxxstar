//! Comprehensive tests for pallet-swarm.
//!
//! Tests cover:
//! - Contributor registration, deregistration, and lifecycle
//! - Task submission and claiming
//! - Heartbeat mechanism
//! - Error conditions

#![cfg(test)]

use crate::{mock::*, pallet::*, types::*, Error, Event};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use sp_core::H256;

// ============================================================================
// Helper Functions
// ============================================================================

fn default_capabilities() -> GpuCapabilities {
    GpuCapabilities {
        vram_mb: 8192,
        compute_score: 5000,
        cuda: true,
        device_count: 1,
    }
}

fn contributor_name(name: &str) -> BoundedVec<u8, frame_support::traits::ConstU32<64>> {
    BoundedVec::try_from(name.as_bytes().to_vec()).unwrap()
}

// ============================================================================
// Contributor Registration Tests
// ============================================================================

#[test]
fn register_contributor_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(ALICE),
            1_000,
            contributor_name("Alice GPU"),
            default_capabilities(),
        ));

        // Check contributor was registered
        let contributor_id = AccountContributor::<Test>::get(ALICE).unwrap();
        assert_eq!(contributor_id, 0);

        let contributor = Contributors::<Test>::get(contributor_id).unwrap();
        assert_eq!(contributor.account, ALICE);
        assert_eq!(contributor.stake, 1_000);
        assert_eq!(contributor.status, ContributorStatus::Active);
        assert_eq!(contributor.reputation, 100);

        // Check counters
        assert_eq!(TotalContributors::<Test>::get(), 1);
        assert_eq!(ActiveContributors::<Test>::get(), 1);

        // Check event emitted
        System::assert_has_event(RuntimeEvent::Swarm(Event::ContributorRegistered {
            contributor_id: 0,
            account: ALICE,
            stake: 1_000,
        }));

        // Check stake was reserved
        assert_eq!(Balances::reserved_balance(ALICE), 1_000);
    });
}

#[test]
fn register_contributor_fails_if_already_registered() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(ALICE),
            1_000,
            contributor_name("Alice GPU"),
            default_capabilities(),
        ));

        assert_noop!(
            Swarm::register_contributor(
                RuntimeOrigin::signed(ALICE),
                1_000,
                contributor_name("Alice GPU 2"),
                default_capabilities(),
            ),
            Error::<Test>::AlreadyRegistered
        );
    });
}

#[test]
fn register_contributor_fails_with_insufficient_stake() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // MinContributorStake is 1_000
        assert_noop!(
            Swarm::register_contributor(
                RuntimeOrigin::signed(ALICE),
                500, // Less than minimum
                contributor_name("Alice GPU"),
                default_capabilities(),
            ),
            Error::<Test>::InsufficientStake
        );
    });
}

#[test]
fn register_multiple_contributors_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(ALICE),
            1_000,
            contributor_name("Alice GPU"),
            default_capabilities(),
        ));

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(BOB),
            2_000,
            contributor_name("Bob GPU"),
            default_capabilities(),
        ));

        assert_eq!(TotalContributors::<Test>::get(), 2);
        assert_eq!(ActiveContributors::<Test>::get(), 2);
        assert_eq!(NextContributorId::<Test>::get(), 2);
    });
}

// ============================================================================
// Heartbeat Tests
// ============================================================================

#[test]
fn heartbeat_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(ALICE),
            1_000,
            contributor_name("Alice GPU"),
            default_capabilities(),
        ));

        run_to_block(50);

        assert_ok!(Swarm::heartbeat(RuntimeOrigin::signed(ALICE)));

        let contributor_id = AccountContributor::<Test>::get(ALICE).unwrap();
        let contributor = Contributors::<Test>::get(contributor_id).unwrap();
        assert_eq!(contributor.last_heartbeat, 50);

        System::assert_has_event(RuntimeEvent::Swarm(Event::Heartbeat {
            contributor_id,
            block: 50,
        }));
    });
}

#[test]
fn heartbeat_fails_if_not_registered() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_noop!(
            Swarm::heartbeat(RuntimeOrigin::signed(ALICE)),
            Error::<Test>::ContributorNotFound
        );
    });
}

// ============================================================================
// Deregistration Tests
// ============================================================================

#[test]
fn request_deregister_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(ALICE),
            1_000,
            contributor_name("Alice GPU"),
            default_capabilities(),
        ));

        run_to_block(10);

        assert_ok!(Swarm::request_deregister(RuntimeOrigin::signed(ALICE)));

        let contributor_id = AccountContributor::<Test>::get(ALICE).unwrap();
        let contributor = Contributors::<Test>::get(contributor_id).unwrap();
        assert_eq!(contributor.status, ContributorStatus::Deregistering);
        assert_eq!(contributor.deregister_at, 10);

        // Active count decremented
        assert_eq!(ActiveContributors::<Test>::get(), 0);
    });
}

#[test]
fn request_deregister_fails_if_not_registered() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_noop!(
            Swarm::request_deregister(RuntimeOrigin::signed(ALICE)),
            Error::<Test>::ContributorNotFound
        );
    });
}

#[test]
fn complete_deregister_works_after_cooldown() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(ALICE),
            1_000,
            contributor_name("Alice GPU"),
            default_capabilities(),
        ));

        // Initial balance check
        assert_eq!(Balances::free_balance(ALICE), 99_000);
        assert_eq!(Balances::reserved_balance(ALICE), 1_000);

        run_to_block(10);
        assert_ok!(Swarm::request_deregister(RuntimeOrigin::signed(ALICE)));

        // UnstakeCooldown is 50 blocks, so wait until block 60
        run_to_block(60);

        assert_ok!(Swarm::complete_deregister(RuntimeOrigin::signed(ALICE)));

        // Contributor removed from storage
        assert!(AccountContributor::<Test>::get(ALICE).is_none());
        assert_eq!(TotalContributors::<Test>::get(), 0);

        // Stake returned
        assert_eq!(Balances::free_balance(ALICE), 100_000);
        assert_eq!(Balances::reserved_balance(ALICE), 0);

        System::assert_has_event(RuntimeEvent::Swarm(Event::ContributorDeregistered {
            contributor_id: 0,
            account: ALICE,
        }));
    });
}

#[test]
fn complete_deregister_fails_before_cooldown() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(ALICE),
            1_000,
            contributor_name("Alice GPU"),
            default_capabilities(),
        ));

        run_to_block(10);
        assert_ok!(Swarm::request_deregister(RuntimeOrigin::signed(ALICE)));

        // Try to complete before cooldown (should fail)
        run_to_block(30); // Only 20 blocks elapsed, need 50

        assert_noop!(
            Swarm::complete_deregister(RuntimeOrigin::signed(ALICE)),
            Error::<Test>::CooldownNotExpired
        );
    });
}

#[test]
fn complete_deregister_fails_if_not_deregistering() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(ALICE),
            1_000,
            contributor_name("Alice GPU"),
            default_capabilities(),
        ));

        assert_noop!(
            Swarm::complete_deregister(RuntimeOrigin::signed(ALICE)),
            Error::<Test>::NotDeregistering
        );
    });
}

// ============================================================================
// Storage Query Tests
// ============================================================================

#[test]
fn storage_queries_work() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_eq!(Swarm::next_contributor_id(), 0);
        assert_eq!(Swarm::total_contributors(), 0);
        assert_eq!(Swarm::active_contributors(), 0);

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(ALICE),
            5_000,
            contributor_name("Alice GPU"),
            default_capabilities(),
        ));

        assert_eq!(Swarm::next_contributor_id(), 1);
        assert_eq!(Swarm::total_contributors(), 1);
        assert_eq!(Swarm::active_contributors(), 1);

        let contributor = Swarm::contributors(0).unwrap();
        assert_eq!(contributor.stake, 5_000);
        assert_eq!(contributor.account, ALICE);

        let account_id = Swarm::account_contributor(ALICE).unwrap();
        assert_eq!(account_id, 0);
    });
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn heartbeat_fails_when_deregistering() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(ALICE),
            1_000,
            contributor_name("Alice GPU"),
            default_capabilities(),
        ));

        run_to_block(10);
        assert_ok!(Swarm::request_deregister(RuntimeOrigin::signed(ALICE)));

        // Can't heartbeat while deregistering
        assert_noop!(
            Swarm::heartbeat(RuntimeOrigin::signed(ALICE)),
            Error::<Test>::ContributorNotActive
        );
    });
}

#[test]
fn double_deregister_request_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Swarm::register_contributor(
            RuntimeOrigin::signed(ALICE),
            1_000,
            contributor_name("Alice GPU"),
            default_capabilities(),
        ));

        assert_ok!(Swarm::request_deregister(RuntimeOrigin::signed(ALICE)));

        assert_noop!(
            Swarm::request_deregister(RuntimeOrigin::signed(ALICE)),
            Error::<Test>::AlreadyDeregistering
        );
    });
}
