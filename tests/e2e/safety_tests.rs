// tests/e2e/safety_tests.rs
// Comprehensive test suite for safety features

use frame_support::{assert_err, assert_ok};
use pallet_x3_atomic_kernel::{BundleStatus, Config, Pallet as AtomicKernel};
use sp_core::H256;
use sp_runtime::traits::Hash;
use x3_chain_runtime::{Runtime, System};

mod mock;
use mock::*;

#[test]
fn test_nonce_replay_protection() {
    new_test_ext().execute_with(|| {
        // Setup: create a bundle and finalize it
        let bundle_id = H256::random();
        let receipt_root = H256::random();
        let finalized_block = 100;
        let finality_cert = H256::random();
        
        // Anchor finality certificate
        FinalityCertAnchors::<Runtime>::insert(finalized_block, finality_cert);
        
        // Submit and execute bundle
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(1),
            vec![],
            200
        ));
        
        // Finalize the bundle
        assert_ok!(AtomicKernel::finalize_atomic_bundle(
            RuntimeOrigin::signed(1),
            bundle_id,
            receipt_root,
            finality_cert,
            finalized_block
        ));
        
        // Attempt to replay the same bundle (should fail)
        assert_err!(
            AtomicKernel::finalize_atomic_bundle(
                RuntimeOrigin::signed(1),
                bundle_id,
                receipt_root,
                finality_cert,
                finalized_block
            ),
            "NonceAlreadyUsed"
        );
    });
}

#[test]
fn benchmark_gas_abstraction() {
    new_test_ext().execute_with(|| {
        use pallet_x3_atomic_kernel::BundleStatus;
        use sp_core::H256;
        
        // Create bundles with different operation types
        let mut results = Vec::new();
        
        // Simple lock operation
        let bundle_id = H256::random();
        let receipt_root = H256::random();
        let finalized_block = 100;
        let finality_cert = H256::random();
        
        FinalityCertAnchors::<Runtime>::insert(finalized_block, finality_cert);
        
        let start_gas = System::block_weight().total().ref_time();
        
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(1),
            vec![],
            200
        ));
        
        assert_ok!(AtomicKernel::finalize_atomic_bundle(
            RuntimeOrigin::signed(1),
            bundle_id,
            receipt_root,
            finality_cert,
            finalized_block
        ));
        
        let end_gas = System::block_weight().total().ref_time();
        let gas_used = end_gas - start_gas;
        
        results.push(("lock", gas_used));
        
        // Complex swap operation
        let bundle_id = H256::random();
        let start_gas = System::block_weight().total().ref_time();
        
        // Simulate complex swap operation
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(1),
            vec![
                BundleLeg::Swap {
                    amount_in: 100,
                    min_out: 95,
                    asset_in: AssetId::from(1),
                    asset_out: AssetId::from(2),
                    route: vec![]
                }
            ],
            200
        ));
        
        assert_ok!(AtomicKernel::finalize_atomic_bundle(
            RuntimeOrigin::signed(1),
            bundle_id,
            receipt_root,
            finality_cert,
            finalized_block
        ));
        
        let end_gas = System::block_weight().total().ref_time();
        let gas_used = end_gas - start_gas;
        
        results.push(("swap", gas_used));
        
        // Large-scale operation
        let bundle_id = H256::random();
        let start_gas = System::block_weight().total().ref_time();
        
        // Simulate large-scale operation
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(1),
            vec![
                BundleLeg::Lock { amount: 1000, asset: AssetId::from(1) },
                BundleLeg::Swap {
                    amount_in: 500,
                    min_out: 475,
                    asset_in: AssetId::from(1),
                    asset_out: AssetId::from(2),
                    route: vec![]
                },
                BundleLeg::Settle { amount: 475, asset: AssetId::from(2) }
            ],
            200
        ));
        
        assert_ok!(AtomicKernel::finalize_atomic_bundle(
            RuntimeOrigin::signed(1),
            bundle_id,
            receipt_root,
            finality_cert,
            finalized_block
        ));
        
        let end_gas = System::block_weight().total().ref_time();
        let gas_used = end_gas - start_gas;
        
        results.push(("large_scale", gas_used));
        
        // Output results for analysis
        println!("Gas abstraction benchmark results:");
        for (op_type, gas) in results {
            println!("{}: {} gas units", op_type, gas);
        }
        
        // Verify all operations stayed within safe gas limits
        for (_, gas) in results {
            assert!(gas < 1_000_000, "Gas consumption exceeded safe limit");
        }
    });
}

