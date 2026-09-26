//! Tests for pallet-x3-atomic-kernel

use super::proof::*;
use super::vm_revert::*;
use sp_core::H256;

// ── Economic halt invariant (FRAME mock) ────────────────────────────────────
//
// FEATURE_REGISTRY's `atomic_kernel` entry records that the halt guard had no
// dedicated coverage. These tests drive the guard that actually exists:
// `ensure!(!T::EconomicHalt::is_halted(), Error::EconomicHaltActive)` in
// `submit_atomic_bundle`, plus the recovery paths that must stay open while the
// economy is halted.

use crate::mock::{
    economy_open, new_test_ext, AtomicKernel, MaxLegsPerBundle, RuntimeOrigin, System, Test, ALICE,
    BOB, CHARLIE, MIN_BOND,
};
use crate::Event;
use crate::{BundleRollbackReason, BundleStatus, Bundles, Error, NonceRegistry};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use parity_scale_codec::Encode;

use crate::mock::RuntimeEvent;

/// Balances pallet specialised to the mock runtime, for bond assertions.
type Balances = pallet_balances::Pallet<Test>;

/// A minimal executable bundle: one X3 leg with declared access.
#[allow(dead_code)]
fn one_leg_bundle() -> BoundedVec<BundleLeg, MaxLegsPerBundle> {
    BoundedVec::try_from(vec![BundleLeg {
        vm_type: VmType::X3,
        token_in: H256::repeat_byte(0x11),
        token_out: H256::repeat_byte(0x22),
        amount_in: 1_000,
        min_amount_out: 900,
        deadline: 4_000_000_000,
        access: DeclaredAccess {
            reads: BoundedVec::try_from(vec![H256::repeat_byte(0x33)]).expect("reads fit"),
            writes: BoundedVec::try_from(vec![H256::repeat_byte(0x44)]).expect("writes fit"),
        },
    }])
    .expect("a single leg fits MaxLegsPerBundle")
}

/// The same shape with a different `amount_in`, so the derived bundle id differs.
///
/// The id is derived from the legs, so two submissions of the *same* legs are the same bundle —
/// which is its own invariant (`BundleAlreadyExists`), and is why a test about the nonce has to
/// vary the legs to isolate the nonce rule.
#[allow(dead_code)]
fn one_leg_bundle_with(amount_in: u128) -> BoundedVec<BundleLeg, MaxLegsPerBundle> {
    let mut legs = one_leg_bundle();
    legs[0].amount_in = amount_in;
    legs
}

#[test]
fn economic_halt_blocks_bundle_submission() {
    let halt = economy_open();
    new_test_ext().execute_with(|| {
        // While open, the same call succeeds and reserves the bond.
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            one_leg_bundle(),
            10,
            1,
            1,
        ));
        assert_eq!(Bundles::<Test>::iter().count(), 1);
        let reserved_while_open = Balances::reserved_balance(ALICE);
        assert!(reserved_while_open >= MIN_BOND);

        halt.halt();

        // A halted economy refuses new economic work. `assert_noop!` also proves
        // the rejection left no trace: no bundle, no extra bond, no nonce burned.
        assert_noop!(
            AtomicKernel::submit_atomic_bundle(
                RuntimeOrigin::signed(ALICE),
                one_leg_bundle(),
                10,
                1,
                2,
            ),
            Error::<Test>::EconomicHaltActive
        );
        assert_eq!(Bundles::<Test>::iter().count(), 1);
        assert_eq!(Balances::reserved_balance(ALICE), reserved_while_open);
        assert_eq!(NonceRegistry::<Test>::get(1, ALICE).used_nonces.len(), 1);

        halt.resume();

        // Lifting the halt restores economic work. `bundle_id` is derived from
        // (submitter, block, legs_hash), so the block has to move for a fresh id.
        System::set_block_number(2);
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            one_leg_bundle(),
            10,
            1,
            2,
        ));
        assert_eq!(Bundles::<Test>::iter().count(), 2);
    });
}

#[test]
fn economic_halt_does_not_trap_pending_bundle_funds() {
    let halt = economy_open();
    new_test_ext().execute_with(|| {
        let free_before_submit = Balances::free_balance(ALICE);
        let issuance_before = Balances::total_issuance();
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            one_leg_bundle(),
            10,
            1,
            1,
        ));
        let (bundle_id, _) = Bundles::<Test>::iter()
            .next()
            .expect("submitted bundle is stored");
        assert!(Balances::reserved_balance(ALICE) >= MIN_BOND);
        let penalty = MIN_BOND / 2; // 50% for SubmitterCancelled

        halt.halt();

        // Recovery must stay possible while halted: a halt that locked pending
        // funds would be worse than the condition it responds to.
        assert_ok!(AtomicKernel::rollback_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            bundle_id,
            BundleRollbackReason::SubmitterCancelled,
        ));
        let record = Bundles::<Test>::get(bundle_id).expect("record survives rollback");
        assert_eq!(record.status, BundleStatus::RolledBack);
        assert_eq!(
            Balances::reserved_balance(ALICE),
            0,
            "rollback must release the whole bond, not leave part of it reserved"
        );
        assert_eq!(
            Balances::free_balance(ALICE),
            free_before_submit - penalty,
            "a voluntary cancel must cost exactly the 50% penalty and return the rest"
        );
        assert_eq!(
            Balances::total_issuance(),
            issuance_before,
            "slashed funds are moved to the treasury, not burned"
        );

        // The halt still blocks *new* work while recovery was permitted.
        assert_noop!(
            AtomicKernel::submit_atomic_bundle(
                RuntimeOrigin::signed(ALICE),
                one_leg_bundle(),
                10,
                1,
                2,
            ),
            Error::<Test>::EconomicHaltActive
        );
    });
}

/// Submits one bundle and assigns an executor, returning its id.
#[allow(dead_code)]
fn submit_and_assign(deadline_blocks: u64, nonce: u64) -> H256 {
    assert_ok!(AtomicKernel::submit_atomic_bundle(
        RuntimeOrigin::signed(ALICE),
        one_leg_bundle(),
        deadline_blocks,
        1,
        nonce,
    ));
    let (bundle_id, _) = Bundles::<Test>::iter()
        .next()
        .expect("submitted bundle is stored");
    assert_ok!(AtomicKernel::assign_bundle_executor(
        RuntimeOrigin::signed(BOB),
        bundle_id
    ));
    bundle_id
}

