//! Unit tests for pallet-x3-treasury-policy.
//!
//! ## Coverage
//!
//! 1. `fund_settlement_vault` succeeds within cap and updates the deployed tracker.
//! 2. `fund_settlement_vault` above cap returns `AllocationCapExceeded`.
//! 3. `fund_settlement_vault` above operator threshold queues a pending action
//!    and emits `GovernanceApprovalRequired` without touching balances.
//! 4. `approve_governance_action` applies the deferred funding and clears the
//!    pending entry.
//! 5. `fund_settlement_vault` rejects an `InsuranceLoss`-typed vault with
//!    `InsuranceReserveUnreachable`.
//! 6. `withdraw_insurance_reserve` is callable only by the governance origin.

use crate::{
    pallet::{
        AllocationCaps, Error, InsuranceReserveBalance, OperatorFundingThreshold,
        PendingGovernanceActions, TotalDeployedSettlementFloat, TreasuryDeployedByLaneClass,
    },
    AllocationCapKey, Balance,
};
use frame_support::{assert_noop, assert_ok};
use pallet_x3_inventory::types::{LaneClass, OwnerType, VaultType};
use sp_runtime::DispatchError;

use super::mock::*;

// ---------------------------------------------------------------------------
// Shared test constants
// ---------------------------------------------------------------------------

const CHAIN_ID: u32 = 1;
const ASSET_ID: u32 = 100;

/// A vault that will be registered as `SettlementFloat`.
const VAULT_A: [u8; 32] = [0xAAu8; 32];

/// A vault that will be registered as `InsuranceLoss`.
const VAULT_INS: [u8; 32] = [0xBBu8; 32];

/// Allocation cap used in several tests.
const CAP: Balance = 1_000_000;

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

/// Create a `SettlementFloat` vault in the inventory pallet.
fn setup_settlement_vault() {
    assert_ok!(pallet_x3_inventory::pallet::Pallet::<Test>::create_vault(
        RuntimeOrigin::root(),
        VAULT_A,
        VaultType::SettlementFloat,
        OwnerType::Treasury,
        CHAIN_ID,
        ASSET_ID,
        0u128,         // critical_min
        0u128,         // min_band
        500_000u128,   // target_band
        2_000_000u128, // max_band
    ));
}

/// Create an `InsuranceLoss` vault in the inventory pallet.
fn setup_insurance_loss_vault() {
    assert_ok!(pallet_x3_inventory::pallet::Pallet::<Test>::create_vault(
        RuntimeOrigin::root(),
        VAULT_INS,
        VaultType::InsuranceLoss,
        OwnerType::Treasury,
        CHAIN_ID,
        ASSET_ID,
        0u128,
        0u128,
        500_000u128,
        2_000_000u128,
    ));
}

/// Set an allocation cap for the given lane class.
fn set_cap(cap: Balance, lane: LaneClass) {
    let key = AllocationCapKey {
        chain_id: CHAIN_ID,
        asset_id: ASSET_ID,
        lane_class: lane,
    };
    assert_ok!(X3TreasuryPolicy::set_allocation_cap(
        RuntimeOrigin::root(),
        key,
        cap
    ));
}

/// Set the operator funding threshold (above which governance is required).
fn set_threshold(threshold: Balance) {
    assert_ok!(X3TreasuryPolicy::set_operator_funding_threshold(
        RuntimeOrigin::root(),
        threshold
    ));
}

// ---------------------------------------------------------------------------
// Test 1: successful in-cap funding updates the deployment ledger
// ---------------------------------------------------------------------------

