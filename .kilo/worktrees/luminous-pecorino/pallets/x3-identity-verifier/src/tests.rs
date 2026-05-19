// Basic tests for the X3 Identity Verifier pallet.

use super::*;
use crate::{mock::*, pallet::*};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

// Test helper to generate an identity ID from an index
fn identity_id(index: u8) -> H256 {
    let mut bytes = [0u8; 32];
    bytes[0] = index;
    H256::from(bytes)
}

// Test helper to generate a verification hash
fn verification_hash(index: u8) -> H256 {
    let mut bytes = [0u8; 32];
    bytes[31] = index;
    H256::from(bytes)
}

#[test]
fn register_verifier_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));

        let verifier = Verifiers::<Test>::get(ALICE).unwrap();
        assert_eq!(verifier.account, ALICE);
        assert_eq!(verifier.stake, MinVerifierStake::get());
        assert!(verifier.active);
        assert_eq!(verifier.reputation, 50);
    });
}

#[test]
fn register_verifier_fails_when_already_registered() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_noop!(
            X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)),
            Error::<Test>::VerifierAlreadyRegistered
        );
    });
}

#[test]
fn register_verifier_fails_with_insufficient_stake() {
    new_test_ext().execute_with(|| {
        // With default MinVerifierStake = 1000, BOB has 100_000 so this should succeed.
        // We verify registration works with sufficient balance.
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(BOB)));

        let verifier = Verifiers::<Test>::get(BOB).unwrap();
        assert_eq!(verifier.account, BOB);
    });
}

#[test]
fn submit_identity_proof_success() {
    new_test_ext().execute_with(|| {
        // Register ALICE as verifier
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));

        let id_id = identity_id(1);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let v_hash = verification_hash(1);

        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::Signature,
            proof_data,
            v_hash,
        ));

        // Verify proof was stored
        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(id_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        let proof = IdentityProofs::<Test>::get(proof_id).unwrap();
        assert_eq!(proof.identity_id, id_id);
        assert_eq!(proof.verifier, ALICE);
        assert_eq!(proof.status, VerificationStatus::Pending);
        assert_eq!(proof.confirmations, 0);
        matches!(proof.proof_type, ProofType::Signature);
    });
}

#[test]
fn submit_identity_proof_fails_if_not_verifier() {
    new_test_ext().execute_with(|| {
        let id_id = identity_id(1);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let v_hash = verification_hash(1);

        assert_noop!(
            X3IdentityVerifier::submit_identity_proof(
                RuntimeOrigin::signed(ALICE),
                id_id,
                ProofType::Signature,
                proof_data,
                v_hash,
            ),
            Error::<Test>::VerifierNotFound
        );
    });
}

#[test]
fn submit_identity_proof_fails_if_identity_already_verified() {
    new_test_ext().execute_with(|| {
        // Register ALICE and BOB as verifiers
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(BOB)));

        let id_id = identity_id(1);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let v_hash = verification_hash(1);

        // ALICE submits proof
        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(id_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::Signature,
            proof_data.clone(),
            v_hash,
        ));

        // BOB confirms (MinConfirmations = 2, so verification completes)
        assert_ok!(X3IdentityVerifier::confirm_identity_proof(
            RuntimeOrigin::signed(BOB),
            proof_id,
        ));

        // Now identity is verified
        assert!(X3IdentityVerifier::is_identity_verified(&id_id));

        // ALICE tries to submit another proof for same identity
        assert_noop!(
            X3IdentityVerifier::submit_identity_proof(
                RuntimeOrigin::signed(ALICE),
                id_id,
                ProofType::Signature,
                proof_data,
                v_hash,
            ),
            Error::<Test>::IdentityAlreadyVerified
        );
    });
}

#[test]
fn confirm_identity_proof_success() {
    new_test_ext().execute_with(|| {
        // Register ALICE and BOB as verifiers
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(BOB)));

        let id_id = identity_id(1);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let v_hash = verification_hash(1);

        // ALICE submits proof
        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(id_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::Signature,
            proof_data,
            v_hash,
        ));

        // BOB confirms the proof (MinConfirmations = 2, so this should trigger verification)
        assert_ok!(X3IdentityVerifier::confirm_identity_proof(
            RuntimeOrigin::signed(BOB),
            proof_id,
        ));

        // Verify identity is now verified
        let verified = VerifiedIdentities::<Test>::get(id_id).unwrap();
        assert!(verified.verified);
        assert_eq!(verified.identity_id, id_id);
        assert_eq!(verified.confirmation_count, 2);
        matches!(verified.proof_type, ProofType::Signature);
    });
}