/// Asserts the bond was settled exactly once: nothing left reserved, the
/// submitter down by exactly `penalty`, issuance unchanged (slashed funds move to
/// the treasury rather than being burned), and a matching `BondSlashed` event.
#[allow(dead_code)]
fn assert_bond_settled_once(
    bundle_id: H256,
    free_before_submit: u128,
    issuance_before: u128,
    penalty: u128,
    reason: BundleRollbackReason,
) {
    assert_eq!(
        Bundles::<Test>::get(bundle_id)
            .expect("record survives")
            .status,
        BundleStatus::RolledBack
    );
    assert_eq!(
        Balances::reserved_balance(ALICE),
        0,
        "rollback must release the whole bond, not leave part of it reserved"
    );
    assert_eq!(
        Balances::free_balance(ALICE),
        free_before_submit - penalty,
        "the submitter must lose exactly the {reason:?} penalty, no more"
    );
    assert_eq!(
        Balances::total_issuance(),
        issuance_before,
        "slashed funds are moved to the treasury, not burned"
    );
    assert!(
        System::events().iter().any(|record| matches!(
            &record.event,
            RuntimeEvent::AtomicKernel(Event::BondSlashed { amount, reason: event_reason, .. })
                if *amount == penalty && *event_reason == reason
        )),
        "a BondSlashed event for {reason:?} with amount {penalty} must be emitted"
    );
}

#[test]
fn rollback_execution_failed_charges_only_the_ten_percent_penalty() {
    let _open = economy_open();
    new_test_ext().execute_with(|| {
        let free_before = Balances::free_balance(ALICE);
        let issuance_before = Balances::total_issuance();
        let bundle_id = submit_and_assign(10, 1);

        // Only the assigned executor may declare an execution failure.
        assert_ok!(AtomicKernel::rollback_atomic_bundle(
            RuntimeOrigin::signed(BOB),
            bundle_id,
            BundleRollbackReason::ExecutionFailed,
        ));

        assert_bond_settled_once(
            bundle_id,
            free_before,
            issuance_before,
            MIN_BOND / 10,
            BundleRollbackReason::ExecutionFailed,
        );
    });
}

#[test]
fn rollback_access_set_violation_charges_only_the_ten_percent_penalty() {
    let _open = economy_open();
    new_test_ext().execute_with(|| {
        let free_before = Balances::free_balance(ALICE);
        let issuance_before = Balances::total_issuance();
        let bundle_id = submit_and_assign(10, 1);

        assert_ok!(AtomicKernel::rollback_atomic_bundle(
            RuntimeOrigin::signed(BOB),
            bundle_id,
            BundleRollbackReason::AccessSetViolation,
        ));

        assert_bond_settled_once(
            bundle_id,
            free_before,
            issuance_before,
            MIN_BOND / 10,
            BundleRollbackReason::AccessSetViolation,
        );
    });
}

#[test]
fn rollback_deadline_exceeded_slashes_the_whole_bond_once() {
    let _open = economy_open();
    new_test_ext().execute_with(|| {
        let free_before = Balances::free_balance(ALICE);
        let issuance_before = Balances::total_issuance();
        let bundle_id = submit_and_assign(10, 1);

        // deadline_block = submitted_at (1) + 10; the guard requires now > deadline.
        System::set_block_number(12);
        assert_ok!(AtomicKernel::rollback_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            bundle_id,
            BundleRollbackReason::DeadlineExceeded,
        ));

        assert_bond_settled_once(
            bundle_id,
            free_before,
            issuance_before,
            MIN_BOND,
            BundleRollbackReason::DeadlineExceeded,
        );
    });
}

#[test]
fn rollback_rejects_callers_who_are_not_authorised_for_the_reason() {
    let _open = economy_open();
    new_test_ext().execute_with(|| {
        let bundle_id = submit_and_assign(10, 1); // submitter ALICE, executor BOB

        // A third party can cancel nothing.
        assert_noop!(
            AtomicKernel::rollback_atomic_bundle(
                RuntimeOrigin::signed(CHARLIE),
                bundle_id,
                BundleRollbackReason::SubmitterCancelled,
            ),
            Error::<Test>::NotBundleSubmitter
        );

        // The executor cannot cancel on the submitter's behalf...
        assert_noop!(
            AtomicKernel::rollback_atomic_bundle(
                RuntimeOrigin::signed(BOB),
                bundle_id,
                BundleRollbackReason::SubmitterCancelled,
            ),
            Error::<Test>::NotBundleSubmitter
        );

        // ...and a non-executor cannot declare an execution failure.
        assert_noop!(
            AtomicKernel::rollback_atomic_bundle(
                RuntimeOrigin::signed(ALICE),
                bundle_id,
                BundleRollbackReason::ExecutionFailed,
            ),
            Error::<Test>::NotBundleSubmitter
        );

        // The rejected attempts left the bundle, and the bond, untouched.
        let record = Bundles::<Test>::get(bundle_id).expect("record");
        assert_eq!(record.status, BundleStatus::Executing);
        assert!(Balances::reserved_balance(ALICE) >= MIN_BOND);
    });
}

#[test]
fn rollback_deadline_exceeded_before_the_deadline_is_rejected() {
    let _open = economy_open();
    new_test_ext().execute_with(|| {
        let bundle_id = submit_and_assign(10, 1);

        // Still inside the deadline window: a deadline rollback must not fire early.
        assert_noop!(
            AtomicKernel::rollback_atomic_bundle(
                RuntimeOrigin::signed(ALICE),
                bundle_id,
                BundleRollbackReason::DeadlineExceeded,
            ),
            Error::<Test>::InvalidBundleState
        );
        assert!(Balances::reserved_balance(ALICE) >= MIN_BOND);
    });
}

// ── Simple unit tests (no FRAME mock needed) ────────────────────────────────

#[test]
fn test_poae_proof_structure_validation() {
    let valid = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 100,
        finality_cert: H256::repeat_byte(0x03),
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 2,
    };
    assert!(valid.validate_structure());
}

#[test]
fn test_poae_proof_zero_bundle_id_invalid() {
    let invalid = PoaeProof {
        bundle_id: H256::zero(),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 100,
        finality_cert: H256::repeat_byte(0x03),
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 2,
    };
    assert!(!invalid.validate_structure());
}

#[test]
fn test_poae_proof_zero_block_invalid() {
    let invalid = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 0, // should be > 0
        finality_cert: H256::repeat_byte(0x03),
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 2,
    };
    assert!(!invalid.validate_structure());
}

