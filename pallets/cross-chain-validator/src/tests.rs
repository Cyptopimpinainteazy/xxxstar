use crate::mock::*;
use crate::LastEvmHeader;
use frame_support::{assert_err, assert_noop, assert_ok};
use sp_core::H256;
use sp_runtime::BuildStorage;

// ---------------------------------------------------------------------------
// Helpers
//
// `AdminOrigin = EnsureRoot<u64>` in mock.rs, so the authorized-submitter set is
// managed through `RuntimeOrigin::root()`.  Header submission itself is gated by
// membership in `AuthorizedSubmitters` (C01 remediation): an account must be
// enrolled before it can submit any EVM/SVM header.
// ---------------------------------------------------------------------------

/// Enroll `who` as an authorized header submitter (governance/Root action).
fn enroll(who: u64) {
    assert_ok!(crate::Pallet::<MockRuntime>::set_authorized_submitters(
        RuntimeOrigin::root(),
        vec![who],
    ));
}

/// A 32-byte non-zero Merkle leaf for a valid single-leaf EVM proof. For a
/// single-leaf tree the pallet's `merkle_root_of` returns the leaf itself, so the
/// claimed root must equal `H256::from(&[byte; 32])` for the proof to verify.
fn evm_leaf(byte: u8) -> Vec<u8> {
    vec![byte; 32]
}

/// Build a well-formed SVM validator set with `n` distinct non-zero 32-byte
/// entries (no duplicates, no zero/blank signers).
fn svm_validators(first_byte: u8) -> Vec<u8> {
    let mut set = Vec::with_capacity(64);
    set.extend_from_slice(&[first_byte; 32]);
    set.extend_from_slice(&[first_byte.wrapping_add(1); 32]);
    set
}

/// Singleton externalities (block number starts at 0).
fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<MockRuntime>::default()
        .build_storage()
        .unwrap()
        .into()
}

// ---------------------------------------------------------------------------
// Original happy-path tests, preserved and made consistent with the C01
// security contract (authorized submitter enrolled; roots/validator sets now
// satisfy genuine Merkle + well-formedness checks).
// ---------------------------------------------------------------------------

#[test]
fn test_evm_header_validation() {
    new_test_ext().execute_with(|| {
        enroll(1);
        let block_number = 100u64;
        let block_hash = H256::from([1u8; 32]);
        let state_root = H256::from([2u8; 32]);
        let proof = evm_leaf(4);
        let merkle_root = H256::from([4u8; 32]);

        assert_ok!(crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            block_number,
            block_hash,
            state_root,
            merkle_root,
            proof,
        ));

        let stored = LastEvmHeader::<MockRuntime>::get();
        assert!(stored.is_some());
        let header = stored.unwrap();
        assert_eq!(header.block_number, block_number);
        assert_eq!(header.block_hash, block_hash);
    });
}

#[test]
fn test_invalid_evm_header_zero_block() {
    new_test_ext().execute_with(|| {
        enroll(1);
        let proof = evm_leaf(4);
        assert!(crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            0,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([4u8; 32]),
            proof,
        )
        .is_err());
    });
}

#[test]
fn test_svm_header_validation() {
    new_test_ext().execute_with(|| {
        enroll(1);
        let slot = 200u64;
        let block_hash = H256::from([10u8; 32]);
        let state_root = H256::from([11u8; 32]);
        let validator_set = svm_validators(12);
        let parent_slot_hashes = vec![H256::from([13u8; 32]); 3];

        assert_ok!(crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            slot,
            block_hash,
            state_root,
            validator_set,
            parent_slot_hashes.clone(),
        ));

        let stored = crate::LastSvmHeader::<MockRuntime>::get();
        assert!(stored.is_some());
        let header = stored.unwrap();
        assert_eq!(header.slot, slot);
        assert_eq!(header.block_hash, block_hash);
        assert_eq!(header.parent_slot_hashes.len(), 3);
    });
}

