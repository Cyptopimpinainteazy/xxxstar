//! Unit tests for the DePIN GPU Marketplace pallet.
//!
//! # Invariants tested:
//! - DEPIN-MARKET-002: Revenue split conservation
//! - DEPIN-MARKET-003: Escrow lifecycle
//! - DEPIN-MARKET-005: Provider slashing on failure

use crate::{mock::*, types::*, Error, Event};
use frame_support::BoundedVec;
use frame_support::{assert_noop, assert_ok};

fn default_gpu_specs() -> GpuSpecification {
    GpuSpecification {
        model: BoundedVec::try_from(b"NVIDIA A100 80GB".to_vec()).unwrap(),
        vram_mb: 81_920,
        compute_units: 108,
        tier: GpuTier::Datacenter,
        tensor_cores: true,
        confidential_compute: true,
        benchmark_score: 95_000,
    }
}

fn default_gpu_requirements() -> GpuRequirements {
    GpuRequirements {
        min_tier: GpuTier::Consumer,
        min_vram_mb: 8_192,
        min_compute_units: 16,
        requires_tensor_cores: false,
        requires_confidential: false,
    }
}

fn default_job_type() -> DePinJobType {
    DePinJobType::AiInference {
        model_hash: sp_core::H256::repeat_byte(0x42),
        input_size: 1024,
    }
}

// ──────────────────────────────────────────────────────────────
// Provider Registration
// ──────────────────────────────────────────────────────────────

#[test]
fn register_provider_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(DepinMarketplace::register_provider(
            RuntimeOrigin::signed(1),
            default_gpu_specs(),
            10,
        ));

        let provider = DepinMarketplace::providers(1).unwrap();
        assert_eq!(provider.status, ProviderStatus::Active);
        assert_eq!(provider.reputation, 5_000);
        assert_eq!(provider.stake, 1_000);

        // Balance should be reduced by stake
        assert_eq!(Balances::free_balance(1), 99_000);
        assert_eq!(Balances::reserved_balance(1), 1_000);
    });
}

#[test]
fn register_provider_fails_duplicate() {
    new_test_ext().execute_with(|| {
        assert_ok!(DepinMarketplace::register_provider(
            RuntimeOrigin::signed(1),
            default_gpu_specs(),
            10,
        ));
        assert_noop!(
            DepinMarketplace::register_provider(RuntimeOrigin::signed(1), default_gpu_specs(), 10,),
            Error::<Test>::ProviderAlreadyRegistered
        );
    });
}

#[test]
fn register_provider_fails_insufficient_stake() {
    new_test_ext().execute_with(|| {
        // Account 99 has no balance
        assert_noop!(
            DepinMarketplace::register_provider(
                RuntimeOrigin::signed(99),
                default_gpu_specs(),
                10,
            ),
            Error::<Test>::InsufficientStake
        );
    });
}

// ──────────────────────────────────────────────────────────────
// Deregistration
// ──────────────────────────────────────────────────────────────

#[test]
fn deregister_provider_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(DepinMarketplace::register_provider(
            RuntimeOrigin::signed(1),
            default_gpu_specs(),
            10,
        ));
        assert_ok!(DepinMarketplace::deregister_provider(
            RuntimeOrigin::signed(1)
        ));

        assert!(DepinMarketplace::providers(1).is_none());
        assert_eq!(Balances::free_balance(1), 100_000); // stake returned
        assert_eq!(Balances::reserved_balance(1), 0);
    });
}

// ──────────────────────────────────────────────────────────────
// Order Submission & Escrow
// ──────────────────────────────────────────────────────────────

/// # Invariant: DEPIN-MARKET-003
#[test]
fn escrow_lifecycle() {
    new_test_ext().execute_with(|| {
        // Register provider
        assert_ok!(DepinMarketplace::register_provider(
            RuntimeOrigin::signed(1),
            default_gpu_specs(),
            10,
        ));

        let customer_balance_before = Balances::free_balance(10);

        // Submit order (customer = 10)
        assert_ok!(DepinMarketplace::submit_order(
            RuntimeOrigin::signed(10),
            default_job_type(),
            default_gpu_requirements(),
            5_000, // max_price
            100,   // duration
        ));

        // Customer balance reduced by escrow
        assert_eq!(Balances::free_balance(10), customer_balance_before - 5_000);

        // Escrow held in pallet account
        let escrow_balance = Balances::free_balance(DepinMarketplace::account_id());
        assert!(escrow_balance >= 5_000);

        // Pending orders should have 1 entry
        assert_eq!(DepinMarketplace::pending_orders().len(), 1);
        let job_id = DepinMarketplace::pending_orders()[0].job_id;

        // Provider accepts
        assert_ok!(DepinMarketplace::accept_order(
            RuntimeOrigin::signed(1),
            job_id,
        ));

        // Order removed from pending, active job created
        assert_eq!(DepinMarketplace::pending_orders().len(), 0);
        assert!(DepinMarketplace::active_jobs(job_id).is_some());

        // Provider completes
        assert_ok!(DepinMarketplace::complete_job(
            RuntimeOrigin::signed(1),
            job_id,
            sp_core::H256::repeat_byte(0xAA),
            1000,
        ));

        // Job removed
        assert!(DepinMarketplace::active_jobs(job_id).is_none());

        // Provider earned revenue
        let provider = DepinMarketplace::providers(1).unwrap();
        assert_eq!(provider.total_jobs_completed, 1);
        assert!(provider.total_revenue > 0u128.into());
    });
}

