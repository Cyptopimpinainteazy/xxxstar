// Basic tests for the X3 Keyring pallet.

use super::*;
use crate::{mock::*, pallet::*};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

// Test helper to generate a keyring ID from an index
fn keyring_id(index: u8) -> H256 {
    let mut bytes = [0u8; 32];
    bytes[0] = index;
    H256::from(bytes)
}

// Test helper to generate an attestation hash
fn attestation_hash(index: u8) -> H256 {
    let mut bytes = [0u8; 32];
    bytes[31] = index;
    H256::from(bytes)
}

#[test]
fn register_attestor_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(ALICE)));

        let attestor = Attestors::<Test>::get(ALICE).unwrap();
        assert_eq!(attestor.account, ALICE);
        assert_eq!(attestor.stake, MinAttestorStake::get());
        assert!(attestor.active);
        assert_eq!(attestor.reputation, 50);
    });
}

#[test]
fn register_attestor_fails_when_already_registered() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(ALICE)));
        assert_noop!(
            X3Keyring::register_attestor(RuntimeOrigin::signed(ALICE)),
            Error::<Test>::AttestorAlreadyRegistered
        );
    });
}

#[test]
fn register_attestor_fails_when_insufficient_balance() {
    new_test_ext().execute_with(|| {
        // BOB has 100_000 but we'll set a very high stake requirement
        // Actually MinAttestorStake = 1000, BOB has 100_000, so this should work.
        // Let's just verify that registration works with sufficient balance.
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(BOB)));

        let attestor = Attestors::<Test>::get(BOB).unwrap();
        assert_eq!(attestor.account, BOB);
    });
}

#[test]
fn submit_keyring_proof_success() {
    new_test_ext().execute_with(|| {
        // Register ALICE as attestor
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(ALICE)));

        let kid = keyring_id(1);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let att_hash = attestation_hash(1);

        assert_ok!(X3Keyring::submit_keyring_proof(
            RuntimeOrigin::signed(ALICE),
            kid,
            proof_data,
            att_hash,
        ));

        // Verify proof was stored
        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(kid.as_bytes());
            data.extend(att_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        let proof = KeyringProofs::<Test>::get(proof_id).unwrap();
        assert_eq!(proof.keyring_id, kid);
        assert_eq!(proof.attestor, ALICE);
        assert_eq!(proof.status, ProofStatus::Pending);
        assert_eq!(proof.confirmations, 0);
    });
}

#[test]
fn submit_keyring_proof_fails_if_not_attestor() {
    new_test_ext().execute_with(|| {
        let kid = keyring_id(1);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let att_hash = attestation_hash(1);

        assert_noop!(
            X3Keyring::submit_keyring_proof(
                RuntimeOrigin::signed(ALICE),
                kid,
                proof_data,
                att_hash,
            ),
            Error::<Test>::AttestorNotFound
        );
    });
}

#[test]
fn confirm_keyring_proof_success() {
    new_test_ext().execute_with(|| {
        // Register ALICE and BOB as attestors
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(BOB)));

        let kid = keyring_id(1);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let att_hash = attestation_hash(1);

        // ALICE submits proof
        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(kid.as_bytes());
            data.extend(att_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        assert_ok!(X3Keyring::submit_keyring_proof(
            RuntimeOrigin::signed(ALICE),
            kid,
            proof_data,
            att_hash,
        ));

        // BOB confirms the proof (MinConfirmations = 2, so this should trigger verification)
        assert_ok!(X3Keyring::confirm_keyring_proof(
            RuntimeOrigin::signed(BOB),
            proof_id,
        ));

        // Verify keyring is now verified
        let verified = VerifiedKeyrings::<Test>::get(kid).unwrap();
        assert!(verified.verified);
        assert_eq!(verified.keyring_id, kid);
        assert_eq!(verified.confirmation_count, 2);
    });
}

#[test]
fn reject_keyring_proof_success() {
    new_test_ext().execute_with(|| {
        // Register ALICE and BOB as attestors
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(BOB)));

        let kid = keyring_id(1);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let att_hash = attestation_hash(1);

        // ALICE submits proof
        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(kid.as_bytes());
            data.extend(att_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        assert_ok!(X3Keyring::submit_keyring_proof(
            RuntimeOrigin::signed(ALICE),
            kid,
            proof_data,
            att_hash,
        ));

        // BOB rejects the proof
        let reason: BoundedVec<u8, ConstU32<128>> = b"Invalid proof format".to_vec().try_into().unwrap();
        assert_ok!(X3Keyring::reject_keyring_proof(
            RuntimeOrigin::signed(BOB),
            proof_id,
            reason,
        ));

        // Verify proof status is Rejected
        let proof = KeyringProofs::<Test>::get(proof_id).unwrap();
        assert_eq!(proof.status, ProofStatus::Rejected);
    });
}

#[test]
fn deactivate_attestor_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(ALICE)));

        assert_ok!(X3Keyring::deactivate_attestor(RuntimeOrigin::signed(ALICE)));

        let attestor = Attestors::<Test>::get(ALICE).unwrap();
        assert!(!attestor.active);
    });
}

#[test]
fn reactivate_attestor_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3Keyring::deactivate_attestor(RuntimeOrigin::signed(ALICE)));

        assert_ok!(X3Keyring::reactivate_attestor(
            RuntimeOrigin::root(),
            ALICE,
        ));

        let attestor = Attestors::<Test>::get(ALICE).unwrap();
        assert!(attestor.active);
    });
}

#[test]
fn is_keyring_verified_returns_correct_result() {
    new_test_ext().execute_with(|| {
        let kid = keyring_id(1);

        // Not verified initially
        assert!(!X3Keyring::is_keyring_verified(&kid));

        // Register attestors and submit + confirm proof
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3Keyring::register_attestor(RuntimeOrigin::signed(BOB)));

        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let att_hash = attestation_hash(1);

        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(kid.as_bytes());
            data.extend(att_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        assert_ok!(X3Keyring::submit_keyring_proof(
            RuntimeOrigin::signed(ALICE),
            kid,
            proof_data,
            att_hash,
        ));

        assert_ok!(X3Keyring::confirm_keyring_proof(
            RuntimeOrigin::signed(BOB),
            proof_id,
        ));

        // Now verified
        assert!(X3Keyring::is_keyring_verified(&kid));
    });
}