#[test]
fn test_invalid_svm_header_zero_slot() {
    new_test_ext().execute_with(|| {
        enroll(1);
        assert!(crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            0,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            svm_validators(3),
            vec![H256::from([4u8; 32])],
        )
        .is_err());
    });
}

#[test]
fn test_merkle_root_caching() {
    new_test_ext().execute_with(|| {
        enroll(1);
        let block_number = 150u64;
        // Proof leaf byte 53 => claimed root must equal H256([53;32]).
        let merkle_root = H256::from([53u8; 32]);

        assert_ok!(crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            block_number,
            H256::from([51u8; 32]),
            H256::from([52u8; 32]),
            merkle_root,
            vec![53u8; 32],
        ));

        let is_verified =
            crate::Pallet::<MockRuntime>::is_evm_merkle_root_verified(block_number, merkle_root);
        assert!(is_verified);

        let wrong_root = H256::from([99u8; 32]);
        let is_wrong_verified =
            crate::Pallet::<MockRuntime>::is_evm_merkle_root_verified(block_number, wrong_root);
        assert!(!is_wrong_verified);
    });
}

#[test]
fn test_validator_set_caching() {
    new_test_ext().execute_with(|| {
        enroll(1);
        let slot = 300u64;
        let validator_set = svm_validators(60);
        let validator_set_hash = H256::from(sp_io::hashing::blake2_256(&validator_set));
        let parent_slot_hashes = vec![H256::from([63u8; 32]); 3];

        assert_ok!(crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            slot,
            H256::from([61u8; 32]),
            H256::from([62u8; 32]),
            validator_set,
            parent_slot_hashes,
        ));

        let is_verified =
            crate::Pallet::<MockRuntime>::is_svm_validator_set_verified(slot, validator_set_hash);
        assert!(is_verified);

        let wrong_set_hash = H256::from([99u8; 32]);
        let is_wrong_verified =
            crate::Pallet::<MockRuntime>::is_svm_validator_set_verified(slot, wrong_set_hash);
        assert!(!is_wrong_verified);
    });
}

#[test]
fn test_validation_statistics_update() {
    new_test_ext().execute_with(|| {
        enroll(1);
        let initial_stats = crate::ValidationStats::<MockRuntime>::get();
        assert_eq!(initial_stats.evm_headers_validated, 0);
        assert_eq!(initial_stats.svm_headers_validated, 0);

        assert_ok!(crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            100,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([4u8; 32]),
            vec![4u8; 32],
        ));

        assert_ok!(crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            200,
            H256::from([10u8; 32]),
            H256::from([11u8; 32]),
            svm_validators(12),
            vec![H256::from([13u8; 32])],
        ));

        let updated_stats = crate::ValidationStats::<MockRuntime>::get();
        assert_eq!(updated_stats.evm_headers_validated, 1);
        assert_eq!(updated_stats.svm_headers_validated, 1);
    });
}

#[test]
fn test_cross_chain_settlement_scenario() {
    new_test_ext().execute_with(|| {
        enroll(1);

        // EVM header (source chain). Claimed root must match a single-leaf proof.
        let evm_block = 1000u64;
        let evm_merkle_root = H256::from([73u8; 32]);
        assert_ok!(crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            evm_block,
            H256::from([71u8; 32]),
            H256::from([72u8; 32]),
            evm_merkle_root,
            vec![73u8; 32],
        ));

        // SVM header (destination chain), well-formed validator set.
        let svm_slot = 2000u64;
        let svm_validator_set = svm_validators(80);
        let svm_validator_set_hash = H256::from(sp_io::hashing::blake2_256(&svm_validator_set));
        assert_ok!(crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            svm_slot,
            H256::from([81u8; 32]),
            H256::from([82u8; 32]),
            svm_validator_set,
            vec![H256::from([83u8; 32])],
        ));

        let evm_verified =
            crate::Pallet::<MockRuntime>::is_evm_merkle_root_verified(evm_block, evm_merkle_root);
        let svm_verified = crate::Pallet::<MockRuntime>::is_svm_validator_set_verified(
            svm_slot,
            svm_validator_set_hash,
        );
        assert!(evm_verified);
        assert!(svm_verified);

        let stats = crate::ValidationStats::<MockRuntime>::get();
        assert_eq!(stats.evm_headers_validated, 1);
        assert_eq!(stats.svm_headers_validated, 1);
    });
}

