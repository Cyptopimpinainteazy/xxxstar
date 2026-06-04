//! E2E Test: Settlement Engine ↔ Atomic Kernel Integration
//!
//! Real runtime-backed tests using the atomic kernel mock to validate the
//! complete settlement ↔ kernel flow: submit, assign, finalize via settlement
//! path, rollback, and auto-expiry.
//!
//! Documentation of the expected Phase 1 wiring flow:
//!
//! ```text
//! [Settlement Pallet] → create_intent()
//!   ↓ [Settlement Intent Created]
//! [Escrow Module] → lock_escrow()
//!   ↓ [Assets Locked in X3]
//! [Atomic Kernel Pallet] → submit_atomic_bundle()
//!   ↓ [Kernel Processes Bundle]
//!   ↓ [PoAE Generated]
//! [Settlement Pallet] → finalize_with_proof()
//!   ↓ [Settlement Finalized]
//!   ↓ [Assets Released to Recipients]
//! ```

#![cfg(test)]

use frame_support::{assert_err, assert_ok, BoundedVec};
use pallet_x3_atomic_kernel::{self as atomic_kernel, BundleStatus};
use pallet_x3_atomic_kernel::mock::{
    new_test_ext, run_to_block, test_leg, ALICE, BOB, CHARLIE, AtomicKernel, RuntimeEvent,
    RuntimeOrigin, System, Test,
};
use pallet_x3_atomic_kernel::proof::{BundleLeg, VmType};
use sp_core::H256;

/// E2E Settlement → Atomic Kernel → Finalization Flow
///
/// Submits a bundle, assigns an executor, then finalizes via the
/// settlement-specific path (`finalize_with_settlement`) using CHARLIE
/// (the settlement origin). Verifies the bundle transitions to Finalized.
#[test]
fn settlement_e2e_flow() {
    new_test_ext().execute_with(|| {
        let legs_vec = vec![test_leg(VmType::Evm), test_leg(VmType::Svm)];
        let legs: BoundedVec<BundleLeg, <Test as atomic_kernel::Config>::MaxLegsPerBundle> =
            legs_vec.try_into().expect("legs should fit into MaxLegsPerBundle");

        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            legs,
            10,
            1,
            1,
        ));

        run_to_block(2);

        let bundle_id = System::events()
            .iter()
            .find_map(|record| match &record.event {
                RuntimeEvent::AtomicKernel(atomic_kernel::Event::BundleSubmitted { bundle_id, .. }) => {
                    Some(*bundle_id)
                }
                _ => None,
            })
            .expect("BundleSubmitted event must exist");

        let pending = atomic_kernel::Bundles::<Test>::get(bundle_id).expect("bundle must exist");
        assert_eq!(pending.status, BundleStatus::Pending);

        assert_ok!(AtomicKernel::assign_bundle_executor(
            RuntimeOrigin::signed(BOB),
            bundle_id,
        ));

        run_to_block(3);

        // Finalize via the settlement path (CHARLIE = SettlementOnlyOrigin)
        assert_ok!(AtomicKernel::finalize_with_settlement(
            RuntimeOrigin::signed(CHARLIE),
            bundle_id,
            H256::repeat_byte(0xBB), // settlement_intent_id
            H256::repeat_byte(0xAA), // receipt_root
            H256::zero(),            // finality_cert
        ));

        run_to_block(4);

        let finalized = atomic_kernel::Bundles::<Test>::get(bundle_id).expect("bundle must exist");
        assert_eq!(finalized.status, BundleStatus::Finalized);
        assert!(System::events().iter().any(|record| {
            matches!(
                &record.event,
                RuntimeEvent::AtomicKernel(atomic_kernel::Event::BundleFinalized { bundle_id: id, .. }) if *id == bundle_id
            )
        }));
    });
}

