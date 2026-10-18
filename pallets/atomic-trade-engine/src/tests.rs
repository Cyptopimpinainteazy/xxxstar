//! Unit tests for the AtomicTradeEngine pallet
//!
//! Tests cover:
//! - Trade batch creation and validation
//! - Batch execution with success and failure paths
//! - Checkpoint creation and rollback
//! - AMM adapter registration
//! - Slippage protection
//! - Cross-VM trade execution

use crate::mock::*;
use crate::types::{AmmProtocol, VmType};
use crate::*;
use frame_support::traits::Currency;
use frame_support::{assert_noop, assert_ok, BoundedVec};
use invariant_macros::invariant;
use sp_core::H256;

// Invariants: PALLET-TRADE-001

// ============================================================================
// Helper Functions
// ============================================================================

#[test]
fn settlement_registration_failure_rolls_back_native_execution_and_fees() {
    assert_settlement_registration_failure(false);
}

#[test]
fn settlement_registration_failure_rolls_back_kernel_execution_and_fees() {
    assert_settlement_registration_failure(true);
}

fn assert_settlement_registration_failure(via_kernel: bool) {
    new_test_ext().execute_with(|| {
        let owner = account(1);
        let comit_id = H256::from_low_u64_be(9003);
        pallet_x3_kernel::AuthorizedAccounts::<Test>::insert(owner, ());
        ProtocolFeeBps::set(20);
        sp_io::storage::set(SETTLEMENT_TEST_MODE, &[1]);
        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(owner),
            vec![evm_leg(1_000_000_000_000, 1)],
            500,
            100,
            0,
        ));
        let batch_id = AtomicTradeEngine::pending_batches(owner)[0];
        let balances = (
            Balances::free_balance(owner),
            Balances::free_balance(account(3)),
            Balances::free_balance(ProtocolTreasury::get()),
            Balances::total_issuance(),
        );
        assert_ok!(if via_kernel {
            AtomicTradeEngine::execute_trade_batch_via_kernel_comit_v2(
                RuntimeOrigin::signed(owner),
                batch_id,
                comit_id,
            )
        } else {
            AtomicTradeEngine::execute_trade_batch(RuntimeOrigin::signed(owner), batch_id)
        });
        let batch = AtomicTradeEngine::trade_batches(batch_id).unwrap();
        assert_eq!(batch.status, BatchStatus::Failed);
        assert!(batch
            .legs
            .iter()
            .all(|leg| leg.status == TradeLegStatus::Pending
                && leg.actual_amount_out.is_none()
                && leg.gas_used == 0));
        assert_eq!(
            (
                Balances::free_balance(owner),
                Balances::free_balance(account(3)),
                Balances::free_balance(ProtocolTreasury::get()),
                Balances::total_issuance()
            ),
            balances
        );
        assert!(!sp_io::storage::exists(VM_TEST_WRITE));
        assert!(!sp_io::storage::exists(SETTLEMENT_TEST_WRITE));
        assert_eq!(pallet_x3_kernel::Nonces::<Test>::get(owner), 0);
        assert!(!pallet_x3_kernel::SubmittedComits::<Test>::contains_key(
            comit_id
        ));
        assert_eq!(AtomicTradeEngine::completed_batch_count(), 0);
        assert_eq!(AtomicTradeEngine::total_volume(), 0);
        assert_eq!(AtomicTradeEngine::failed_batch_count(), 1);
        assert!(AtomicTradeEngine::pending_batches(owner).is_empty());
        assert_eq!(AtomicTradeEngine::checkpoints(batch_id).len(), 1);
        assert!(!System::events().iter().any(|record| matches!(
            record.event,
            RuntimeEvent::AtlasKernel(
                pallet_x3_kernel::Event::ComitSubmitted { .. }
                    | pallet_x3_kernel::Event::FeeDeducted { .. }
            )
        )));
        assert!(!System::events().iter().any(|record| matches!(
            record.event,
            RuntimeEvent::AtomicTradeEngine(
                Event::TradeLegCompleted { .. }
                    | Event::TradeBatchCompleted { .. }
                    | Event::ProtocolFeeCollected { .. }
                    | Event::TradeBatchExecutedViaKernelComitV2 { .. }
            )
        )));
        assert!(System::events().iter().any(|record| matches!(
            record.event,
            RuntimeEvent::AtomicTradeEngine(Event::TradeBatchFailed {
                failed_leg_index: 0,
                reason: BatchFailureReason::SettlementRegistrationFailed,
                ..
            })
        )));
        assert_noop!(
            AtomicTradeEngine::execute_trade_batch(RuntimeOrigin::signed(owner), batch_id),
            Error::<Test>::BatchNotPending
        );
    });
}

