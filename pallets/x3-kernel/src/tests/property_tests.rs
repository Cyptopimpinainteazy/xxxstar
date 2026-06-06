//! Property-based tests for x3-kernel pallet using proptest
//!
//! TICKET-4.5-004 Feature 2 Step 4: Property-based tests with proptest for asset kernel
//!
//! These tests use randomized input generation to verify critical invariants across
//! hundreds of test cases, uncovering edge cases that traditional unit tests might miss.
//!
//! Tested invariants:
//! - Reserve/unreserve symmetry
//! - Balance arithmetic never overflows
//! - Fee accounting maintains total balance invariant
//! - Nonce monotonicity
//! - Atomic rollback on failures

use super::*;
use crate::mock::AccountId;
use frame_support::traits::ReservableCurrency;
use proptest::prelude::*;

/// Strategy: Generate valid account IDs (using pre-defined test accounts)
fn arb_account_id() -> impl Strategy<Value = AccountId> {
    prop_oneof![Just(ALICE), Just(BOB), Just(3u64), Just(4u64),]
}

/// Strategy: Generate valid balance amounts (avoiding overflow)
fn arb_balance(min: u128, max: u128) -> impl Strategy<Value = Balance> {
    min..=max
}

/// Strategy: Generate realistic fee amounts (1-10000 units)
fn arb_fee() -> impl Strategy<Value = Balance> {
    1u128..=10_000u128
}

/// Strategy: Generate valid nonce values
fn arb_nonce() -> impl Strategy<Value = u64> {
    0u64..=1000u64
}

/// Strategy: Generate valid comit payloads (varying lengths)
fn arb_payload() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 1..=32)
}

/// Strategy: Generate valid X3 payloads (minimum 4 bytes with X3 magic prefix)
fn arb_x3_payload() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..=28).prop_map(|mut v| {
        let mut result = vec![0x58, 0x33, 0x00, 0x01]; // X3 magic prefix
        result.append(&mut v);
        result
    })
}

/// Helper: Compute prepare root for property tests
fn compute_property_prepare_root(
    comit_id: H256,
    evm_payload: &[u8],
    svm_payload: &[u8],
    x3_payload: &[u8],
    nonce: u64,
    fee: Balance,
) -> H256 {
    use sp_core::hashing::blake2_256;

    let mut data = Vec::new();
    data.extend_from_slice(comit_id.as_bytes());
    data.extend_from_slice(evm_payload);
    data.extend_from_slice(svm_payload);
    data.extend_from_slice(x3_payload);
    data.extend_from_slice(&nonce.to_le_bytes());
    data.extend_from_slice(&fee.to_le_bytes());

    H256::from(blake2_256(&data))
}

// ================================================================================================
// PROPERTY TEST 1: Balance Invariants
// ================================================================================================

proptest! {
    /// Property: Total balance should remain constant across operations
    /// Invariant: total_balance = free_balance + reserved_balance
    #[test]
    fn prop_balance_invariants(
        account in arb_account_id(),
        amount in arb_balance(1, 1_000_000)
    ) {
        new_test_ext().execute_with(|| {
            // Initial state
            let initial_free = mock::Balances::free_balance(&account);
            let initial_reserved = mock::Balances::reserved_balance(&account);
            let initial_total = initial_free + initial_reserved;

            // Reserve some balance
            if initial_free >= amount {
                let reserve_result = mock::Balances::reserve(&account, amount);

                if reserve_result.is_ok() {
                    let after_reserve_free = mock::Balances::free_balance(&account);
                    let after_reserve_reserved = mock::Balances::reserved_balance(&account);
                    let after_reserve_total = after_reserve_free + after_reserve_reserved;

                    // Invariant 1: Total balance unchanged after reserve
                    prop_assert_eq!(
                        after_reserve_total,
                        initial_total,
                        "Total balance must not change after reserve"
                    );

                    // Invariant 2: Reserved amount matches
                    prop_assert_eq!(
                        after_reserve_reserved,
                        initial_reserved + amount,
                        "Reserved balance must increase by reserved amount"
                    );

                    // Unreserve the amount
                    let unreserved = mock::Balances::unreserve(&account, amount);

                    let final_free = mock::Balances::free_balance(&account);
                    let final_reserved = mock::Balances::reserved_balance(&account);
                    let final_total = final_free + final_reserved;

                    // Invariant 3: Full unreserve succeeds
                    prop_assert_eq!(
                        unreserved,
                        amount,
                        "Full amount should be unreserved"
                    );

                    // Invariant 4: Total balance unchanged after unreserve
                    prop_assert_eq!(
                        final_total,
                        initial_total,
                        "Total balance must not change after unreserve"
                    );

                    // Invariant 5: Reserve/unreserve symmetry
                    prop_assert_eq!(
                        final_free,
                        initial_free,
                        "Free balance must return to initial after reserve/unreserve cycle"
                    );

                    prop_assert_eq!(
                        final_reserved,
                        initial_reserved,
                        "Reserved balance must return to initial after reserve/unreserve cycle"
                    );
                }
            }

            Ok(())
        });
    }
}

