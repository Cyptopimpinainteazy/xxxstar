use crate::mock::*;
use crate::pallet::{
    EvidenceRecords, PendingObligationCount, PendingObligations, SolvencySnapshots,
};
use crate::{Error, Event};
use frame_support::{assert_ok, traits::{ConstU32, Hooks}, BoundedVec};
use pallet_x3_inventory::{
    pallet::Pallet as InventoryPallet,
    types::{LaneClass, LaneId, LiquiditySourceType, OwnerType, ReservationId, ReservationStatus, RouteId, VaultId, VaultType},
};
use pallet_x3_reservation::pallet::{Pallet as ReservationPallet, ReservationState};
use crate::types::{
    PostSubmissionContext, QuoteContext, ReservationContext, SubmissionContext,
};

// ──────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────

fn vault_id(n: u8) -> VaultId {
    let mut id = [0u8; 32];
    id[0] = n;
    id
}

fn lane_id(n: u8) -> LaneId {
    let mut id = [0u8; 32];
    id[0] = n;
    id
}

fn route_id(n: u8) -> RouteId {
    let mut id = [0u8; 32];
    id[0] = n;
    id
}

fn reservation_id(n: u8) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = n;
    id
}

fn setup_vault_and_lane(v_id: VaultId, l_id: LaneId) {
    assert_ok!(pallet_x3_inventory::Pallet::<Test>::create_vault(
        frame_system::RawOrigin::Root.into(),
        v_id,
        VaultType::SettlementFloat,
        OwnerType::Protocol,
        1u32,
        1u32,
        0u128, // critical_min
        50u128, // min_band
        100u128, // target_band
        200u128, // max_band
    ));
    assert_ok!(pallet_x3_inventory::Pallet::<Test>::fund_vault(
        frame_system::RawOrigin::Root.into(),
        v_id,
        1000u128,
    ));
    let allowed: BoundedVec<LiquiditySourceType, MaxLiquiditySources> =
        BoundedVec::try_from(vec![LiquiditySourceType::ProtocolFloat]).unwrap();
    assert_ok!(pallet_x3_inventory::Pallet::<Test>::register_lane(
        frame_system::RawOrigin::Root.into(),
        l_id,
        1u32,
        2u32,
        1u32,
        2u32,
        LaneClass::A,
        allowed,
        5000u128,
        5000u128,
    ));
}

// ──────────────────────────────────────────
// check_pre_quote tests (TICKET-4.5-007)
// ──────────────────────────────────────────

#[test]
fn pre_quote_passes_for_healthy_vault_and_lane() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(1);
        let l_id = lane_id(1);
        let r_id = route_id(1);
        setup_vault_and_lane(v_id, l_id);

        let ctx = QuoteContext {
            lane_id: l_id,
            vault_id: v_id,
            amount: 100u128,
            route_id: r_id,
        };

        let result = crate::pallet::Pallet::<Test>::check_pre_quote(&ctx);
        assert!(result.passed);
        assert!(result.failed_checks.is_empty());

        System::assert_has_event(RuntimeEvent::X3Solvency(Event::SolvencyGateChecked {
            route_id: r_id,
            gate: crate::GateKind::PreQuote,
            passed: true,
            snapshot_hash: result.snapshot_hash,
        }));
    });
}

#[test]
fn pre_quote_fails_when_lane_not_found() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(1);
        let l_id = lane_id(99); // not created
        let r_id = route_id(1);

        // vault still exists
        assert_ok!(pallet_x3_inventory::Pallet::<Test>::create_vault(
            frame_system::RawOrigin::Root.into(),
            v_id,
            VaultType::SettlementFloat,
            OwnerType::Protocol,
            1u32,
            1u32,
            0u128, 50u128, 100u128, 200u128,
        ));

        let ctx = QuoteContext { lane_id: l_id, vault_id: v_id, amount: 50u128, route_id: r_id };
        let result = crate::pallet::Pallet::<Test>::check_pre_quote(&ctx);
        assert!(!result.passed);
        assert!(result.failed_checks.contains(&crate::types::SolvencyCheck::LaneFrozen));
    });
}