/// # Invariant: DEPIN-MARKET-002
#[test]
fn revenue_split_conservation() {
    new_test_ext().execute_with(|| {
        // Register provider
        assert_ok!(DepinMarketplace::register_provider(
            RuntimeOrigin::signed(1),
            default_gpu_specs(),
            10,
        ));

        // Submit and complete a job
        assert_ok!(DepinMarketplace::submit_order(
            RuntimeOrigin::signed(10),
            default_job_type(),
            default_gpu_requirements(),
            10_000,
            100,
        ));

        let job_id = DepinMarketplace::pending_orders()[0].job_id;
        assert_ok!(DepinMarketplace::accept_order(
            RuntimeOrigin::signed(1),
            job_id
        ));

        let provider_balance_before = Balances::free_balance(1);

        assert_ok!(DepinMarketplace::complete_job(
            RuntimeOrigin::signed(1),
            job_id,
            sp_core::H256::repeat_byte(0xBB),
            500,
        ));

        // Validator got 55% of 10_000 = 5_500
        let provider_balance_after = Balances::free_balance(1);
        let validator_earned = provider_balance_after - provider_balance_before;
        assert_eq!(validator_earned, 5_500);

        // Total burned should be 25% of 10_000 = 2_500
        assert_eq!(DepinMarketplace::total_burned(), 2_500);

        // Total revenue = 10_000
        assert_eq!(DepinMarketplace::total_revenue(), 10_000);
    });
}

// ──────────────────────────────────────────────────────────────
// Slashing
// ──────────────────────────────────────────────────────────────

/// # Invariant: DEPIN-MARKET-005
#[test]
fn slash_on_failure() {
    new_test_ext().execute_with(|| {
        // Register provider
        assert_ok!(DepinMarketplace::register_provider(
            RuntimeOrigin::signed(1),
            default_gpu_specs(),
            10,
        ));

        let reserved_before = Balances::reserved_balance(1);

        // Submit order
        assert_ok!(DepinMarketplace::submit_order(
            RuntimeOrigin::signed(10),
            default_job_type(),
            default_gpu_requirements(),
            5_000,
            100,
        ));

        let job_id = DepinMarketplace::pending_orders()[0].job_id;
        assert_ok!(DepinMarketplace::accept_order(
            RuntimeOrigin::signed(1),
            job_id
        ));

        // Report failure
        assert_ok!(DepinMarketplace::report_job_failure(
            RuntimeOrigin::signed(1),
            job_id,
            JobFailureReason::ExecutionError,
        ));

        // Provider stake reduced (10% slash = 100)
        let reserved_after = Balances::reserved_balance(1);
        assert!(reserved_after < reserved_before);

        // Customer refunded
        assert_eq!(Balances::free_balance(10), 1_000_000);

        // Provider reputation decreased
        let provider = DepinMarketplace::providers(1).unwrap();
        assert!(provider.reputation < 5_000);
    });
}

// ──────────────────────────────────────────────────────────────
// Order Cancellation
// ──────────────────────────────────────────────────────────────

#[test]
fn cancel_order_refunds_escrow() {
    new_test_ext().execute_with(|| {
        let before = Balances::free_balance(10);

        assert_ok!(DepinMarketplace::submit_order(
            RuntimeOrigin::signed(10),
            default_job_type(),
            default_gpu_requirements(),
            3_000,
            100,
        ));

        let job_id = DepinMarketplace::pending_orders()[0].job_id;
        assert_ok!(DepinMarketplace::cancel_order(
            RuntimeOrigin::signed(10),
            job_id
        ));

        // Full refund
        assert_eq!(Balances::free_balance(10), before);
        assert_eq!(DepinMarketplace::pending_orders().len(), 0);
    });
}

// ──────────────────────────────────────────────────────────────
// Marketplace Pause
// ──────────────────────────────────────────────────────────────

#[test]
fn marketplace_pause_blocks_operations() {
    new_test_ext().execute_with(|| {
        assert_ok!(DepinMarketplace::pause_marketplace(RuntimeOrigin::root()));
        assert!(DepinMarketplace::is_paused());

        assert_noop!(
            DepinMarketplace::register_provider(RuntimeOrigin::signed(1), default_gpu_specs(), 10,),
            Error::<Test>::MarketplacePaused
        );

        assert_ok!(DepinMarketplace::resume_marketplace(RuntimeOrigin::root()));
        assert!(!DepinMarketplace::is_paused());

        assert_ok!(DepinMarketplace::register_provider(
            RuntimeOrigin::signed(1),
            default_gpu_specs(),
            10,
        ));
    });
}