#[test]
fn successful_registration_commits_vm_fee_and_settlement_effects_together() {
    new_test_ext().execute_with(|| {
        ProtocolFeeBps::set(20);
        sp_io::storage::set(SETTLEMENT_TEST_MODE, &[0]);
        let amount = 1_000_000_000_000;
        let fee = amount * 20 / 10_000;
        let before_owner = Balances::free_balance(account(1));
        let before_recipient = Balances::free_balance(account(3));
        let issuance = Balances::total_issuance();
        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            vec![evm_leg(amount, 1)],
            500,
            100,
            0,
        ));
        let batch_id = AtomicTradeEngine::pending_batches(account(1))[0];
        assert_ok!(AtomicTradeEngine::execute_trade_batch(
            RuntimeOrigin::signed(account(1)),
            batch_id
        ));
        assert_eq!(
            AtomicTradeEngine::trade_batches(batch_id).unwrap().status,
            BatchStatus::Completed
        );
        assert_eq!(
            Balances::free_balance(account(1)),
            before_owner - fee - SETTLEMENT_TEST_TRANSFER
        );
        assert_eq!(
            Balances::free_balance(account(3)),
            before_recipient + SETTLEMENT_TEST_TRANSFER
        );
        assert_eq!(Balances::free_balance(ProtocolTreasury::get()), fee);
        assert_eq!(Balances::total_issuance(), issuance);
        assert!(sp_io::storage::exists(VM_TEST_WRITE));
        assert_eq!(
            sp_io::storage::get(SETTLEMENT_TEST_WRITE).unwrap().as_ref(),
            batch_id.as_bytes()
        );
    });
}

#[test]
fn aggregate_input_overflow_is_rejected_before_native_execution_changes_state() {
    assert_aggregate_input_overflow(true);
}

#[test]
fn aggregate_input_overflow_is_rejected_before_kernel_execution_changes_state() {
    assert_aggregate_input_overflow(false);
}

fn assert_aggregate_input_overflow(via_native: bool) {
    new_test_ext().execute_with(|| {
        let owner = account(1);
        pallet_x3_kernel::AuthorizedAccounts::<Test>::insert(owner, ());
        LAST_EVM_CALLER.with(|last| last.set(None));
        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(owner),
            vec![evm_leg(u128::MAX, 1), svm_leg(1, 1)],
            500,
            100,
            0,
        ));
        let batch_id = AtomicTradeEngine::pending_batches(owner)[0];
        assert_noop!(
            if !via_native {
                AtomicTradeEngine::execute_trade_batch_via_kernel_comit_v2(
                    RuntimeOrigin::signed(owner),
                    batch_id,
                    H256::from_low_u64_be(9001),
                )
            } else {
                AtomicTradeEngine::execute_trade_batch(RuntimeOrigin::signed(owner), batch_id)
            },
            Error::<Test>::ArithmeticOverflow
        );
        LAST_EVM_CALLER.with(|last| assert_eq!(last.get(), None));
        assert_eq!(pallet_x3_kernel::Nonces::<Test>::get(owner), 0);
        assert_eq!(
            AtomicTradeEngine::trade_batches(batch_id).unwrap().status,
            BatchStatus::Pending
        );
    });
}

#[test]
fn aggregate_input_at_u128_limit_still_executes_in_both_paths() {
    for via_kernel in [false, true] {
        new_test_ext().execute_with(|| {
            let owner = account(1);
            pallet_x3_kernel::AuthorizedAccounts::<Test>::insert(owner, ());
            assert_ok!(AtomicTradeEngine::create_trade_batch(
                RuntimeOrigin::signed(owner),
                vec![evm_leg(u128::MAX - 1, 1), svm_leg(1, 1)],
                500,
                100,
                0,
            ));
            let batch_id = AtomicTradeEngine::pending_batches(owner)[0];
            assert_ok!(if via_kernel {
                AtomicTradeEngine::execute_trade_batch_via_kernel_comit_v2(
                    RuntimeOrigin::signed(owner),
                    batch_id,
                    H256::from_low_u64_be(9002),
                )
            } else {
                AtomicTradeEngine::execute_trade_batch(RuntimeOrigin::signed(owner), batch_id)
            });
            assert_eq!(
                AtomicTradeEngine::trade_batches(batch_id).unwrap().status,
                BatchStatus::Completed
            );
            assert_eq!(AtomicTradeEngine::total_volume(), u128::MAX);
        });
    }
}

