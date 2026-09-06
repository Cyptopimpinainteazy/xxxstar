use crate::mock::*;
use crate::LastEvmHeader;
use frame_support::{assert_err, assert_ok};
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
        let big: Vec<u8> = (0..(10240u32 / 32 + 2)).flat_map(|i| {
            let b = ((i % 250) as u8).wrapping_add(1);
            std::iter::repeat(b).take(32)
        }).collect();
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