#[test]
fn fund_settlement_vault_within_cap_succeeds_and_updates_trackers() {
    new_test_ext().execute_with(|| {
        setup_settlement_vault();
        set_cap(CAP, LaneClass::A);
        // Threshold well above our amount — no governance deferral.
        set_threshold(1_000_000_000u128);

        let amount: Balance = 500_000;
        assert_ok!(X3TreasuryPolicy::fund_settlement_vault(
            RuntimeOrigin::signed(OPERATOR),
            VAULT_A,
            amount,
            LaneClass::A,
            CHAIN_ID,
            ASSET_ID,
        ));

        // Lane-class tracker updated.
        assert_eq!(
            TreasuryDeployedByLaneClass::<Test>::get(&LaneClass::A),
            amount
        );
        // Global settlement float tracker updated.
        assert_eq!(TotalDeployedSettlementFloat::<Test>::get(), amount);
        // No pending action should exist.
        assert!(PendingGovernanceActions::<Test>::get(VAULT_A).is_none());
    });
}

// ---------------------------------------------------------------------------
// Test 2: funding above cap is rejected
// ---------------------------------------------------------------------------

#[test]
fn fund_settlement_vault_above_cap_returns_allocation_cap_exceeded() {
    new_test_ext().execute_with(|| {
        setup_settlement_vault();
        set_cap(CAP, LaneClass::A);
        set_threshold(1_000_000_000u128);

        // Exactly one unit above the cap.
        assert_noop!(
            X3TreasuryPolicy::fund_settlement_vault(
                RuntimeOrigin::signed(OPERATOR),
                VAULT_A,
                CAP + 1,
                LaneClass::A,
                CHAIN_ID,
                ASSET_ID,
            ),
            Error::<Test>::AllocationCapExceeded
        );

        // Balances must be untouched.
        assert_eq!(TreasuryDeployedByLaneClass::<Test>::get(&LaneClass::A), 0);
        assert_eq!(TotalDeployedSettlementFloat::<Test>::get(), 0);
    });
}

/// Cumulative deployment also triggers the cap on a second call.
#[test]
fn fund_settlement_vault_cumulative_deployment_triggers_cap() {
    new_test_ext().execute_with(|| {
        setup_settlement_vault();
        set_cap(CAP, LaneClass::B);
        set_threshold(1_000_000_000u128);

        // First call consumes 800_000.
        assert_ok!(X3TreasuryPolicy::fund_settlement_vault(
            RuntimeOrigin::signed(OPERATOR),
            VAULT_A,
            800_000,
            LaneClass::B,
            CHAIN_ID,
            ASSET_ID,
        ));

        // Second call of 300_000 would push deployed to 1_100_000 > CAP (1_000_000).
        assert_noop!(
            X3TreasuryPolicy::fund_settlement_vault(
                RuntimeOrigin::signed(OPERATOR),
                VAULT_A,
                300_000,
                LaneClass::B,
                CHAIN_ID,
                ASSET_ID,
            ),
            Error::<Test>::AllocationCapExceeded
        );
    });
}

// ---------------------------------------------------------------------------
// Test 3: funding above operator threshold queues a pending action
// ---------------------------------------------------------------------------

#[test]
fn fund_settlement_vault_above_threshold_defers_to_governance() {
    new_test_ext().execute_with(|| {
        setup_settlement_vault();
        set_cap(CAP, LaneClass::A);
        // Threshold is 100; our amount is 500_000 — well above.
        set_threshold(100u128);

        let amount: Balance = 500_000;
        assert_ok!(X3TreasuryPolicy::fund_settlement_vault(
            RuntimeOrigin::signed(OPERATOR),
            VAULT_A,
            amount,
            LaneClass::A,
            CHAIN_ID,
            ASSET_ID,
        ));

        // Balances must NOT be updated — action is deferred.
        assert_eq!(TreasuryDeployedByLaneClass::<Test>::get(&LaneClass::A), 0);
        assert_eq!(TotalDeployedSettlementFloat::<Test>::get(), 0);

        // Pending action must be stored with correct fields.
        let pending = PendingGovernanceActions::<Test>::get(VAULT_A);
        assert!(pending.is_some(), "pending action must exist");
        let (pending_amount, pending_lane, submitted_block) = pending.unwrap();
        assert_eq!(pending_amount, amount);
        assert_eq!(pending_lane, LaneClass::A);
        assert_eq!(submitted_block, 1u64); // block_number was set to 1 in new_test_ext
    });
}

