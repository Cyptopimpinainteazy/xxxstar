//! Unit tests for `pallet-x3-partner`.
//!
//! Covers:
//! 1. `register_partner` — creates partner with health 10 000 bps.
//! 2. `update_health_metrics` — bad fill rate drops health below threshold and emits
//!    `HealthBelowThreshold`.
//! 3. `record_exposure` — adding beyond the limit returns `ExposureCapExceeded`.
//! 4. `add_approved_lane` — fails for a lane that does not exist in inventory.
//! 5. `terminate_partner` — permanently blocks reinstatement.

use crate::{
    mock::{new_test_ext, RuntimeEvent, RuntimeOrigin, System, Test, X3Inventory, X3Partner},
    pallet::{Error, Event, Partners, MIN_PARTNER_HEALTH_BPS},
};
use frame_support::{assert_noop, assert_ok};
use pallet_x3_inventory::types::{AssetId, ChainId, LaneClass, LiquiditySourceType};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const PARTNER_A: [u8; 32] = [0xAA; 32];
const PARTNER_B: [u8; 32] = [0xBB; 32];
const LANE_1: [u8; 32] = [0x11; 32];
const LANE_UNKNOWN: [u8; 32] = [0xFF; 32];

const EXPOSURE_LIMIT: u128 = 1_000_000;

fn root() -> RuntimeOrigin {
    RuntimeOrigin::root()
}

fn signed(who: u64) -> RuntimeOrigin {
    RuntimeOrigin::signed(who)
}

/// Register a lane in the inventory pallet so partner tests can reference it.
fn setup_lane(lane_id: [u8; 32]) {
    use frame_support::BoundedVec;
    assert_ok!(X3Inventory::register_lane(
        root(),
        lane_id,
        1u32 as ChainId,
        2u32 as ChainId,
        10u32 as AssetId,
        20u32 as AssetId,
        LaneClass::B,
        BoundedVec::try_from(vec![LiquiditySourceType::Partner]).unwrap(),
        500_000u128,
        400_000u128,
    ));
}

// ---------------------------------------------------------------------------
// Test 1: register_partner creates with health 10 000
// ---------------------------------------------------------------------------

#[test]
fn register_partner_creates_with_perfect_health() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));

        let state = Partners::<Test>::get(PARTNER_A).expect("partner must exist after registration");

        assert_eq!(state.partner_id, PARTNER_A);
        assert_eq!(
            state.status,
            pallet_x3_inventory::types::PartnerStatus::Active
        );
        assert_eq!(state.health_score_bps, 10_000);
        assert_eq!(state.exposure_limit, EXPOSURE_LIMIT);
        assert_eq!(state.current_exposure, 0u128);
        assert!(state.approved_lanes.is_empty());

        // Confirm event was emitted.
        System::assert_has_event(RuntimeEvent::X3Partner(Event::PartnerRegistered {
            partner_id: PARTNER_A,
            exposure_limit: EXPOSURE_LIMIT,
        }));
    });
}

#[test]
fn register_partner_duplicate_returns_already_exists() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));
        assert_noop!(
            X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT),
            Error::<Test>::PartnerAlreadyExists
        );
    });
}

// ---------------------------------------------------------------------------
// Test 2: bad fill rate drops health below threshold and emits HealthBelowThreshold
// ---------------------------------------------------------------------------

