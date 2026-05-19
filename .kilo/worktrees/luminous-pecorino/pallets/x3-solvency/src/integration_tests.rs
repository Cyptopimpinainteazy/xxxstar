//! Phase 4.5 end-to-end integration tests.
//!
//! Exercises all 7 required integration paths from the Phase 4.5 implementation plan
//! using `sp_io::TestExternalities` (mock externalities) against the real pallet code
//! — no live node required.
//!
//! # Build note
//!
//! Substrate's macro-generated storage-map iteration code uses substantial stack space.
//! Two things are required:
//!
//! 1. Set `RUST_MIN_STACK=134217728` (128 MiB) so the **main thread** has enough stack.
//! 2. Pass `--test-threads=1` so all test functions run on that enlarged main thread
//!    rather than being dispatched to default-stack spawned threads (which would SIGABRT).
//!
//! ```text
//! RUST_MIN_STACK=134217728 cargo test -p pallet-x3-solvency integration_tests -- --test-threads=1
//! ```
//!
//! # Path inventory
//!
//! | # | Test function | What it covers |
//! |---|--------------|----------------|
//! | 1 | `path1_pre_quote_gate_blocks_frozen_lane`            | check_pre_quote fails with LaneFrozen for a frozen lane |
//! | 2 | `path2_reservation_reserves_vault_balance`           | request_reservation atomically locks vault inventory |
//! | 3 | `path3_pre_submission_gate_blocks_expired_reservation`| check_pre_submission fails with ReservationExpired |
//! | 4 | `path4_post_submission_records_pending_obligation`    | record_post_submission seals obligation + evidence |
//! | 5 | `path5_settlement_releases_reservation_and_confirms_vault` | consume + confirm_settlement full flow |
//! | 6 | `path6_vault_below_min_band_transitions_to_degraded`  | vault status → Degraded when balance < min_band |
//! | 7 | `path7_frozen_lane_rejects_all_new_reservations`      | all reservation entry points reject frozen lane |
//!
//! Each test uses the Phase 4.5 spec vault:
//!   `available = 1_000 / critical_min = 100 / min_band = 200 / target = 500 / max = 900`
//! and a `LaneClass::C` lane with `exposure_cap = unsettled_cap = 5_000`.

use crate::mock::*;
use crate::pallet::{EvidenceRecords, PendingObligationCount, PendingObligations};
use crate::types::{PostSubmissionContext, QuoteContext, ReservationContext, SubmissionContext};
use crate::Event as SolvencyEvent;
use frame_support::{assert_noop, assert_ok, BoundedVec};
use pallet_x3_inventory::{
    pallet::{Lanes, Vaults},
    types::{
        FreezeReason, LaneClass, LaneId, LaneStatus, LiquiditySourceType, OwnerType,
        ReservationId, ReservationStatus, RouteId, VaultId, VaultStatus, VaultType,
    },
};

// ─────────────────────────────────────────────────────────────────────────────
// Shared test helpers
// ─────────────────────────────────────────────────────────────────────────────

fn vid(n: u8) -> VaultId {
    let mut id = [0u8; 32];
    id[0] = n;
    id
}

fn lid(n: u8) -> LaneId {
    let mut id = [0u8; 32];
    id[0] = n;
    id
}

fn rid(n: u8) -> RouteId {
    let mut id = [0u8; 32];
    id[0] = n;
    id
}

fn resv(n: u8) -> ReservationId {
    let mut id = [0u8; 32];
    id[0] = n;
    id
}

/// Create a vault using the Phase 4.5 spec bands and fund it so `available = 1_000`.
///
/// Band layout: `critical_min = 100 / min_band = 200 / target = 500 / max = 900`.
///
/// `create_vault` starts with `available = 0` (< `critical_min = 100`), so the
/// vault is initially `Frozen`.  `fund_vault(1_000)` calls `refresh_vault_status`
/// internally and transitions the vault to `Active` (1_000 ≥ min_band = 200).
fn spec_vault(v_id: VaultId) {
    assert_ok!(pallet_x3_inventory::Pallet::<Test>::create_vault(
        frame_system::RawOrigin::Root.into(),
        v_id,
        VaultType::SettlementFloat,
        OwnerType::Protocol,
        1u32,    // chain_id
        1u32,    // asset_id
        100u128, // critical_min
        200u128, // min_band
        500u128, // target_band
        900u128, // max_band
    ));
    assert_ok!(pallet_x3_inventory::Pallet::<Test>::fund_vault(
        frame_system::RawOrigin::Root.into(),
        v_id,
        1_000u128,
    ));
}