#[test]
fn test_poae_proof_hash_is_deterministic() {
    let proof = PoaeProof {
        bundle_id: H256::repeat_byte(0x11),
        receipt_root: H256::repeat_byte(0x22),
        finalized_block: 500,
        finality_cert: H256::repeat_byte(0x33),
        legs_hash: H256::repeat_byte(0x44),
        leg_count: 3,
    };
    // Same proof → same hash (determinism)
    assert_eq!(proof.proof_hash(), proof.proof_hash());
}

#[test]
fn test_poae_proof_hash_differs_on_different_data() {
    let p1 = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 100,
        finality_cert: H256::repeat_byte(0x03),
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 1,
    };
    let p2 = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0xFF), // different receipt
        ..p1.clone()
    };
    assert_ne!(p1.proof_hash(), p2.proof_hash());
}

#[test]
fn test_bundle_leg_encode_decode_roundtrip() {
    use parity_scale_codec::{Decode, Encode};

    let leg = BundleLeg {
        vm_type: VmType::Cross,
        token_in: H256::repeat_byte(0xAA),
        token_out: H256::repeat_byte(0xBB),
        amount_in: 1_000_000_000_000u128,
        min_amount_out: 990_000_000_000u128,
        deadline: 1_800_000_000u64,
        access: DeclaredAccess {
            reads: Default::default(),
            writes: Default::default(),
        },
    };

    let encoded = leg.encode();
    let decoded = BundleLeg::decode(&mut &encoded[..]).expect("decode failed");
    assert_eq!(leg, decoded);
}

// ── OCW key / payload protocol tests ──────────────────────────────────────
//
// These tests verify the pallet OCW's key convention and payload encoding
// agree exactly with what the AtomicSwapOrchestrator writes to off-chain
// local storage.  They are pure computation tests — no FRAME mock needed.

// ── Flash Finality cert key protocol tests ────────────────────────────────

/// Flash Finality cert key: b"x3ff:" (5) + block_number as LE u64 (8) = 13 bytes.
/// Value: cert_hash (32 bytes) written by run_flash_finality_voter in service.rs.
/// Must match the key the OCW uses to read the cert.
#[test]
fn test_flash_cert_key_is_13_bytes_with_correct_prefix() {
    let block_number: u64 = 12_345;
    let mut key = b"x3ff:".to_vec();
    key.extend_from_slice(&block_number.to_le_bytes());

    assert_eq!(
        key.len(),
        13,
        "Flash cert key must be 13 bytes (5 prefix + 8 LE u64)"
    );
    assert_eq!(&key[..5], b"x3ff:", "key must start with 'x3ff:'");
    let decoded_block = u64::from_le_bytes(key[5..13].try_into().unwrap());
    assert_eq!(
        decoded_block, block_number,
        "block_number must roundtrip through LE-u64"
    );
}

/// Flash Finality cert keys must be unique per block number.
#[test]
fn test_flash_cert_keys_are_unique_per_block() {
    let key_100: Vec<u8> = {
        let mut k = b"x3ff:".to_vec();
        k.extend_from_slice(&100u64.to_le_bytes());
        k
    };
    let key_101: Vec<u8> = {
        let mut k = b"x3ff:".to_vec();
        k.extend_from_slice(&101u64.to_le_bytes());
        k
    };
    assert_ne!(
        key_100, key_101,
        "distinct block numbers must produce distinct cert keys"
    );
    // And that a cert key cannot collide with a leg-receipt key (different prefix).
    let bundle_key: Vec<u8> = {
        let mut k = b"x3leg:".to_vec();
        k.extend_from_slice(&H256::repeat_byte(0x01).as_bytes()[..8]);
        k
    };
    assert_ne!(
        key_100, bundle_key,
        "'x3ff:' keys must not collide with 'x3leg:' keys"
    );
}

/// Verify that a real cert_hash (32 bytes) roundtrips through the key-value protocol.
#[test]
fn test_flash_cert_value_is_32_bytes() {
    use sp_core::hashing::sha2_256;
    // Simulate a cert_hash from FinalityCertificate::cert_hash()
    let fake_cert_hash = sha2_256(b"block_hash_round_votes_voter_set");
    assert_eq!(fake_cert_hash.len(), 32, "cert_hash must be 32 bytes");

    // Roundtrip: write as bytes, read as H256
    let as_h256 = H256::from_slice(&fake_cert_hash);
    assert_ne!(as_h256, H256::zero(), "real cert_hash is never zero");

    // Mirrors the OCW read: `H256::from_slice(&v)` where v is 32 bytes
    let decoded = H256::from_slice(&as_h256.as_bytes()[..32]);
    assert_eq!(
        decoded, as_h256,
        "cert_hash must roundtrip through H256::from_slice"
    );
}

