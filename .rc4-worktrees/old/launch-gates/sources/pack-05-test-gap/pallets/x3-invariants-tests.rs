//! Unit tests for pallet-x3-invariants.

use super::*;
use crate::{mock::*, InvariantKind};
use frame_support::{assert_noop, assert_ok};
use pallet::{
    AgentCount, ConstitutionHash, HaltOnViolation, LastObservedIssuance, MaxAgents,
    MaxProposalDepth, MaxSupply, ProposalDepth, ViolationCount,
};

fn init_block(block: u64) {
    System::set_block_number(block);
}

// ── set_bounds ────────────────────────────────────────────────────────────────

#[test]
fn set_bounds_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Invariants::set_bounds(
            RuntimeOrigin::root(),
            500_000_000_000_000_000,
            5_000,
            50
        ));
        assert_eq!(MaxSupply::<Test>::get(), 500_000_000_000_000_000);
        assert_eq!(MaxAgents::<Test>::get(), 5_000);
        assert_eq!(MaxProposalDepth::<Test>::get(), 50);
    });
}

#[test]
fn set_bounds_rejects_weakening() {
    new_test_ext().execute_with(|| {
        // Initial bound: 1B tokens. Trying to raise it is a weakening → rejected.
        assert_noop!(
            Invariants::set_bounds(
                RuntimeOrigin::root(),
                2_000_000_000_000_000_000, // higher than genesis 1B
                10_000,
                100,
            ),
            Error::<Test>::BoundWeakeningNotAllowed
        );
    });
}

#[test]
fn set_bounds_requires_root() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Invariants::set_bounds(RuntimeOrigin::signed(1), 100, 100, 100),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

// ── report_issuance ───────────────────────────────────────────────────────────

#[test]
fn report_issuance_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Invariants::report_issuance(RuntimeOrigin::root(), 42_000));
        assert_eq!(LastObservedIssuance::<Test>::get(), 42_000);
    });
}

// ── enforce_all — no violations ───────────────────────────────────────────────

#[test]
fn enforce_all_passes_when_within_bounds() {
    new_test_ext().execute_with(|| {
        init_block(1);
        LastObservedIssuance::<Test>::put(999_999_999_000_000_000u128);
        AgentCount::<Test>::put(9_999);
        ProposalDepth::<Test>::put(99);

        Invariants::enforce_all(1u64);

        assert_eq!(ViolationCount::<Test>::get(), 0);
        System::assert_has_event(RuntimeEvent::Invariants(Event::InvariantCheckPassed {
            block: 1,
        }));
    });
}

// ── enforce_all — supply violation ────────────────────────────────────────────

#[test]
fn enforce_all_emits_event_on_supply_violation() {
    new_test_ext().execute_with(|| {
        init_block(2);
        // Exceed max supply
        LastObservedIssuance::<Test>::put(2_000_000_000_000_000_000u128);

        Invariants::enforce_all(2u64);

        assert_eq!(ViolationCount::<Test>::get(), 1);
        System::assert_has_event(RuntimeEvent::Invariants(Event::InvariantViolated {
            block: 2,
            invariant: InvariantKind::MaxSupply,
            observed: 2_000_000_000_000_000_000,
            bound: 1_000_000_000_000_000_000,
        }));
    });
}

// ── enforce_all — agent count violation ───────────────────────────────────────

#[test]
fn enforce_all_emits_event_on_agent_violation() {
    new_test_ext().execute_with(|| {
        init_block(3);
        AgentCount::<Test>::put(20_000); // exceeds max 10_000

        Invariants::enforce_all(3u64);

        assert_eq!(ViolationCount::<Test>::get(), 1);
        System::assert_has_event(RuntimeEvent::Invariants(Event::InvariantViolated {
            block: 3,
            invariant: InvariantKind::MaxAgents,
            observed: 20_000,
            bound: 10_000,
        }));
    });
}

// ── enforce_all — proposal depth violation ────────────────────────────────────

#[test]
fn enforce_all_emits_event_on_depth_violation() {
    new_test_ext().execute_with(|| {
        init_block(4);
        ProposalDepth::<Test>::put(200); // exceeds max 100

        Invariants::enforce_all(4u64);

        assert_eq!(ViolationCount::<Test>::get(), 1);
        System::assert_has_event(RuntimeEvent::Invariants(Event::InvariantViolated {
            block: 4,
            invariant: InvariantKind::MaxProposalDepth,
            observed: 200,
            bound: 100,
        }));
    });
}

// ── multiple violations in one block ─────────────────────────────────────────

#[test]
fn multiple_violations_accumulate() {
    new_test_ext().execute_with(|| {
        LastObservedIssuance::<Test>::put(9_000_000_000_000_000_000u128);
        AgentCount::<Test>::put(99_999);
        ProposalDepth::<Test>::put(999);

        Invariants::enforce_all(5u64);

        assert_eq!(ViolationCount::<Test>::get(), 3);
    });
}

// ── all_invariants_hold ───────────────────────────────────────────────────────

#[test]
fn all_invariants_hold_returns_true_when_clean() {
    new_test_ext().execute_with(|| {
        assert!(Invariants::all_invariants_hold());
    });
}

#[test]
fn all_invariants_hold_returns_false_on_supply_breach() {
    new_test_ext().execute_with(|| {
        LastObservedIssuance::<Test>::put(u128::MAX);
        assert!(!Invariants::all_invariants_hold());
    });
}

// ── halt_on_violation flag ────────────────────────────────────────────────────

#[test]
fn set_halt_on_violation_toggles_flag() {
    new_test_ext().execute_with(|| {
        assert_eq!(HaltOnViolation::<Test>::get(), false);
        assert_ok!(Invariants::set_halt_on_violation(
            RuntimeOrigin::root(),
            true
        ));
        assert_eq!(HaltOnViolation::<Test>::get(), true);
        assert_ok!(Invariants::set_halt_on_violation(
            RuntimeOrigin::root(),
            false
        ));
        assert_eq!(HaltOnViolation::<Test>::get(), false);
    });
}

// ── constitution hash ─────────────────────────────────────────────────────────

#[test]
fn set_constitution_hash_works() {
    new_test_ext().execute_with(|| {
        let hash = [0xABu8; 32];
        assert_ok!(Invariants::set_constitution_hash(
            RuntimeOrigin::root(),
            hash
        ));
        assert_eq!(ConstitutionHash::<Test>::get(), hash);
    });
}

#[test]
fn set_constitution_hash_rejects_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Invariants::set_constitution_hash(RuntimeOrigin::root(), [0u8; 32]),
            Error::<Test>::InvalidConstitutionHash
        );
    });
}

// ── InvariantReporter trait ───────────────────────────────────────────────────

#[test]
fn invariant_reporter_increments_and_decrements_agents() {
    new_test_ext().execute_with(|| {
        use crate::InvariantReporter;
        assert_eq!(AgentCount::<Test>::get(), 0);
        Invariants::increment_agent_count();
        Invariants::increment_agent_count();
        assert_eq!(AgentCount::<Test>::get(), 2);
        Invariants::decrement_agent_count();
        assert_eq!(AgentCount::<Test>::get(), 1);
    });
}

#[test]
fn invariant_reporter_sets_proposal_depth() {
    new_test_ext().execute_with(|| {
        use crate::InvariantReporter;
        Invariants::set_proposal_depth(42);
        assert_eq!(ProposalDepth::<Test>::get(), 42);
    });
}