#[test]
fn second_leg_failure_clears_rolled_back_success_metadata_and_events() {
    new_test_ext().execute_with(|| {
        let amount = 1_000_000_000_000_000_000u128;
        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            vec![evm_leg(amount, 1), evm_leg(0, u128::MAX)],
            500,
            100,
            0,
        ));
        let batch_id = AtomicTradeEngine::pending_batches(account(1))[0];
        let balance_before = Balances::free_balance(account(1));
        assert_ok!(AtomicTradeEngine::execute_trade_batch(
            RuntimeOrigin::signed(account(1)),
            batch_id,
        ));
        let batch = AtomicTradeEngine::trade_batches(batch_id).unwrap();
        assert_eq!(batch.status, BatchStatus::Failed);
        assert_eq!(batch.legs[1].status, TradeLegStatus::Failed);
        assert_eq!(batch.legs[0].status, TradeLegStatus::Pending);
        assert!(batch
            .legs
            .iter()
            .all(|leg| leg.actual_amount_out.is_none() && leg.gas_used == 0));
        assert_eq!(Balances::free_balance(account(1)), balance_before);
        assert_eq!(AtomicTradeEngine::completed_batch_count(), 0);
        assert_eq!(AtomicTradeEngine::failed_batch_count(), 1);
        assert!(AtomicTradeEngine::pending_batches(account(1)).is_empty());
        assert!(!System::events().iter().any(|record| matches!(
            record.event,
            RuntimeEvent::AtomicTradeEngine(
                Event::TradeLegCompleted { .. } | Event::TradeBatchCompleted { .. }
            )
        )));
        assert!(System::events().iter().any(|record| matches!(
            record.event,
            RuntimeEvent::AtomicTradeEngine(Event::TradeBatchFailed {
                failed_leg_index: 1,
                ..
            })
        )));
        assert_noop!(
            AtomicTradeEngine::execute_trade_batch(RuntimeOrigin::signed(account(1)), batch_id),
            Error::<Test>::BatchNotPending
        );
    });
}

#[test]
fn exhausted_transaction_layers_reject_batch_without_panicking_or_changing_state() {
    use frame_support::storage::{
        transactional::TRANSACTIONAL_LIMIT, with_transaction, TransactionOutcome,
    };
    use sp_runtime::{DispatchError, TransactionalError};

    fn execute_at_depth(depth: u32, batch_id: H256) -> Result<(), DispatchError> {
        if depth == 0 {
            return AtomicTradeEngine::execute_trade_batch(
                RuntimeOrigin::signed(account(1)),
                batch_id,
            );
        }
        with_transaction(|| match execute_at_depth(depth - 1, batch_id) {
            Ok(()) => TransactionOutcome::Commit(Ok(())),
            Err(error) => TransactionOutcome::Rollback(Err(error)),
        })
    }

    new_test_ext().execute_with(|| {
        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            vec![evm_leg(1_000_000_000_000_000_000, 1)],
            500,
            100,
            0,
        ));
        let batch_id = AtomicTradeEngine::pending_batches(account(1))[0];
        // The dispatchable itself opens one layer; exhaust the additional
        // boundary around its legs through real nested FRAME transactions.
        assert_noop!(
            execute_at_depth(TRANSACTIONAL_LIMIT - 1, batch_id),
            TransactionalError::LimitReached
        );
        assert_eq!(
            AtomicTradeEngine::trade_batches(batch_id).unwrap().status,
            BatchStatus::Pending
        );
    });
}

/// Create a test trade leg input
fn create_test_leg(
    vm_type: VmType,
    protocol: AmmProtocol,
    amount_in: u128,
    min_out: u128,
) -> TradeLegInput {
    TradeLegInput {
        amm_protocol: protocol,
        vm_type,
        asset_in: H256::from_low_u64_be(1),
        asset_out: H256::from_low_u64_be(2),
        amount_in,
        min_amount_out: min_out,
        route_data: BoundedVec::try_from(vec![0u8; 20]).unwrap(), // Mock recipient address
    }
}

/// Create an EVM trade leg
fn evm_leg(amount: u128, min_out: u128) -> TradeLegInput {
    create_test_leg(VmType::Evm, AmmProtocol::UniswapV2, amount, min_out)
}

#[test]
fn evm_trade_uses_authenticated_batch_owner() {
    new_test_ext().execute_with(|| {
        let owner = account(1);
        LAST_EVM_CALLER.with(|last| last.set(None));
        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(owner),
            vec![evm_leg(1_000_000_000_000_000_000, 900_000_000_000_000_000)],
            100,
            100,
            0,
        ));
        let id = AtomicTradeEngine::pending_batches(owner)[0];
        assert_noop!(
            AtomicTradeEngine::execute_trade_batch(RuntimeOrigin::signed(account(2)), id,),
            Error::<Test>::Unauthorized
        );
        LAST_EVM_CALLER.with(|last| assert_eq!(last.get(), None));
        assert_ok!(AtomicTradeEngine::execute_trade_batch(
            RuntimeOrigin::signed(owner),
            id
        ));
        LAST_EVM_CALLER.with(|last| {
            assert_eq!(
                last.get(),
                Some(pallet_x3_kernel::adapters::authenticated_evm_address(
                    &owner
                ))
            )
        });
    });
}

/// Create an SVM trade leg
fn svm_leg(amount: u128, min_out: u128) -> TradeLegInput {
    create_test_leg(VmType::Svm, AmmProtocol::Raydium, amount, min_out)
}

/// Create an X3 trade leg
fn x3_leg(amount: u128, min_out: u128) -> TradeLegInput {
    // Protocol is currently informational in the pallet implementation; choose a stable default.
    create_test_leg(VmType::X3, AmmProtocol::ConstantProduct, amount, min_out)
}

// ============================================================================
// Trade Batch Creation Tests
// ============================================================================