// ---------------------------------------------------------------------------
// Adversarial tests proving C01 is closed.  Every attack is rejected with the
// specific error and NO storage write occurs.
// ---------------------------------------------------------------------------

/// (a) A signed origin that is NOT an authorized submitter is rejected before
/// any write. Also proves the fail-closed posture: empty authorized set accepts
/// nothing even from otherwise-valid callers.
#[test]
fn test_c01_rejects_unenrolled_origin_without_write() {
    new_test_ext().execute_with(|| {
        // Account 2 submits, but only account 1 is enrolled.
        enroll(1);
        let ok = crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(2),
            100,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([4u8; 32]),
            vec![4u8; 32],
        );
        assert_err!(ok, crate::Error::<MockRuntime>::NotAuthorizedSubmitter);

        assert!(LastEvmHeader::<MockRuntime>::get().is_none());
        let stats = crate::ValidationStats::<MockRuntime>::get();
        assert_eq!(stats.evm_headers_validated, 0);
    });
}

/// (a2) Fail-closed: with an EMPTY authorized set nothing can be submitted.
#[test]
fn test_c01_fail_closed_when_no_authorized_submitters() {
    new_test_ext().execute_with(|| {
        // No enrollment at all.
        let ok = crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            100,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            svm_validators(3),
            vec![H256::from([4u8; 32])],
        );
        assert_err!(ok, crate::Error::<MockRuntime>::NotAuthorizedSubmitter);
        assert!(crate::LastSvmHeader::<MockRuntime>::get().is_none());
    });
}

fn finality_key(seed: u8) -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
}

fn finality_certificate(
    chain: u8,
    height: u64,
    block_hash: H256,
    state_root: H256,
    auxiliary_root: H256,
    keys: &[ed25519_dalek::SigningKey],
) -> Vec<u8> {
    use ed25519_dalek::Signer;
    use x3_verification_router::header_finality::{
        encode_header_certificate, header_finality_message, FinalizedHeader,
    };

    let header = FinalizedHeader {
        chain,
        height,
        block_hash: block_hash.0,
        state_root: state_root.0,
        auxiliary_root: auxiliary_root.0,
    };
    let message = header_finality_message(&header);
    let attestations = keys
        .iter()
        .map(|key| {
            (
                key.verifying_key().to_bytes(),
                key.sign(&message).to_bytes(),
            )
        })
        .collect::<Vec<_>>();
    encode_header_certificate(header, &attestations)
}