/// Settlement origin is gated: ALICE (X3LangOrigin) must not be able to call
/// `finalize_with_settlement`. Only CHARLIE (SettlementOnlyOrigin) may.
#[test]
fn settlement_wrong_origin_rejected() {
    new_test_ext().execute_with(|| {
        let legs_vec = vec![test_leg(VmType::Evm)];
        let legs: BoundedVec<BundleLeg, <Test as atomic_kernel::Config>::MaxLegsPerBundle> =
            legs_vec.try_into().expect("legs should fit");

        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            legs,
            10,
            1,
            1,
        ));

        run_to_block(2);

        let bundle_id = System::events()
            .iter()
            .find_map(|record| match &record.event {
                RuntimeEvent::AtomicKernel(atomic_kernel::Event::BundleSubmitted { bundle_id, .. }) => {
                    Some(*bundle_id)
                }
                _ => None,
            })
            .expect("BundleSubmitted event must exist");

        assert_ok!(AtomicKernel::assign_bundle_executor(
            RuntimeOrigin::signed(BOB),
            bundle_id,
        ));

        run_to_block(3);

        // ALICE is X3LangOrigin, NOT SettlementOnlyOrigin → must fail
        assert_err!(
            AtomicKernel::finalize_with_settlement(
                RuntimeOrigin::signed(ALICE),
                bundle_id,
                H256::repeat_byte(0xBB),
                H256::repeat_byte(0xAA),
                H256::zero(),
            ),
            sp_runtime::DispatchError::BadOrigin,
        );

        // BOB is also not SettlementOnlyOrigin → must fail
        assert_err!(
            AtomicKernel::finalize_with_settlement(
                RuntimeOrigin::signed(BOB),
                bundle_id,
                H256::repeat_byte(0xBB),
                H256::repeat_byte(0xAA),
                H256::zero(),
            ),
            sp_runtime::DispatchError::BadOrigin,
        );
    });
}

/// Bundle rollback works correctly: submit → rollback → verify RolledBack
/// status and BundleRolledBack event.
#[test]
fn settlement_bundle_rollback() {
    new_test_ext().execute_with(|| {
        let legs_vec = vec![test_leg(VmType::Evm), test_leg(VmType::Svm)];
        let legs: BoundedVec<BundleLeg, <Test as atomic_kernel::Config>::MaxLegsPerBundle> =
            legs_vec.try_into().expect("legs should fit");

        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            legs,
            10,
            1,
            1,
        ));

        run_to_block(2);

        let bundle_id = System::events()
            .iter()
            .find_map(|record| match &record.event {
                RuntimeEvent::AtomicKernel(atomic_kernel::Event::BundleSubmitted { bundle_id, .. }) => {
                    Some(*bundle_id)
                }
                _ => None,
            })
            .expect("BundleSubmitted event must exist");

        // Submitter can cancel their own bundle
        assert_ok!(AtomicKernel::rollback_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            bundle_id,
            atomic_kernel::BundleRollbackReason::SubmitterCancelled,
        ));

        let rolled_back = atomic_kernel::Bundles::<Test>::get(bundle_id).expect("bundle must exist");
        assert_eq!(rolled_back.status, BundleStatus::RolledBack);
        assert!(System::events().iter().any(|record| {
            matches!(
                &record.event,
                RuntimeEvent::AtomicKernel(atomic_kernel::Event::BundleRolledBack { bundle_id: id, .. }) if *id == bundle_id
            )
        }));

        // Leg receipts should be cleaned up after rollback
        assert!(atomic_kernel::BundleLegReceipts::<Test>::get(bundle_id).is_empty());
    });
}

/// Auto-expiry: bundles that pass their deadline are automatically rolled
/// back by `on_initialize`.
#[test]
fn settlement_bundle_auto_expiry() {
    new_test_ext().execute_with(|| {
        let legs_vec = vec![test_leg(VmType::Evm)];
        let legs: BoundedVec<BundleLeg, <Test as atomic_kernel::Config>::MaxLegsPerBundle> =
            legs_vec.try_into().expect("legs should fit");

        // Submit with deadline_blocks = 2 (will expire at block 3)
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            legs,
            2, // deadline_blocks
            1,
            1,
        ));

        let bundle_id = System::events()
            .iter()
            .find_map(|record| match &record.event {
                RuntimeEvent::AtomicKernel(atomic_kernel::Event::BundleSubmitted { bundle_id, .. }) => {
                    Some(*bundle_id)
                }
                _ => None,
            })
            .expect("BundleSubmitted event must exist");

        // Advance past deadline
        run_to_block(5);

        let rolled_back = atomic_kernel::Bundles::<Test>::get(bundle_id).expect("bundle must exist");
        assert_eq!(rolled_back.status, BundleStatus::RolledBack);
        assert!(System::events().iter().any(|record| {
            matches!(
                &record.event,
                RuntimeEvent::AtomicKernel(atomic_kernel::Event::BundleRolledBack {
                    bundle_id: id,
                    reason: atomic_kernel::BundleRollbackReason::DeadlineExceeded,
                }) if *id == bundle_id
            )
        }));

        // Leg receipts should be cleaned up after auto-expiry rollback
        assert!(atomic_kernel::BundleLegReceipts::<Test>::get(bundle_id).is_empty());
    });
}