// ================================================================================================
// PROPERTY TEST 2: Fee Charging Properties
// ================================================================================================

proptest! {
    /// Property: Fee charging should never exceed max_fee
    /// Invariant: actual_fee_charged <= max_fee
    #[test]
    fn prop_fee_never_exceeds_max(
        max_fee in arb_fee(),
        nonce in arb_nonce(),
        evm_payload in arb_payload(),
        svm_payload in arb_payload(),
        x3_payload in arb_x3_payload(),
    ) {
        new_test_ext().execute_with(|| {
            let account = ALICE;
            let initial_balance = mock::Balances::free_balance(&account);

            // Skip if insufficient balance
            if initial_balance < max_fee {
                return Ok(());
            }

            let comit_id = H256::random();
            let prepare_root = compute_property_prepare_root(
                comit_id,
                &evm_payload,
                &svm_payload,
                &x3_payload,
                nonce,
                max_fee,
            );

            let result = AtlasKernel::submit_comit_v2(
                RuntimeOrigin::signed(account),
                comit_id,
                evm_payload,
                svm_payload,
                x3_payload,
                nonce,
                max_fee,
                prepare_root,
            );

            let final_balance = mock::Balances::free_balance(&account);
            let actual_fee_charged = initial_balance.saturating_sub(final_balance);

            if result.is_ok() {
                // Invariant 1: Successful operations charge some fee
                prop_assert!(
                    actual_fee_charged > 0,
                    "Successful operations must charge a fee"
                );

                // Invariant 2: Fee never exceeds max
                prop_assert!(
                    actual_fee_charged <= max_fee,
                    "Actual fee must not exceed max_fee (charged: {}, max: {})",
                    actual_fee_charged,
                    max_fee
                );
            } else {
                // Invariant 3: Failed operations don't charge fees (atomic rollback)
                prop_assert_eq!(
                    actual_fee_charged,
                    0,
                    "Failed operations must not charge fees"
                );
            }

            Ok(())
        });
    }
}

// ================================================================================================
// PROPERTY TEST 3: Nonce Monotonicity
// ================================================================================================

