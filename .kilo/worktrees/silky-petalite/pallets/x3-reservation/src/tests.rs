use crate::{mock::*, pallet::*};
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use pallet_x3_inventory::{pallet::Vaults, types::*};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn vault_id(seed: u8) -> VaultId {
    [seed; 32]
}
fn lane_id(seed: u8) -> LaneId {
    [seed; 32]
}
fn res_id(seed: u8) -> ReservationId {
    [seed; 32]
}
fn route_id(seed: u8) -> RouteId {
    [seed; 32]
}
fn snapshot(seed: u8) -> [u8; 32] {
    [seed; 32]
}

/// Create an active vault funded with `balance`.
fn setup_vault(id: VaultId, balance: u128) {
    assert_ok!(X3Inventory::create_vault(
        RuntimeOrigin::root(),
        id,
        VaultType::SettlementFloat,
        OwnerType::Protocol,
        1,
        100,
        100,   // critical_min
        500,   // min_band
        1_000, // target_band
        5_000, // max_band
    ));
    assert_ok!(X3Inventory::fund_vault(RuntimeOrigin::root(), id, balance));
}

/// Create an active lane.
fn setup_lane(id: LaneId, unsettled_cap: u128) {
    use frame_support::BoundedVec;
    let sources: BoundedVec<LiquiditySourceType, MaxLiquiditySources> =
        BoundedVec::try_from(vec![LiquiditySourceType::ProtocolFloat]).unwrap();
    assert_ok!(X3Inventory::register_lane(
        RuntimeOrigin::root(),
        id,
        1,
        2,
        100,
        200,
        LaneClass::C,
        sources,
        1_000_000,
        unsettled_cap,
    ));
}

// ---------------------------------------------------------------------------
// TICKET-4.5-005: request_reservation
// ---------------------------------------------------------------------------

#[test]
fn request_reservation_stores_state_and_emits_event() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(1);
        let lid = lane_id(1);
        let rid = res_id(1);
        let roid = route_id(1);

        setup_vault(vid, 2_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            rid,
            roid,
            vid,
            lid,
            500,
            snapshot(0xaa),
        ));

        // State stored.
        let state = Reservations::<Test>::get(rid).expect("reservation must exist");
        assert_eq!(state.status, ReservationStatus::Active);
        assert_eq!(state.amount, 500u128);
        assert_eq!(state.vault_id, vid);
        assert_eq!(state.solvency_snapshot_hash, snapshot(0xaa));

        // Route mapping stored.
        assert_eq!(ReservationsByRoute::<Test>::get(roid), Some(rid));

        // Event emitted.
        System::assert_has_event(
            Event::ReservationCreated {
                reservation_id: rid,
                route_id: roid,
                vault_id: vid,
                lane_id: lid,
                amount: 500,
                expiry_block: 1 + 100, // block 1 + TTL 100
                solvency_snapshot_hash: snapshot(0xaa),
            }
            .into(),
        );
    });
}

#[test]
fn request_reservation_locks_vault_available_balance() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(2);
        let lid = lane_id(2);
        setup_vault(vid, 1_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(2),
            route_id(2),
            vid,
            lid,
            600,
            snapshot(0),
        ));

        let vault = Vaults::<Test>::get(vid).unwrap();
        assert_eq!(vault.available_balance, 400);
        assert_eq!(vault.reserved_balance, 600);
    });
}

#[test]
fn request_reservation_increments_lane_unsettled_notional() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(3);
        let lid = lane_id(3);
        setup_vault(vid, 2_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(3),
            route_id(3),
            vid,
            lid,
            300,
            snapshot(0),
        ));

        assert_eq!(X3Inventory::lane_unsettled_notional(lid), 300u128);
        assert_eq!(X3Inventory::global_unsettled_notional(), 300u128);
    });
}

#[test]
fn request_reservation_rejects_frozen_lane() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(4);
        let lid = lane_id(4);
        setup_vault(vid, 2_000);
        setup_lane(lid, 1_000_000);

        // Freeze the lane.
        assert_ok!(X3Inventory::freeze_lane(
            RuntimeOrigin::root(),
            lid,
            FreezeReason::OperatorManual,
        ));

        assert_noop!(
            X3Reservation::request_reservation(
                RuntimeOrigin::root(),
                res_id(4),
                route_id(4),
                vid,
                lid,
                100,
                snapshot(0),
            ),
            Error::<Test>::LaneFrozen
        );
    });
}

