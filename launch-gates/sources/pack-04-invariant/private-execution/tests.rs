//! Unit tests for the Private Execution pallet.
//!
//! # Invariants tested:
//! - PRIV-EXEC-004: Attestation verified before joining confidential set
//! - PRIV-EXEC-005: Premium fee correctly collected and split

use crate::{mock::*, types::*, Error, Event};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

fn dummy_attestation() -> Vec<u8> {
    // Non-empty = passes simplified verification
    b"NVIDIA-CC-ATTESTATION-REPORT-V1-MOCK".to_vec()
}

fn dummy_enclave_key() -> [u8; 32] {
    [0xAA; 32]
}

// ──────────────────────────────────────────────────────────────
// Validator Registration
// ──────────────────────────────────────────────────────────────

/// # Invariant: PRIV-EXEC-004
#[test]
fn reject_unattested() {
    new_test_ext().execute_with(|| {
        // Enable private execution
        assert_ok!(PrivateExecution::set_enabled(RuntimeOrigin::root(), true));

        // Empty attestation should fail
        assert_noop!(
            PrivateExecution::register_confidential_validator(
                RuntimeOrigin::signed(1),
                b"NVIDIA H100".to_vec(),
                vec![], // empty = invalid
                dummy_enclave_key(),
            ),
            Error::<Test>::InvalidAttestation
        );

        // Valid attestation should succeed
        assert_ok!(PrivateExecution::register_confidential_validator(
            RuntimeOrigin::signed(1),
            b"NVIDIA H100".to_vec(),
            dummy_attestation(),
            dummy_enclave_key(),
        ));

        let att = PrivateExecution::confidential_validators(1).unwrap();
        assert_eq!(att.status, EnclaveStatus::Verified);
        assert_eq!(PrivateExecution::confidential_validator_count(), 1);
    });
}

#[test]
fn register_multiple_validators() {
    new_test_ext().execute_with(|| {
        assert_ok!(PrivateExecution::set_enabled(RuntimeOrigin::root(), true));

        for i in 1..=3u64 {
            assert_ok!(PrivateExecution::register_confidential_validator(
                RuntimeOrigin::signed(i),
                b"NVIDIA H100".to_vec(),
                dummy_attestation(),
                [i as u8; 32],
            ));
        }

        assert_eq!(PrivateExecution::confidential_validator_count(), 3);
    });
}

#[test]
fn deregister_validator() {
    new_test_ext().execute_with(|| {
        assert_ok!(PrivateExecution::set_enabled(RuntimeOrigin::root(), true));

        assert_ok!(PrivateExecution::register_confidential_validator(
            RuntimeOrigin::signed(1),
            b"NVIDIA H100".to_vec(),
            dummy_attestation(),
            dummy_enclave_key(),
        ));

        assert_ok!(PrivateExecution::deregister_confidential_validator(
            RuntimeOrigin::signed(1)
        ));

        assert!(PrivateExecution::confidential_validators(1).is_none());
        assert_eq!(PrivateExecution::confidential_validator_count(), 0);
    });
}

// ──────────────────────────────────────────────────────────────
// Private Transaction Submission
// ──────────────────────────────────────────────────────────────

fn setup_quorum() {
    assert_ok!(PrivateExecution::set_enabled(RuntimeOrigin::root(), true));

    // Register 2 validators (MinConfidentialQuorum = 2)
    for i in 1..=2u64 {
        assert_ok!(PrivateExecution::register_confidential_validator(
            RuntimeOrigin::signed(i),
            b"NVIDIA H100".to_vec(),
            dummy_attestation(),
            [i as u8; 32],
        ));
    }

    // Set DKG committee key
    assert_ok!(PrivateExecution::set_committee_key(
        RuntimeOrigin::root(),
        vec![0xBB; 32],
    ));
}