#[test]
fn update_health_metrics_emits_below_threshold_when_fill_rate_is_low() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));

        // Worst-case fill reliability (0 bps) with other metrics at neutral.
        // fill_score        = 0
        // rejection_score   = 10_000
        // stale_score       = 10_000
        // dispute_score     = 10_000
        // response_score    = 10_000
        // weighted_sum = 0*3 + 10_000*2 + 10_000 + 10_000*2 + 10_000 = 70_000
        // health = 70_000 / 9 = 7_777 (still above threshold)
        //
        // To drop below 5_000 we need fill_score so low that even other perfect scores
        // cannot save it.  Setting fill=0, disputes=20 (all lose 10_000):
        // fill_score        = 0        (weight 3 → contributes 0)
        // rejection_score   = 10_000   (weight 2 → contributes 20_000)
        // stale_score       = 10_000   (weight 1 → contributes 10_000)
        // dispute_score     = 0        (weight 2 → contributes 0)    (20 * 500 = 10_000 penalty)
        // response_score    = 10_000   (weight 1 → contributes 10_000)
        // weighted_sum = 40_000 → health = 40_000 / 9 = 4_444  (< 5_000 ✓)
        assert_ok!(X3Partner::update_health_metrics(
            root(),
            PARTNER_A,
            /*quote_response_time_ms_p95=*/ 0,
            /*fill_reliability_bps=*/ 0,
            /*rejected_reservation_bps=*/ 0,
            /*stale_quote_bps=*/ 0,
            /*dispute_count=*/ 20,
        ));

        let state = Partners::<Test>::get(PARTNER_A).unwrap();
        assert!(
            state.health_score_bps < MIN_PARTNER_HEALTH_BPS,
            "health {health} must be below {MIN_PARTNER_HEALTH_BPS}",
            health = state.health_score_bps,
        );

        // PartnerHealthUpdated must be present.
        System::assert_has_event(RuntimeEvent::X3Partner(Event::PartnerHealthUpdated {
            partner_id: PARTNER_A,
            health_score_bps: state.health_score_bps,
        }));

        // HealthBelowThreshold must also be present.
        System::assert_has_event(RuntimeEvent::X3Partner(Event::HealthBelowThreshold {
            partner_id: PARTNER_A,
            health_score_bps: state.health_score_bps,
        }));
    });
}

#[test]
fn update_health_metrics_perfect_scores_yield_ten_thousand() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));

        assert_ok!(X3Partner::update_health_metrics(
            root(),
            PARTNER_A,
            0,       // zero response time
            10_000,  // perfect fill
            0,       // zero rejections
            0,       // zero stale
            0,       // zero disputes
        ));

        let state = Partners::<Test>::get(PARTNER_A).unwrap();
        assert_eq!(state.health_score_bps, 10_000);
    });
}

// ---------------------------------------------------------------------------
// Test 3: record_exposure exceeding limit returns ExposureCapExceeded
// ---------------------------------------------------------------------------

#[test]
fn record_exposure_exceeding_limit_returns_error() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));

        // Adding exactly the limit must succeed.
        assert_ok!(X3Partner::record_exposure(
            signed(1),
            PARTNER_A,
            EXPOSURE_LIMIT,
            true,
        ));

        // Adding any more must fail.
        assert_noop!(
            X3Partner::record_exposure(signed(1), PARTNER_A, 1u128, true),
            Error::<Test>::ExposureCapExceeded
        );
    });
}

#[test]
fn record_exposure_subtract_does_not_underflow() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));

        // Subtract more than current exposure (0) — must saturate at 0, not panic.
        assert_ok!(X3Partner::record_exposure(
            signed(1),
            PARTNER_A,
            500_000u128,
            false,
        ));

        let state = Partners::<Test>::get(PARTNER_A).unwrap();
        assert_eq!(state.current_exposure, 0u128);
    });
}

// ---------------------------------------------------------------------------
// Test 4: add_approved_lane fails for non-existent lane
// ---------------------------------------------------------------------------

#[test]
fn add_approved_lane_fails_for_nonexistent_lane() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));

        assert_noop!(
            X3Partner::add_approved_lane(root(), PARTNER_A, LANE_UNKNOWN),
            Error::<Test>::LaneNotFound
        );
    });
}

#[test]
fn add_approved_lane_succeeds_for_existing_lane() {
    new_test_ext().execute_with(|| {
        setup_lane(LANE_1);
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));

        assert_ok!(X3Partner::add_approved_lane(root(), PARTNER_A, LANE_1));

        let state = Partners::<Test>::get(PARTNER_A).unwrap();
        assert!(state.approved_lanes.contains(&LANE_1));

        System::assert_has_event(RuntimeEvent::X3Partner(Event::LaneApproved {
            partner_id: PARTNER_A,
            lane_id: LANE_1,
        }));
    });
}

#[test]
fn add_approved_lane_duplicate_returns_error() {
    new_test_ext().execute_with(|| {
        setup_lane(LANE_1);
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));
        assert_ok!(X3Partner::add_approved_lane(root(), PARTNER_A, LANE_1));

        assert_noop!(
            X3Partner::add_approved_lane(root(), PARTNER_A, LANE_1),
            Error::<Test>::LaneAlreadyApproved
        );
    });
}

// ---------------------------------------------------------------------------
// Test 5: terminate_partner blocks reinstate
// ---------------------------------------------------------------------------