// ---------------------------------------------------------------------------
// Test 4: approve_governance_action applies the deferred funding
// ---------------------------------------------------------------------------

#[test]
fn approve_governance_action_applies_deferred_funding() {
    new_test_ext().execute_with(|| {
        setup_settlement_vault();
        set_cap(CAP, LaneClass::C);
        set_threshold(100u128);

        let amount: Balance = 200_000;
        // Queue the pending action.
        assert_ok!(X3TreasuryPolicy::fund_settlement_vault(
            RuntimeOrigin::signed(OPERATOR),
            VAULT_A,
            amount,
            LaneClass::C,
            CHAIN_ID,
            ASSET_ID,
        ));
        assert!(PendingGovernanceActions::<Test>::get(VAULT_A).is_some());

        // Governance approves.
        assert_ok!(X3TreasuryPolicy::approve_governance_action(
            RuntimeOrigin::root(),
            VAULT_A
        ));

        // Balances now updated.
        assert_eq!(
            TreasuryDeployedByLaneClass::<Test>::get(&LaneClass::C),
            amount
        );
        assert_eq!(TotalDeployedSettlementFloat::<Test>::get(), amount);

        // Pending action removed.
        assert!(
            PendingGovernanceActions::<Test>::get(VAULT_A).is_none(),
            "pending action must be cleared after approval"
        );
    });
}

/// Attempting to approve a vault with no pending action returns `NoPendingAction`.
#[test]
fn approve_governance_action_no_pending_action_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3TreasuryPolicy::approve_governance_action(RuntimeOrigin::root(), VAULT_A),
            Error::<Test>::NoPendingAction
        );
    });
}

/// Reject removes the pending action without applying balance changes.
#[test]
fn reject_governance_action_discards_pending_action() {
    new_test_ext().execute_with(|| {
        setup_settlement_vault();
        set_cap(CAP, LaneClass::A);
        set_threshold(100u128);

        assert_ok!(X3TreasuryPolicy::fund_settlement_vault(
            RuntimeOrigin::signed(OPERATOR),
            VAULT_A,
            50_000,
            LaneClass::A,
            CHAIN_ID,
            ASSET_ID,
        ));

        assert_ok!(X3TreasuryPolicy::reject_governance_action(
            RuntimeOrigin::root(),
            VAULT_A
        ));

        // Pending action removed; balances untouched.
        assert!(PendingGovernanceActions::<Test>::get(VAULT_A).is_none());
        assert_eq!(TreasuryDeployedByLaneClass::<Test>::get(&LaneClass::A), 0);
        assert_eq!(TotalDeployedSettlementFloat::<Test>::get(), 0);
    });
}

// ---------------------------------------------------------------------------
// Test 5: InsuranceLoss vault is rejected by fund_settlement_vault
// ---------------------------------------------------------------------------

#[test]
fn fund_settlement_vault_rejects_insurance_loss_vault_type() {
    new_test_ext().execute_with(|| {
        setup_insurance_loss_vault();
        set_cap(CAP, LaneClass::C);
        set_threshold(1_000_000_000u128);

        assert_noop!(
            X3TreasuryPolicy::fund_settlement_vault(
                RuntimeOrigin::signed(OPERATOR),
                VAULT_INS,
                100u128,
                LaneClass::C,
                CHAIN_ID,
                ASSET_ID,
            ),
            Error::<Test>::InsuranceReserveUnreachable
        );
    });
}

/// A non-existent vault also returns `VaultNotFound` (not panics).
#[test]
fn fund_settlement_vault_nonexistent_vault_returns_vault_not_found() {
    new_test_ext().execute_with(|| {
        let unknown_vault = [0xFFu8; 32];
        set_cap(CAP, LaneClass::A);
        set_threshold(1_000_000_000u128);

        assert_noop!(
            X3TreasuryPolicy::fund_settlement_vault(
                RuntimeOrigin::signed(OPERATOR),
                unknown_vault,
                100u128,
                LaneClass::A,
                CHAIN_ID,
                ASSET_ID,
            ),
            Error::<Test>::VaultNotFound
        );
    });
}