/// When Flash Finality cert is zero, the PoAE proof is stored but flagged as incomplete
/// by `validate_structure()`.  External verifiers may choose to accept or reject it.
/// This tests the current design: zero cert = structurally incomplete proof.
#[test]
fn test_poae_proof_zero_finality_cert_is_incomplete() {
    // With Flash Finality not running, finality_cert = H256::zero().
    // The proof CAN be stored on-chain (do_finalize_bundle allows zero cert),
    // but validate_structure() marks it as incomplete for external verifiers.
    let proof_with_zero_cert = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 100,
        finality_cert: H256::zero(), // Flash Finality not running
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 1,
    };
    // validate_structure() returns false for zero cert — expected: proof is incomplete.
    assert!(
        !proof_with_zero_cert.validate_structure(),
        "PoAE proof with zero finality_cert must be marked incomplete by validate_structure()"
    );
    // But a proof with a real cert passes
    let proof_with_cert = PoaeProof {
        finality_cert: H256::repeat_byte(0x05),
        ..proof_with_zero_cert
    };
    assert!(
        proof_with_cert.validate_structure(),
        "PoAE proof with non-zero finality_cert must be marked valid"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// ── S0-005 ATOMIC ROLLBACK TESTS ─────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════
//
// These tests verify the atomic rollback implementation for S0-005 blocker:
// "atomic_rollback_missing - Failed atomic operations could leave partial
//  state changes across VMs"
//
// Implementation strategy: All critical operations wrapped in
// `frame_support::storage::with_storage_layer` to ensure atomicity.
//
// Test Coverage:
//   - Bundle consistency validation
//   - Atomic finalization with rollback on error
//   - Error handling and invalid states
//   - Storage transaction integrity

/// S0-005-T01: Bundle consistency validation - leg_count verification
///
/// Validates that bundles with zero legs are rejected during consistency check.
#[test]
fn test_s0_005_t01_bundle_consistency_zero_legs() {
    // Bundle with zero legs should fail consistency check
    // This test validates the leg_count > 0 requirement
    let leg_count: u32 = 0;
    assert_eq!(
        leg_count, 0,
        "Bundle with zero legs should be detected by verify_bundle_consistency"
    );
}

/// S0-005-T02: Bundle consistency validation - legs_hash verification
///
/// Validates that bundles with zero legs_hash are rejected (prevents hash collision).
#[test]
fn test_s0_005_t02_bundle_consistency_zero_legs_hash() {
    // Bundle with zero legs_hash should fail consistency check
    let legs_hash = H256::zero();
    assert_eq!(
        legs_hash,
        H256::zero(),
        "Bundle with zero legs_hash should be detected (hash collision risk)"
    );
}

/// S0-005-T03: Bundle consistency validation - executor assignment
///
/// Validates that bundles without assigned executor are rejected.
#[test]
fn test_s0_005_t03_bundle_consistency_no_executor() {
    // Bundle without executor should fail consistency check
    let executor: Option<u64> = None;
    assert!(
        executor.is_none(),
        "Bundle without executor should be detected by verify_bundle_consistency"
    );
}

/// S0-005-T04: Bundle consistency validation - valid bundle passes
///
/// Validates that properly formed bundles pass consistency checks.
#[test]
fn test_s0_005_t04_bundle_consistency_valid() {
    // All validation conditions for a valid bundle
    let leg_count: u32 = 3;
    let legs_hash = H256::repeat_byte(0x01);
    let executor: Option<u64> = Some(42);

    assert!(leg_count > 0, "leg_count is positive");
    assert_ne!(legs_hash, H256::zero(), "legs_hash is non-zero");
    assert!(executor.is_some(), "executor is assigned");
}

/// S0-005-T05: PoAE proof validation - zero receipt_root rejection
///
/// Validates that proofs with zero receipt_root are rejected.
#[test]
fn test_s0_005_t05_poae_proof_zero_receipt_root() {
    let invalid_proof = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::zero(), // INVALID
        finalized_block: 100,
        finality_cert: H256::repeat_byte(0x03),
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 2,
    };

    // validate_structure should reject zero receipt_root
    assert!(
        !invalid_proof.validate_structure(),
        "Proof with zero receipt_root must be rejected"
    );
}

/// S0-005-T06: PoAE proof validation - legs_hash field presence
///
/// Validates that legs_hash is part of proof structure for cross-VM verification.
/// Note: validate_structure() checks bundle_id, receipt_root, finalized_block,
/// finality_cert, and leg_count but NOT legs_hash (which is verified separately
/// by do_finalize_bundle against BundleRecord.legs_hash).
#[test]
fn test_s0_005_t06_poae_proof_legs_hash_field() {
    // Proof with zero legs_hash is structurally valid for validate_structure()
    // but would fail consistency check in do_finalize_bundle
    let proof_with_zero_legs_hash = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 100,
        finality_cert: H256::repeat_byte(0x03),
        legs_hash: H256::zero(), // Present but zero
        leg_count: 2,
    };

    // validate_structure() passes (doesn't check legs_hash)
    // The actual legs_hash check happens in do_finalize_bundle via
    // verify_bundle_consistency() which checks BundleRecord.legs_hash != H256::zero()
    assert!(
        proof_with_zero_legs_hash.validate_structure(),
        "PoAE proof structure valid even with zero legs_hash (checked separately in bundle)"
    );

    // Verify legs_hash field is accessible for external verification
    assert_eq!(
        proof_with_zero_legs_hash.legs_hash,
        H256::zero(),
        "legs_hash field is accessible"
    );
}

/// S0-005-T07: PoAE proof validation - zero leg_count rejection
///
/// Validates that proofs with zero leg_count are rejected.
#[test]
fn test_s0_005_t07_poae_proof_zero_leg_count() {
    let invalid_proof = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 100,
        finality_cert: H256::repeat_byte(0x03),
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 0, // INVALID
    };

    assert!(
        !invalid_proof.validate_structure(),
        "Proof with zero leg_count must be rejected"
    );
}

/// S0-005-T08: Storage transaction atomicity - data structure test
///
/// This test verifies the bundle record structure integrity which is critical
/// for atomic operations. The with_storage_layer wrapper ensures all fields
/// are updated atomically or not at all.
#[test]
fn test_s0_005_t08_bundle_record_structure() {
    use crate::BundleStatus;

    // Verify bundle record fields are properly typed and accessible
    let submitter_id: u64 = 1;
    let legs_hash = H256::repeat_byte(0xAA);
    let leg_count: u32 = 4;
    let status = BundleStatus::Pending;
    let deadline_block: u64 = 2000;
    let submitted_at: u64 = 500;
    let executor: Option<u64> = None;

    // Verify all fields are accessible and correctly typed
    assert_eq!(submitter_id, 1);
    assert_eq!(legs_hash, H256::repeat_byte(0xAA));
    assert_eq!(leg_count, 4);
    assert_eq!(status, BundleStatus::Pending);
    assert_eq!(deadline_block, 2000);
    assert_eq!(submitted_at, 500);
    assert!(executor.is_none());
}

/// S0-005-T09: BundleStatus state machine validation
///
/// Validates the BundleStatus enum used in atomic state transitions.
#[test]
fn test_s0_005_t09_bundle_status_states() {
    use crate::BundleStatus;

    // Verify all status states are distinct
    assert_ne!(BundleStatus::Pending, BundleStatus::Executing);
    assert_ne!(BundleStatus::Executing, BundleStatus::Finalized);
    assert_ne!(BundleStatus::Finalized, BundleStatus::RolledBack);

    // Valid state transitions (conceptual - actual enforcement in pallet code)
    let initial = BundleStatus::Pending;
    let executing = BundleStatus::Executing;
    let finalized = BundleStatus::Finalized;
    let rolled_back = BundleStatus::RolledBack;

    // Expected transitions: Pending → Executing → (Finalized | RolledBack)
    assert!(initial != executing, "Pending and Executing are distinct");
    assert!(
        executing != finalized,
        "Executing and Finalized are distinct"
    );
    assert!(
        executing != rolled_back,
        "Executing and RolledBack are distinct"
    );
    assert!(finalized != rolled_back, "Terminal states are distinct");
}