#[test]
fn finalized_evm_and_svm_headers_require_distinct_attester_quorum() {
    new_test_ext().execute_with(|| {
        use x3_verification_router::header_finality::{
            HEADER_FINALITY_ETHEREUM, HEADER_FINALITY_SOLANA,
        };

        let evm_keys = [finality_key(1), finality_key(2), finality_key(3)];
        let svm_keys = [finality_key(4), finality_key(5), finality_key(6)];
        assert_ok!(crate::Pallet::<MockRuntime>::set_header_attesters(
            RuntimeOrigin::root(),
            evm_keys
                .iter()
                .map(|key| key.verifying_key().to_bytes())
                .collect(),
            2,
            svm_keys
                .iter()
                .map(|key| key.verifying_key().to_bytes())
                .collect(),
            2,
        ));

        let evm_block = H256::repeat_byte(0x11);
        let evm_state = H256::repeat_byte(0x22);
        let evm_receipts = H256::repeat_byte(0x33);
        let evm_cert = finality_certificate(
            HEADER_FINALITY_ETHEREUM,
            42,
            evm_block,
            evm_state,
            evm_receipts,
            &evm_keys[..2],
        );
        assert_ok!(crate::Pallet::<MockRuntime>::validate_finalized_evm_header(
            RuntimeOrigin::signed(99),
            42,
            evm_block,
            evm_state,
            evm_receipts,
            evm_cert,
        ));
        assert_eq!(
            crate::LastFinalizedEvmHeader::<MockRuntime>::get().map(|header| (
                header.block_number,
                header.block_hash,
                header.state_root
            )),
            Some((42, evm_block, evm_state))
        );
        assert_eq!(
            crate::EvmMerkleRoots::<MockRuntime>::get(42),
            Some(evm_receipts)
        );

        let svm_block = H256::repeat_byte(0x44);
        let svm_state = H256::repeat_byte(0x55);
        let svm_validators = H256::repeat_byte(0x66);
        let svm_cert = finality_certificate(
            HEADER_FINALITY_SOLANA,
            7,
            svm_block,
            svm_state,
            svm_validators,
            &svm_keys[..1],
        );
        assert_noop!(
            crate::Pallet::<MockRuntime>::validate_finalized_svm_header(
                RuntimeOrigin::signed(99),
                7,
                svm_block,
                svm_state,
                svm_validators,
                svm_cert,
            ),
            crate::Error::<MockRuntime>::InvalidHeaderCertificate
        );
        assert!(!crate::LastFinalizedSvmHeader::<MockRuntime>::exists());

        let svm_cert = finality_certificate(
            HEADER_FINALITY_SOLANA,
            7,
            svm_block,
            svm_state,
            svm_validators,
            &svm_keys[..2],
        );
        assert_ok!(crate::Pallet::<MockRuntime>::validate_finalized_svm_header(
            RuntimeOrigin::signed(99),
            7,
            svm_block,
            svm_state,
            svm_validators,
            svm_cert,
        ));
        assert_eq!(
            crate::LastFinalizedSvmHeader::<MockRuntime>::get().map(|header| (
                header.slot,
                header.block_hash,
                header.state_root
            )),
            Some((7, svm_block, svm_state))
        );
        assert_eq!(
            crate::FinalizedSvmHeaders::<MockRuntime>::get(7).map(|header| (
                header.slot,
                header.block_hash,
                header.state_root
            )),
            Some((7, svm_block, svm_state)),
            "the finalized slot must remain available after newer slots finalize"
        );
        assert_eq!(
            crate::SvmValidatorSets::<MockRuntime>::get(7),
            Some(svm_validators)
        );
    });
}