// ---------------------------------------------------------------------------
// Test 6: withdraw_insurance_reserve requires governance origin
// ---------------------------------------------------------------------------

#[test]
fn withdraw_insurance_reserve_requires_governance_origin() {
    new_test_ext().execute_with(|| {
        // Deposit some funds as governance.
        assert_ok!(X3TreasuryPolicy::deposit_insurance_reserve(
            RuntimeOrigin::root(),
            100_000u128
        ));
        assert_eq!(InsuranceReserveBalance::<Test>::get(), 100_000u128);

        // Operator (signed) cannot withdraw.
        assert_noop!(
            X3TreasuryPolicy::withdraw_insurance_reserve(
                RuntimeOrigin::signed(OPERATOR),
                50_000u128
            ),
            DispatchError::BadOrigin
        );
        // Balance unchanged.
        assert_eq!(InsuranceReserveBalance::<Test>::get(), 100_000u128);

        // Governance (root) can withdraw.
        assert_ok!(X3TreasuryPolicy::withdraw_insurance_reserve(
            RuntimeOrigin::root(),
            50_000u128
        ));
        assert_eq!(InsuranceReserveBalance::<Test>::get(), 50_000u128);
    });
}

/// Operator cannot deposit into the insurance reserve either.
#[test]
fn deposit_insurance_reserve_requires_governance_origin() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3TreasuryPolicy::deposit_insurance_reserve(RuntimeOrigin::signed(OPERATOR), 100u128),
            DispatchError::BadOrigin
        );
    });
}

/// Depositing beyond `MaxInsuranceReserve` is rejected.
#[test]
fn deposit_insurance_reserve_at_max_returns_error() {
    new_test_ext().execute_with(|| {
        let max = MaxInsuranceReserve::get();

        // Deposit exactly at the max — must succeed.
        assert_ok!(X3TreasuryPolicy::deposit_insurance_reserve(
            RuntimeOrigin::root(),
            max
        ));
        assert_eq!(InsuranceReserveBalance::<Test>::get(), max);

        // One more unit exceeds the cap.
        assert_noop!(
            X3TreasuryPolicy::deposit_insurance_reserve(RuntimeOrigin::root(), 1u128),
            Error::<Test>::InsuranceReserveAtMax
        );
    });
}

/// Withdrawing more than the reserve balance is rejected.
#[test]
fn withdraw_insurance_reserve_insufficient_balance_returns_error() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3TreasuryPolicy::deposit_insurance_reserve(
            RuntimeOrigin::root(),
            1_000u128
        ));
        assert_noop!(
            X3TreasuryPolicy::withdraw_insurance_reserve(RuntimeOrigin::root(), 1_001u128),
            Error::<Test>::InsuranceReserveInsufficient
        );
    });
}

// ---------------------------------------------------------------------------
// Additional coverage: AllocationCapNotSet
// ---------------------------------------------------------------------------

#[test]
fn fund_settlement_vault_without_cap_returns_allocation_cap_not_set() {
    new_test_ext().execute_with(|| {
        setup_settlement_vault();
        set_threshold(1_000_000_000u128);
        // Deliberately do NOT call set_cap.

        assert_noop!(
            X3TreasuryPolicy::fund_settlement_vault(
                RuntimeOrigin::signed(OPERATOR),
                VAULT_A,
                100u128,
                LaneClass::A,
                CHAIN_ID,
                ASSET_ID,
            ),
            Error::<Test>::AllocationCapNotSet
        );
    });
}

// ---------------------------------------------------------------------------
// Withdraw from vault (operator ledger)
// ---------------------------------------------------------------------------

