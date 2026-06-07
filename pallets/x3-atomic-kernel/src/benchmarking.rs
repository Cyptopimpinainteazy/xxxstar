use super::*;

#[allow(unused)]
use crate::Pallet as X3AtomicKernel;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::traits::{Currency, Get};
use frame_support::BoundedVec;
use frame_system::pallet_prelude::BlockNumberFor;
use frame_system::RawOrigin;
use sp_core::H256;
use sp_runtime::traits::SaturatedConversion;

benchmarks! {
    // Benchmark submitting an atomic bundle with variable number of legs.
    // Cost scales with leg count due to encoding overhead.
    submit_atomic_bundle {
        let b in 1 .. T::MaxLegsPerBundle::get();
        let caller: T::AccountId = whitelisted_caller();

        let mut legs = Vec::new();
        for _i in 0..b {
            legs.push(proof::BundleLeg {
                vm_type: proof::VmType::Svm,
                token_in: H256::repeat_byte(0),
                token_out: H256::repeat_byte(1),
                amount_in: 100,
                min_amount_out: 0,
                deadline: 10_000,
                access: proof::DeclaredAccess {
                    reads: Default::default(),
                    writes: Default::default(),
                },
            });
        }

        let _ = T::Currency::make_free_balance_be(&caller, (T::MinBond::get() * 10u128).saturated_into());
        let legs = BoundedVec::<proof::BundleLeg, T::MaxLegsPerBundle>::try_from(legs)
            .expect("legs within MaxLegsPerBundle");

    }: _(RawOrigin::Signed(caller.clone()), legs, 1000u32.into())
    verify {
        // Verify bundle was created and is in Pending status
        let bundle_id = Bundles::<T>::iter_keys().next().expect("bundle should exist");
        let bundle = Bundles::<T>::get(bundle_id).expect("bundle should be retrievable");
        assert_eq!(bundle.status, BundleStatus::Pending);
        assert_eq!(bundle.submitter, caller);
    }

    // Benchmark assigning an executor to a pending bundle.
    // Lightweight state transition: only updates executor field and status.
    assign_bundle_executor {
        let caller: T::AccountId = whitelisted_caller();
        let executor: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, (T::MinBond::get() * 10u128).saturated_into());

        let legs = BoundedVec::<proof::BundleLeg, T::MaxLegsPerBundle>::try_from(vec![proof::BundleLeg {
            vm_type: proof::VmType::Svm,
            token_in: H256::repeat_byte(0),
            token_out: H256::repeat_byte(1),
            amount_in: 100,
            min_amount_out: 0,
            deadline: 10_000,
            access: proof::DeclaredAccess {
                reads: Default::default(),
                writes: Default::default(),
            },
        }]).expect("within MaxLegsPerBundle");
        X3AtomicKernel::<T>::submit_atomic_bundle(RawOrigin::Signed(caller.clone()).into(), legs, 1000u32.into()).unwrap();
        let bundle_id = Bundles::<T>::iter_keys().next().unwrap();

    }: _(RawOrigin::Signed(executor.clone()), bundle_id)
    verify {
        let bundle = Bundles::<T>::get(bundle_id).unwrap();
        assert_eq!(bundle.status, BundleStatus::Executing);
        assert_eq!(bundle.executor, Some(executor));
    }

    // Benchmark finalizing a bundle with PoAE proof generation.
    // Requires bundle to be in Executing state.
    // Stores proof on-chain for external verifiers.
    finalize_atomic_bundle {
        let caller: T::AccountId = whitelisted_caller();
        let executor: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, (T::MinBond::get() * 10u128).saturated_into());
        let _ = T::Currency::make_free_balance_be(&executor, (T::MinBond::get() * 10u128).saturated_into());

        let legs = BoundedVec::<proof::BundleLeg, T::MaxLegsPerBundle>::try_from(vec![proof::BundleLeg {
            vm_type: proof::VmType::Svm,
            token_in: H256::repeat_byte(0),
            token_out: H256::repeat_byte(1),
            amount_in: 100,
            min_amount_out: 0,
            deadline: 10_000,
            access: proof::DeclaredAccess {
                reads: Default::default(),
                writes: Default::default(),
            },
        }]).expect("within MaxLegsPerBundle");
        X3AtomicKernel::<T>::submit_atomic_bundle(RawOrigin::Signed(caller.clone()).into(), legs, 1000u32.into()).unwrap();
        let bundle_id = Bundles::<T>::iter_keys().next().unwrap();

        X3AtomicKernel::<T>::assign_bundle_executor(RawOrigin::Signed(executor.clone()).into(), bundle_id).unwrap();

        let receipt_root = H256::repeat_byte(0x11);
        let finality_cert = H256::repeat_byte(0x22);
        let block_number: BlockNumberFor<T> = 1u32.into();
    }: _(RawOrigin::Signed(executor), bundle_id, receipt_root, finality_cert, block_number)
    verify {
        let bundle = Bundles::<T>::get(bundle_id).unwrap();
        assert_eq!(bundle.status, BundleStatus::Finalized);
        let proof = PoaeProofs::<T>::get(bundle_id).expect("proof should be stored");
        assert_eq!(proof.bundle_id, bundle_id);
        assert_eq!(proof.receipt_root, receipt_root);
    }

    // Benchmark rolling back a bundle with ExecutionFailed reason.
    // Slashes a portion of the submitter's bond.
    rollback_atomic_bundle {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, (T::MinBond::get() * 10u128).saturated_into());

        let legs = BoundedVec::<proof::BundleLeg, T::MaxLegsPerBundle>::try_from(vec![proof::BundleLeg {
            vm_type: proof::VmType::Svm,
            token_in: H256::repeat_byte(0),
            token_out: H256::repeat_byte(1),
            amount_in: 100,
            min_amount_out: 0,
            deadline: 10_000,
            access: proof::DeclaredAccess {
                reads: Default::default(),
                writes: Default::default(),
            },
        }]).expect("within MaxLegsPerBundle");
        X3AtomicKernel::<T>::submit_atomic_bundle(RawOrigin::Signed(caller.clone()).into(), legs, 1000u32.into()).unwrap();
        let bundle_id = Bundles::<T>::iter_keys().next().unwrap();

    }: _(RawOrigin::Signed(caller), bundle_id, BundleRollbackReason::ExecutionFailed)
    verify {
        let bundle = Bundles::<T>::get(bundle_id).unwrap();
        assert_eq!(bundle.status, BundleStatus::RolledBack);
    }

    // Benchmark rolling back a bundle with SubmitterCancelled reason.
    // No slashing - full bond returned to submitter.
    rollback_atomic_bundle_cancel {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, (T::MinBond::get() * 10u128).saturated_into());

        let legs = BoundedVec::<proof::BundleLeg, T::MaxLegsPerBundle>::try_from(vec![proof::BundleLeg {
            vm_type: proof::VmType::Svm,
            token_in: H256::repeat_byte(0),
            token_out: H256::repeat_byte(1),
            amount_in: 100,
            min_amount_out: 0,
            deadline: 10_000,
            access: proof::DeclaredAccess {
                reads: Default::default(),
                writes: Default::default(),
            },
        }]).expect("within MaxLegsPerBundle");
        X3AtomicKernel::<T>::submit_atomic_bundle(RawOrigin::Signed(caller.clone()).into(), legs, 1000u32.into()).unwrap();
        let bundle_id = Bundles::<T>::iter_keys().next().unwrap();

    }: rollback_atomic_bundle(RawOrigin::Signed(caller), bundle_id, BundleRollbackReason::SubmitterCancelled)
    verify {
        let bundle = Bundles::<T>::get(bundle_id).unwrap();
        assert_eq!(bundle.status, BundleStatus::RolledBack);
    }

    // Benchmark submitting finalization result via unsigned extrinsic (OCW path).
    // This is the on-chain finalization path called by the off-chain orchestrator.
    submit_finalization_result {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, (T::MinBond::get() * 10u128).saturated_into());

        let legs = BoundedVec::<proof::BundleLeg, T::MaxLegsPerBundle>::try_from(vec![proof::BundleLeg {
            vm_type: proof::VmType::Svm,
            token_in: H256::repeat_byte(0),
            token_out: H256::repeat_byte(1),
            amount_in: 100,
            min_amount_out: 0,
            deadline: 10_000,
            access: proof::DeclaredAccess {
                reads: Default::default(),
                writes: Default::default(),
            },
        }]).expect("within MaxLegsPerBundle");
        X3AtomicKernel::<T>::submit_atomic_bundle(RawOrigin::Signed(caller.clone()).into(), legs, 1000u32.into()).unwrap();
        let bundle_id = Bundles::<T>::iter_keys().next().unwrap();

        X3AtomicKernel::<T>::assign_bundle_executor(RawOrigin::Signed(caller.clone()).into(), bundle_id).unwrap();

        let receipt_root = H256::repeat_byte(0x11);
        let finality_cert = H256::zero(); // Zero cert when Flash Finality not running
        let committed_at_ns = 1000000000u64;

    }: _(RawOrigin::None, bundle_id, receipt_root, finality_cert, committed_at_ns)
    verify {
        let bundle = Bundles::<T>::get(bundle_id).unwrap();
        assert_eq!(bundle.status, BundleStatus::Finalized);
    }

    // Benchmark recording a Flash Finality certificate anchor on-chain.
    // Called by off-chain worker to anchor cert hash for validation.
    record_flash_finality_anchor {
        let block_num = 100u64;
        let cert = H256::repeat_byte(0x33);
    }: _(RawOrigin::None, block_num, cert)
    verify {
        let anchored = FinalityCertAnchors::<T>::get(block_num).expect("anchor should be stored");
        assert_eq!(anchored, cert);
    }

    impl_benchmark_test_suite!(X3AtomicKernel, crate::tests::new_test_ext(), crate::tests::Test);
}