#[test]
fn governance_can_revoke_header_attesters_and_the_path_fails_closed() {
    new_test_ext().execute_with(|| {
        use x3_verification_router::header_finality::{
            HEADER_FINALITY_ETHEREUM, MAX_HEADER_CERTIFICATE_BYTES,
        };

        let keys = [finality_key(1), finality_key(2)];
        assert_ok!(crate::Pallet::<MockRuntime>::set_header_attesters(
            RuntimeOrigin::root(),
            keys.iter()
                .map(|key| key.verifying_key().to_bytes())
                .collect(),
            1,
            Vec::new(),
            0,
        ));

        let block_hash = H256::repeat_byte(0x11);
        let state_root = H256::repeat_byte(0x22);
        let receipts_root = H256::repeat_byte(0x33);
        let certificate = finality_certificate(
            HEADER_FINALITY_ETHEREUM,
            42,
            block_hash,
            state_root,
            receipts_root,
            &keys[..1],
        );
        assert_ok!(crate::Pallet::<MockRuntime>::validate_finalized_evm_header(
            RuntimeOrigin::signed(99),
            42,
            block_hash,
            state_root,
            receipts_root,
            certificate.clone(),
        ));

        assert_ok!(crate::Pallet::<MockRuntime>::set_header_attesters(
            RuntimeOrigin::root(),
            Vec::new(),
            0,
            Vec::new(),
            0,
        ));
        assert_noop!(
            crate::Pallet::<MockRuntime>::validate_finalized_evm_header(
                RuntimeOrigin::signed(99),
                43,
                H256::repeat_byte(0x44),
                H256::repeat_byte(0x55),
                H256::repeat_byte(0x66),
                certificate,
            ),
            crate::Error::<MockRuntime>::InvalidHeaderCertificate
        );
        assert_eq!(
            crate::LastFinalizedEvmHeader::<MockRuntime>::get().map(|header| header.block_number),
            Some(42)
        );

        assert_noop!(
            crate::Pallet::<MockRuntime>::validate_finalized_evm_header(
                RuntimeOrigin::signed(99),
                43,
                H256::repeat_byte(0x44),
                H256::repeat_byte(0x55),
                H256::repeat_byte(0x66),
                vec![0u8; MAX_HEADER_CERTIFICATE_BYTES + 1],
            ),
            crate::Error::<MockRuntime>::PayloadTooLarge
        );
    });
}

#[test]
fn delayed_finalized_headers_are_retained_without_regressing_the_head() {
    new_test_ext().execute_with(|| {
        use x3_verification_router::header_finality::HEADER_FINALITY_ETHEREUM;

        let key = finality_key(3);
        assert_ok!(crate::Pallet::<MockRuntime>::set_header_attesters(
            RuntimeOrigin::root(),
            vec![key.verifying_key().to_bytes()],
            1,
            Vec::new(),
            0,
        ));

        let finalize = |height: u64, seed: u8| {
            let block_hash = H256::repeat_byte(seed);
            let state_root = H256::repeat_byte(seed.wrapping_add(1));
            let receipts_root = H256::repeat_byte(seed.wrapping_add(2));
            let certificate = finality_certificate(
                HEADER_FINALITY_ETHEREUM,
                height,
                block_hash,
                state_root,
                receipts_root,
                std::slice::from_ref(&key),
            );
            crate::Pallet::<MockRuntime>::validate_finalized_evm_header(
                RuntimeOrigin::signed(99),
                height,
                block_hash,
                state_root,
                receipts_root,
                certificate,
            )
        };

        assert_ok!(finalize(44, 0x10));
        assert_ok!(finalize(43, 0x20));
        assert_eq!(
            crate::LastFinalizedEvmHeader::<MockRuntime>::get().map(|header| header.block_number),
            Some(44)
        );
        assert_eq!(
            crate::FinalizedEvmHeaders::<MockRuntime>::get(43).map(|header| header.block_hash),
            Some(H256::repeat_byte(0x20))
        );
        assert_eq!(
            crate::FinalizedEvmHeaders::<MockRuntime>::get(44).map(|header| header.block_hash),
            Some(H256::repeat_byte(0x10))
        );
    });
}

#[test]
fn duplicate_attesters_are_rejected_without_overwriting_the_previous_set() {
    new_test_ext().execute_with(|| {
        let key = finality_key(1).verifying_key().to_bytes();
        assert_noop!(
            crate::Pallet::<MockRuntime>::set_header_attesters(
                RuntimeOrigin::root(),
                vec![key, key],
                1,
                vec![finality_key(2).verifying_key().to_bytes()],
                1,
            ),
            crate::Error::<MockRuntime>::InvalidAttesterSet
        );
        assert!(crate::EvmHeaderAttesters::<MockRuntime>::get().is_empty());
        assert!(crate::SvmHeaderAttesters::<MockRuntime>::get().is_empty());
    });
}