#[test]
fn pre_quote_fails_when_vault_has_insufficient_balance() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(2);
        let l_id = lane_id(2);
        let r_id = route_id(2);
        setup_vault_and_lane(v_id, l_id);

        let ctx = QuoteContext {
            lane_id: l_id,
            vault_id: v_id,
            amount: 9999u128, // more than funded (1000)
            route_id: r_id,
        };
        let result = crate::pallet::Pallet::<Test>::check_pre_quote(&ctx);
        assert!(!result.passed);
        assert!(result.failed_checks.contains(&crate::types::SolvencyCheck::InsufficientVault));
    });
}

#[test]
fn pre_quote_fails_when_unsettled_cap_breached() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(3);
        let l_id = lane_id(3);
        let r_id = route_id(3);
        // register lane with tiny unsettled_cap
        assert_ok!(pallet_x3_inventory::Pallet::<Test>::create_vault(
            frame_system::RawOrigin::Root.into(),
            v_id,
            VaultType::SettlementFloat,
            OwnerType::Protocol,
            1u32, 1u32,
            0u128, 50u128, 100u128, 200u128,
        ));
        assert_ok!(pallet_x3_inventory::Pallet::<Test>::fund_vault(
            frame_system::RawOrigin::Root.into(), v_id, 1000u128,
        ));
        let allowed: BoundedVec<LiquiditySourceType, MaxLiquiditySources> =
            BoundedVec::try_from(vec![LiquiditySourceType::ProtocolFloat]).unwrap();
        assert_ok!(pallet_x3_inventory::Pallet::<Test>::register_lane(
            frame_system::RawOrigin::Root.into(),
            l_id, 1u32, 2u32, 1u32, 2u32, LaneClass::A,
            allowed, 5000u128, 10u128, // unsettled_cap = 10
        ));

        let ctx = QuoteContext {
            lane_id: l_id, vault_id: v_id, amount: 100u128, route_id: r_id,
        };
        let result = crate::pallet::Pallet::<Test>::check_pre_quote(&ctx);
        assert!(!result.passed);
        assert!(result.failed_checks.contains(&crate::types::SolvencyCheck::UnsettledCapBreached));
    });
}

#[test]
fn pre_quote_snapshot_is_stored() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(4);
        let l_id = lane_id(4);
        let r_id = route_id(4);
        setup_vault_and_lane(v_id, l_id);

        let ctx = QuoteContext { lane_id: l_id, vault_id: v_id, amount: 10u128, route_id: r_id };
        let result = crate::pallet::Pallet::<Test>::check_pre_quote(&ctx);

        let snapshot = SolvencySnapshots::<Test>::get(result.snapshot_hash);
        assert!(snapshot.is_some());
        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.passed, result.passed);
        assert_eq!(snapshot.route_id, r_id);
    });
}

// ──────────────────────────────────────────
// check_pre_reservation tests (TICKET-4.5-007)
// ──────────────────────────────────────────

#[test]
fn pre_reservation_passes_for_new_route() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(5);
        let l_id = lane_id(5);
        let r_id = route_id(5);
        setup_vault_and_lane(v_id, l_id);

        let ctx = ReservationContext {
            lane_id: l_id, vault_id: v_id, amount: 50u128, route_id: r_id,
        };
        let result = crate::pallet::Pallet::<Test>::check_pre_reservation(&ctx);
        assert!(result.passed);
        assert!(result.failed_checks.is_empty());
    });
}

#[test]
fn pre_reservation_fails_for_duplicate_route() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(6);
        let l_id = lane_id(6);
        let r_id = route_id(6);
        setup_vault_and_lane(v_id, l_id);

        // Create a reservation for this route via the reservation pallet directly
        pallet_x3_reservation::pallet::ReservationsByRoute::<Test>::insert(r_id, reservation_id(6));

        let ctx = ReservationContext {
            lane_id: l_id, vault_id: v_id, amount: 50u128, route_id: r_id,
        };
        let result = crate::pallet::Pallet::<Test>::check_pre_reservation(&ctx);
        assert!(!result.passed);
        assert!(result.failed_checks.contains(&crate::types::SolvencyCheck::RouteDuplicate));
    });
}

// ──────────────────────────────────────────
// check_pre_submission tests (TICKET-4.5-008)
// ──────────────────────────────────────────

