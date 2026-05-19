//! Tests for `pallet-x3-reconciliation`.
//!
//! Coverage target: ≥ 20 distinct test cases spanning:
//! - Supply report submission
//! - Reconciliation divergence arithmetic
//! - Halt triggering and lifting
//! - Governance power mapping and alert emission
//! - Access-control rejection for non-governance origins

use crate::{
    mock::*,
    pallet::{
        CanonicalSupply, ChainSupplyReports, GovernancePowerByChain, LastReconciliation,
        MintHaltSince, TotalGovernancePower,
    },
    ReconciliationStatus,
};
use frame_support::{assert_noop, assert_ok};
use pallet_x3_reconciliation::Pallet as Reconciliation;

// ── Helper ────────────────────────────────────────────────────────────────────

fn root() -> RuntimeOrigin {
    RuntimeOrigin::root()
}

fn signed(id: u64) -> RuntimeOrigin {
    RuntimeOrigin::signed(id)
}

// ── 1. Supply report submission ────────────────────────────────────────────────

#[test]
fn submit_chain_supply_report_stores_report() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            1_000_000
        ));
        let report = ChainSupplyReports::<Test>::get(1).expect("report must exist");
        assert_eq!(report.chain_id, 1);
        assert_eq!(report.wrapped_supply, 1_000_000);
        assert_eq!(report.reported_at, 1);
    });
}

#[test]
fn submit_chain_supply_report_overwrites_previous() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            500
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            999
        ));
        assert_eq!(
            ChainSupplyReports::<Test>::get(1).unwrap().wrapped_supply,
            999
        );
    });
}

#[test]
fn submit_chain_supply_report_rejects_non_governance() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Reconciliation::<Test>::submit_chain_supply_report(signed(1), 1, 100),
            frame_support::error::BadOrigin
        );
    });
}

// ── 2. Canonical supply ────────────────────────────────────────────────────────

#[test]
fn set_canonical_supply_stores_value() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            1_000_000
        ));
        assert_eq!(CanonicalSupply::<Test>::get(), 1_000_000);
    });
}

#[test]
fn set_canonical_supply_rejects_non_governance() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Reconciliation::<Test>::set_canonical_supply(signed(42), 999),
            frame_support::error::BadOrigin
        );
    });
}

// ── 3. Reconciliation — divergence arithmetic ──────────────────────────────────

#[test]
fn run_reconciliation_zero_divergence_when_exact_match() {
    new_test_ext().execute_with(|| {
        let canonical = 1_000_000u128;
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            canonical
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            canonical
        ));
        assert_ok!(Reconciliation::<Test>::run_reconciliation(root()));
        let rec = LastReconciliation::<Test>::get().unwrap();
        assert_eq!(rec.divergence_bps, 0);
        assert!(rec.passed);
    });
}

#[test]
fn run_reconciliation_computes_correct_divergence_bps() {
    new_test_ext().execute_with(|| {
        // canonical = 1_000_000, wrapped = 999_000  → diff = 1_000 → 10 bps
        let canonical = 1_000_000u128;
        let wrapped = 999_000u128;
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            canonical
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            wrapped
        ));
        assert_ok!(Reconciliation::<Test>::run_reconciliation(root()));
        let rec = LastReconciliation::<Test>::get().unwrap();
        // diff=1000, bps = 1000 * 10000 / 1_000_000 = 10
        assert_eq!(rec.divergence_bps, 10);
        assert_eq!(rec.sum_wrapped, wrapped);
        assert_eq!(rec.canonical_supply, canonical);
    });
}

#[test]
fn run_reconciliation_multi_chain_sums_correctly() {
    new_test_ext().execute_with(|| {
        // canonical = 2_000_000; chain1=1_000_000, chain2=980_000 → sum=1_980_000
        // diff = 20_000; bps = 20_000 * 10_000 / 2_000_000 = 100
        let canonical = 2_000_000u128;
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            canonical
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            2,
            980_000
        ));
        assert_ok!(Reconciliation::<Test>::run_reconciliation(root()));
        let rec = LastReconciliation::<Test>::get().unwrap();
        assert_eq!(rec.sum_wrapped, 1_980_000);
        assert_eq!(rec.divergence_bps, 100);
    });
}

#[test]
fn run_reconciliation_within_tolerance_does_not_halt() {
    new_test_ext().execute_with(|| {
        // tolerance = 1 bps; set supply perfectly matched → 0 bps divergence
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::run_reconciliation(root()));
        assert!(!Reconciliation::<Test>::is_minting_halted());
        assert_eq!(MintHaltSince::<Test>::get(), None);
    });
}