#[test]
fn fuzz_test_cross_vm() {
    new_test_ext().execute_with(|| {
        use pallet_x3_atomic_kernel::{BundleLeg, BundleStatus};
        use sp_core::H256;
        
        // Generate 100 random test cases
        for i in 0..100 {
            let bundle_id = H256::random();
            let receipt_root = H256::random();
            let finalized_block = 100 + i as u64;
            let finality_cert = H256::random();
            
            FinalityCertAnchors::<Runtime>::insert(finalized_block, finality_cert);
            
            // Create random legs
            let mut legs = Vec::new();
            for j in 0..(i % 5 + 1) {
                legs.push(BundleLeg::Lock {
                    amount: (i * 100 + j * 10) as u128,
                    asset: AssetId::from(j as u32)
                });
                
                if j % 2 == 0 {
                    legs.push(BundleLeg::Swap {
                        amount_in: (i * 50 + j * 5) as u128,
                        min_out: (i * 45 + j * 4) as u128,
                        asset_in: AssetId::from(j as u32),
                        asset_out: AssetId::from((j + 1) as u32),
                        route: vec![]
                    });
                }
            }
            
            // Submit and finalize bundle
            assert_ok!(AtomicKernel::submit_atomic_bundle(
                RuntimeOrigin::signed(1),
                legs.clone(),
                200 + i as u64
            ));
            
            assert_ok!(AtomicKernel::finalize_atomic_bundle(
                RuntimeOrigin::signed(1),
                bundle_id,
                receipt_root,
                finality_cert,
                finalized_block
            ));
            
            // Verify finalization
            let record = Bundles::<Runtime>::get(bundle_id).unwrap();
            assert_eq!(record.status, BundleStatus::Finalized);
            
            // Verify nonce replay protection
            assert_err!(
                AtomicKernel::finalize_atomic_bundle(
                    RuntimeOrigin::signed(1),
                    bundle_id,
                    receipt_root,
                    finality_cert,
                    finalized_block
                ),
                "NonceAlreadyUsed"
            );
        }
    });
}

#[test]
fn test_circuit_breaker_validation() {
    new_test_ext().execute_with(|| {
        use pallet_x3_atomic_kernel::Error;
        
        // Simulate conditions that trigger circuit breaker
        // 1. Submit multiple failing bundles
        for _ in 0..5 {
            let bundle_id = H256::random();
            
            // Submit bundle with invalid data
            assert_err!(
                AtomicKernel::submit_atomic_bundle(
                    RuntimeOrigin::signed(1),
                    vec![],
                    0 // Invalid deadline
                ),
                Error::<Runtime>::DeadlineExpired
            );
            
            // Force rollback
            assert_ok!(AtomicKernel::rollback_atomic_bundle(
                RuntimeOrigin::signed(1),
                bundle_id,
                BundleRollbackReason::ExecutionFailed
            ));
        }
        
        // 2. Verify circuit breaker activation
        assert!(CircuitBreaker::<Runtime>::is_triggered());
        
        // 3. Attempt new operation - should be blocked
        assert_err!(
            AtomicKernel::submit_atomic_bundle(
                RuntimeOrigin::signed(1),
                vec![],
                200
            ),
            Error::<Runtime>::EconomicHaltActive
        );
        
        // 4. Reset circuit breaker after cooldown
        System::set_block_number(1000);
        
        // 5. Verify operations are allowed again
        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(1),
            vec![],
            200
        ));
    });
}
