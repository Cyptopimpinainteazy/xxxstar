//! Unit tests for pallet-x3-auction (18 tests).

use crate::mock::*;
use crate::{AuctionStatus, Auctions, Error, Event, ExpiryQueue};
use frame_support::{assert_noop, assert_ok};
use frame_system::pallet_prelude::BlockNumberFor;

// ── helpers ────────────────────────────────────────────────────────────────────

fn create_default_auction(seller: u64) -> u64 {
    assert_ok!(Auction::create_auction(
        RuntimeOrigin::signed(seller),
        1,          // asset_id
        100,        // start_price
        200,        // reserve_price
        10u64,      // duration_blocks
    ));
    crate::NextAuctionId::<Test>::get() - 1
}

fn advance_to(block: u64) {
    for b in (System::block_number() + 1)..=block {
        System::set_block_number(b);
        Auction::on_initialize(b);
    }
}

// ── create_auction ─────────────────────────────────────────────────────────────

#[test]
fn create_auction_stores_state() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        let state = Auctions::<Test>::get(id).expect("auction stored");
        assert_eq!(state.seller, 1);
        assert_eq!(state.asset_id, 1);
        assert_eq!(state.start_price, 100);
        assert_eq!(state.reserve_price, 200);
        assert_eq!(state.status, AuctionStatus::Active);
        assert_eq!(state.bid_count, 0);
    });
}

#[test]
fn create_auction_emits_event() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        System::assert_has_event(RuntimeEvent::Auction(Event::AuctionCreated {
            auction_id: id,
            seller: 1,
            asset_id: 1,
            start_price: 100,
            reserve_price: 200,
            end_block: 11,
        }));
    });
}

#[test]
fn create_auction_increments_active_count() {
    new_test_ext().execute_with(|| {
        assert_eq!(Auction::active_auction_count(), 0);
        create_default_auction(1);
        assert_eq!(Auction::active_auction_count(), 1);
        create_default_auction(2);
        assert_eq!(Auction::active_auction_count(), 2);
    });
}

#[test]
fn create_auction_fails_when_cap_reached() {
    new_test_ext().execute_with(|| {
        for i in 0..50 {
            assert_ok!(Auction::create_auction(
                RuntimeOrigin::signed(i as u64),
                i,
                100,
                200,
                10u64,
            ));
        }
        assert_noop!(
            Auction::create_auction(RuntimeOrigin::signed(99), 99, 100, 200, 10u64),
            Error::<Test>::MaxActiveAuctionsReached
        );
    });
}

// ── place_bid ─────────────────────────────────────────────────────────────────

#[test]
fn place_bid_works_first_bid() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_ok!(Auction::place_bid(RuntimeOrigin::signed(2), id, 100));
        let state = Auctions::<Test>::get(id).unwrap();
        assert_eq!(state.current_bid, 100);
        assert_eq!(state.leading_bidder, Some(2));
        assert_eq!(state.bid_count, 1);
    });
}

#[test]
fn place_bid_emits_event() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_ok!(Auction::place_bid(RuntimeOrigin::signed(2), id, 150));
        System::assert_has_event(RuntimeEvent::Auction(Event::BidPlaced {
            auction_id: id,
            bidder: 2,
            amount: 150,
        }));
    });
}

#[test]
fn place_bid_enforces_min_increment() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        // First bid sets current_bid to 150.
        assert_ok!(Auction::place_bid(RuntimeOrigin::signed(2), id, 150));
        // Next bid must be >= 150 + 1% = 151.5 → 152 (integer truncation gives 151).
        assert_noop!(
            Auction::place_bid(RuntimeOrigin::signed(3), id, 150),
            Error::<Test>::BidTooLow
        );
        assert_ok!(Auction::place_bid(RuntimeOrigin::signed(3), id, 152));
    });
}

#[test]
fn place_bid_on_nonexistent_auction_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Auction::place_bid(RuntimeOrigin::signed(1), 999, 100),
            Error::<Test>::AuctionNotFound
        );
    });
}

// ── cancel_auction ────────────────────────────────────────────────────────────