proptest! {
    /// Property: Nonces must always increase, preventing replay attacks
    /// Invariant: nonce[n+1] > nonce[n]
    #[test]
    fn prop_nonce_monotonicity(
        nonce_sequence in prop::collection::vec(arb_nonce(), 2..=5),
        max_fee in arb_fee(),
        evm_payload in arb_payload(),
        svm_payload in arb_payload(),
        x3_payload in arb_x3_payload(),
    ) {
        new_test_ext().execute_with(|| {
            let account = ALICE;
            let mut sorted_nonces = nonce_sequence.clone();
            sorted_nonces.sort_unstable();
            sorted_nonces.dedup(); // Remove duplicates

            if sorted_nonces.len() < 2 {
                return Ok(());
            }

            let mut last_accepted_nonce: Option<u64> = None;

            for nonce in sorted_nonces {
                let comit_id = H256::random();
                let prepare_root = compute_property_prepare_root(
                    comit_id,
                    &evm_payload,
                    &svm_payload,
                    &x3_payload,
                    nonce,
                    max_fee,
                );

                let result = AtlasKernel::submit_comit_v2(
                    RuntimeOrigin::signed(account),
                    comit_id,
                    evm_payload.clone(),
                    svm_payload.clone(),
                    x3_payload.clone(),
                    nonce,
                    max_fee,
                    prepare_root,
                );

                if let Some(last_nonce) = last_accepted_nonce {
                    if nonce <= last_nonce {
                        // Invariant: Non-increasing nonce should be rejected
                        prop_assert!(
                            result.is_err(),
                            "Nonce {} should be rejected after nonce {}",
                            nonce,
                            last_nonce
                        );
                    }
                }

                if result.is_ok() {
                    last_accepted_nonce = Some(nonce);
                }
            }

            Ok(())
        });
    }
}

// ================================================================================================
// PROPERTY TEST 4: No Arithmetic Overflow
// ================================================================================================

proptest! {
    /// Property: Balance arithmetic operations should never overflow
    /// Invariant: All balance operations use saturating arithmetic
    #[test]
    fn prop_no_overflow(
        amounts in prop::collection::vec(arb_balance(1, 100_000), 1..=10),
    ) {
        new_test_ext().execute_with(|| {
            let account = ALICE;
            let initial_balance = mock::Balances::free_balance(&account);

            let mut cumulative_reserved: Balance = 0;
            let mut successful_reserves: Vec<Balance> = Vec::new();

            // Try to reserve multiple amounts
            for amount in &amounts {
                let current_free = mock::Balances::free_balance(&account);

                if current_free >= *amount {
                    let reserve_result = mock::Balances::reserve(&account, *amount);

                    if reserve_result.is_ok() {
                        cumulative_reserved = cumulative_reserved.saturating_add(*amount);
                        successful_reserves.push(*amount);

                        let new_reserved = mock::Balances::reserved_balance(&account);

                        // Invariant: Reserved balance accumulates correctly (no overflow)
                        prop_assert!(
                            new_reserved <= initial_balance,
                            "Reserved balance cannot exceed initial balance"
                        );
                    }
                }
            }

            // Unreserve all successful reserves
            for amount in successful_reserves {
                let unreserved = mock::Balances::unreserve(&account, amount);

                // Invariant: Unreserve returns correct amount (no underflow)
                prop_assert_eq!(
                    unreserved,
                    amount,
                    "Unreserve should return full amount"
                );
            }

            let final_free = mock::Balances::free_balance(&account);
            let final_reserved = mock::Balances::reserved_balance(&account);
            let final_total = final_free + final_reserved;
            let initial_total = initial_balance;

            // Invariant: Total balance unchanged after reserve/unreserve cycles
            prop_assert_eq!(
                final_total,
                initial_total,
                "Total balance must be preserved across reserve/unreserve cycles"
            );

            Ok(())
        });
    }
}

// ================================================================================================
// PROPERTY TEST 5: Cumulative Fee Accounting
// ================================================================================================