#[test]
fn pre_submission_passes_with_valid_context() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(7);
        let l_id = lane_id(7);
        let r_id = route_id(7);
        let res_id = reservation_id(7);
        setup_vault_and_lane(v_id, l_id);

        // Insert a live reservation
        let res = ReservationState {
            route_id: r_id,
            vault_id: v_id,
            lane_id: l_id,
            amount: 50u128,
            expiry_block: 1000u64,
            status: ReservationStatus::Active,
            solvency_snapshot_hash: [0u8; 32],
        };
        pallet_x3_reservation::pallet::Reservations::<Test>::insert(res_id, res);
        pallet_x3_reservation::pallet::ReservationsByRoute::<Test>::insert(r_id, res_id);

        let ctx = SubmissionContext {
            reservation_id: res_id,
            route_id: r_id,
            vault_id: v_id,
            lane_id: l_id,
            amount: 50u128,
            quote_block: 1u64,
            slippage_bps: 10u32,
            max_slippage_bps: 100u32,
        };
        let result = crate::pallet::Pallet::<Test>::check_pre_submission(&ctx);
        assert!(result.passed, "failed checks: {:?}", result.failed_checks);
    });
}

#[test]
fn pre_submission_fails_for_stale_quote() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(8);
        let l_id = lane_id(8);
        let r_id = route_id(8);
        let res_id = reservation_id(8);
        setup_vault_and_lane(v_id, l_id);

        let res = ReservationState {
            route_id: r_id,
            vault_id: v_id,
            lane_id: l_id,
            amount: 50u128,
            expiry_block: 1000u64,
            status: ReservationStatus::Active,
            solvency_snapshot_hash: [0u8; 32],
        };
        pallet_x3_reservation::pallet::Reservations::<Test>::insert(res_id, res);
        pallet_x3_reservation::pallet::ReservationsByRoute::<Test>::insert(r_id, res_id);

        // Advance block number past stale threshold (QuoteStalenessBlocks = 20 in mock)
        System::set_block_number(100);

        let ctx = SubmissionContext {
            reservation_id: res_id,
            route_id: r_id,
            vault_id: v_id,
            lane_id: l_id,
            amount: 50u128,
            quote_block: 1u64, // stale
            slippage_bps: 10u32,
            max_slippage_bps: 100u32,
        };
        let result = crate::pallet::Pallet::<Test>::check_pre_submission(&ctx);
        assert!(!result.passed);
        assert!(result.failed_checks.contains(&crate::types::SolvencyCheck::QuoteStale));
    });
}

#[test]
fn pre_submission_fails_for_excessive_slippage() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(9);
        let l_id = lane_id(9);
        let r_id = route_id(9);
        let res_id = reservation_id(9);
        setup_vault_and_lane(v_id, l_id);

        let res = ReservationState {
            route_id: r_id,
            vault_id: v_id,
            lane_id: l_id,
            amount: 50u128,
            expiry_block: 1000u64,
            status: ReservationStatus::Active,
            solvency_snapshot_hash: [0u8; 32],
        };
        pallet_x3_reservation::pallet::Reservations::<Test>::insert(res_id, res);
        pallet_x3_reservation::pallet::ReservationsByRoute::<Test>::insert(r_id, res_id);

        let ctx = SubmissionContext {
            reservation_id: res_id,
            route_id: r_id,
            vault_id: v_id,
            lane_id: l_id,
            amount: 50u128,
            quote_block: 1u64,
            slippage_bps: 500u32, // > max_slippage_bps (100)
            max_slippage_bps: 100u32,
        };
        let result = crate::pallet::Pallet::<Test>::check_pre_submission(&ctx);
        assert!(!result.passed);
        assert!(result.failed_checks.contains(&crate::types::SolvencyCheck::SlippageExceeded));
    });
}