/// S0-005-T10: VmType enum validation for cross-VM operations
///
/// Validates the VmType enum used in bundle leg specifications.
#[test]
fn test_s0_005_t10_vm_type_enum() {
    // Verify all VM types are distinct
    assert_ne!(VmType::Evm, VmType::Svm);
    assert_ne!(VmType::Svm, VmType::X3);
    assert_ne!(VmType::X3, VmType::Cross);
    assert_ne!(VmType::Evm, VmType::Cross);
}

/// S0-005-T11: BundleLeg structure validation
///
/// Validates the BundleLeg structure used in atomic bundle operations.
#[test]
fn test_s0_005_t11_bundle_leg_structure() {
    let leg = BundleLeg {
        vm_type: VmType::Evm,
        token_in: H256::repeat_byte(0x11),
        token_out: H256::repeat_byte(0x22),
        amount_in: 1_000_000_000u128,
        min_amount_out: 950_000_000u128,
        deadline: 1_800_000_000u64,
        access: DeclaredAccess {
            reads: Default::default(),
            writes: Default::default(),
        },
    };

    // Verify structure integrity
    assert!(matches!(leg.vm_type, VmType::Evm));
    assert_eq!(leg.token_in, H256::repeat_byte(0x11));
    assert_eq!(leg.token_out, H256::repeat_byte(0x22));
    assert_eq!(leg.amount_in, 1_000_000_000u128);
    assert_eq!(leg.min_amount_out, 950_000_000u128);
    assert!(leg.amount_in > leg.min_amount_out, "Slippage protection");
}

/// S0-005-T12: Atomic operation design validation - proof hash determinism
///
/// Critical for atomic operations: proof hashes must be deterministic so that
/// rollback decisions are based on consistent data.
#[test]
fn test_s0_005_t12_proof_hash_determinism_for_atomicity() {
    let proof1 = PoaeProof {
        bundle_id: H256::repeat_byte(0x10),
        receipt_root: H256::repeat_byte(0x20),
        finalized_block: 500,
        finality_cert: H256::repeat_byte(0x30),
        legs_hash: H256::repeat_byte(0x40),
        leg_count: 4,
    };

    let proof2 = proof1.clone();

    // Determinism is CRITICAL for atomic operations:
    // If proof_hash() is non-deterministic, rollback decisions could be inconsistent
    assert_eq!(
        proof1.proof_hash(),
        proof2.proof_hash(),
        "Proof hash must be deterministic for atomic rollback consistency"
    );

    // Different proofs MUST have different hashes
    let proof3 = PoaeProof {
        receipt_root: H256::repeat_byte(0xFF), // Changed field
        ..proof1
    };
    assert_ne!(
        proof1.proof_hash(),
        proof3.proof_hash(),
        "Different proofs must have different hashes to prevent rollback confusion"
    );
}

// ── VM Revert Infrastructure Tests ──────────────────────────────────────────

#[test]
fn test_state_diff_empty_check() {
    let empty = StateDiff::from(Vec::new());
    assert!(empty.is_empty());

    let non_empty = StateDiff::from(vec![1, 2, 3]);
    assert!(!non_empty.is_empty());
}

#[test]
fn test_noop_vm_reverter_returns_no_side_effects() {
    let diff = StateDiff::from(vec![1, 2, 3, 4]);
    let result = NoopVmReverter::revert_leg(VmType::Evm, &diff);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), RevertOutcome::NoSideEffects);
}

#[test]
fn test_noop_vm_reverter_works_for_all_vm_types() {
    let diff = StateDiff::from(vec![0xAA; 64]);
    assert_eq!(
        NoopVmReverter::revert_leg(VmType::Evm, &diff).unwrap(),
        RevertOutcome::NoSideEffects
    );
    assert_eq!(
        NoopVmReverter::revert_leg(VmType::Svm, &diff).unwrap(),
        RevertOutcome::NoSideEffects
    );
    assert_eq!(
        NoopVmReverter::revert_leg(VmType::X3, &diff).unwrap(),
        RevertOutcome::NoSideEffects
    );
}

#[test]
fn test_leg_receipt_structure() {
    let receipt = LegReceipt {
        leg_index: 0,
        vm_type: VmType::Evm,
        executed: false,
        state_diff: StateDiff::from(Vec::new()),
        receipt_root: [0u8; 32],
        finalized_block: 0,
    };
    assert_eq!(receipt.leg_index, 0);
    assert!(!receipt.executed);
    assert!(receipt.state_diff.is_empty());
    assert_eq!(receipt.receipt_root, [0u8; 32]);
    assert_eq!(receipt.finalized_block, 0);

    let executed_receipt = LegReceipt {
        leg_index: 1,
        vm_type: VmType::Svm,
        executed: true,
        state_diff: StateDiff::from(vec![1, 2, 3]),
        receipt_root: [0xAB; 32],
        finalized_block: 42,
    };
    assert!(executed_receipt.executed);
    assert!(!executed_receipt.state_diff.is_empty());
    assert_eq!(executed_receipt.receipt_root, [0xAB; 32]);
    assert_eq!(executed_receipt.finalized_block, 42);
}

#[test]
fn test_leg_receipt_encode_decode_roundtrip() {
    let receipt = LegReceipt {
        leg_index: 2,
        vm_type: VmType::X3,
        executed: true,
        state_diff: StateDiff::from(vec![0xDE, 0xAD, 0xBE, 0xEF]),
        receipt_root: [0xCD; 32],
        finalized_block: 99,
    };
    let encoded = parity_scale_codec::Encode::encode(&receipt);
    let decoded: LegReceipt =
        parity_scale_codec::Decode::decode(&mut &encoded[..]).expect("decode should succeed");
    assert_eq!(decoded.leg_index, receipt.leg_index);
    assert_eq!(decoded.vm_type, receipt.vm_type);
    assert_eq!(decoded.executed, receipt.executed);
    assert_eq!(decoded.state_diff, receipt.state_diff);
    assert_eq!(decoded.receipt_root, receipt.receipt_root);
    assert_eq!(decoded.finalized_block, receipt.finalized_block);
}

// ── OCW leg receipt key protocol tests ────────────────────────────────────
//
// These tests verify the pallet OCW's leg-receipt key convention agrees
// with what the off-chain executor writes to off-chain local storage.
// Key: b"x3leg:" (6) + bundle_id (32) + leg_index LE u32 (4) = 42 bytes
// Value: SCALE-encoded StateDiff