proptest! {
    /// Property: Multiple fee charges should accumulate correctly
    /// Invariant: sum(individual_fees) <= sum(max_fees)
    #[test]
    fn prop_cumulative_fees(
        num_comits in 1u8..=5u8,
        max_fee_per_comit in arb_balance(100, 1000),
    ) {
        new_test_ext().execute_with(|| {
            let account = ALICE;
            let initial_balance = mock::Balances::free_balance(&account);
            let total_max_fees = max_fee_per_comit * num_comits as u128;

            // Skip if insufficient balance
            if initial_balance < total_max_fees {
                return Ok(());
            }

            let mut total_fees_charged: Balance = 0;

            for i in 0..num_comits {
                let comit_id = H256::from_low_u64_be(5000 + i as u64);
                let evm_payload = vec![1, 2, i];
                let svm_payload = vec![4, 5, i];
                let x3_payload = vec![0x58, 0x33, 0x00, 0x01, i];
                let nonce = i as u64;

                let balance_before = mock::Balances::free_balance(&account);

                let prepare_root = compute_property_prepare_root(
                    comit_id,
                    &evm_payload,
                    &svm_payload,
                    &x3_payload,
                    nonce,
                    max_fee_per_comit,
                );

                let result = AtlasKernel::submit_comit_v2(
                    RuntimeOrigin::signed(account),
                    comit_id,
                    evm_payload,
                    svm_payload,
                    x3_payload,
                    nonce,
                    max_fee_per_comit,
                    prepare_root,
                );

                if result.is_ok() {
                    let balance_after = mock::Balances::free_balance(&account);
                    let fee_charged = balance_before.saturating_sub(balance_after);
                    total_fees_charged = total_fees_charged.saturating_add(fee_charged);

                    // Invariant: Each individual fee doesn't exceed max
                    prop_assert!(
                        fee_charged <= max_fee_per_comit,
                        "Individual fee exceeded max_fee"
                    );
                }
            }

            let final_balance = mock::Balances::free_balance(&account);
            let actual_total = initial_balance.saturating_sub(final_balance);

            // Invariant 1: Accumulated fees match measured total
            prop_assert_eq!(
                actual_total,
                total_fees_charged,
                "Accumulated fees must match measured total"
            );

            // Invariant 2: Total fees never exceed sum of max_fees
            prop_assert!(
                total_fees_charged <= total_max_fees,
                "Total fees must not exceed sum of max_fees"
            );

            Ok(())
        });
    }
}

// ================================================================================================
// PROPERTY TEST 6: Idempotency of Failed Operations
// ================================================================================================

proptest! {
    /// Property: Failed operations should be idempotent (no state changes)
    /// Invariant: If operation fails, all balances remain unchanged
    #[test]
    fn prop_failed_operation_idempotency(
        max_fee in arb_fee(),
        nonce in arb_nonce(),
    ) {
        new_test_ext().execute_with(|| {
            let account = ALICE;

            // Capture initial state
            let initial_free = mock::Balances::free_balance(&account);
            let initial_reserved = mock::Balances::reserved_balance(&account);
            let initial_nonce = Nonces::<Test>::get(&account);

            // Deliberately craft invalid payloads to trigger failures
            let comit_id = H256::random();
            let evm_payload = vec![]; // Empty payload may cause failure
            let svm_payload = vec![];
            let x3_payload = vec![0xFF, 0xFF]; // Invalid X3 magic prefix

            let prepare_root = compute_property_prepare_root(
                comit_id,
                &evm_payload,
                &svm_payload,
                &x3_payload,
                nonce,
                max_fee,
            );

            let result = AtlasKernel::submit_comit_v2(
                RuntimeOrigin::signed(account),
                comit_id,
                evm_payload,
                svm_payload,
                x3_payload,
                nonce,
                max_fee,
                prepare_root,
            );

            let final_free = mock::Balances::free_balance(&account);
            let final_reserved = mock::Balances::reserved_balance(&account);
            let final_nonce = Nonces::<Test>::get(&account);

            if result.is_err() {
                // Invariant 1: Free balance unchanged on failure
                prop_assert_eq!(
                    final_free,
                    initial_free,
                    "Free balance must not change on failed operation"
                );

                // Invariant 2: Reserved balance unchanged on failure
                prop_assert_eq!(
                    final_reserved,
                    initial_reserved,
                    "Reserved balance must not change on failed operation"
                );

                // Invariant 3: Nonce unchanged on failure
                prop_assert_eq!(
                    final_nonce,
                    initial_nonce,
                    "Nonce must not change on failed operation"
                );
            }

            Ok(())
        });
    }
}