#[test]
fn attester_certificate_cannot_downgrade_a_native_evm_anchor() {
    new_test_ext().execute_with(|| {
        use x3_verification_router::header_finality::HEADER_FINALITY_ETHEREUM;

        let key = finality_key(1);
        assert_ok!(crate::Pallet::<MockRuntime>::set_header_attesters(
            RuntimeOrigin::root(),
            vec![key.verifying_key().to_bytes()],
            1,
            Vec::new(),
            0,
        ));
        crate::LastFinalizedEvmHeader::<MockRuntime>::put(crate::EvmHeaderInfo {
            block_number: 42,
            block_hash: H256::repeat_byte(0x11),
            state_root: H256::repeat_byte(0x12),
            merkle_root: H256::repeat_byte(0x13),
            validator_set_hash: H256::repeat_byte(0x14),
            verified_at_block: 1,
            validation_proof: Vec::new(),
        });
        crate::EvmHeaderFinalitySource::<MockRuntime>::put(
            crate::EvmHeaderFinalitySourceKind::BeaconSyncCommittee,
        );

        let certificate = finality_certificate(
            HEADER_FINALITY_ETHEREUM,
            43,
            H256::repeat_byte(0x21),
            H256::repeat_byte(0x22),
            H256::repeat_byte(0x23),
            &[key],
        );
        assert_noop!(
            crate::Pallet::<MockRuntime>::validate_finalized_evm_header(
                RuntimeOrigin::signed(99),
                43,
                H256::repeat_byte(0x21),
                H256::repeat_byte(0x22),
                H256::repeat_byte(0x23),
                certificate,
            ),
            crate::Error::<MockRuntime>::NativeEvmFinalityActive
        );
        assert_eq!(
            crate::LastFinalizedEvmHeader::<MockRuntime>::get().map(|header| header.block_number),
            Some(42)
        );
        assert_eq!(
            crate::EvmHeaderFinalitySource::<MockRuntime>::get(),
            crate::EvmHeaderFinalitySourceKind::BeaconSyncCommittee
        );
    });
}

/// (b) Arbitrary one-byte "junk" proof is rejected (not a multiple of 32) by an
/// otherwise authorized submitter, with no write.
#[test]
fn test_c01_rejects_junk_one_byte_proof() {
    new_test_ext().execute_with(|| {
        enroll(1);
        let ok = crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            100,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([4u8; 32]),
            vec![0xabu8], // 1 byte
        );
        assert_err!(ok, crate::Error::<MockRuntime>::ProofNotMultipleOf32);
        assert!(LastEvmHeader::<MockRuntime>::get().is_none());
    });
}

/// (b2) An all-zero single leaf is a blank commitment and must be rejected.
#[test]
fn test_c01_rejects_blank_zero_leaf() {
    new_test_ext().execute_with(|| {
        enroll(1);
        let ok = crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            100,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([3u8; 32]),
            vec![0u8; 32], // zero leaf
        );
        assert_err!(ok, crate::Error::<MockRuntime>::MalformedProofData);
        assert!(LastEvmHeader::<MockRuntime>::get().is_none());
    });
}

/// (c) The claimed Merkle root is no longer ignored: a structurally valid proof
/// whose recomputed root differs from the claimed root is rejected with no write.
#[test]
fn test_c01_rejects_merkle_root_mismatch() {
    new_test_ext().execute_with(|| {
        enroll(1);
        // Leaf byte = 4  => true root = H256([4;32]). Claim a DIFFERENT root [9;32].
        let ok = crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            100,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([9u8; 32]), // mismatch
            vec![4u8; 32],
        );
        assert_err!(ok, crate::Error::<MockRuntime>::MerkleRootMismatch);
        assert!(LastEvmHeader::<MockRuntime>::get().is_none());
    });
}