/// Register an `Active` `LaneClass::C` lane with generous caps (5_000 each).
fn spec_lane(l_id: LaneId) {
    let sources: BoundedVec<LiquiditySourceType, MaxLiquiditySources> =
        BoundedVec::try_from(vec![LiquiditySourceType::ProtocolFloat]).unwrap();
    assert_ok!(pallet_x3_inventory::Pallet::<Test>::register_lane(
        frame_system::RawOrigin::Root.into(),
        l_id,
        1u32, // source_chain
        2u32, // dest_chain
        1u32, // source_asset
        2u32, // dest_asset
        LaneClass::C,
        sources,
        5_000u128, // exposure_cap
        5_000u128, // unsettled_cap
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// Path 1 — pre-quote gate blocks a frozen lane
// ─────────────────────────────────────────────────────────────────────────────

/// Path 1: Router requests route → `check_pre_quote` → frozen lane blocks the gate.
///
/// The solvency gate must reject any quote for a lane in `Frozen` status,
/// regardless of vault health.  `SolvencyCheck::LaneFrozen` must appear in
/// `failed_checks` and the `SolvencyGateChecked { passed: false }` event must fire.
#[test]
fn path1_pre_quote_gate_blocks_frozen_lane() {
    new_test_ext().execute_with(|| {
        let v = vid(10);
        let l = lid(10);
        let r = rid(10);
        spec_vault(v);
        spec_lane(l);

        // Confirm the lane is initially Active.
        assert_eq!(Lanes::<Test>::get(l).unwrap().status, LaneStatus::Active);

        // Freeze the lane via the operator-override extrinsic.
        assert_ok!(pallet_x3_inventory::Pallet::<Test>::freeze_lane(
            frame_system::RawOrigin::Root.into(),
            l,
            FreezeReason::OperatorManual,
        ));

        // Confirm the freeze is stored.
        assert_eq!(
            Lanes::<Test>::get(l).unwrap().status,
            LaneStatus::Frozen,
            "lane must be Frozen after freeze_lane"
        );

        // Path 1: check_pre_quote must fail because the lane is frozen.
        let ctx = QuoteContext { lane_id: l, vault_id: v, amount: 100u128, route_id: r };
        let result = crate::pallet::Pallet::<Test>::check_pre_quote(&ctx);

        assert!(!result.passed, "check_pre_quote must fail for a frozen lane");
        assert!(
            result.failed_checks.contains(&crate::types::SolvencyCheck::LaneFrozen),
            "failed_checks must include LaneFrozen; got: {:?}",
            result.failed_checks
        );

        System::assert_has_event(RuntimeEvent::X3Solvency(SolvencyEvent::SolvencyGateChecked {
            route_id: r,
            gate: crate::GateKind::PreQuote,
            passed: false,
            snapshot_hash: result.snapshot_hash,
        }));
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Path 2 — reservation reserves vault balance atomically
// ─────────────────────────────────────────────────────────────────────────────

/// Path 2: User confirms → `request_reservation` atomically locks vault inventory.
///
/// After a successful `request_reservation`:
/// - `Reservations` storage contains the record with `Active` status.
/// - `ReservationsByRoute` maps the route ID to the reservation ID.
/// - `vault.available_balance` decreases by the reserved amount.
/// - `vault.reserved_balance` increases by the same amount.
/// - A `ReservationCreated` event is emitted.
#[test]
fn path2_reservation_reserves_vault_balance() {
    new_test_ext().execute_with(|| {
        let v = vid(20);
        let l = lid(20);
        let r = rid(20);
        let res = resv(20);
        let amount = 300u128;
        spec_vault(v);
        spec_lane(l);

        // Baseline: vault is funded with 1_000 available, nothing reserved.
        let before = Vaults::<Test>::get(v).unwrap();
        assert_eq!(before.available_balance, 1_000u128);
        assert_eq!(before.reserved_balance, 0u128);
        assert_eq!(before.status, VaultStatus::Active);

        // Path 2: request_reservation must lock the amount in the vault.
        assert_ok!(pallet_x3_reservation::Pallet::<Test>::request_reservation(
            frame_system::RawOrigin::Root.into(),
            res, r, v, l, amount, [0u8; 32],
        ));

        // Reservation must exist with Active status.
        let state = pallet_x3_reservation::pallet::Reservations::<Test>::get(res)
            .expect("reservation must be stored");
        assert_eq!(state.route_id, r);
        assert_eq!(state.vault_id, v);
        assert_eq!(state.lane_id, l);
        assert_eq!(state.amount, amount);
        assert_eq!(state.status, ReservationStatus::Active);

        // Route → reservation mapping must be populated.
        assert_eq!(
            pallet_x3_reservation::pallet::ReservationsByRoute::<Test>::get(r),
            Some(res),
        );

        // Vault balances must reflect the lock.
        let after = Vaults::<Test>::get(v).unwrap();
        assert_eq!(after.available_balance, 1_000u128 - amount, "available must decrease");
        assert_eq!(after.reserved_balance, amount, "reserved must increase");

        // ReservationCreated event must be emitted.
        System::assert_has_event(RuntimeEvent::X3Reservation(
            pallet_x3_reservation::pallet::Event::ReservationCreated {
                reservation_id: res,
                route_id: r,
                vault_id: v,
                lane_id: l,
                amount,
                expiry_block: state.expiry_block,
                solvency_snapshot_hash: [0u8; 32],
            },
        ));
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Path 3 — pre-submission gate blocks an expired reservation
// ─────────────────────────────────────────────────────────────────────────────

/// Path 3: Router submits → `check_pre_submission` → expired reservation causes gate failure.
///
/// `is_reservation_valid` checks `now < expiry_block`.  At exactly `expiry_block`
/// the reservation is invalid and the gate pushes `SolvencyCheck::ReservationExpired`
/// into `failed_checks`.
///
/// TTL is 100 blocks (from mock); created at block 1 → `expiry_block = 101`.
/// Advancing to block 101 makes `now < 101` false.
#[test]
fn path3_pre_submission_gate_blocks_expired_reservation() {
    new_test_ext().execute_with(|| {
        let v = vid(30);
        let l = lid(30);
        let r = rid(30);
        let res = resv(30);
        let amount = 100u128;
        spec_vault(v);
        spec_lane(l);

        // Create reservation at block 1; expiry_block = 1 + 100 = 101.
        assert_ok!(pallet_x3_reservation::Pallet::<Test>::request_reservation(
            frame_system::RawOrigin::Root.into(),
            res, r, v, l, amount, [0u8; 32],
        ));
        let expiry = pallet_x3_reservation::pallet::Reservations::<Test>::get(res)
            .unwrap()
            .expiry_block;

        // Advance to exactly expiry_block → is_reservation_valid returns false.
        System::set_block_number(expiry);

        // Path 3: check_pre_submission must fail with ReservationExpired.
        let ctx = SubmissionContext {
            reservation_id: res,
            route_id: r,
            vault_id: v,
            lane_id: l,
            amount,
            quote_block: 1u64,
            slippage_bps: 10u32,
            max_slippage_bps: 100u32,
        };
        let result = crate::pallet::Pallet::<Test>::check_pre_submission(&ctx);

        assert!(!result.passed, "check_pre_submission must fail for expired reservation");
        assert!(
            result
                .failed_checks
                .contains(&crate::types::SolvencyCheck::ReservationExpired),
            "failed_checks must include ReservationExpired; got: {:?}",
            result.failed_checks
        );

        System::assert_has_event(RuntimeEvent::X3Solvency(SolvencyEvent::SolvencyGateChecked {
            route_id: r,
            gate: crate::GateKind::PreSubmission,
            passed: false,
            snapshot_hash: result.snapshot_hash,
        }));
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Path 4 — post-submission records a pending obligation
// ─────────────────────────────────────────────────────────────────────────────

/// Path 4: `record_post_submission` called in the same extrinsic as `check_pre_submission`.
///
/// After a successful pre-submission gate, the obligation must be sealed in
/// `PendingObligations`, an `EvidenceRecord` written, `PendingObligationCount`
/// incremented to 1, and a `PendingObligationRecorded` event emitted.
#[test]
fn path4_post_submission_records_pending_obligation() {
    new_test_ext().execute_with(|| {
        let v = vid(40);
        let l = lid(40);
        let r = rid(40);
        let res = resv(40);
        let amount = 150u128;
        let snap: [u8; 32] = [0x04u8; 32];
        spec_vault(v);
        spec_lane(l);

        // Create a live reservation (block 1 → expiry 101).
        assert_ok!(pallet_x3_reservation::Pallet::<Test>::request_reservation(
            frame_system::RawOrigin::Root.into(),
            res, r, v, l, amount, [0u8; 32],
        ));

        // Step A: verify the pre-submission gate passes with a fresh quote.
        // block = 1, quote_block = 1 → staleness = 0 ≤ QuoteStalenessBlocks(20). ✓
        let gate = crate::pallet::Pallet::<Test>::check_pre_submission(&SubmissionContext {
            reservation_id: res,
            route_id: r,
            vault_id: v,
            lane_id: l,
            amount,
            quote_block: 1u64,
            slippage_bps: 0u32,
            max_slippage_bps: 100u32,
        });
        assert!(
            gate.passed,
            "pre_submission gate must pass before recording obligation; failed: {:?}",
            gate.failed_checks
        );

        // Step B (Path 4): record_post_submission — must be called in the same
        // extrinsic as check_pre_submission in production.
        assert_ok!(crate::pallet::Pallet::<Test>::record_post_submission(
            PostSubmissionContext {
                reservation_id: res,
                route_id: r,
                vault_id: v,
                lane_id: l,
                amount,
                submission_block: 1u64,
                submission_hash: snap,
            }
        ));

        // PendingObligation must be stored.
        assert!(
            PendingObligations::<Test>::contains_key(r),
            "PendingObligations must contain the route"
        );
        let ob = PendingObligations::<Test>::get(r).unwrap();
        assert_eq!(ob.route_id, r);
        assert_eq!(ob.reservation_id, res);
        assert_eq!(ob.amount, amount);

        // Evidence record must be sealed.
        assert!(
            EvidenceRecords::<Test>::contains_key(r),
            "EvidenceRecords must contain the route"
        );

        // Obligation counter must be 1.
        assert_eq!(PendingObligationCount::<Test>::get(), 1);

        // PendingObligationRecorded event must be emitted.
        System::assert_has_event(RuntimeEvent::X3Solvency(
            SolvencyEvent::PendingObligationRecorded {
                route_id: r,
                reservation_id: res,
                snapshot_hash: snap,
            },
        ));
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Path 5 — settlement releases reservation and confirms vault
// ─────────────────────────────────────────────────────────────────────────────

/// Path 5: Settlement confirmed → `consume_reservation`, `confirm_settlement`, accounting updated.
///
/// Full end-to-end flow:
/// 1. `request_reservation(300)` → `available 1_000→700 / reserved 0→300`.
/// 2. `consume_reservation`      → `release_inventory` then `record_pending_out`:
///    `reserved 300→0 / available 700→1_000→700 / pending_out 0→300`.
/// 3. `confirm_settlement(300)`  → `pending_out 300→0`.
///
/// Reservation status must transition to `Consumed` and correct events must fire.
#[test]
fn path5_settlement_releases_reservation_and_confirms_vault() {
    new_test_ext().execute_with(|| {
        let v = vid(50);
        let l = lid(50);
        let r = rid(50);
        let res = resv(50);
        let amount = 300u128;
        spec_vault(v);
        spec_lane(l);

        // ── Step 1: create reservation ─────────────────────────────────────
        assert_ok!(pallet_x3_reservation::Pallet::<Test>::request_reservation(
            frame_system::RawOrigin::Root.into(),
            res, r, v, l, amount, [0u8; 32],
        ));
        let v1 = Vaults::<Test>::get(v).unwrap();
        assert_eq!(v1.available_balance, 700u128);
        assert_eq!(v1.reserved_balance, amount);
        assert_eq!(v1.pending_out_balance, 0u128);

        // ── Step 2: consume reservation — reserved→pending_out ─────────────
        assert_ok!(pallet_x3_reservation::Pallet::<Test>::consume_reservation(
            frame_system::RawOrigin::Root.into(),
            res,
        ));

        let v2 = Vaults::<Test>::get(v).unwrap();
        assert_eq!(v2.reserved_balance, 0u128, "reserved must be 0 after consume");
        assert_eq!(v2.pending_out_balance, amount, "pending_out must equal consumed amount");
        assert_eq!(v2.available_balance, 700u128, "available unchanged during consume");

        // Reservation status must be Consumed.
        assert_eq!(
            pallet_x3_reservation::pallet::Reservations::<Test>::get(res)
                .unwrap()
                .status,
            ReservationStatus::Consumed,
        );

        System::assert_has_event(RuntimeEvent::X3Reservation(
            pallet_x3_reservation::pallet::Event::ReservationConsumed {
                reservation_id: res,
                route_id: r,
                solvency_snapshot_hash: [0u8; 32],
            },
        ));

        // ── Step 3: confirm settlement — pending_out cleared ───────────────
        assert_ok!(pallet_x3_inventory::Pallet::<Test>::confirm_settlement(
            frame_system::RawOrigin::Root.into(),
            v,
            amount,
        ));

        let v3 = Vaults::<Test>::get(v).unwrap();
        assert_eq!(v3.pending_out_balance, 0u128, "pending_out must clear after settlement");

        System::assert_has_event(RuntimeEvent::X3Inventory(
            pallet_x3_inventory::pallet::Event::SettlementConfirmed { vault_id: v, amount },
        ));
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Path 6 — vault below min_band transitions to Degraded
// ─────────────────────────────────────────────────────────────────────────────

/// Path 6: Vault drops below `min_band` → status transitions to `Degraded`.
///
/// `reserve_inventory` (called internally by `request_reservation`) calls
/// `refresh_vault_status` after mutating the vault.  When `available_balance`
/// drops below `min_band` (200) but stays at or above `critical_min` (100),
/// the vault transitions to `Degraded`.
///
/// This is the observable trigger that signals a rebalance is needed.
/// A `VaultStatusChanged { old: Active, new: Degraded }` event must be in the log.
///
/// Calculation: reserve 850 → available = 1_000 − 850 = 150.
///   150 < min_band (200) → Degraded.
///   150 ≥ critical_min (100) → not Frozen.
#[test]
fn path6_vault_below_min_band_transitions_to_degraded() {
    new_test_ext().execute_with(|| {
        let v = vid(60);
        let l = lid(60);
        let r = rid(60);
        let res = resv(60);
        spec_vault(v);
        spec_lane(l);

        // Verify initial conditions after spec_vault.
        let init = Vaults::<Test>::get(v).unwrap();
        assert_eq!(init.status, VaultStatus::Active, "vault must start Active");
        assert_eq!(init.available_balance, 1_000u128);
        assert_eq!(init.min_band, 200u128);
        assert_eq!(init.critical_min, 100u128);

        // Reserve 850 → available drops to 150 (below min_band=200, above critical_min=100).
        assert_ok!(pallet_x3_reservation::Pallet::<Test>::request_reservation(
            frame_system::RawOrigin::Root.into(),
            res, r, v, l, 850u128, [0u8; 32],
        ));

        let after = Vaults::<Test>::get(v).unwrap();
        assert_eq!(after.available_balance, 150u128, "available must be 150 after reserving 850");
        assert_eq!(
            after.status,
            VaultStatus::Degraded,
            "vault must be Degraded when available ({}) < min_band ({})",
            after.available_balance,
            after.min_band,
        );

        // VaultStatusChanged(Active → Degraded) must appear in the event log.
        System::assert_has_event(RuntimeEvent::X3Inventory(
            pallet_x3_inventory::pallet::Event::VaultStatusChanged {
                vault_id: v,
                old_status: VaultStatus::Active,
                new_status: VaultStatus::Degraded,
            },
        ));
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Path 7 — frozen lane rejects all new reservations
// ─────────────────────────────────────────────────────────────────────────────

/// Path 7: Lane frozen → all reservation paths return `LaneFrozen`.
///
/// Three entry points are exercised:
///
/// **7a** — `pallet_x3_reservation::request_reservation` must return
/// `Error::<Test>::LaneFrozen` and leave vault balances untouched.
///
/// **7b** — `check_pre_quote` must return `passed = false` with
/// `SolvencyCheck::LaneFrozen` in `failed_checks`.
///
/// **7c** — `check_pre_reservation` must also return `passed = false` with
/// `SolvencyCheck::LaneFrozen` in `failed_checks`.
#[test]
fn path7_frozen_lane_rejects_all_new_reservations() {
    new_test_ext().execute_with(|| {
        let v = vid(70);
        let l = lid(70);
        let r = rid(70);
        let res = resv(70);
        let amount = 100u128;
        spec_vault(v);
        spec_lane(l);

        // Freeze the lane.
        assert_ok!(pallet_x3_inventory::Pallet::<Test>::freeze_lane(
            frame_system::RawOrigin::Root.into(),
            l,
            FreezeReason::OperatorManual,
        ));
        assert_eq!(Lanes::<Test>::get(l).unwrap().status, LaneStatus::Frozen);

        // ── 7a: pallet extrinsic must be rejected with LaneFrozen ──────────
        assert_noop!(
            pallet_x3_reservation::Pallet::<Test>::request_reservation(
                frame_system::RawOrigin::Root.into(),
                res, r, v, l, amount, [0u8; 32],
            ),
            pallet_x3_reservation::pallet::Error::<Test>::LaneFrozen
        );

        // Storage must be pristine (assert_noop guarantees no storage change,
        // but let's also verify the relevant maps explicitly).
        assert!(
            pallet_x3_reservation::pallet::Reservations::<Test>::get(res).is_none(),
            "Reservations must be empty after LaneFrozen rejection"
        );
        assert!(
            pallet_x3_reservation::pallet::ReservationsByRoute::<Test>::get(r).is_none(),
            "ReservationsByRoute must be empty after LaneFrozen rejection"
        );

        // ── 7b: check_pre_quote must fail with LaneFrozen ──────────────────
        let qr = crate::pallet::Pallet::<Test>::check_pre_quote(&QuoteContext {
            lane_id: l,
            vault_id: v,
            amount,
            route_id: r,
        });
        assert!(!qr.passed, "check_pre_quote must fail for frozen lane");
        assert!(
            qr.failed_checks.contains(&crate::types::SolvencyCheck::LaneFrozen),
            "check_pre_quote must report LaneFrozen; got: {:?}",
            qr.failed_checks
        );

        // ── 7c: check_pre_reservation must also fail with LaneFrozen ───────
        let rr = crate::pallet::Pallet::<Test>::check_pre_reservation(&ReservationContext {
            lane_id: l,
            vault_id: v,
            amount,
            route_id: r,
        });
        assert!(!rr.passed, "check_pre_reservation must fail for frozen lane");
        assert!(
            rr.failed_checks.contains(&crate::types::SolvencyCheck::LaneFrozen),
            "check_pre_reservation must report LaneFrozen; got: {:?}",
            rr.failed_checks
        );

        // Vault balances must be completely unchanged.
        let vault_after = Vaults::<Test>::get(v).unwrap();
        assert_eq!(vault_after.available_balance, 1_000u128, "available must be unchanged");
        assert_eq!(vault_after.reserved_balance, 0u128, "reserved must be 0");
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Bonus: full happy-path through all 7 checkpoints in sequence
// ─────────────────────────────────────────────────────────────────────────────

/// Exercises all 7 checkpoints in a single, linear end-to-end flow to confirm
/// that no cross-pallet state corruption occurs across the complete pipeline.
#[test]
fn full_happy_path_all_seven_checkpoints() {
    new_test_ext().execute_with(|| {
        let v = vid(99);
        let l = lid(99);
        let r = rid(99);
        let res = resv(99);
        spec_vault(v);
        spec_lane(l);

        // Checkpoint 1: indicative quote.
        let qr = crate::pallet::Pallet::<Test>::check_pre_quote(&QuoteContext {
            lane_id: l, vault_id: v, amount: 250u128, route_id: r,
        });
        assert!(qr.passed, "CP1 pre-quote must pass");

        // Checkpoint 2: reservation locks inventory.
        assert_ok!(pallet_x3_reservation::Pallet::<Test>::request_reservation(
            frame_system::RawOrigin::Root.into(),
            res, r, v, l, 250u128, qr.snapshot_hash,
        ));
        assert_eq!(Vaults::<Test>::get(v).unwrap().available_balance, 750u128, "CP2");

        // Checkpoint 3: pre-submission gate.
        let sr = crate::pallet::Pallet::<Test>::check_pre_submission(&SubmissionContext {
            reservation_id: res,
            route_id: r,
            vault_id: v,
            lane_id: l,
            amount: 250u128,
            quote_block: System::block_number(),
            slippage_bps: 5u32,
            max_slippage_bps: 100u32,
        });
        assert!(sr.passed, "CP3 pre-submission must pass; failed: {:?}", sr.failed_checks);

        // Checkpoint 4: obligation recorded.
        assert_ok!(crate::pallet::Pallet::<Test>::record_post_submission(
            PostSubmissionContext {
                reservation_id: res,
                route_id: r,
                vault_id: v,
                lane_id: l,
                amount: 250u128,
                submission_hash: sr.snapshot_hash,
                submission_block: System::block_number(),
            }
        ));
        assert!(PendingObligations::<Test>::contains_key(r), "CP4 obligation must be stored");
        assert_eq!(PendingObligationCount::<Test>::get(), 1, "CP4 count must be 1");

        // Checkpoint 5a: consume reservation.
        assert_ok!(pallet_x3_reservation::Pallet::<Test>::consume_reservation(
            frame_system::RawOrigin::Root.into(),
            res,
        ));
        assert_eq!(
            Vaults::<Test>::get(v).unwrap().pending_out_balance,
            250u128,
            "CP5a pending_out"
        );

        // Checkpoint 5b: confirm settlement.
        assert_ok!(pallet_x3_inventory::Pallet::<Test>::confirm_settlement(
            frame_system::RawOrigin::Root.into(),
            v,
            250u128,
        ));
        assert_eq!(
            Vaults::<Test>::get(v).unwrap().pending_out_balance,
            0u128,
            "CP5b pending_out cleared"
        );

        // Checkpoint 6: vault is still Active (750 ≥ min_band=200).
        assert_eq!(
            Vaults::<Test>::get(v).unwrap().status,
            VaultStatus::Active,
            "CP6 vault must remain Active"
        );

        // Checkpoint 7: freeze the lane and verify new reservation is rejected.
        let r2 = rid(100);
        let res2 = resv(100);
        assert_ok!(pallet_x3_inventory::Pallet::<Test>::freeze_lane(
            frame_system::RawOrigin::Root.into(),
            l,
            FreezeReason::OperatorManual,
        ));
        assert_noop!(
            pallet_x3_reservation::Pallet::<Test>::request_reservation(
                frame_system::RawOrigin::Root.into(),
                res2, r2, v, l, 100u128, [0u8; 32],
            ),
            pallet_x3_reservation::pallet::Error::<Test>::LaneFrozen
        );
    });
}