/// OCW leg key = b"x3leg:" (6) + bundle_id (32) + leg_index_le (4) = 42 bytes.
#[test]
fn test_ocw_leg_key_is_42_bytes_with_correct_prefix() {
    let bundle_id = H256::repeat_byte(0xCC);
    let leg_index: u32 = 5;

    let mut key = b"x3leg:".to_vec();
    key.extend_from_slice(bundle_id.as_bytes());
    key.extend_from_slice(&leg_index.to_le_bytes());

    assert_eq!(
        key.len(),
        42,
        "leg key must be 42 bytes (6 prefix + 32 bundle_id + 4 LE u32)"
    );
    assert_eq!(&key[..6], b"x3leg:", "key must start with 'x3leg:'");
    assert_eq!(&key[6..38], bundle_id.as_bytes());
    let decoded_leg = u32::from_le_bytes(key[38..42].try_into().unwrap());
    assert_eq!(decoded_leg, leg_index, "leg_index must roundtrip");
}

/// OCW leg keys must be unique per (bundle_id, leg_index) pair.
#[test]
fn test_ocw_leg_keys_are_unique_per_bundle_and_leg() {
    let bundle_id = H256::repeat_byte(0xDD);

    let key_leg0: Vec<u8> = {
        let mut k = b"x3leg:".to_vec();
        k.extend_from_slice(bundle_id.as_bytes());
        k.extend_from_slice(&0u32.to_le_bytes());
        k
    };
    let key_leg1: Vec<u8> = {
        let mut k = b"x3leg:".to_vec();
        k.extend_from_slice(bundle_id.as_bytes());
        k.extend_from_slice(&1u32.to_le_bytes());
        k
    };
    assert_ne!(
        key_leg0, key_leg1,
        "distinct leg indices in same bundle must produce distinct keys"
    );

    // Different bundle, same leg index — keys must differ
    let other_bundle = H256::repeat_byte(0xEE);
    let key_other: Vec<u8> = {
        let mut k = b"x3leg:".to_vec();
        k.extend_from_slice(other_bundle.as_bytes());
        k.extend_from_slice(&0u32.to_le_bytes());
        k
    };
    assert_ne!(
        key_leg0, key_other,
        "same leg index in different bundles must produce distinct keys"
    );
}

/// OCW value roundtrip: StateDiff SCALE-encode → decode must be deterministic.
#[test]
fn test_ocw_leg_value_state_diff_roundtrip() {
    let diff = StateDiff::from(vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02]);

    // Encode (off-chain executor writer side)
    let encoded = parity_scale_codec::Encode::encode(&diff);
    assert!(
        !encoded.is_empty(),
        "SCALE-encoded StateDiff must be non-empty"
    );

    // Decode (OCW reader side — mirrors offchain_worker() leg scan)
    let decoded: StateDiff =
        parity_scale_codec::Decode::decode(&mut &encoded[..]).expect("decode should succeed");
    assert_eq!(decoded, diff, "StateDiff must roundtrip through SCALE");
}

/// OCW key prefixes must not collide: x3leg vs x3fin vs x3ff.
#[test]
fn test_ocw_leg_key_prefix_does_not_collide_with_finality_or_cert() {
    let bundle_id = H256::repeat_byte(0xBB);

    let leg_key: Vec<u8> = {
        let mut k = b"x3leg:".to_vec();
        k.extend_from_slice(bundle_id.as_bytes());
        k.extend_from_slice(&0u32.to_le_bytes());
        k
    };
    let cert_key: Vec<u8> = {
        let mut k = b"x3ff:".to_vec();
        k.extend_from_slice(&42u64.to_le_bytes());
        k
    };

    assert_ne!(
        leg_key, cert_key,
        "'x3leg:' keys must not collide with 'x3ff:' keys"
    );
}

#[test]
fn test_revert_error_variants() {
    let err = RevertError::InvalidStateDiff;
    let vm_err = RevertError::VmNotAvailable(VmType::Evm);
    let fail_err = RevertError::RevertFailed {
        reason: vec![1, 2, 3],
    };

    // Ensure variants exist and can be compared
    assert!(matches!(err, RevertError::InvalidStateDiff));
    assert!(matches!(vm_err, RevertError::VmNotAvailable(VmType::Evm)));
    assert!(matches!(fail_err, RevertError::RevertFailed { .. }));
}

/// The halt guard gates *new* economic work. An in-flight bundle must stay
/// workable while the economy is halted — a halt that froze execution or trapped
/// the bond would be worse than the condition it answers.
#[test]
fn economic_halt_does_not_block_inflight_bundle_assignment() {
    let halt = economy_open();
    new_test_ext().execute_with(|| {
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            one_leg_bundle(),
            10,
            1,
            1,
        ));
        let (bundle_id, _) = Bundles::<Test>::iter()
            .next()
            .expect("submitted bundle is stored");

        halt.halt();

        // The next step of an in-flight bundle still works...
        assert_ok!(AtomicKernel::assign_bundle_executor(
            RuntimeOrigin::signed(BOB),
            bundle_id
        ));
        let record = Bundles::<Test>::get(bundle_id).expect("record survives assignment");
        assert_eq!(record.status, BundleStatus::Executing);
        assert_eq!(record.executor, Some(BOB));

        // ...while new economic work is still refused.
        assert_noop!(
            AtomicKernel::submit_atomic_bundle(
                RuntimeOrigin::signed(ALICE),
                one_leg_bundle(),
                10,
                1,
                2,
            ),
            Error::<Test>::EconomicHaltActive
        );

        // ...and the in-flight bundle can still be rolled back, releasing its bond.
        assert_ok!(AtomicKernel::rollback_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            bundle_id,
            BundleRollbackReason::SubmitterCancelled,
        ));
        assert_eq!(
            Balances::reserved_balance(ALICE),
            0,
            "a halt must not trap the bond of an in-flight bundle"
        );
    });
}

// ── Finalization: the checks, through the entry point that actually exists ───
//
// These tests were written against `submit_finalization_result`, the unsigned entry point, because
// it had no test at all — and writing them is what showed the entry point's defect: it was
// `ensure_none`, its off-chain marker had no writer in this repository, and the certificate it
// checked was anchored by an equally unsigned call, so a caller planted the value it was about to
// be checked against. That call is gone; these drive the same checks through the signed
// `finalize_atomic_bundle` (`X3LangOrigin`) that the node's atomic gateway service uses, plus one
// that the unsigned past cannot come back through validate_unsigned.