#[test]
fn cancel_auction_by_seller_with_no_bids() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_ok!(Auction::cancel_auction(RuntimeOrigin::signed(1), id));
        let state = Auctions::<Test>::get(id).unwrap();
        assert_eq!(state.status, AuctionStatus::Cancelled);
        assert_eq!(Auction::active_auction_count(), 0);
    });
}

#[test]
fn cancel_auction_fails_with_bids() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_ok!(Auction::place_bid(RuntimeOrigin::signed(2), id, 100));
        assert_noop!(
            Auction::cancel_auction(RuntimeOrigin::signed(1), id),
            Error::<Test>::AuctionHasBids
        );
    });
}

#[test]
fn cancel_auction_fails_for_non_seller() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_noop!(
            Auction::cancel_auction(RuntimeOrigin::signed(2), id),
            Error::<Test>::NotSeller
        );
    });
}

// ── settle_auction ────────────────────────────────────────────────────────────

#[test]
fn settle_auction_succeeds_when_reserve_met() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_ok!(Auction::place_bid(RuntimeOrigin::signed(2), id, 200));
        advance_to(12);
        assert_ok!(Auction::settle_auction(RuntimeOrigin::signed(3), id));
        let state = Auctions::<Test>::get(id).unwrap();
        assert_eq!(state.status, AuctionStatus::Settled);
    });
}

#[test]
fn settle_auction_fails_when_reserve_not_met() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_ok!(Auction::place_bid(RuntimeOrigin::signed(2), id, 150));
        advance_to(12);
        assert_noop!(
            Auction::settle_auction(RuntimeOrigin::signed(3), id),
            Error::<Test>::ReservePriceNotMet
        );
    });
}

#[test]
fn settle_auction_fails_while_still_active() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_ok!(Auction::place_bid(RuntimeOrigin::signed(2), id, 250));
        assert_noop!(
            Auction::settle_auction(RuntimeOrigin::signed(3), id),
            Error::<Test>::AuctionNotEnded
        );
    });
}

// ── extend_auction ────────────────────────────────────────────────────────────

#[test]
fn extend_auction_by_governance() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        let before = Auctions::<Test>::get(id).unwrap().end_block;
        assert_ok!(Auction::extend_auction(RuntimeOrigin::root(), id, 5u64));
        let after = Auctions::<Test>::get(id).unwrap().end_block;
        assert_eq!(after, before + 5);
    });
}

#[test]
fn extend_auction_requires_governance_origin() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_noop!(
            Auction::extend_auction(RuntimeOrigin::signed(1), id, 5u64),
            frame_support::error::BadOrigin
        );
    });
}

// ── force_cancel ─────────────────────────────────────────────────────────────

#[test]
fn force_cancel_by_governance() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_ok!(Auction::place_bid(RuntimeOrigin::signed(2), id, 100));
        assert_ok!(Auction::force_cancel(RuntimeOrigin::root(), id));
        let state = Auctions::<Test>::get(id).unwrap();
        assert_eq!(state.status, AuctionStatus::Cancelled);
    });
}

#[test]
fn force_cancel_requires_governance_origin() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_noop!(
            Auction::force_cancel(RuntimeOrigin::signed(1), id),
            frame_support::error::BadOrigin
        );
    });
}

// ── on_initialize ─────────────────────────────────────────────────────────────

#[test]
fn on_initialize_transitions_active_to_ended() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        advance_to(11);
        let state = Auctions::<Test>::get(id).unwrap();
        assert_eq!(state.status, AuctionStatus::Ended);
        assert_eq!(Auction::active_auction_count(), 0);
    });
}

#[test]
fn on_initialize_emits_auction_ended_event() {
    new_test_ext().execute_with(|| {
        let id = create_default_auction(1);
        assert_ok!(Auction::place_bid(RuntimeOrigin::signed(2), id, 250));
        advance_to(11);
        System::assert_has_event(RuntimeEvent::Auction(Event::AuctionEnded {
            auction_id: id,
            winner: Some(2),
            final_price: 250,
        }));
    });
}