#[test]
fn submit_private_tx_requires_quorum() {
    new_test_ext().execute_with(|| {
        assert_ok!(PrivateExecution::set_enabled(RuntimeOrigin::root(), true));

        // No validators registered yet
        assert_noop!(
            PrivateExecution::submit_private_transaction(
                RuntimeOrigin::signed(10),
                H256::repeat_byte(0x01),
                vec![0xCA; 256],
                H256::repeat_byte(0x02),
                1_000u128,
            ),
            Error::<Test>::InsufficientQuorum
        );
    });
}

/// # Invariant: PRIV-EXEC-005
#[test]
fn fee_premium_accounting() {
    new_test_ext().execute_with(|| {
        setup_quorum();

        let user_balance_before = Balances::free_balance(10);
        let base_fee: u128 = 10_000;

        assert_ok!(PrivateExecution::submit_private_transaction(
            RuntimeOrigin::signed(10),
            H256::repeat_byte(0x01),
            vec![0xCA; 256],
            H256::repeat_byte(0x02),
            base_fee,
        ));

        // Premium = 1.5% of 10_000 = 150
        // Total fee = 10_000 + 150 = 10_150
        let user_balance_after = Balances::free_balance(10);
        let fee_charged = user_balance_before - user_balance_after;
        assert_eq!(fee_charged, 10_150);

        // Premium tracked
        assert_eq!(PrivateExecution::total_premium_fees(), 150);

        // TX recorded
        let record = PrivateExecution::private_transactions(H256::repeat_byte(0x01)).unwrap();
        assert_eq!(record.status, PrivateTxStatus::Pending);
        assert_eq!(record.fee_paid, 10_150);
    });
}

// ──────────────────────────────────────────────────────────────
// State Diff Commitment
// ──────────────────────────────────────────────────────────────

#[test]
fn commit_state_diff_works() {
    new_test_ext().execute_with(|| {
        setup_quorum();

        let tx_hash = H256::repeat_byte(0x01);

        // Submit private TX
        assert_ok!(PrivateExecution::submit_private_transaction(
            RuntimeOrigin::signed(10),
            tx_hash,
            vec![0xCA; 256],
            H256::repeat_byte(0x02),
            1_000u128,
        ));

        // Validator 1 commits state diff
        assert_ok!(PrivateExecution::commit_encrypted_state_diff(
            RuntimeOrigin::signed(1),
            tx_hash,
            vec![0xDE; 128],         // encrypted state changes
            H256::repeat_byte(0x03), // commitment
            None,                    // no ZK proof
            [0x51; 64],              // enclave signature (placeholder)
        ));

        // TX status updated
        let record = PrivateExecution::private_transactions(tx_hash).unwrap();
        assert_eq!(record.status, PrivateTxStatus::Committed);
        assert_eq!(record.executed_by, Some(1));

        // State diff stored
        let diffs = PrivateExecution::encrypted_state_diffs(1); // block 1
        assert_eq!(diffs.len(), 1);
    });
}

// ──────────────────────────────────────────────────────────────
// DKG Key Rotation
// ──────────────────────────────────────────────────────────────

#[test]
fn dkg_key_rotation() {
    new_test_ext().execute_with(|| {
        assert_ok!(PrivateExecution::set_committee_key(
            RuntimeOrigin::root(),
            vec![0xAA; 32],
        ));
        assert_eq!(PrivateExecution::dkg_epoch(), 1);

        assert_ok!(PrivateExecution::set_committee_key(
            RuntimeOrigin::root(),
            vec![0xBB; 32],
        ));
        assert_eq!(PrivateExecution::dkg_epoch(), 2);
    });
}

// ──────────────────────────────────────────────────────────────
// Enable/Disable Toggle
// ──────────────────────────────────────────────────────────────

#[test]
fn toggle_private_execution() {
    new_test_ext().execute_with(|| {
        assert!(!PrivateExecution::is_enabled());

        assert_ok!(PrivateExecution::set_enabled(RuntimeOrigin::root(), true));
        assert!(PrivateExecution::is_enabled());

        assert_ok!(PrivateExecution::set_enabled(RuntimeOrigin::root(), false));
        assert!(!PrivateExecution::is_enabled());
    });
}