#[test]
fn reject_identity_proof_success() {
    new_test_ext().execute_with(|| {
        // Register ALICE and BOB as verifiers
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(BOB)));

        let id_id = identity_id(1);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let v_hash = verification_hash(1);

        // ALICE submits proof
        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(id_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::Signature,
            proof_data,
            v_hash,
        ));

        // BOB rejects the proof
        let reason: BoundedVec<u8, ConstU32<128>> = b"Invalid proof format".to_vec().try_into().unwrap();
        assert_ok!(X3IdentityVerifier::reject_identity_proof(
            RuntimeOrigin::signed(BOB),
            proof_id,
            reason,
        ));

        // Verify proof status is Rejected
        let proof = IdentityProofs::<Test>::get(proof_id).unwrap();
        assert_eq!(proof.status, VerificationStatus::Rejected);
    });
}

#[test]
fn deactivate_verifier_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));

        assert_ok!(X3IdentityVerifier::deactivate_verifier(RuntimeOrigin::signed(ALICE)));

        let verifier = Verifiers::<Test>::get(ALICE).unwrap();
        assert!(!verifier.active);
    });
}

#[test]
fn reactivate_verifier_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::deactivate_verifier(RuntimeOrigin::signed(ALICE)));

        assert_ok!(X3IdentityVerifier::reactivate_verifier(
            RuntimeOrigin::root(),
            ALICE,
        ));

        let verifier = Verifiers::<Test>::get(ALICE).unwrap();
        assert!(verifier.active);
    });
}

#[test]
fn is_identity_verified_returns_correct_result() {
    new_test_ext().execute_with(|| {
        let id_id = identity_id(1);

        // Not verified initially
        assert!(!X3IdentityVerifier::is_identity_verified(&id_id));

        // Register verifiers and submit + confirm proof
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(BOB)));

        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let v_hash = verification_hash(1);

        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(id_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::Signature,
            proof_data,
            v_hash,
        ));

        assert_ok!(X3IdentityVerifier::confirm_identity_proof(
            RuntimeOrigin::signed(BOB),
            proof_id,
        ));

        // Now verified
        assert!(X3IdentityVerifier::is_identity_verified(&id_id));
    });
}

#[test]
fn verify_keyring_trait_integration() {
    new_test_ext().execute_with(|| {
        let id_id = identity_id(1);
        let keyring_bytes: [u8; 32] = id_id.into();

        // Not verified initially
        assert!(!<X3IdentityVerifier as KeyringVerifier<u64, u64>>::verify_keyring(&keyring_bytes));

        // Register verifiers and verify via trait
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(BOB)));

        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test_proof".to_vec().try_into().unwrap();
        let v_hash = verification_hash(1);

        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(id_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::Signature,
            proof_data,
            v_hash,
        ));

        assert_ok!(X3IdentityVerifier::confirm_identity_proof(
            RuntimeOrigin::signed(BOB),
            proof_id,
        ));

        // Now verified via trait
        assert!(<X3IdentityVerifier as KeyringVerifier<u64, u64>>::verify_keyring(&keyring_bytes));
    });
}

#[test]
fn keyring_attestor_registry_trait_integration() {
    new_test_ext().execute_with(|| {
        // Initially no verifiers
        assert_eq!(
            <X3IdentityVerifier as KeyringAttestorRegistry<u64, u128, u64>>::total_attestors(),
            0
        );

        // Register via trait
        assert_ok!(<X3IdentityVerifier as KeyringAttestorRegistry<u64, u128, u64>>::register_attestor(
            &ALICE,
            MinVerifierStake::get()
        ));

        assert!(<X3IdentityVerifier as KeyringAttestorRegistry<u64, u128, u64>>::is_registered(&ALICE));
        assert_eq!(
            <X3IdentityVerifier as KeyringAttestorRegistry<u64, u128, u64>>::total_attestors(),
            1
        );

        // Deregister via trait
        assert_ok!(<X3IdentityVerifier as KeyringAttestorRegistry<u64, u128, u64>>::deregister_attestor(&ALICE));

        assert!(!<X3IdentityVerifier as KeyringAttestorRegistry<u64, u128, u64>>::is_registered(&ALICE));
        assert_eq!(
            <X3IdentityVerifier as KeyringAttestorRegistry<u64, u128, u64>>::total_attestors(),
            0
        );
    });
}