#[test]
#[invariant("PALLET-TRADE-001")]
fn create_trade_batch_works() {
    new_test_ext().execute_with(|| {
        let account = account(1);
        let legs = vec![evm_leg(1_000_000_000_000_000_000, 900_000_000_000_000_000)];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account),
            legs,
            100, // 1% slippage
            100, // deadline block
            0,   // nonce
        ));

        // Verify nonce was incremented
        assert_eq!(AtomicTradeEngine::get_trade_nonce(&account), 1);

        // Verify pending batches
        let pending = AtomicTradeEngine::pending_batches(account);
        assert_eq!(pending.len(), 1);
    });
}

#[test]
fn create_batch_fails_with_empty_legs() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AtomicTradeEngine::create_trade_batch(
                RuntimeOrigin::signed(account(1)),
                vec![], // Empty legs
                100,
                100,
                0,
            ),
            Error::<Test>::EmptyTradeBatch
        );
    });
}

#[test]
fn create_batch_fails_with_too_many_legs() {
    new_test_ext().execute_with(|| {
        // Create more legs than MaxTradeLegs (16)
        let legs: Vec<_> = (0..20)
            .map(|_| evm_leg(1_000_000_000, 900_000_000))
            .collect();

        assert_noop!(
            AtomicTradeEngine::create_trade_batch(
                RuntimeOrigin::signed(account(1)),
                legs,
                100,
                100,
                0,
            ),
            Error::<Test>::TooManyTradeLegs
        );
    });
}

#[test]
fn create_batch_fails_with_invalid_slippage() {
    new_test_ext().execute_with(|| {
        let legs = vec![evm_leg(1_000_000_000, 900_000_000)];

        // Too high slippage (> 50%)
        assert_noop!(
            AtomicTradeEngine::create_trade_batch(
                RuntimeOrigin::signed(account(1)),
                legs.clone(),
                6000, // 60% - exceeds MAX_SLIPPAGE_BPS
                100,
                0,
            ),
            Error::<Test>::InvalidSlippageTolerance
        );

        // Zero slippage (< MIN_SLIPPAGE_BPS)
        assert_noop!(
            AtomicTradeEngine::create_trade_batch(
                RuntimeOrigin::signed(account(1)),
                legs,
                0, // 0% - below MIN_SLIPPAGE_BPS
                100,
                0,
            ),
            Error::<Test>::InvalidSlippageTolerance
        );
    });
}

#[test]
fn create_batch_fails_with_expired_deadline() {
    new_test_ext().execute_with(|| {
        run_to_block(10);

        let legs = vec![evm_leg(1_000_000_000, 900_000_000)];

        assert_noop!(
            AtomicTradeEngine::create_trade_batch(
                RuntimeOrigin::signed(account(1)),
                legs,
                100,
                5, // Deadline in the past
                0,
            ),
            Error::<Test>::DeadlineExpired
        );
    });
}

#[test]
fn create_batch_fails_with_invalid_nonce() {
    new_test_ext().execute_with(|| {
        let legs = vec![evm_leg(1_000_000_000, 900_000_000)];

        // First batch should work with nonce 0
        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs.clone(),
            100,
            100,
            0,
        ));

        // Second batch should fail with nonce 0 (expected 1)
        assert_noop!(
            AtomicTradeEngine::create_trade_batch(
                RuntimeOrigin::signed(account(1)),
                legs.clone(),
                100,
                100,
                0, // Should be 1
            ),
            Error::<Test>::InvalidTradeNonce
        );

        // Should work with correct nonce
        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            100,
            100,
            1,
        ));
    });
}

// ============================================================================
// Trade Batch Execution Tests
// ============================================================================

#[test]
fn execute_single_evm_leg_works() {
    new_test_ext().execute_with(|| {
        let amount = 1_000_000_000_000_000_000u128;
        // Mock adapter returns 98%, so min_out should be below that
        let min_out = amount * 97 / 100;

        let legs = vec![evm_leg(amount, min_out)];

        // Create batch
        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            100,
            100,
            0,
        ));

        // Get batch ID
        let pending = AtomicTradeEngine::pending_batches(account(1));
        let batch_id = pending[0];

        // Execute batch
        assert_ok!(AtomicTradeEngine::execute_trade_batch(
            RuntimeOrigin::signed(account(1)),
            batch_id,
        ));

        // Verify batch completed
        let batch = AtomicTradeEngine::trade_batches(batch_id).unwrap();
        assert_eq!(batch.status, BatchStatus::Completed);

        // Verify metrics updated
        assert_eq!(AtomicTradeEngine::completed_batch_count(), 1);
        assert!(AtomicTradeEngine::total_volume() > 0);
    });
}

#[test]
fn execute_single_svm_leg_works() {
    new_test_ext().execute_with(|| {
        let amount = 1_000_000_000u128;
        // Mock adapter returns 97%
        let min_out = amount * 96 / 100;

        let legs = vec![svm_leg(amount, min_out)];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            100,
            100,
            0,
        ));

        let pending = AtomicTradeEngine::pending_batches(account(1));
        let batch_id = pending[0];

        assert_ok!(AtomicTradeEngine::execute_trade_batch(
            RuntimeOrigin::signed(account(1)),
            batch_id,
        ));

        let batch = AtomicTradeEngine::trade_batches(batch_id).unwrap();
        assert_eq!(batch.status, BatchStatus::Completed);
    });
}