/// (d) A far-future height (beyond now + MaxHeaderLookahead) is rejected so an
/// attacker cannot poison the high-water mark / block later legitimate headers.
#[test]
fn test_c01_rejects_far_future_height() {
    new_test_ext().execute_with(|| {
        enroll(1);
        let ok = crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            u64::MAX, // absurd far-future height
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([4u8; 32]),
            vec![4u8; 32],
        );
        assert_err!(ok, crate::Error::<MockRuntime>::FarFutureHeader);
        assert!(LastEvmHeader::<MockRuntime>::get().is_none());
    });
}

/// (e1) Quorum cannot be laundered via byte length: an oversized proof (beyond
/// MAX_PROOF_BYTES) crafted from many leaves is rejected by size bound, no write.
#[test]
fn test_c01_rejects_oversized_proof_payload() {
    new_test_ext().execute_with(|| {
        enroll(1);
        // 10 KiB cap = 10240 bytes. Push past it with nonzero leaves.
        let big: Vec<u8> = (0..(10240u32 / 32 + 2))
            .flat_map(|i| {
                let b = ((i % 250) as u8).wrapping_add(1);
                std::iter::repeat_n(b, 32)
            })
            .collect();
        let ok = crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            100,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([4u8; 32]),
            big,
        );
        assert_err!(ok, crate::Error::<MockRuntime>::PayloadTooLarge);
        assert!(LastEvmHeader::<MockRuntime>::get().is_none());
    });
}

/// (e2) A crafted SVM validator set with DUPLICATE non-zero signers is rejected
/// (no self-selected/duplicate 'validator' laundering), no write.
#[test]
fn test_c01_rejects_duplicate_svm_signers() {
    new_test_ext().execute_with(|| {
        enroll(1);
        // Two identical 32-byte entries [7;32] [7;32] => duplicate signer.
        let mut dup = Vec::with_capacity(64);
        dup.extend_from_slice(&[7u8; 32]);
        dup.extend_from_slice(&[7u8; 32]);
        let ok = crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            100,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            dup,
            vec![H256::from([3u8; 32])],
        );
        assert_err!(ok, crate::Error::<MockRuntime>::DuplicateValidator);
        assert!(crate::LastSvmHeader::<MockRuntime>::get().is_none());
    });
}

/// (e3) No governance change by a non-admin origin: only `AdminOrigin` (Root in
/// the mock, `EnsureRoot`) can manage the authorized-submitter set. Any other
/// signed origin is rejected by the origin check (frame-level `BadOrigin`) before
/// the pallet body runs.
#[test]
fn test_c01_only_admin_can_change_authorized_set() {
    new_test_ext().execute_with(|| {
        // Signed (non-root) caller attempts to enroll itself.
        let ok = crate::Pallet::<MockRuntime>::set_authorized_submitters(
            RuntimeOrigin::signed(5),
            vec![5],
        );
        assert_eq!(ok, Err(frame_support::sp_runtime::DispatchError::BadOrigin));
        // The authorized set is unchanged (still empty => submission fails closed).
        assert!(crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(5),
            100,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([4u8; 32]),
            vec![4u8; 32],
        )
        .is_err());
    });
}

// ---------------------------------------------------------------------------
// Header anchor
//
// A receipt proof carries a header of its own — the receipts root it walks to and
// the head height its depth is measured from are both fields of the proof. The
// verifier's anchor is what decides which header it is allowed to be about, and
// on X3 that answer has to come from this pallet's storage: the root an
// authorized submitter attested for the height.
// ---------------------------------------------------------------------------

#[test]
fn the_anchor_is_empty_until_a_header_is_attested() {
    use x3_verification_router::evm_receipt::EvmHeaderAnchor;

    new_test_ext().execute_with(|| {
        assert_eq!(crate::Pallet::<MockRuntime>::attested_head(), None);
        assert!(crate::Pallet::<MockRuntime>::anchored_header(100).is_none());
    });
}