#[test]
fn terminate_partner_blocks_reinstate() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));

        // Suspend then terminate.
        assert_ok!(X3Partner::suspend_partner(root(), PARTNER_A));
        assert_ok!(X3Partner::terminate_partner(root(), PARTNER_A));

        System::assert_has_event(RuntimeEvent::X3Partner(Event::PartnerTerminated {
            partner_id: PARTNER_A,
        }));

        // Reinstate must be refused.
        assert_noop!(
            X3Partner::reinstate_partner(root(), PARTNER_A),
            Error::<Test>::CannotReinstateTerminated
        );
    });
}

#[test]
fn reinstate_partner_blocked_when_health_below_threshold() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));

        // Drive health below threshold.
        assert_ok!(X3Partner::update_health_metrics(
            root(),
            PARTNER_A,
            0,
            0,
            0,
            0,
            20,
        ));
        assert_ok!(X3Partner::suspend_partner(root(), PARTNER_A));

        assert_noop!(
            X3Partner::reinstate_partner(root(), PARTNER_A),
            Error::<Test>::PartnerUnhealthy
        );
    });
}

#[test]
fn reinstate_partner_succeeds_after_suspend_with_good_health() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));
        assert_ok!(X3Partner::suspend_partner(root(), PARTNER_A));

        // Health is still 10_000 (default), so reinstate should succeed.
        assert_ok!(X3Partner::reinstate_partner(root(), PARTNER_A));

        let state = Partners::<Test>::get(PARTNER_A).unwrap();
        assert_eq!(
            state.status,
            pallet_x3_inventory::types::PartnerStatus::Active
        );

        System::assert_has_event(RuntimeEvent::X3Partner(Event::PartnerReinstated {
            partner_id: PARTNER_A,
        }));
    });
}

// ---------------------------------------------------------------------------
// is_partner_eligible helper tests
// ---------------------------------------------------------------------------

#[test]
fn is_partner_eligible_returns_true_for_valid_active_partner_on_approved_lane() {
    new_test_ext().execute_with(|| {
        setup_lane(LANE_1);
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));
        assert_ok!(X3Partner::add_approved_lane(root(), PARTNER_A, LANE_1));

        assert!(crate::pallet::is_partner_eligible::<Test>(PARTNER_A, LANE_1));
    });
}

#[test]
fn is_partner_eligible_returns_false_for_suspended_partner() {
    new_test_ext().execute_with(|| {
        setup_lane(LANE_1);
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));
        assert_ok!(X3Partner::add_approved_lane(root(), PARTNER_A, LANE_1));
        assert_ok!(X3Partner::suspend_partner(root(), PARTNER_A));

        assert!(!crate::pallet::is_partner_eligible::<Test>(PARTNER_A, LANE_1));
    });
}

#[test]
fn is_partner_eligible_returns_false_for_unknown_partner() {
    new_test_ext().execute_with(|| {
        setup_lane(LANE_1);
        assert!(!crate::pallet::is_partner_eligible::<Test>(PARTNER_B, LANE_1));
    });
}

#[test]
fn is_partner_eligible_returns_false_for_unapproved_lane() {
    new_test_ext().execute_with(|| {
        setup_lane(LANE_1);
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));
        // Lane not approved for PARTNER_A.

        assert!(!crate::pallet::is_partner_eligible::<Test>(PARTNER_A, LANE_1));
    });
}

// ---------------------------------------------------------------------------
// remove_approved_lane test
// ---------------------------------------------------------------------------

#[test]
fn remove_approved_lane_cleans_both_indexes() {
    new_test_ext().execute_with(|| {
        setup_lane(LANE_1);
        assert_ok!(X3Partner::register_partner(root(), PARTNER_A, EXPOSURE_LIMIT));
        assert_ok!(X3Partner::add_approved_lane(root(), PARTNER_A, LANE_1));
        assert_ok!(X3Partner::remove_approved_lane(root(), PARTNER_A, LANE_1));

        let state = Partners::<Test>::get(PARTNER_A).unwrap();
        assert!(!state.approved_lanes.contains(&LANE_1));

        let lane_partners = crate::pallet::LanePartners::<Test>::get(LANE_1);
        assert!(!lane_partners.contains(&PARTNER_A));

        System::assert_has_event(RuntimeEvent::X3Partner(Event::LaneRevoked {
            partner_id: PARTNER_A,
            lane_id: LANE_1,
        }));
    });
}