#[test]
fn request_reservation_rejects_duplicate_route() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(5);
        let lid = lane_id(5);
        setup_vault(vid, 2_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(5),
            route_id(5),
            vid,
            lid,
            100,
            snapshot(0),
        ));

        // Same route_id, different reservation_id.
        assert_noop!(
            X3Reservation::request_reservation(
                RuntimeOrigin::root(),
                res_id(6),   // different res_id
                route_id(5), // same route_id
                vid,
                lid,
                50,
                snapshot(0),
            ),
            Error::<Test>::RouteAlreadyHasReservation
        );
    });
}

#[test]
fn request_reservation_rejects_unsettled_cap_breach() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(6);
        let lid = lane_id(6);
        setup_vault(vid, 2_000);
        // Set cap at 200 — reserving 300 should fail.
        setup_lane(lid, 200);

        assert_noop!(
            X3Reservation::request_reservation(
                RuntimeOrigin::root(),
                res_id(7),
                route_id(7),
                vid,
                lid,
                300,
                snapshot(0),
            ),
            pallet_x3_inventory::pallet::Error::<Test>::UnsettledCapExceeded
        );
    });
}

// ---------------------------------------------------------------------------
// TICKET-4.5-005: release_reservation
// ---------------------------------------------------------------------------

#[test]
fn release_reservation_restores_vault_balance() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(10);
        let lid = lane_id(10);
        setup_vault(vid, 1_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(10),
            route_id(10),
            vid,
            lid,
            400,
            snapshot(0),
        ));

        assert_ok!(X3Reservation::release_reservation(
            RuntimeOrigin::root(),
            res_id(10),
        ));

        let vault = Vaults::<Test>::get(vid).unwrap();
        assert_eq!(vault.available_balance, 1_000);
        assert_eq!(vault.reserved_balance, 0);

        let state = Reservations::<Test>::get(res_id(10)).unwrap();
        assert_eq!(state.status, ReservationStatus::Released);

        // Route mapping cleared.
        assert_eq!(ReservationsByRoute::<Test>::get(route_id(10)), None);
    });
}

#[test]
fn release_reservation_decrements_unsettled_notional() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(11);
        let lid = lane_id(11);
        setup_vault(vid, 1_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(11),
            route_id(11),
            vid,
            lid,
            250,
            snapshot(0),
        ));

        assert_ok!(X3Reservation::release_reservation(
            RuntimeOrigin::root(),
            res_id(11),
        ));

        assert_eq!(X3Inventory::lane_unsettled_notional(lid), 0u128);
        assert_eq!(X3Inventory::global_unsettled_notional(), 0u128);
    });
}

#[test]
fn release_already_released_returns_error() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(12);
        let lid = lane_id(12);
        setup_vault(vid, 1_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(12),
            route_id(12),
            vid,
            lid,
            100,
            snapshot(0),
        ));
        assert_ok!(X3Reservation::release_reservation(
            RuntimeOrigin::root(),
            res_id(12),
        ));
        assert_noop!(
            X3Reservation::release_reservation(RuntimeOrigin::root(), res_id(12)),
            Error::<Test>::ReservationAlreadyReleased
        );
    });
}

// ---------------------------------------------------------------------------
// TICKET-4.5-005: consume_reservation
// ---------------------------------------------------------------------------

#[test]
fn consume_reservation_moves_to_pending_out() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(20);
        let lid = lane_id(20);
        setup_vault(vid, 1_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(20),
            route_id(20),
            vid,
            lid,
            400,
            snapshot(0),
        ));

        assert_ok!(X3Reservation::consume_reservation(
            RuntimeOrigin::root(),
            res_id(20),
        ));

        let vault = Vaults::<Test>::get(vid).unwrap();
        assert_eq!(vault.available_balance, 600);
        assert_eq!(vault.reserved_balance, 0);
        assert_eq!(vault.pending_out_balance, 400);

        let state = Reservations::<Test>::get(res_id(20)).unwrap();
        assert_eq!(state.status, ReservationStatus::Consumed);

        assert_eq!(ReservationsByRoute::<Test>::get(route_id(20)), None);
    });
}