#[test]
fn execute_single_x3_leg_works() {
    new_test_ext().execute_with(|| {
        let amount = 1_000_000_000_000_000_000u128;
        // Mock adapter returns 99%
        let min_out = amount * 98 / 100;

        let legs = vec![x3_leg(amount, min_out)];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            100,
            100,
            0,
        ));

        let pending = AtomicTradeEngine::pending_batches(account(1));
        let batch_id = pending[0];

        assert_ok!(AtomicTradeEngine::execute_trade_batch(
            RuntimeOrigin::signed(account(1)),
            batch_id,
        ));

        let batch = AtomicTradeEngine::trade_batches(batch_id).unwrap();
        assert_eq!(batch.status, BatchStatus::Completed);
    });
}

#[test]
fn execute_triple_vm_batch_via_kernel_comit_v2_works() {
    new_test_ext().execute_with(|| {
        let who = account(1);
        pallet_x3_kernel::AuthorizedAccounts::<Test>::insert(who, ());

        let evm_amount = 1_000_000_000_000_000_000u128;
        let svm_amount = 1_000_000_000u128;
        let x3_amount = 2_000_000_000_000_000_000u128;

        let legs = vec![
            evm_leg(evm_amount, evm_amount * 97 / 100),
            svm_leg(svm_amount, svm_amount * 96 / 100),
            x3_leg(x3_amount, x3_amount * 98 / 100),
        ];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(who),
            legs,
            500,
            100,
            0,
        ));

        let pending = AtomicTradeEngine::pending_batches(who);
        let batch_id = pending[0];
        let comit_id = H256::from_low_u64_be(5001);

        assert_ok!(AtomicTradeEngine::execute_trade_batch_via_kernel_comit_v2(
            RuntimeOrigin::signed(who),
            batch_id,
            comit_id,
        ));

        // Prove the kernel path executed: kernel nonce increments.
        assert_eq!(pallet_x3_kernel::Nonces::<Test>::get(who), 1);

        let batch = AtomicTradeEngine::trade_batches(batch_id).unwrap();
        assert_eq!(batch.status, BatchStatus::Completed);
    });
}

#[test]
fn kernel_comit_failure_is_rolled_back_but_batch_is_marked_failed() {
    new_test_ext().execute_with(|| {
        let who = account(1);
        pallet_x3_kernel::AuthorizedAccounts::<Test>::insert(who, ());

        // Force kernel fee withdrawal to fail AFTER nonce mutation inside submit_comit_v2.
        // This should be rolled back by AtomicTradeEngine's nested transaction wrapper.
        Balances::make_free_balance_be(&who, 0);

        let legs = vec![
            evm_leg(1_000_000_000_000_000_000u128, 1),
            svm_leg(1_000_000_000u128, 1),
            x3_leg(2_000_000_000_000_000_000u128, 1),
        ];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(who),
            legs,
            500,
            100,
            0,
        ));

        let pending = AtomicTradeEngine::pending_batches(who);
        let batch_id = pending[0];
        let comit_id = H256::from_low_u64_be(5002);

        // The AtomicTradeEngine call returns Ok but marks the batch failed.
        assert_ok!(AtomicTradeEngine::execute_trade_batch_via_kernel_comit_v2(
            RuntimeOrigin::signed(who),
            batch_id,
            comit_id,
        ));

        // Critical property: kernel nonce did NOT increment because kernel changes were rolled back.
        assert_eq!(pallet_x3_kernel::Nonces::<Test>::get(who), 0);

        let batch = AtomicTradeEngine::trade_batches(batch_id).unwrap();
        assert_eq!(batch.status, BatchStatus::Failed);
    });
}

#[test]
fn execute_multi_leg_batch_works() {
    new_test_ext().execute_with(|| {
        let amount = 1_000_000_000_000_000_000u128;

        // Chain: EVM -> SVM
        // EVM returns 98%, SVM returns 97% of that
        let evm_out = amount * 98 / 100;
        let svm_out = evm_out * 97 / 100;

        let legs = vec![
            evm_leg(amount, evm_out * 95 / 100),
            svm_leg(0, svm_out * 95 / 100), // 0 means use carry from previous
        ];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            500, // 5% slippage to cover both legs
            100,
            0,
        ));

        let pending = AtomicTradeEngine::pending_batches(account(1));
        let batch_id = pending[0];

        assert_ok!(AtomicTradeEngine::execute_trade_batch(
            RuntimeOrigin::signed(account(1)),
            batch_id,
        ));

        let batch = AtomicTradeEngine::trade_batches(batch_id).unwrap();
        assert_eq!(batch.status, BatchStatus::Completed);
        assert_eq!(batch.legs.len(), 2);

        // Both legs should be completed
        assert!(batch
            .legs
            .iter()
            .all(|l| l.status == TradeLegStatus::Completed));
    });
}

