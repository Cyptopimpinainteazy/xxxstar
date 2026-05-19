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

// ── Phase 0 constitutional controls ───────────────────────────────────────────

/// Registering an authority and then activating the kill switch must set
/// `KillSwitches` to `true` for that module.
#[test]
fn register_and_activate_kill_switch() {
    use crate::emergency::ModuleId;
    use pallet::KillSwitches;
    use parity_scale_codec::Encode;
    use sp_io::hashing::blake2_256;

    new_test_ext().execute_with(|| {
        let module_id: ModuleId = [1u8; 32];
        let account_id: u64 = 42;
        let authority_id = blake2_256(&account_id.encode());

        assert_ok!(Invariants::register_emergency_authority(
            RuntimeOrigin::root(),
            module_id,
            authority_id,
            1_000u64, // expires far in the future
            false,    // evidence not required
            [0xAAu8; 32],
        ));

        // Activate from the registered account at a block well before expiry.
        System::set_block_number(1);
        assert_ok!(Invariants::activate_kill_switch(
            RuntimeOrigin::signed(account_id),
            module_id,
            None,
        ));

        assert!(
            KillSwitches::<Test>::get(module_id),
            "kill switch should be active after activation"
        );
    });
}

/// Activation must fail with `AuthorityExpired` when the current block is at or
/// past the record's `expires_at_block`.
#[test]
fn activate_fails_when_expired() {
    use crate::emergency::ModuleId;
    use parity_scale_codec::Encode;
    use sp_io::hashing::blake2_256;

    new_test_ext().execute_with(|| {
        let module_id: ModuleId = [2u8; 32];
        let account_id: u64 = 42;
        let authority_id = blake2_256(&account_id.encode());

        assert_ok!(Invariants::register_emergency_authority(
            RuntimeOrigin::root(),
            module_id,
            authority_id,
            5u64, // expires at block 5
            false,
            [0u8; 32],
        ));

        // At block 5: `5 < 5` is false, so the record is considered expired.
        System::set_block_number(5);
        assert_noop!(
            Invariants::activate_kill_switch(RuntimeOrigin::signed(account_id), module_id, None,),
            Error::<Test>::AuthorityExpired
        );
    });
}

/// Activation must fail with `NotTheRegisteredAuthority` when the signed origin
/// does not match the `authority_id` stored in the registry.
#[test]
fn activate_fails_wrong_origin() {
    use crate::emergency::ModuleId;
    use parity_scale_codec::Encode;
    use sp_io::hashing::blake2_256;

    new_test_ext().execute_with(|| {
        let module_id: ModuleId = [3u8; 32];
        let registered_account: u64 = 42;
        let authority_id = blake2_256(&registered_account.encode());

        assert_ok!(Invariants::register_emergency_authority(
            RuntimeOrigin::root(),
            module_id,
            authority_id,
            1_000u64,
            false,
            [0u8; 32],
        ));

        System::set_block_number(1);

        // Account 99 is not the registered authority; its hash won't match.
        assert_noop!(
            Invariants::activate_kill_switch(RuntimeOrigin::signed(99u64), module_id, None,),
            Error::<Test>::NotTheRegisteredAuthority
        );
    });
}

/// When `requires_evidence = true`, omitting the evidence hash must return
/// `EvidenceRequired` and the kill switch must remain inactive.
#[test]
fn evidence_required_but_missing() {
    use crate::emergency::ModuleId;
    use pallet::KillSwitches;
    use parity_scale_codec::Encode;
    use sp_io::hashing::blake2_256;

    new_test_ext().execute_with(|| {
        let module_id: ModuleId = [4u8; 32];
        let account_id: u64 = 42;
        let authority_id = blake2_256(&account_id.encode());

        assert_ok!(Invariants::register_emergency_authority(
            RuntimeOrigin::root(),
            module_id,
            authority_id,
            1_000u64,
            true, // evidence IS required
            [0u8; 32],
        ));

        System::set_block_number(1);

        assert_noop!(
            Invariants::activate_kill_switch(
                RuntimeOrigin::signed(account_id),
                module_id,
                None, // evidence hash omitted
            ),
            Error::<Test>::EvidenceRequired
        );

        // Kill switch must remain inactive.
        assert!(!KillSwitches::<Test>::get(module_id));
    });
}

/// Registering a canonical truth source, reading it back, and then removing it
/// must leave the storage entry absent.
#[test]
fn canonical_truth_roundtrip() {
    use crate::emergency::DomainId;
    use pallet::CanonicalTruthMap;

    new_test_ext().execute_with(|| {
        let domain_id: DomainId = [5u8; 32];
        let pallet_name_hash = [0x10u8; 32];
        let storage_item_hash = [0x20u8; 32];
        let description_hash = [0x30u8; 32];

        // Register.
        assert_ok!(Invariants::register_canonical_truth_source(
            RuntimeOrigin::root(),
            domain_id,
            pallet_name_hash,
            storage_item_hash,
            description_hash,
        ));

        // Verify stored fields.
        let record = CanonicalTruthMap::<Test>::get(domain_id)
            .expect("canonical truth record must exist after registration");
        assert_eq!(record.domain_id, domain_id);
        assert_eq!(record.pallet_name_hash, pallet_name_hash);
        assert_eq!(record.storage_item_hash, storage_item_hash);
        assert_eq!(record.description_hash, description_hash);

        // Also verify via the public helper.
        assert!(crate::get_canonical_source::<Test>(domain_id).is_some());

        // Remove.
        assert_ok!(Invariants::remove_canonical_truth_source(
            RuntimeOrigin::root(),
            domain_id,
        ));

        assert!(CanonicalTruthMap::<Test>::get(domain_id).is_none());
        assert!(crate::get_canonical_source::<Test>(domain_id).is_none());
    });
}