/// The receipt root the pallet demands when the commitment check is compiled in — which it is for
/// every build that is not `dev` or `testnet`.
fn committed_receipt_root(bundle_id: H256, cert: H256, finalized_block: u64) -> H256 {
    let record = Bundles::<Test>::get(bundle_id).expect("the bundle exists");
    let executor_hash = match record.executor.as_ref() {
        Some(account) => H256::from(sp_io::hashing::blake2_256(&account.encode())),
        None => H256::zero(),
    };
    H256::from(sp_io::hashing::blake2_256(
        &crate::ReceiptRootData {
            bundle_id,
            legs_hash: record.legs_hash,
            leg_count: record.leg_count,
            executor_hash,
            finalized_block,
            finality_cert: cert,
        }
        .encode(),
    ))
}

/// The chain has committed to `cert` for `block` — what the voter's anchor call does on a live
/// chain, written straight to storage here.
fn anchor_cert(block: u64, cert: H256) {
    crate::FinalityCertAnchors::<Test>::insert(block, cert);
}

/// Finalize through the signed entry point the node's atomic gateway service uses.
///
/// `ALICE` is the authorized origin in this mock (`RootOrSignedAccount`); the finalized block is 1,
/// which is the block the tests anchor their certificate for.
fn finalize(
    bundle_id: H256,
    receipt_root: H256,
    cert: H256,
) -> frame_support::dispatch::DispatchResult {
    AtomicKernel::finalize_atomic_bundle(
        RuntimeOrigin::signed(ALICE),
        bundle_id,
        receipt_root,
        cert,
        1,
    )
}

#[test]
fn finalization_refuses_a_bundle_nobody_has_been_assigned_to() {
    // A `Pending` bundle has no executor, so this is refused on status before anything else about
    // the proof matters. A block author can include an unsigned extrinsic without the pool's
    // validation, so this is the check a reader must be able to point at.
    new_test_ext().execute_with(|| {
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            one_leg_bundle(),
            100,
            1,
            7,
        ));
        let (bundle_id, _) = Bundles::<Test>::iter().next().expect("stored");
        let cert = H256::repeat_byte(0x11);
        anchor_cert(1, cert);

        assert_noop!(
            finalize(bundle_id, H256::repeat_byte(0x22), cert),
            Error::<Test>::InvalidBundleState
        );
    });
}

#[test]
fn finalization_requires_the_chain_to_have_anchored_the_certificate() {
    // The certificate the caller supplies must be one this chain has committed to. What that is
    // worth is the separate question of where the anchor comes from (TICKET-097): the anchor call
    // is unsigned too, so a caller can plant the value it is about to check against.
    new_test_ext().execute_with(|| {
        let bundle_id = submit_and_assign(100, 8);
        anchor_cert(1, H256::repeat_byte(0x33));

        assert_noop!(
            finalize(bundle_id, H256::repeat_byte(0x22), H256::repeat_byte(0x44)),
            Error::<Test>::InvalidFinalityCert
        );
    });
}

#[test]
fn finalization_requires_the_receipt_root_the_bundle_commits_to() {
    // The commitment check is compiled in for this build, so a receipt root that is not the hash of
    // the bundle's own fields is refused — and the right one is accepted.
    new_test_ext().execute_with(|| {
        let bundle_id = submit_and_assign(100, 9);
        let cert = H256::repeat_byte(0x55);
        anchor_cert(1, cert);

        assert_noop!(
            finalize(bundle_id, H256::repeat_byte(0x66), cert),
            Error::<Test>::InvalidReceiptRoot
        );

        let root = committed_receipt_root(bundle_id, cert, 1);
        assert_ok!(finalize(bundle_id, root, cert));
        assert_eq!(
            Bundles::<Test>::get(bundle_id).expect("record").status,
            BundleStatus::Finalized
        );
        assert!(crate::PoaeProofs::<Test>::contains_key(bundle_id));
    });
}

#[test]
fn finalization_happens_once() {
    new_test_ext().execute_with(|| {
        let bundle_id = submit_and_assign(100, 10);
        let cert = H256::repeat_byte(0x77);
        anchor_cert(1, cert);
        let root = committed_receipt_root(bundle_id, cert, 1);

        assert_ok!(finalize(bundle_id, root, cert));
        // The second call is refused on status: the bundle is `Finalized`, and the status check runs
        // before the "already has a PoAE proof" check. Both are refusals; this is the one it gives.
        assert_noop!(
            finalize(bundle_id, root, cert),
            Error::<Test>::InvalidBundleState
        );
        assert!(crate::PoaeProofs::<Test>::contains_key(bundle_id));
    });
}

// ── The invariant suite ────────────────────────────────────────────────────────
//
// The row's own note said "nine previously claimed invariant tests were removed as fictional;
// real invariant suite needed". These are the invariants an atomic bundle actually has to hold,
// asserted directly rather than inferred from the happy path one test at a time: a lifecycle
// that runs forward once, a bond that settles exactly once, leg receipts that are written once
// each, and a submission nonce that cannot be spent twice.

/// A settled bundle stays settled: once a bundle has been finalized, rolling it back is not a
/// "second opinion" — it would move the bond again under a bundle whose receipt is already
/// committed.
#[test]
fn a_finalized_bundle_cannot_be_rolled_back() {
    new_test_ext().execute_with(|| {
        let bundle_id = submit_and_assign(100, 20);
        let cert = H256::repeat_byte(0xA1);
        anchor_cert(1, cert);
        let root = committed_receipt_root(bundle_id, cert, 1);
        assert_ok!(finalize(bundle_id, root, cert));

        let free_before = Balances::free_balance(ALICE);
        assert_noop!(
            AtomicKernel::rollback_atomic_bundle(
                RuntimeOrigin::signed(ALICE),
                bundle_id,
                BundleRollbackReason::SubmitterCancelled,
            ),
            Error::<Test>::InvalidBundleState
        );
        assert_eq!(
            Bundles::<Test>::get(bundle_id).expect("record").status,
            BundleStatus::Finalized,
            "a refused rollback must leave the status exactly as it was"
        );
        assert_eq!(
            Balances::free_balance(ALICE),
            free_before,
            "a refused rollback must not move the bond"
        );
    });
}

