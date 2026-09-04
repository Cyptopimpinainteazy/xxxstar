//! Tests for the x3-da pallet.

use crate::{mock::*, pallet::*};
use frame_support::traits::Currency;
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

fn h256(n: u8) -> H256 {
    H256::from([n; 32])
}

#[test]
fn submit_blob_commitment_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0xAA),
            512,
            None,
            None,
        ));

        // Blob should be stored
        let blob = X3Da::blobs(h256(0xAA)).expect("blob should exist");
        assert_eq!(blob.blob_id, 0);
        assert_eq!(blob.size_bytes, 512);
        assert_eq!(blob.status, 0); // Pending
        assert_eq!(blob.batch_id, None);
        assert_eq!(blob.erasure_root, None);
        assert_eq!(blob.kzg_commitment, None);

        // Metrics updated
        assert_eq!(X3Da::total_bytes_committed(), 512);
        assert_eq!(X3Da::next_blob_id(), 1);

        // Event emitted
        System::assert_has_event(
            Event::<Test>::BlobCommitted {
                blob_id: 0,
                data_hash: h256(0xAA),
                size_bytes: 512,
                submitter: 1,
            }
            .into(),
        );
    });
}

#[test]
fn submit_blob_with_batch_id() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0xBB),
            256,
            Some(42), // linked to sequencer batch 42
            None,
        ));

        let blob = X3Da::blobs(h256(0xBB)).unwrap();
        assert_eq!(blob.batch_id, Some(42));
    });
}

#[test]
fn submit_blob_with_erasure_root() {
    new_test_ext().execute_with(|| {
        let erasure = h256(0xEE);
        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0xCC),
            128,
            None,
            Some(erasure),
        ));

        let blob = X3Da::blobs(h256(0xCC)).unwrap();
        assert_eq!(blob.erasure_root, Some(erasure));
    });
}

#[test]
fn blob_too_large_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Da::submit_blob_commitment(
                RuntimeOrigin::signed(1),
                h256(0xAA),
                1025, // MaxBlobSize = 1024
                None,
                None,
            ),
            Error::<Test>::BlobTooLarge
        );
    });
}

#[test]
fn duplicate_blob_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0xDD),
            100,
            None,
            None,
        ));

        // Same hash again should fail
        assert_noop!(
            X3Da::submit_blob_commitment(RuntimeOrigin::signed(1), h256(0xDD), 100, None, None,),
            Error::<Test>::BlobAlreadyExists
        );
    });
}

#[test]
fn submit_shard_proof_works() {
    new_test_ext().execute_with(|| {
        // First submit a blob
        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0xAA),
            512,
            None,
            Some(h256(0xEE)),
        ));

        // Submit shard proof
        assert_ok!(X3Da::submit_shard_proof(
            RuntimeOrigin::signed(2),
            h256(0xAA), // blob hash
            0,          // shard index
            h256(0x11), // proof hash
        ));

        // Proof should be stored
        let proofs = X3Da::shard_proofs(h256(0xAA));
        assert_eq!(proofs.len(), 1);
        assert_eq!(proofs[0].shard_index, 0);
        assert_eq!(proofs[0].proof_hash, h256(0x11));

        // Event emitted
        System::assert_has_event(
            Event::<Test>::ShardProofSubmitted {
                blob_hash: h256(0xAA),
                shard_index: 0,
                attester: 2,
            }
            .into(),
        );
    });
}

#[test]
fn shard_proof_for_nonexistent_blob_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Da::submit_shard_proof(
                RuntimeOrigin::signed(1),
                h256(0xFF), // doesn't exist
                0,
                h256(0x11),
            ),
            Error::<Test>::BlobNotFound
        );
    });
}

#[test]
fn too_many_shard_proofs_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0xAA),
            256,
            None,
            Some(h256(0xEE)),
        ));

        // Submit MaxShardProofs (8) proofs
        for i in 0..8u32 {
            assert_ok!(X3Da::submit_shard_proof(
                RuntimeOrigin::signed(i as u64 + 10),
                h256(0xAA),
                i,
                h256(i as u8),
            ));
        }

        // 9th should fail
        assert_noop!(
            X3Da::submit_shard_proof(RuntimeOrigin::signed(99), h256(0xAA), 8, h256(0x99),),
            Error::<Test>::TooManyShardProofs
        );
    });
}

#[test]
fn multiple_blobs_tracked_independently() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0x01),
            100,
            None,
            None,
        ));
        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(2),
            h256(0x02),
            200,
            Some(1),
            None,
        ));
        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(3),
            h256(0x03),
            300,
            None,
            Some(h256(0xEE)),
        ));

        assert_eq!(X3Da::total_bytes_committed(), 600);
        assert_eq!(X3Da::next_blob_id(), 3);

        // Each has correct ID
        assert_eq!(X3Da::blobs(h256(0x01)).unwrap().blob_id, 0);
        assert_eq!(X3Da::blobs(h256(0x02)).unwrap().blob_id, 1);
        assert_eq!(X3Da::blobs(h256(0x03)).unwrap().blob_id, 2);
    });
}

#[test]
fn is_blob_available_helper() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0xAA),
            100,
            None,
            None,
        ));

        // Initially not available (status = 0 = Pending)
        assert!(!X3Da::is_blob_available(h256(0xAA)));
        // Non-existent blob also not available
        assert!(!X3Da::is_blob_available(h256(0xFF)));
    });
}

#[test]
fn get_blob_helper() {
    new_test_ext().execute_with(|| {
        assert!(X3Da::get_blob(h256(0xAA)).is_none());

        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0xAA),
            100,
            None,
            None,
        ));

        let blob = X3Da::get_blob(h256(0xAA));
        assert!(blob.is_some());
        assert_eq!(blob.unwrap().size_bytes, 100);
    });
}

#[test]
fn blob_bytes_accumulate() {
    new_test_ext().execute_with(|| {
        assert_eq!(X3Da::total_bytes_committed(), 0);

        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0x01),
            100,
            None,
            None,
        ));
        assert_eq!(X3Da::total_bytes_committed(), 100);

        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0x02),
            250,
            None,
            None,
        ));
        assert_eq!(X3Da::total_bytes_committed(), 350);

        assert_ok!(X3Da::submit_blob_commitment(
            RuntimeOrigin::signed(1),
            h256(0x03),
            1024,
            None,
            None,
        ));
        assert_eq!(X3Da::total_bytes_committed(), 1374);
    });
}

#[test]
fn submit_blob_commitment_insufficient_funds_fails() {
    new_test_ext().execute_with(|| {
        // Ensure the submitter has no funds.
        Balances::make_free_balance_be(&1, 0);

        assert_noop!(
            X3Da::submit_blob_commitment(RuntimeOrigin::signed(1), h256(0xEE), 100, None, None,),
            Error::<Test>::InsufficientFee
        );
    });
}
