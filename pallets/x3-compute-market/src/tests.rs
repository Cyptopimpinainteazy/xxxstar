//! Unit tests for pallet-x3-compute-market (20 tests).

use crate::mock::*;
use crate::{
    ActiveListingCount, ActiveSessions, ComputeListings, ComputeTier, Error, Event, ExpiryQueue,
    ListingStatus, NextListingId, NextSessionId, ProviderSessions, ProviderStake, SessionStatus,
    TotalComputeRevenue,
};
use frame_support::{assert_noop, assert_ok};

// ── helpers ────────────────────────────────────────────────────────────────────

/// Stake enough for a provider to be eligible to list and return the listing id.
fn stake_and_list(provider: u64) -> u64 {
    assert_ok!(ComputeMarket::stake_as_provider(
        RuntimeOrigin::signed(provider),
        1_000_000,
    ));
    assert_ok!(ComputeMarket::create_listing(
        RuntimeOrigin::signed(provider),
        ComputeTier::BotRental,
        10, // price_per_block
        4,  // capacity_units
    ));
    NextListingId::<Test>::get() - 1
}

/// Rent from a listing for `duration` blocks and return the session id.
fn rent(renter: u64, listing_id: u64, duration: u64) -> u64 {
    assert_ok!(ComputeMarket::rent_compute(
        RuntimeOrigin::signed(renter),
        listing_id,
        duration,
    ));
    NextSessionId::<Test>::get() - 1
}

/// Advance block number through `on_initialize` for each block.
fn advance_to(block: u64) {
    for b in (System::block_number() + 1)..=block {
        System::set_block_number(b);
        ComputeMarket::on_initialize(b);
    }
}

// ── stake_as_provider ─────────────────────────────────────────────────────────

#[test]
fn stake_adds_to_provider_stake() {
    new_test_ext().execute_with(|| {
        assert_ok!(ComputeMarket::stake_as_provider(
            RuntimeOrigin::signed(1),
            500_000,
        ));
        assert_eq!(ProviderStake::<Test>::get(1), 500_000);

        // Second stake is additive.
        assert_ok!(ComputeMarket::stake_as_provider(
            RuntimeOrigin::signed(1),
            500_000,
        ));
        assert_eq!(ProviderStake::<Test>::get(1), 1_000_000);
    });
}

#[test]
fn stake_emits_provider_staked_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(ComputeMarket::stake_as_provider(
            RuntimeOrigin::signed(42),
            999,
        ));
        System::assert_has_event(RuntimeEvent::ComputeMarket(Event::ProviderStaked {
            provider: 42,
            amount: 999,
        }));
    });
}

// ── create_listing ────────────────────────────────────────────────────────────

#[test]
fn create_listing_requires_sufficient_stake() {
    new_test_ext().execute_with(|| {
        // No stake at all.
        assert_noop!(
            ComputeMarket::create_listing(RuntimeOrigin::signed(1), ComputeTier::BotRental, 10, 2,),
            Error::<Test>::InsufficientStake,
        );

        // Stake below minimum.
        assert_ok!(ComputeMarket::stake_as_provider(
            RuntimeOrigin::signed(1),
            999_999,
        ));
        assert_noop!(
            ComputeMarket::create_listing(RuntimeOrigin::signed(1), ComputeTier::BotRental, 10, 2,),
            Error::<Test>::InsufficientStake,
        );
    });
}

#[test]
fn create_listing_stores_listing() {
    new_test_ext().execute_with(|| {
        let id = stake_and_list(1);
        let listing = ComputeListings::<Test>::get(id).expect("listing stored");
        assert_eq!(listing.provider, 1);
        assert_eq!(listing.price_per_block, 10);
        assert_eq!(listing.capacity_units, 4);
        assert_eq!(listing.status, ListingStatus::Active);
        assert_eq!(listing.active_sessions, 0);
        assert_eq!(listing.total_earned, 0);
    });
}

#[test]
fn create_listing_increments_active_count() {
    new_test_ext().execute_with(|| {
        assert_eq!(ActiveListingCount::<Test>::get(), 0);
        stake_and_list(1);
        assert_eq!(ActiveListingCount::<Test>::get(), 1);
        stake_and_list(2);
        assert_eq!(ActiveListingCount::<Test>::get(), 2);
    });
}