#[test]
fn the_anchor_reports_the_attested_root_and_head() {
    use x3_verification_router::evm_receipt::EvmHeaderAnchor;

    new_test_ext().execute_with(|| {
        let block_number = 100u64;
        let block_hash = H256::from([1u8; 32]);
        let state_root = H256::from([2u8; 32]);
        let merkle_root = H256::from([4u8; 32]);
        let header = crate::EvmHeaderInfo {
            block_number,
            block_hash,
            state_root,
            merkle_root,
            validator_set_hash: H256::from([3u8; 32]),
            verified_at_block: 1,
            validation_proof: Vec::new(),
        };
        crate::LastFinalizedEvmHeader::<MockRuntime>::put(header.clone());
        crate::FinalizedEvmHeaders::<MockRuntime>::insert(block_number, header);
        crate::EvmHeaderFinalitySource::<MockRuntime>::put(
            crate::EvmHeaderFinalitySourceKind::BeaconSyncCommittee,
        );
        crate::EvmMerkleRoots::<MockRuntime>::insert(block_number, merkle_root);

        let anchored = crate::Pallet::<MockRuntime>::anchored_header(block_number)
            .expect("an attested height answers with its header");
        assert_eq!(anchored.number, block_number);
        assert_eq!(
            anchored.receipts_root, merkle_root.0,
            "the root a receipt proof has to walk to"
        );
        assert_eq!(anchored.state_root, state_root.0);
        assert_eq!(anchored.block_hash, block_hash.0);
        assert_eq!(
            crate::Pallet::<MockRuntime>::attested_head(),
            Some(block_number),
            "depth is measured from the attested head"
        );

        // A newer finalized header does not hide the full evidence for an older
        // deposit.
        let newer = crate::EvmHeaderInfo {
            block_number: block_number + 1,
            block_hash: H256::from([0x11u8; 32]),
            state_root: H256::from([0x12u8; 32]),
            merkle_root: H256::from([0x13u8; 32]),
            validator_set_hash: H256::from([0x14u8; 32]),
            verified_at_block: 2,
            validation_proof: Vec::new(),
        };
        crate::LastFinalizedEvmHeader::<MockRuntime>::put(newer.clone());
        crate::FinalizedEvmHeaders::<MockRuntime>::insert(block_number + 1, newer);
        assert_eq!(
            crate::Pallet::<MockRuntime>::attested_head(),
            Some(block_number + 1)
        );
        let older = crate::Pallet::<MockRuntime>::anchored_header(block_number)
            .expect("the older finalized header remains anchored");
        assert_eq!(older.block_hash, block_hash.0);
        assert_eq!(older.state_root, state_root.0);

        // A height nobody attested has no anchor, so a proof for it is refused
        // rather than checked against the header it carries.
        assert!(crate::Pallet::<MockRuntime>::anchored_header(block_number + 2).is_none());
    });
}

#[test]
fn an_unattested_height_is_never_anchored_even_when_a_later_one_exists() {
    use x3_verification_router::evm_receipt::EvmHeaderAnchor;

    new_test_ext().execute_with(|| {
        let block_number = 200u64;
        let merkle_root = H256::from([7u8; 32]);
        crate::LastFinalizedEvmHeader::<MockRuntime>::put(crate::EvmHeaderInfo {
            block_number,
            block_hash: H256::from([9u8; 32]),
            state_root: H256::from([8u8; 32]),
            merkle_root,
            validator_set_hash: H256::from([6u8; 32]),
            verified_at_block: 1,
            validation_proof: Vec::new(),
        });
        crate::EvmHeaderFinalitySource::<MockRuntime>::put(
            crate::EvmHeaderFinalitySourceKind::BeaconSyncCommittee,
        );
        crate::EvmMerkleRoots::<MockRuntime>::insert(block_number, merkle_root);

        assert_eq!(
            crate::Pallet::<MockRuntime>::attested_head(),
            Some(200),
            "the head moved"
        );
        assert!(
            crate::Pallet::<MockRuntime>::anchored_header(199).is_none(),
            "a height with no attested root is not anchored by a later header"
        );
    });
}