#[test]
fn pre_submission_fails_when_pending_obligation_exists() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(10);
        let l_id = lane_id(10);
        let r_id = route_id(10);
        let res_id = reservation_id(10);
        setup_vault_and_lane(v_id, l_id);

        let res = ReservationState {
            route_id: r_id,
            vault_id: v_id,
            lane_id: l_id,
            amount: 50u128,
            expiry_block: 1000u64,
            status: ReservationStatus::Active,
            solvency_snapshot_hash: [0u8; 32],
        };
        pallet_x3_reservation::pallet::Reservations::<Test>::insert(res_id, res);
        pallet_x3_reservation::pallet::ReservationsByRoute::<Test>::insert(r_id, res_id);

        // Inject an existing pending obligation for this route
        use crate::types::PendingObligation;
        PendingObligations::<Test>::insert(r_id, PendingObligation {
            route_id: r_id,
            reservation_id: res_id,
            amount: 50u128,
            timeout_block: 1000u64,
            snapshot_hash: [0u8; 32],
            submission_hash: [0u8; 32],
        });

        let ctx = SubmissionContext {
            reservation_id: res_id,
            route_id: r_id,
            vault_id: v_id,
            lane_id: l_id,
            amount: 50u128,
            quote_block: 1u64,
            slippage_bps: 10u32,
            max_slippage_bps: 100u32,
        };
        let result = crate::pallet::Pallet::<Test>::check_pre_submission(&ctx);
        assert!(!result.passed);
        assert!(result.failed_checks.contains(&crate::types::SolvencyCheck::ReconciliationLagged));
    });
}

// ──────────────────────────────────────────
// record_post_submission tests (TICKET-4.5-009)
// ──────────────────────────────────────────

#[test]
fn record_post_submission_stores_obligation_and_evidence() {
    new_test_ext().execute_with(|| {
        let r_id = route_id(11);
        let res_id = reservation_id(11);
        let snap: [u8; 32] = [1u8; 32];

        let ctx = PostSubmissionContext {
            reservation_id: res_id,
            route_id: r_id,
            vault_id: vault_id(11),
            lane_id: lane_id(11),
            amount: 100u128,
            submission_block: 1u64,
            submission_hash: snap,
        };

        assert_ok!(crate::pallet::Pallet::<Test>::record_post_submission(ctx));

        // obligation stored
        assert!(PendingObligations::<Test>::contains_key(r_id));
        assert_eq!(PendingObligationCount::<Test>::get(), 1);

        // evidence stored
        assert!(EvidenceRecords::<Test>::contains_key(r_id));

        // event emitted
        System::assert_has_event(RuntimeEvent::X3Solvency(Event::PendingObligationRecorded {
            route_id: r_id,
            reservation_id: res_id,
            snapshot_hash: snap,
        }));
    });
}

#[test]
fn record_post_submission_errors_on_duplicate_route() {
    new_test_ext().execute_with(|| {
        let r_id = route_id(12);
        let res_id = reservation_id(12);
        let snap: [u8; 32] = [2u8; 32];

        let ctx = PostSubmissionContext {
            reservation_id: res_id,
            route_id: r_id,
            vault_id: vault_id(12),
            lane_id: lane_id(12),
            amount: 100u128,
            submission_block: 1u64,
            submission_hash: snap,
        };

        assert_ok!(crate::pallet::Pallet::<Test>::record_post_submission(ctx.clone()));
        let result = crate::pallet::Pallet::<Test>::record_post_submission(ctx);
        assert!(matches!(result, Err(Error::<Test>::ObligationAlreadyExists)));
    });
}

#[test]
fn record_post_submission_marks_snapshot_as_referenced() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(13);
        let l_id = lane_id(13);
        let r_id = route_id(13);
        let res_id = reservation_id(13);
        setup_vault_and_lane(v_id, l_id);

        // First do a pre_quote to create a snapshot
        let q_ctx = QuoteContext { lane_id: l_id, vault_id: v_id, amount: 10u128, route_id: r_id };
        let q_result = crate::pallet::Pallet::<Test>::check_pre_quote(&q_ctx);
        let snap = q_result.snapshot_hash;

        // Then record post submission using that hash
        let ctx = PostSubmissionContext {
            reservation_id: res_id,
            route_id: r_id,
            vault_id: v_id,
            lane_id: l_id,
            amount: 10u128,
            submission_block: 1u64,
            submission_hash: snap,
        };
        assert_ok!(crate::pallet::Pallet::<Test>::record_post_submission(ctx));

        let snapshot = SolvencySnapshots::<Test>::get(snap).unwrap();
        assert!(snapshot.referenced);
    });
}

// ──────────────────────────────────────────
// Snapshot pruning tests (TICKET-4.5-010)
// ──────────────────────────────────────────