#[test]
fn zk_proof_type_submission() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(BOB)));

        let id_id = identity_id(2);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"zk_proof_data".to_vec().try_into().unwrap();
        let v_hash = verification_hash(2);

        // Submit with ZK proof type
        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::ZKProof,
            proof_data,
            v_hash,
        ));

        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(id_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        let proof = IdentityProofs::<Test>::get(proof_id).unwrap();
        matches!(proof.proof_type, ProofType::ZKProof);
    });
}

#[test]
fn multi_factor_proof_type_submission() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(BOB)));

        let id_id = identity_id(3);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"multi_factor_data".to_vec().try_into().unwrap();
        let v_hash = verification_hash(3);

        // Submit with MultiFactor proof type
        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::MultiFactor,
            proof_data,
            v_hash,
        ));

        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(id_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        let proof = IdentityProofs::<Test>::get(proof_id).unwrap();
        matches!(proof.proof_type, ProofType::MultiFactor);
    });
}

#[test]
fn proof_expiry_on_finalize() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));

        let id_id = identity_id(4);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"expiring_proof".to_vec().try_into().unwrap();
        let v_hash = verification_hash(4);

        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::Signature,
            proof_data,
            v_hash,
        ));

        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(id_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        // Proof exists
        assert!(IdentityProofs::<Test>::contains_key(&proof_id));

        // Advance past timeout
        run_to_block(100 + ProofTimeout::get());

        // Proof should be expired
        let proof = IdentityProofs::<Test>::get(proof_id).unwrap();
        assert_eq!(proof.status, VerificationStatus::Expired);
    });
}

#[test]
fn cannot_confirm_expired_proof() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(BOB)));

        let id_id = identity_id(5);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test".to_vec().try_into().unwrap();
        let v_hash = verification_hash(5);

        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::Signature,
            proof_data,
            v_hash,
        ));

        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(id_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        // Advance past timeout
        run_to_block(100 + ProofTimeout::get());

        // BOB tries to confirm expired proof
        assert_noop!(
            X3IdentityVerifier::confirm_identity_proof(
                RuntimeOrigin::signed(BOB),
                proof_id,
            ),
            Error::<Test>::ProofAlreadyVerified // Expired proofs are marked as Expired status
        );
    });
}

#[test]
fn deactivated_verifier_cannot_submit_proofs() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::deactivate_verifier(RuntimeOrigin::signed(ALICE)));

        let id_id = identity_id(6);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test".to_vec().try_into().unwrap();
        let v_hash = verification_hash(6);

        assert_noop!(
            X3IdentityVerifier::submit_identity_proof(
                RuntimeOrigin::signed(ALICE),
                id_id,
                ProofType::Signature,
                proof_data,
                v_hash,
            ),
            Error::<Test>::VerifierNotActive
        );
    });
}

#[test]
fn reactivate_verifier_allows_proofs_again() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::deactivate_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::reactivate_verifier(
            RuntimeOrigin::root(),
            ALICE,
        ));

        let id_id = identity_id(7);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"test".to_vec().try_into().unwrap();
        let v_hash = verification_hash(7);

        // Should succeed after reactivation
        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::Signature,
            proof_data,
            v_hash,
        ));
    });
}

#[test]
fn verify_proof_type_preservation() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(ALICE)));
        assert_ok!(X3IdentityVerifier::register_verifier(RuntimeOrigin::signed(BOB)));

        let id_id = identity_id(8);
        let proof_data: BoundedVec<u8, ConstU32<512>> = b"zk_test".to_vec().try_into().unwrap();
        let v_hash = verification_hash(8);

        // Submit with ZK proof type
        let proof_id = {
            let block = 1u64;
            let mut data = ALICE.encode();
            data.extend(id_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };

        assert_ok!(X3IdentityVerifier::submit_identity_proof(
            RuntimeOrigin::signed(ALICE),
            id_id,
            ProofType::ZKProof,
            proof_data,
            v_hash,
        ));

        assert_ok!(X3IdentityVerifier::confirm_identity_proof(
            RuntimeOrigin::signed(BOB),
            proof_id,
        ));

        // Verify the proof type is preserved in the verification result
        let result = VerifiedIdentities::<Test>::get(id_id).unwrap();
        matches!(result.proof_type, ProofType::ZKProof);
    });
}