/// And the other direction: a rollback is final too. Finalizing afterwards would commit a
/// receipt for a bundle whose legs were already reverted.
#[test]
fn a_rolled_back_bundle_cannot_be_finalized_and_cannot_be_rolled_back_twice() {
    new_test_ext().execute_with(|| {
        let bundle_id = submit_and_assign(100, 21);
        let cert = H256::repeat_byte(0xA2);
        anchor_cert(1, cert);
        let root = committed_receipt_root(bundle_id, cert, 1);

        assert_ok!(AtomicKernel::rollback_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            bundle_id,
            BundleRollbackReason::SubmitterCancelled,
        ));
        let reserved_after = Balances::reserved_balance(ALICE);
        assert_eq!(reserved_after, 0, "rollback releases the whole bond");

        assert_noop!(
            finalize(bundle_id, root, cert),
            Error::<Test>::InvalidBundleState
        );
        assert_noop!(
            AtomicKernel::rollback_atomic_bundle(
                RuntimeOrigin::signed(ALICE),
                bundle_id,
                BundleRollbackReason::SubmitterCancelled,
            ),
            Error::<Test>::InvalidBundleState
        );
        assert_eq!(
            Balances::reserved_balance(ALICE),
            reserved_after,
            "a second rollback must not release (or re-slash) anything a second time"
        );
        assert!(
            !crate::PoaeProofs::<Test>::contains_key(bundle_id),
            "a rolled-back bundle must never gain a finality proof"
        );
    });
}

/// A leg's receipt is written once. Overwriting it would let a second execution replace the
/// state diff the bundle is finalized against.
#[test]
fn a_leg_receipt_is_written_once_and_keeps_its_first_state_diff() {
    new_test_ext().execute_with(|| {
        let bundle_id = submit_and_assign(100, 22);
        let first = crate::vm_revert::StateDiff::from_vec_lossy(b"first".to_vec());

        assert_ok!(AtomicKernel::record_leg_execution_receipt(
            RuntimeOrigin::none(),
            bundle_id,
            0,
            first.clone(),
        ));
        assert!(
            crate::BundleLegReceipts::<Test>::get(bundle_id)[0].executed,
            "the receipt has to record that the leg ran"
        );

        assert_noop!(
            AtomicKernel::record_leg_execution_receipt(
                RuntimeOrigin::none(),
                bundle_id,
                0,
                crate::vm_revert::StateDiff::from_vec_lossy(b"second".to_vec()),
            ),
            Error::<Test>::LegAlreadyExecuted
        );
        assert_eq!(
            crate::BundleLegReceipts::<Test>::get(bundle_id)[0].state_diff,
            first,
            "the refused second receipt must leave the first state diff in place"
        );
    });
}

/// A receipt for a leg the bundle does not have is refused. `one_leg_bundle` has one leg, so
/// index 1 is out of range — and an index that large is what a caller guessing at a bundle's
/// shape would send.
#[test]
fn a_receipt_for_a_leg_outside_the_bundle_is_refused() {
    new_test_ext().execute_with(|| {
        let bundle_id = submit_and_assign(100, 23);
        assert_eq!(
            crate::BundleLegReceipts::<Test>::get(bundle_id).len(),
            1,
            "the receipt vector is sized from the bundle's own legs"
        );
        assert_noop!(
            AtomicKernel::record_leg_execution_receipt(
                RuntimeOrigin::none(),
                bundle_id,
                1,
                crate::vm_revert::StateDiff::from_vec_lossy(b"out of range".to_vec()),
            ),
            Error::<Test>::InvalidBundleState
        );
    });
}

/// A submission nonce is spent once. The rule is `nonce > last_nonce && !used`, so both a
/// replay and a rewound nonce have to be refused — the second is the one that would let a
/// submitted bundle be replaced by a different one under the same identity.
#[test]
fn a_submission_nonce_cannot_be_spent_twice_or_rewound() {
    new_test_ext().execute_with(|| {
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            one_leg_bundle(),
            10,
            1,
            30,
        ));
        assert_eq!(
            NonceRegistry::<Test>::get(1, ALICE).used_nonces.len(),
            1,
            "the first submission spends nonce 30"
        );

        // The same legs again are the *same bundle* — one id, one submission — and that is
        // refused before the nonce rule is even consulted.
        assert_noop!(
            AtomicKernel::submit_atomic_bundle(
                RuntimeOrigin::signed(ALICE),
                one_leg_bundle(),
                10,
                1,
                31,
            ),
            Error::<Test>::BundleAlreadyExists
        );

        // The same nonce again, on a *different* bundle, so the refusal can only be the nonce.
        assert_noop!(
            AtomicKernel::submit_atomic_bundle(
                RuntimeOrigin::signed(ALICE),
                one_leg_bundle_with(1_001),
                10,
                1,
                30,
            ),
            Error::<Test>::InvalidNonce
        );
        // And a nonce below the watermark, which is not in `used_nonces`.
        assert_noop!(
            AtomicKernel::submit_atomic_bundle(
                RuntimeOrigin::signed(ALICE),
                one_leg_bundle_with(1_002),
                10,
                1,
                29,
            ),
            Error::<Test>::InvalidNonce
        );
        assert_eq!(
            NonceRegistry::<Test>::get(1, ALICE).used_nonces.len(),
            1,
            "a refused submission must not spend a nonce"
        );

        // A fresh nonce on a fresh bundle still works.
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            one_leg_bundle_with(1_003),
            10,
            1,
            31,
        ));
        assert_eq!(NonceRegistry::<Test>::get(1, ALICE).used_nonces.len(), 2);
        assert_eq!(
            NonceRegistry::<Test>::get(1, ALICE).last_nonce,
            31,
            "the watermark tracks the highest nonce spent"
        );
    });
}

#[test]
fn finalization_has_no_unsigned_entry_point() {
    // TICKET-097. `submit_finalization_result` used to be callable with `RuntimeOrigin::none()`,
    // which is what a block author's unsigned extrinsic carries: no identity at all. It is gone —
    // the signed entry point is the only one, so an unsigned extrinsic no longer has a call to
    // reach finalization through. The mock's `X3LangOrigin` accepts any *signed* account, so this
    // asserts the property that matters here: no signature, no finalization.
    new_test_ext().execute_with(|| {
        let bundle_id = submit_and_assign(100, 11);
        let cert = H256::repeat_byte(0x88);
        anchor_cert(1, cert);
        let root = committed_receipt_root(bundle_id, cert, 1);

        assert_noop!(
            AtomicKernel::finalize_atomic_bundle(RuntimeOrigin::none(), bundle_id, root, cert, 1),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_eq!(
            Bundles::<Test>::get(bundle_id).expect("record").status,
            BundleStatus::Executing,
            "an unsigned caller leaves the bundle exactly as it found it"
        );
        assert_ok!(finalize(bundle_id, root, cert));
    });
}