#[test]
fn execute_fails_with_slippage_exceeded() {
    new_test_ext().execute_with(|| {
        let amount = 1_000_000_000_000_000_000u128;
        // Mock returns 98%, but we require 99%
        let min_out = amount * 99 / 100;

        let legs = vec![evm_leg(amount, min_out)];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            100,
            100,
            0,
        ));

        let pending = AtomicTradeEngine::pending_batches(account(1));
        let batch_id = pending[0];

        // Execute - returns Ok but batch status is Failed due to slippage
        assert_ok!(AtomicTradeEngine::execute_trade_batch(
            RuntimeOrigin::signed(account(1)),
            batch_id,
        ));

        // Verify batch marked as Failed (returns Ok, but batch was processed and failed)
        let batch = AtomicTradeEngine::trade_batches(batch_id).unwrap();
        assert_eq!(batch.status, BatchStatus::Failed);
        assert_eq!(AtomicTradeEngine::failed_batch_count(), 1);
    });
}

#[test]
fn execute_fails_for_non_owner() {
    new_test_ext().execute_with(|| {
        let legs = vec![evm_leg(1_000_000_000, 900_000_000)];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            100,
            100,
            0,
        ));

        let pending = AtomicTradeEngine::pending_batches(account(1));
        let batch_id = pending[0];

        // Different account tries to execute
        assert_noop!(
            AtomicTradeEngine::execute_trade_batch(RuntimeOrigin::signed(account(2)), batch_id,),
            Error::<Test>::Unauthorized
        );
    });
}

#[test]
fn execute_fails_for_non_existent_batch() {
    new_test_ext().execute_with(|| {
        let fake_batch_id = H256::from_low_u64_be(999);

        assert_noop!(
            AtomicTradeEngine::execute_trade_batch(
                RuntimeOrigin::signed(account(1)),
                fake_batch_id,
            ),
            Error::<Test>::BatchNotFound
        );
    });
}

// ============================================================================
// Cancel Batch Tests
// ============================================================================

#[test]
fn cancel_pending_batch_works() {
    new_test_ext().execute_with(|| {
        let legs = vec![evm_leg(1_000_000_000, 900_000_000)];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            100,
            100,
            0,
        ));

        let pending = AtomicTradeEngine::pending_batches(account(1));
        let batch_id = pending[0];

        assert_ok!(AtomicTradeEngine::cancel_trade_batch(
            RuntimeOrigin::signed(account(1)),
            batch_id,
        ));

        // Batch should be removed
        assert!(AtomicTradeEngine::trade_batches(batch_id).is_none());

        // Should be removed from pending
        let pending_after = AtomicTradeEngine::pending_batches(account(1));
        assert!(pending_after.is_empty());
    });
}

#[test]
fn cancel_fails_for_non_owner() {
    new_test_ext().execute_with(|| {
        let legs = vec![evm_leg(1_000_000_000, 900_000_000)];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            100,
            100,
            0,
        ));

        let pending = AtomicTradeEngine::pending_batches(account(1));
        let batch_id = pending[0];

        assert_noop!(
            AtomicTradeEngine::cancel_trade_batch(RuntimeOrigin::signed(account(2)), batch_id,),
            Error::<Test>::Unauthorized
        );
    });
}

// ============================================================================
// AMM Adapter Registration Tests
// ============================================================================

#[test]
fn register_amm_adapter_works() {
    new_test_ext().execute_with(|| {
        let config = AmmAdapterConfig {
            vm_type: VmType::Evm,
            address: BoundedVec::try_from(vec![0xab; 20]).unwrap(),
            fee_bps: 30, // 0.3%
            enabled: true,
        };

        assert_ok!(AtomicTradeEngine::register_amm_adapter(
            RuntimeOrigin::root(),
            AmmProtocol::UniswapV2,
            config.clone(),
        ));

        // Verify registration
        let stored = AtomicTradeEngine::amm_adapters(AmmProtocol::UniswapV2).unwrap();
        assert_eq!(stored.vm_type, VmType::Evm);
        assert_eq!(stored.fee_bps, 30);
    });
}

#[test]
fn register_amm_adapter_fails_if_already_registered() {
    new_test_ext().execute_with(|| {
        let config = AmmAdapterConfig {
            vm_type: VmType::Evm,
            address: BoundedVec::try_from(vec![0xab; 20]).unwrap(),
            fee_bps: 30,
            enabled: true,
        };

        assert_ok!(AtomicTradeEngine::register_amm_adapter(
            RuntimeOrigin::root(),
            AmmProtocol::UniswapV2,
            config.clone(),
        ));

        assert_noop!(
            AtomicTradeEngine::register_amm_adapter(
                RuntimeOrigin::root(),
                AmmProtocol::UniswapV2,
                config,
            ),
            Error::<Test>::AmmAlreadyRegistered
        );
    });
}