#[test]
fn consume_expired_reservation_returns_error() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(21);
        let lid = lane_id(21);
        setup_vault(vid, 1_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(21),
            route_id(21),
            vid,
            lid,
            100,
            snapshot(0),
        ));

        // Advance past TTL so on_initialize expires it.
        // Created at block 1, TTL=100 → expires at block 101.
        System::set_block_number(101);
        X3Reservation::on_initialize(101);

        assert_noop!(
            X3Reservation::consume_reservation(RuntimeOrigin::root(), res_id(21)),
            Error::<Test>::ReservationExpired
        );
    });
}

// ---------------------------------------------------------------------------
// TICKET-4.5-005: on_initialize expiry
// ---------------------------------------------------------------------------

#[test]
fn on_initialize_expires_reservations_at_expiry_block() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(30);
        let lid = lane_id(30);
        setup_vault(vid, 2_000);
        setup_lane(lid, 1_000_000);

        // Created at block 1 → expires at block 101.
        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(30),
            route_id(30),
            vid,
            lid,
            500,
            snapshot(0xbb),
        ));

        // Advance to expiry block.
        System::set_block_number(101);
        X3Reservation::on_initialize(101);

        // Vault balance restored.
        let vault = Vaults::<Test>::get(vid).unwrap();
        assert_eq!(vault.available_balance, 2_000);
        assert_eq!(vault.reserved_balance, 0);

        // Status transitioned.
        let state = Reservations::<Test>::get(res_id(30)).unwrap();
        assert_eq!(state.status, ReservationStatus::Expired);

        // Route mapping cleared.
        assert_eq!(ReservationsByRoute::<Test>::get(route_id(30)), None);

        // Unsettled notional decremented.
        assert_eq!(X3Inventory::lane_unsettled_notional(lid), 0u128);

        // Event emitted.
        System::assert_has_event(
            Event::ReservationExpired {
                reservation_id: res_id(30),
                route_id: route_id(30),
                solvency_snapshot_hash: snapshot(0xbb),
            }
            .into(),
        );
    });
}

#[test]
fn on_initialize_does_not_expire_before_expiry_block() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(31);
        let lid = lane_id(31);
        setup_vault(vid, 1_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(31),
            route_id(31),
            vid,
            lid,
            200,
            snapshot(0),
        ));

        // Run on_initialize before TTL passes.
        System::set_block_number(50);
        X3Reservation::on_initialize(50);

        // Reservation still active.
        let state = Reservations::<Test>::get(res_id(31)).unwrap();
        assert_eq!(state.status, ReservationStatus::Active);

        let vault = Vaults::<Test>::get(vid).unwrap();
        assert_eq!(vault.reserved_balance, 200);
    });
}

// ---------------------------------------------------------------------------
// TICKET-4.5-005: is_reservation_valid
// ---------------------------------------------------------------------------

#[test]
fn is_reservation_valid_returns_true_for_active_unexpired() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(40);
        let lid = lane_id(40);
        setup_vault(vid, 1_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(40),
            route_id(40),
            vid,
            lid,
            100,
            snapshot(0),
        ));

        assert!(X3Reservation::is_reservation_valid(res_id(40)));
    });
}

#[test]
fn is_reservation_valid_returns_false_after_consumption() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(41);
        let lid = lane_id(41);
        setup_vault(vid, 1_000);
        setup_lane(lid, 1_000_000);

        assert_ok!(X3Reservation::request_reservation(
            RuntimeOrigin::root(),
            res_id(41),
            route_id(41),
            vid,
            lid,
            100,
            snapshot(0),
        ));
        assert_ok!(X3Reservation::consume_reservation(
            RuntimeOrigin::root(),
            res_id(41),
        ));

        assert!(!X3Reservation::is_reservation_valid(res_id(41)));
    });
}

#[test]
fn is_reservation_valid_returns_false_for_unknown() {
    new_test_ext().execute_with(|| {
        assert!(!X3Reservation::is_reservation_valid(res_id(0xff)));
    });
}