#[test]
fn run_reconciliation_above_tolerance_marks_degraded() {
    new_test_ext().execute_with(|| {
        // 10 bps divergence > 1 bps tolerance → Degraded (not yet Halted)
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            999_000
        ));
        assert_ok!(Reconciliation::<Test>::run_reconciliation(root()));
        let rec = LastReconciliation::<Test>::get().unwrap();
        assert!(!rec.passed);
        assert_eq!(
            Reconciliation::<Test>::reconciliation_status(),
            ReconciliationStatus::Degraded
        );
    });
}

#[test]
fn run_reconciliation_returns_divergence_bps_via_helper() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            999_000
        ));
        assert_ok!(Reconciliation::<Test>::run_reconciliation(root()));
        assert_eq!(Reconciliation::<Test>::current_divergence_bps(), Some(10));
    });
}

#[test]
fn run_reconciliation_accessible_from_any_signed_origin() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::run_reconciliation(signed(99)));
    });
}

// ── 4. Mint halt — on_initialize trigger ─────────────────────────────────────

#[test]
fn on_initialize_triggers_halt_after_threshold_blocks() {
    new_test_ext().execute_with(|| {
        // Setup divergence above tolerance.
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            999_000
        ));
        assert_ok!(Reconciliation::<Test>::run_reconciliation(root()));
        // Advance past halt threshold (TEST_HALT_THRESHOLD = 600).
        let trigger_block = 1u32 + TEST_HALT_THRESHOLD;
        System::set_block_number(trigger_block.into());
        // Simulate on_initialize.
        use frame_support::traits::Hooks;
        Reconciliation::<Test>::on_initialize(trigger_block.into());
        assert!(Reconciliation::<Test>::is_minting_halted());
        assert_eq!(MintHaltSince::<Test>::get(), Some(trigger_block.into()));
    });
}

#[test]
fn on_initialize_does_not_trigger_halt_before_threshold() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            999_000
        ));
        assert_ok!(Reconciliation::<Test>::run_reconciliation(root()));
        // Only 10 blocks in — well below 600-block threshold.
        System::set_block_number(10);
        use frame_support::traits::Hooks;
        Reconciliation::<Test>::on_initialize(10u32.into());
        assert!(!Reconciliation::<Test>::is_minting_halted());
    });
}

#[test]
fn on_initialize_does_not_double_halt() {
    new_test_ext().execute_with(|| {
        // Manually insert a halt.
        MintHaltSince::<Test>::put(Some(5u32));
        use frame_support::traits::Hooks;
        Reconciliation::<Test>::on_initialize(1000u32.into());
        // Halt block should remain at the original value.
        assert_eq!(MintHaltSince::<Test>::get(), Some(5u32));
    });
}

// ── 5. Lift mint halt ─────────────────────────────────────────────────────────

#[test]
fn lift_mint_halt_clears_halt_when_divergence_within_tolerance() {
    new_test_ext().execute_with(|| {
        // Set up passing reconciliation, then manually set a halt.
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::run_reconciliation(root()));
        MintHaltSince::<Test>::put(Some(1u32));
        assert_ok!(Reconciliation::<Test>::lift_mint_halt(root()));
        assert!(!Reconciliation::<Test>::is_minting_halted());
    });
}

#[test]
fn lift_mint_halt_rejected_when_divergence_still_above_tolerance() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reconciliation::<Test>::set_canonical_supply(
            root(),
            1_000_000
        ));
        assert_ok!(Reconciliation::<Test>::submit_chain_supply_report(
            root(),
            1,
            999_000
        ));
        assert_ok!(Reconciliation::<Test>::run_reconciliation(root()));
        MintHaltSince::<Test>::put(Some(1u32));
        assert_noop!(
            Reconciliation::<Test>::lift_mint_halt(root()),
            crate::pallet::Error::<Test>::DivergenceTooHigh
        );
    });
}

#[test]
fn lift_mint_halt_rejected_when_no_halt_active() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Reconciliation::<Test>::lift_mint_halt(root()),
            crate::pallet::Error::<Test>::HaltNotActive
        );
    });
}

#[test]
fn lift_mint_halt_rejects_non_governance() {
    new_test_ext().execute_with(|| {
        MintHaltSince::<Test>::put(Some(1u32));
        assert_noop!(
            Reconciliation::<Test>::lift_mint_halt(signed(7)),
            frame_support::error::BadOrigin
        );
    });
}