#[test]
fn remove_amm_adapter_works() {
    new_test_ext().execute_with(|| {
        let config = AmmAdapterConfig {
            vm_type: VmType::Svm,
            address: BoundedVec::try_from(vec![0xcd; 32]).unwrap(),
            fee_bps: 25,
            enabled: true,
        };

        assert_ok!(AtomicTradeEngine::register_amm_adapter(
            RuntimeOrigin::root(),
            AmmProtocol::Raydium,
            config,
        ));

        assert_ok!(AtomicTradeEngine::remove_amm_adapter(
            RuntimeOrigin::root(),
            AmmProtocol::Raydium,
        ));

        assert!(AtomicTradeEngine::amm_adapters(AmmProtocol::Raydium).is_none());
    });
}

// ============================================================================
// Liquidity Pool Tests
// ============================================================================

#[test]
fn register_liquidity_pool_works() {
    new_test_ext().execute_with(|| {
        let token_a = H256::from_low_u64_be(1);
        let token_b = H256::from_low_u64_be(2);

        assert_ok!(AtomicTradeEngine::register_liquidity_pool(
            RuntimeOrigin::root(),
            AmmProtocol::UniswapV2,
            VmType::Evm,
            token_a,
            token_b,
            1_000_000_000_000,
            2_000_000_000_000,
            30,
            vec![0x11; 20],
        ));

        let pools: Vec<_> = LiquidityPools::<Test>::iter().collect();
        assert_eq!(pools.len(), 1);
        let (_pool_id, pool) = pools[0].clone();
        assert_eq!(pool.token_a, token_a);
        assert_eq!(pool.token_b, token_b);
        assert_eq!(pool.reserve_a, 1_000_000_000_000);
        assert_eq!(pool.reserve_b, 2_000_000_000_000);
        assert_eq!(pool.fee_bps, 30);
        assert_eq!(pool.protocol, AmmProtocol::UniswapV2);
    });
}

#[test]
fn update_liquidity_pool_works() {
    new_test_ext().execute_with(|| {
        let token_a = H256::from_low_u64_be(3);
        let token_b = H256::from_low_u64_be(4);

        assert_ok!(AtomicTradeEngine::register_liquidity_pool(
            RuntimeOrigin::root(),
            AmmProtocol::Raydium,
            VmType::Svm,
            token_a,
            token_b,
            5_000_000_000_000,
            6_000_000_000_000,
            25,
            vec![0x22; 32],
        ));

        let pool_id = LiquidityPools::<Test>::iter().next().unwrap().0;

        assert_ok!(AtomicTradeEngine::update_liquidity_pool(
            RuntimeOrigin::root(),
            pool_id,
            7_000_000_000_000,
            8_000_000_000_000,
        ));

        let pool = AtomicTradeEngine::liquidity_pools(pool_id).unwrap();
        assert_eq!(pool.reserve_a, 7_000_000_000_000);
        assert_eq!(pool.reserve_b, 8_000_000_000_000);
    });
}

#[test]
fn sync_pool_price_writes_oracle_observation() {
    new_test_ext().execute_with(|| {
        let token_a = H256::from_low_u64_be(9);
        let token_b = H256::from_low_u64_be(10);

        assert_ok!(AtomicTradeEngine::register_liquidity_pool(
            RuntimeOrigin::root(),
            AmmProtocol::AtlasAmm,
            VmType::X3,
            token_a,
            token_b,
            1_000_000_000_000,
            4_000_000_000_000,
            20,
            vec![0x33; 16],
        ));

        let pool_id = LiquidityPools::<Test>::iter().next().unwrap().0;

        assert_ok!(AtomicTradeEngine::sync_pool_price(
            RuntimeOrigin::root(),
            pool_id,
        ));

        let observations = AtomicTradeEngine::price_observations((token_a, token_b));
        assert!(!observations.is_empty());
        assert!(AtomicTradeEngine::twap_data((token_a, token_b)).is_some());
    });
}

// ============================================================================
// Checkpoint Tests
// ============================================================================

#[test]
fn checkpoints_are_created_during_execution() {
    new_test_ext().execute_with(|| {
        let amount = 1_000_000_000_000_000_000u128;
        let legs = vec![evm_leg(amount, amount * 95 / 100)];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            100,
            100,
            0,
        ));

        let pending = AtomicTradeEngine::pending_batches(account(1));
        let batch_id = pending[0];

        assert_ok!(AtomicTradeEngine::execute_trade_batch(
            RuntimeOrigin::signed(account(1)),
            batch_id,
        ));

        // Checkpoints should have been created
        let checkpoints = AtomicTradeEngine::checkpoints(batch_id);
        assert!(!checkpoints.is_empty());
        assert!(checkpoints[0].state_root != H256::zero());
    });
}

// ============================================================================
// Simulation Tests
// ============================================================================

#[test]
fn simulate_trade_path_works() {
    new_test_ext().execute_with(|| {
        let legs = vec![evm_leg(1_000_000_000_000_000_000, 0), svm_leg(0, 0)];

        let result =
            AtomicTradeEngine::simulate_trade_path(&legs, 1_000_000_000_000_000_000u128).unwrap();

        // 2 legs with 0.3% fee each
        // 1e18 * 0.997 * 0.997 = ~994009000000000000
        assert!(result > 990_000_000_000_000_000u128);
        assert!(result < 1_000_000_000_000_000_000u128);
    });
}