#[test]
fn old_unreferenced_snapshots_are_pruned() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(14);
        let l_id = lane_id(14);
        let r_id = route_id(14);
        setup_vault_and_lane(v_id, l_id);

        // Create a snapshot at block 1
        System::set_block_number(1);
        let ctx = QuoteContext { lane_id: l_id, vault_id: v_id, amount: 10u128, route_id: r_id };
        let result = crate::pallet::Pallet::<Test>::check_pre_quote(&ctx);
        let snap = result.snapshot_hash;
        assert!(SolvencySnapshots::<Test>::contains_key(snap));

        // on_initialize uses iter_prefix(prune_before) where prune_before = now - retention
        // Snapshot at block 1 is pruned when prune_before == 1, i.e. now = 1001
        System::set_block_number(1001);
        <crate::pallet::Pallet<Test> as Hooks<u64>>::on_initialize(1001);

        // prune_before = 1001 - 1000 = 1, which equals the block the snapshot was stored at
        assert!(!SolvencySnapshots::<Test>::contains_key(snap));
    });
}

#[test]
fn referenced_snapshots_survive_pruning() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(15);
        let l_id = lane_id(15);
        let r_id = route_id(15);
        let res_id = reservation_id(15);
        setup_vault_and_lane(v_id, l_id);

        System::set_block_number(1);
        let ctx = QuoteContext { lane_id: l_id, vault_id: v_id, amount: 10u128, route_id: r_id };
        let result = crate::pallet::Pallet::<Test>::check_pre_quote(&ctx);
        let snap = result.snapshot_hash;

        // Mark it as referenced via post_submission
        let post_ctx = PostSubmissionContext {
            reservation_id: res_id,
            route_id: r_id,
            vault_id: v_id,
            lane_id: l_id,
            amount: 10u128,
            submission_block: 1u64,
            submission_hash: snap,
        };
        assert_ok!(crate::pallet::Pallet::<Test>::record_post_submission(post_ctx));

        // Use now=1001 so prune_before=1 targets block 1 where snapshot was stored
        System::set_block_number(1001);
        <crate::pallet::Pallet<Test> as Hooks<u64>>::on_initialize(1001);

        // Should still be there since it's referenced
        assert!(SolvencySnapshots::<Test>::contains_key(snap));
    });
}

// ──────────────────────────────────────────
// get_snapshot API test (TICKET-4.5-010)
// ──────────────────────────────────────────

#[test]
fn get_snapshot_returns_stored_record() {
    new_test_ext().execute_with(|| {
        let v_id = vault_id(16);
        let l_id = lane_id(16);
        let r_id = route_id(16);
        setup_vault_and_lane(v_id, l_id);

        let ctx = QuoteContext { lane_id: l_id, vault_id: v_id, amount: 10u128, route_id: r_id };
        let result = crate::pallet::Pallet::<Test>::check_pre_quote(&ctx);
        let snap = result.snapshot_hash;

        let record = crate::pallet::Pallet::<Test>::get_snapshot(snap);
        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.route_id, r_id);
        assert_eq!(record.passed, result.passed);
    });
}

#[test]
fn get_snapshot_returns_none_for_unknown_hash() {
    new_test_ext().execute_with(|| {
        let unknown: [u8; 32] = [0xdeu8; 32];
        assert!(crate::pallet::Pallet::<Test>::get_snapshot(unknown).is_none());
    });
}

// ──────────────────────────────────────────
// Extrinsic test
// ──────────────────────────────────────────

#[test]
fn record_post_submission_extrinsic_works() {
    new_test_ext().execute_with(|| {
        let r_id = route_id(20);
        let res_id = reservation_id(20);
        let snap: [u8; 32] = [5u8; 32];

        let ctx = PostSubmissionContext {
            reservation_id: res_id,
            route_id: r_id,
            vault_id: vault_id(20),
            lane_id: lane_id(20),
            amount: 50u128,
            submission_block: 1u64,
            submission_hash: snap,
        };

        assert_ok!(crate::pallet::Pallet::<Test>::record_post_submission_extrinsic(
            frame_system::RawOrigin::Signed(1).into(),
            ctx,
        ));

        assert!(PendingObligations::<Test>::contains_key(r_id));
    });
}