#[test]
fn withdraw_from_vault_reduces_deployed_trackers() {
    new_test_ext().execute_with(|| {
        setup_settlement_vault();
        set_cap(CAP, LaneClass::B);
        set_threshold(1_000_000_000u128);

        let deposit: Balance = 400_000;
        assert_ok!(X3TreasuryPolicy::fund_settlement_vault(
            RuntimeOrigin::signed(OPERATOR),
            VAULT_A,
            deposit,
            LaneClass::B,
            CHAIN_ID,
            ASSET_ID,
        ));

        let withdraw: Balance = 100_000;
        assert_ok!(X3TreasuryPolicy::withdraw_from_vault(
            RuntimeOrigin::signed(OPERATOR),
            VAULT_A,
            withdraw,
            LaneClass::B,
        ));

        assert_eq!(
            TreasuryDeployedByLaneClass::<Test>::get(&LaneClass::B),
            deposit - withdraw
        );
        assert_eq!(
            TotalDeployedSettlementFloat::<Test>::get(),
            deposit - withdraw
        );
    });
}

/// Withdrawing more than deployed saturates to zero rather than panicking.
#[test]
fn withdraw_from_vault_saturates_at_zero() {
    new_test_ext().execute_with(|| {
        // No prior funding; withdrawal still succeeds at the accounting level.
        assert_ok!(X3TreasuryPolicy::withdraw_from_vault(
            RuntimeOrigin::signed(OPERATOR),
            VAULT_A,
            999_999u128,
            LaneClass::C,
        ));
        assert_eq!(TreasuryDeployedByLaneClass::<Test>::get(&LaneClass::C), 0);
        assert_eq!(TotalDeployedSettlementFloat::<Test>::get(), 0);
    });
}

// ---------------------------------------------------------------------------
// remaining_capacity helper
// ---------------------------------------------------------------------------

#[test]
fn remaining_capacity_returns_none_when_no_cap_set() {
    new_test_ext().execute_with(|| {
        let remaining = crate::remaining_capacity::<Test>(CHAIN_ID, ASSET_ID, LaneClass::A);
        assert!(remaining.is_none());
    });
}

#[test]
fn remaining_capacity_returns_correct_value_after_funding() {
    new_test_ext().execute_with(|| {
        setup_settlement_vault();
        set_cap(CAP, LaneClass::A);
        set_threshold(1_000_000_000u128);

        assert_ok!(X3TreasuryPolicy::fund_settlement_vault(
            RuntimeOrigin::signed(OPERATOR),
            VAULT_A,
            200_000,
            LaneClass::A,
            CHAIN_ID,
            ASSET_ID,
        ));

        let remaining = crate::remaining_capacity::<Test>(CHAIN_ID, ASSET_ID, LaneClass::A);
        assert_eq!(remaining, Some(CAP - 200_000));
    });
}

// ---------------------------------------------------------------------------
// AllocationCaps direct read
// ---------------------------------------------------------------------------

#[test]
fn set_allocation_cap_stores_correct_value() {
    new_test_ext().execute_with(|| {
        let key = AllocationCapKey {
            chain_id: CHAIN_ID,
            asset_id: ASSET_ID,
            lane_class: LaneClass::B,
        };
        assert_ok!(X3TreasuryPolicy::set_allocation_cap(
            RuntimeOrigin::root(),
            key.clone(),
            999_999u128
        ));
        assert_eq!(AllocationCaps::<Test>::get(&key), Some(999_999u128));
    });
}

/// `set_allocation_cap` requires governance — operator call must fail.
#[test]
fn set_allocation_cap_requires_governance_origin() {
    new_test_ext().execute_with(|| {
        let key = AllocationCapKey {
            chain_id: CHAIN_ID,
            asset_id: ASSET_ID,
            lane_class: LaneClass::A,
        };
        assert_noop!(
            X3TreasuryPolicy::set_allocation_cap(RuntimeOrigin::signed(OPERATOR), key, 500_000u128),
            DispatchError::BadOrigin
        );
    });
}

// ---------------------------------------------------------------------------
// OperatorFundingThreshold direct read
// ---------------------------------------------------------------------------

#[test]
fn set_operator_funding_threshold_stores_correct_value() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3TreasuryPolicy::set_operator_funding_threshold(
            RuntimeOrigin::root(),
            12_345u128
        ));
        assert_eq!(OperatorFundingThreshold::<Test>::get(), 12_345u128);
    });
}