// ============================================================================
// Event Tests
// ============================================================================

#[test]
fn events_are_emitted_correctly() {
    new_test_ext().execute_with(|| {
        System::reset_events();

        let amount = 1_000_000_000_000_000_000u128;
        let legs = vec![evm_leg(amount, amount * 95 / 100)];

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            100,
            100,
            0,
        ));

        // Check for TradeBatchCreated event
        let events = System::events();
        assert!(events.iter().any(|e| matches!(
            &e.event,
            RuntimeEvent::AtomicTradeEngine(Event::TradeBatchCreated { .. })
        )));

        let pending = AtomicTradeEngine::pending_batches(account(1));
        let batch_id = pending[0];

        System::reset_events();

        assert_ok!(AtomicTradeEngine::execute_trade_batch(
            RuntimeOrigin::signed(account(1)),
            batch_id,
        ));

        let events = System::events();

        // Should have checkpoint, leg started, leg completed, batch completed events
        assert!(events.iter().any(|e| matches!(
            &e.event,
            RuntimeEvent::AtomicTradeEngine(Event::CheckpointCreated { .. })
        )));
        assert!(events.iter().any(|e| matches!(
            &e.event,
            RuntimeEvent::AtomicTradeEngine(Event::TradeLegStarted { .. })
        )));
        assert!(events.iter().any(|e| matches!(
            &e.event,
            RuntimeEvent::AtomicTradeEngine(Event::TradeLegCompleted { .. })
        )));
        assert!(events.iter().any(|e| matches!(
            &e.event,
            RuntimeEvent::AtomicTradeEngine(Event::TradeBatchCompleted { .. })
        )));
    });
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn max_trade_legs_boundary() {
    new_test_ext().execute_with(|| {
        // Exactly MaxTradeLegs (16) should work
        let legs: Vec<_> = (0..16)
            .map(|_| evm_leg(1_000_000_000, 900_000_000))
            .collect();

        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            500,
            100,
            0,
        ));
    });
}

#[test]
fn zero_amount_leg_fails_gracefully() {
    new_test_ext().execute_with(|| {
        let legs = vec![evm_leg(0, 0)];

        // Should still create batch (validation happens at execution)
        assert_ok!(AtomicTradeEngine::create_trade_batch(
            RuntimeOrigin::signed(account(1)),
            legs,
            100,
            100,
            0,
        ));
    });
}

#[test]
fn concurrent_batches_per_account() {
    new_test_ext().execute_with(|| {
        // Create multiple pending batches
        for i in 0..5 {
            let legs = vec![evm_leg(1_000_000_000, 900_000_000)];
            assert_ok!(AtomicTradeEngine::create_trade_batch(
                RuntimeOrigin::signed(account(1)),
                legs,
                100,
                100,
                i,
            ));
        }

        let pending = AtomicTradeEngine::pending_batches(account(1));
        assert_eq!(pending.len(), 5);
    });
}

// ── GAP-SILENT-FEE-WAIVER: a protocol fee the treasury did not receive is recorded ──

fn execute_one_evm_batch(who: u64) -> u128 {
    let amount = 1_000_000_000_000_000_000u128;
    assert_ok!(AtomicTradeEngine::create_trade_batch(
        RuntimeOrigin::signed(account(who)),
        vec![evm_leg(amount, amount * 97 / 100)],
        100,
        100,
        0,
    ));
    let batch_id = AtomicTradeEngine::pending_batches(account(who))[0];
    assert_ok!(AtomicTradeEngine::execute_trade_batch(
        RuntimeOrigin::signed(account(who)),
        batch_id,
    ));
    assert_eq!(
        AtomicTradeEngine::trade_batches(batch_id).unwrap().status,
        BatchStatus::Completed
    );
    amount
}

#[test]
fn a_collected_protocol_fee_reaches_the_treasury() {
    new_test_ext().execute_with(|| {
        ProtocolFeeBps::set(20);
        let fee = execute_one_evm_batch(1) * 20 / 10_000;
        assert_eq!(Balances::free_balance(ProtocolTreasury::get()), fee);
        System::assert_has_event(RuntimeEvent::AtomicTradeEngine(
            Event::ProtocolFeeCollected {
                who: account(1),
                fee,
            },
        ));
    });
}

#[test]
fn an_uncollected_protocol_fee_is_recorded_as_waived() {
    new_test_ext().execute_with(|| {
        ProtocolFeeBps::set(20);
        // The submitter cannot pay the fee: the batch still completes, and the lost fee is on
        // record instead of vanishing.
        Balances::make_free_balance_be(&account(1), 1);
        let fee = execute_one_evm_batch(1) * 20 / 10_000;
        System::assert_has_event(RuntimeEvent::AtomicTradeEngine(Event::ProtocolFeeWaived {
            who: account(1),
            fee,
        }));
        assert_eq!(Balances::free_balance(ProtocolTreasury::get()), 0);
    });
}