// ── 6. Governance power ────────────────────────────────────────────────────────

#[test]
fn update_governance_power_stores_and_sums_correctly() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reconciliation::<Test>::update_governance_power(
            root(),
            1,
            1_000
        ));
        assert_ok!(Reconciliation::<Test>::update_governance_power(
            root(),
            2,
            2_000
        ));
        assert_eq!(GovernancePowerByChain::<Test>::get(1), 1_000);
        assert_eq!(GovernancePowerByChain::<Test>::get(2), 2_000);
        assert_eq!(TotalGovernancePower::<Test>::get(), 3_000);
        assert_eq!(Reconciliation::<Test>::get_total_governance_power(), 3_000);
    });
}

#[test]
fn update_governance_power_rejects_non_governance() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Reconciliation::<Test>::update_governance_power(signed(3), 1, 500),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn governance_power_divergence_alert_fires_when_above_threshold() {
    new_test_ext().execute_with(|| {
        // chain1=10_000, chain2=100 → chain2 power share diverges greatly from average
        assert_ok!(Reconciliation::<Test>::update_governance_power(
            root(),
            1,
            10_000
        ));
        // Adding chain2 will trigger a divergence alert.
        assert_ok!(Reconciliation::<Test>::update_governance_power(
            root(),
            2,
            100
        ));
        // Verify the alert event was deposited.
        let events = System::events();
        let has_alert = events.iter().any(|r| {
            matches!(
                r.event,
                RuntimeEvent::X3Reconciliation(
                    crate::pallet::Event::GovernancePowerDivergenceAlert { .. }
                )
            )
        });
        assert!(has_alert, "expected GovernancePowerDivergenceAlert event");
    });
}

#[test]
fn governance_power_no_alert_when_within_threshold() {
    new_test_ext().execute_with(|| {
        // Balanced chains — no alert expected.
        assert_ok!(Reconciliation::<Test>::update_governance_power(
            root(),
            1,
            1_000
        ));
        assert_ok!(Reconciliation::<Test>::update_governance_power(
            root(),
            2,
            1_001
        ));
        let events = System::events();
        let has_alert = events.iter().any(|r| {
            matches!(
                r.event,
                RuntimeEvent::X3Reconciliation(
                    crate::pallet::Event::GovernancePowerDivergenceAlert { .. }
                )
            )
        });
        assert!(
            !has_alert,
            "unexpected GovernancePowerDivergenceAlert event"
        );
    });
}

// ── 7. Aggregate governance power ─────────────────────────────────────────────

#[test]
fn aggregate_governance_power_resums_correctly() {
    new_test_ext().execute_with(|| {
        GovernancePowerByChain::<Test>::insert(1u32, 500u128);
        GovernancePowerByChain::<Test>::insert(2u32, 1_500u128);
        assert_ok!(Reconciliation::<Test>::aggregate_governance_power(signed(
            1
        )));
        assert_eq!(Reconciliation::<Test>::get_total_governance_power(), 2_000);
    });
}

#[test]
fn aggregate_governance_power_accessible_from_any_origin() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reconciliation::<Test>::aggregate_governance_power(signed(
            99
        )));
    });
}

// ── 8. Status helpers ─────────────────────────────────────────────────────────

#[test]
fn reconciliation_status_passing_when_no_record() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            Reconciliation::<Test>::reconciliation_status(),
            ReconciliationStatus::Passing
        );
    });
}

#[test]
fn reconciliation_status_halted_when_halt_active() {
    new_test_ext().execute_with(|| {
        MintHaltSince::<Test>::put(Some(1u32));
        assert_eq!(
            Reconciliation::<Test>::reconciliation_status(),
            ReconciliationStatus::Halted
        );
    });
}

#[test]
fn is_minting_halted_false_by_default() {
    new_test_ext().execute_with(|| {
        assert!(!Reconciliation::<Test>::is_minting_halted());
    });
}

#[test]
fn current_divergence_bps_none_when_no_reconciliation() {
    new_test_ext().execute_with(|| {
        assert_eq!(Reconciliation::<Test>::current_divergence_bps(), None);
    });
}

#[test]
fn run_reconciliation_zero_canonical_yields_zero_divergence() {
    new_test_ext().execute_with(|| {
        // Canonical is 0 (unset). Division guard should return 0 bps.
        assert_ok!(Reconciliation::<Test>::run_reconciliation(root()));
        assert_eq!(Reconciliation::<Test>::current_divergence_bps(), Some(0));
    });
}