#[test]
fn create_listing_fails_at_max_listings() {
    new_test_ext().execute_with(|| {
        for i in 1_u64..=10 {
            assert_ok!(ComputeMarket::stake_as_provider(
                RuntimeOrigin::signed(i),
                1_000_000,
            ));
            assert_ok!(ComputeMarket::create_listing(
                RuntimeOrigin::signed(i),
                ComputeTier::BotRental,
                10,
                1,
            ));
        }
        assert_ok!(ComputeMarket::stake_as_provider(
            RuntimeOrigin::signed(99),
            1_000_000,
        ));
        assert_noop!(
            ComputeMarket::create_listing(
                RuntimeOrigin::signed(99),
                ComputeTier::BotRental,
                10,
                1,
            ),
            Error::<Test>::MaxListingsReached,
        );
    });
}

// ── pause_listing / resume_listing ────────────────────────────────────────────

#[test]
fn non_owner_cannot_pause_listing() {
    new_test_ext().execute_with(|| {
        let id = stake_and_list(1);
        assert_noop!(
            ComputeMarket::pause_listing(RuntimeOrigin::signed(99), id),
            Error::<Test>::NotListingOwner,
        );
    });
}

#[test]
fn owner_can_pause_and_resume_listing() {
    new_test_ext().execute_with(|| {
        let id = stake_and_list(1);
        assert_ok!(ComputeMarket::pause_listing(RuntimeOrigin::signed(1), id));
        assert_eq!(
            ComputeListings::<Test>::get(id).unwrap().status,
            ListingStatus::Paused
        );
        System::assert_has_event(RuntimeEvent::ComputeMarket(Event::ListingPaused {
            listing_id: id,
            provider: 1,
        }));

        assert_ok!(ComputeMarket::resume_listing(RuntimeOrigin::signed(1), id));
        assert_eq!(
            ComputeListings::<Test>::get(id).unwrap().status,
            ListingStatus::Active
        );
        System::assert_has_event(RuntimeEvent::ComputeMarket(Event::ListingResumed {
            listing_id: id,
            provider: 1,
        }));
    });
}

#[test]
fn renting_from_paused_listing_fails() {
    new_test_ext().execute_with(|| {
        let id = stake_and_list(1);
        assert_ok!(ComputeMarket::pause_listing(RuntimeOrigin::signed(1), id));
        assert_noop!(
            ComputeMarket::rent_compute(RuntimeOrigin::signed(2), id, 10),
            Error::<Test>::ListingNotActive,
        );
    });
}

// ── rent_compute ──────────────────────────────────────────────────────────────

#[test]
fn rent_compute_creates_session() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 50);
        let session = ActiveSessions::<Test>::get(sid).expect("session stored");
        assert_eq!(session.renter, 2);
        assert_eq!(session.provider, 1);
        assert_eq!(session.price_per_block, 10);
        assert_eq!(session.expiry_block, 51); // block 1 + 50
        assert_eq!(session.status, SessionStatus::Active);
    });
}

#[test]
fn rent_compute_queues_session_in_expiry_queue() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 50);
        assert!(ExpiryQueue::<Test>::get(51u64, sid).is_some());
    });
}

#[test]
fn rent_compute_enforces_capacity() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1); // capacity_units = 4
        for renter in 10_u64..14 {
            assert_ok!(ComputeMarket::rent_compute(
                RuntimeOrigin::signed(renter),
                lid,
                100,
            ));
        }
        // 5th rent must fail.
        assert_noop!(
            ComputeMarket::rent_compute(RuntimeOrigin::signed(99), lid, 100),
            Error::<Test>::CapacityFull,
        );
    });
}

#[test]
fn rent_compute_adds_session_to_provider_sessions() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 10);
        let sessions = ProviderSessions::<Test>::get(1);
        assert!(sessions.contains(&sid));
    });
}

// ── complete_session ──────────────────────────────────────────────────────────

