//! ⚠️ MODEL TEST — Simulated test environment. Uses TestExternalities, not real nodes.
//!
//! Cross-VM Atomic Integration Test
//!
//! Real runtime-backed tests for the full atomic execution flow from EVM to
//! SVM through the atomic kernel. Uses `TestExternalities` from the atomic
//! kernel mock to validate bundle lifecycle, rollback, and expiry.

use frame_support::{assert_err, assert_ok, BoundedVec};
use pallet_x3_atomic_kernel::{self as atomic_kernel, BundleStatus};
use pallet_x3_atomic_kernel::mock::{
    new_test_ext, run_to_block, test_leg, ALICE, BOB, CHARLIE, AtomicKernel, RuntimeEvent,
    RuntimeOrigin, System, Test,
};
use pallet_x3_atomic_kernel::proof::{BundleLeg, VmType};
use sp_core::H256;

/// Submit a bundle with mixed EVM/SVM legs, then roll it back.
/// Verify status transitions to RolledBack and BundleRolledBack event fires.
#[test]
fn cross_vm_submit_and_rollback() {
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

        assert_eq!(
            atomic_kernel::Bundles::<Test>::get(bundle_id).unwrap().status,
            BundleStatus::Pending
        );

        // Leg receipts should be initialized
        let receipts = atomic_kernel::BundleLegReceipts::<Test>::get(bundle_id);
        assert_eq!(receipts.len(), 2);
        assert!(!receipts[0].executed);
        assert!(!receipts[1].executed);

        // Submitter cancels
        assert_ok!(AtomicKernel::rollback_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            bundle_id,
            atomic_kernel::BundleRollbackReason::SubmitterCancelled,
        ));

        let rolled_back = atomic_kernel::Bundles::<Test>::get(bundle_id).unwrap();
        assert_eq!(rolled_back.status, BundleStatus::RolledBack);

        assert!(System::events().iter().any(|record| {
            matches!(
                &record.event,
                RuntimeEvent::AtomicKernel(atomic_kernel::Event::BundleRolledBack { bundle_id: id, .. }) if *id == bundle_id
            )
        }));

        // Leg receipts should be cleaned up
        assert!(atomic_kernel::BundleLegReceipts::<Test>::get(bundle_id).is_empty());
    });
}

/// Full execution flow: submit → assign executor → finalize.
/// Verify all status transitions and events.
#[test]
fn cross_vm_execution_flow() {
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

        assert_eq!(
            atomic_kernel::Bundles::<Test>::get(bundle_id).unwrap().status,
            BundleStatus::Pending
        );

        // Assign executor
        assert_ok!(AtomicKernel::assign_bundle_executor(
            RuntimeOrigin::signed(BOB),
            bundle_id,
        ));

        run_to_block(3);

        assert_eq!(
            atomic_kernel::Bundles::<Test>::get(bundle_id).unwrap().status,
            BundleStatus::Executing
        );

        // Finalize
        assert_ok!(AtomicKernel::finalize_atomic_bundle(
            RuntimeOrigin::signed(CHARLIE),
            bundle_id,
            H256::repeat_byte(0xAA),
            H256::zero(),
            3,
        ));

        run_to_block(4);

        assert_eq!(
            atomic_kernel::Bundles::<Test>::get(bundle_id).unwrap().status,
            BundleStatus::Finalized
        );
    });
}

/// Bundle with short deadline auto-expires via `on_initialize`.
/// Verify auto-rollback fires and receipts are cleaned up.
#[test]
fn cross_vm_deadline_expiry() {
    new_test_ext().execute_with(|| {
        let legs_vec = vec![test_leg(VmType::Evm)];
        let legs: BoundedVec<BundleLeg, <Test as atomic_kernel::Config>::MaxLegsPerBundle> =
            legs_vec.try_into().expect("legs should fit");

        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            legs,
            2, // deadline_blocks = 2 (expires at block 3)
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

        let rolled_back = atomic_kernel::Bundles::<Test>::get(bundle_id).unwrap();
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

        // Receipts cleaned up after auto-expiry
        assert!(atomic_kernel::BundleLegReceipts::<Test>::get(bundle_id).is_empty());
    });
}