#[test]
fn complete_session_records_payment_and_updates_listing() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 50);

        // Advance 10 blocks so blocks_used = 10.
        advance_to(11);

        assert_ok!(ComputeMarket::complete_session(
            RuntimeOrigin::signed(1),
            sid,
        ));

        let session = ActiveSessions::<Test>::get(sid).unwrap();
        assert_eq!(session.status, SessionStatus::Completed);
        // price_per_block=10, blocks_used=10 → total_paid=100
        assert_eq!(session.total_paid, 100);

        let listing = ComputeListings::<Test>::get(lid).unwrap();
        assert_eq!(listing.total_earned, 100);
        assert_eq!(listing.active_sessions, 0); // decremented back

        assert_eq!(TotalComputeRevenue::<Test>::get(), 100);
    });
}

#[test]
fn complete_session_non_provider_fails() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 50);
        assert_noop!(
            ComputeMarket::complete_session(RuntimeOrigin::signed(99), sid),
            Error::<Test>::NotListingOwner,
        );
    });
}

// ── session expiry via on_initialize ─────────────────────────────────────────

#[test]
fn expired_session_transitions_to_expired() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 10); // expires at block 11

        advance_to(11);

        let session = ActiveSessions::<Test>::get(sid).unwrap();
        assert_eq!(session.status, SessionStatus::Expired);
    });
}

#[test]
fn expired_session_emits_event() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 10); // expires at block 11

        advance_to(11);

        System::assert_has_event(RuntimeEvent::ComputeMarket(Event::SessionExpired {
            session_id: sid,
        }));
    });
}

#[test]
fn expiry_decrements_listing_active_sessions() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let _sid = rent(2, lid, 10);
        assert_eq!(
            ComputeListings::<Test>::get(lid).unwrap().active_sessions,
            1
        );

        advance_to(11);

        assert_eq!(
            ComputeListings::<Test>::get(lid).unwrap().active_sessions,
            0
        );
    });
}

// ── dispute_session / resolve_dispute ─────────────────────────────────────────

#[test]
fn dispute_session_sets_disputed_status() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 50);
        assert_ok!(ComputeMarket::dispute_session(
            RuntimeOrigin::signed(2),
            sid,
        ));
        let session = ActiveSessions::<Test>::get(sid).unwrap();
        assert_eq!(session.status, SessionStatus::Disputed);
    });
}

#[test]
fn only_renter_can_dispute() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 50);
        // Provider (1) is not the renter.
        assert_noop!(
            ComputeMarket::dispute_session(RuntimeOrigin::signed(1), sid),
            Error::<Test>::NotListingOwner,
        );
    });
}

#[test]
fn resolve_dispute_renter_wins_marks_expired() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 50);
        assert_ok!(ComputeMarket::dispute_session(
            RuntimeOrigin::signed(2),
            sid,
        ));
        assert_ok!(ComputeMarket::resolve_dispute(
            RuntimeOrigin::root(),
            sid,
            true, // renter_wins
        ));
        let session = ActiveSessions::<Test>::get(sid).unwrap();
        assert_eq!(session.status, SessionStatus::Expired);
        assert_eq!(session.total_paid, 0);
        assert_eq!(TotalComputeRevenue::<Test>::get(), 0);
    });
}

#[test]
fn resolve_dispute_provider_wins_marks_completed() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 50);
        advance_to(6); // 5 blocks elapsed

        assert_ok!(ComputeMarket::dispute_session(
            RuntimeOrigin::signed(2),
            sid,
        ));
        assert_ok!(ComputeMarket::resolve_dispute(
            RuntimeOrigin::root(),
            sid,
            false, // provider wins
        ));
        let session = ActiveSessions::<Test>::get(sid).unwrap();
        assert_eq!(session.status, SessionStatus::Completed);
        // price=10, blocks_used=5 → 50
        assert_eq!(session.total_paid, 50);
        assert_eq!(TotalComputeRevenue::<Test>::get(), 50);
    });
}

#[test]
fn resolve_dispute_requires_governance_origin() {
    new_test_ext().execute_with(|| {
        let lid = stake_and_list(1);
        let sid = rent(2, lid, 50);
        assert_ok!(ComputeMarket::dispute_session(
            RuntimeOrigin::signed(2),
            sid,
        ));
        // Signed origin is not governance.
        assert_noop!(
            ComputeMarket::resolve_dispute(RuntimeOrigin::signed(99), sid, true),
            frame_support::error::BadOrigin,
        );
    });
}
