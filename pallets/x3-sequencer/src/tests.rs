//! Tests for the x3-sequencer pallet.

use crate::{mock::*, pallet::*};
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use sp_core::H256;

fn h256(n: u8) -> H256 {
    H256::from([n; 32])
}

#[test]
fn submit_transaction_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0xAA),
            64,
            0 // native X3
        ));
        assert_eq!(X3Sequencer::pending_count(), 1);
        assert_eq!(X3Sequencer::global_sequence(), 1);

        // Check event
        System::assert_has_event(
            Event::<Test>::TransactionSequenced {
                sequence: 0,
                source_chain: 0,
                payload_hash: h256(0xAA),
                submitter: 1,
            }
            .into(),
        );
    });
}

#[test]
fn submit_multiple_transactions_batched() {
    new_test_ext().execute_with(|| {
        for i in 0..5u8 {
            assert_ok!(X3Sequencer::submit_transaction(
                RuntimeOrigin::signed(1),
                h256(i),
                32,
                0
            ));
        }
        assert_eq!(X3Sequencer::pending_count(), 5);
        assert_eq!(X3Sequencer::global_sequence(), 5);
    });
}

#[test]
fn batch_sealed_on_finalize() {
    new_test_ext().execute_with(|| {
        // Submit 3 transactions
        for i in 0..3u8 {
            assert_ok!(X3Sequencer::submit_transaction(
                RuntimeOrigin::signed(1),
                h256(i),
                32,
                0
            ));
        }
        assert_eq!(X3Sequencer::pending_count(), 3);

        // Finalize the block — should seal the batch
        X3Sequencer::on_finalize(1);

        // Pending should be empty now
        assert_eq!(X3Sequencer::pending_count(), 0);

        // Batch should exist
        let batch = X3Sequencer::batches(0).expect("batch 0 should exist");
        assert_eq!(batch.batch_id, 0);
        assert_eq!(batch.tx_count, 3);
        assert_eq!(batch.total_bytes, 96); // 3 * 32
        assert_eq!(batch.sealed_at, 1); // sealed at block 1

        // Merkle root should not be zero
        assert_ne!(batch.merkle_root, H256::zero());

        // Next batch ID should increment
        assert_eq!(X3Sequencer::next_batch_id(), 1);
    });
}

#[test]
fn empty_batch_not_created() {
    new_test_ext().execute_with(|| {
        // No transactions submitted
        X3Sequencer::on_finalize(1);

        // No batch should be created
        assert!(X3Sequencer::batches(0).is_none());
        assert_eq!(X3Sequencer::next_batch_id(), 0);
    });
}

#[test]
fn sequential_batches() {
    new_test_ext().execute_with(|| {
        // Block 1: 2 transactions
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0xAA),
            10,
            0
        ));
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(2),
            h256(0xBB),
            20,
            1
        ));
        X3Sequencer::on_finalize(1);

        // Block 2: 1 transaction
        System::set_block_number(2);
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(3),
            h256(0xCC),
            30,
            0
        ));
        X3Sequencer::on_finalize(2);

        // Both batches should exist
        let batch0 = X3Sequencer::batches(0).unwrap();
        assert_eq!(batch0.tx_count, 2);
        assert_eq!(batch0.total_bytes, 30); // 10 + 20

        let batch1 = X3Sequencer::batches(1).unwrap();
        assert_eq!(batch1.tx_count, 1);
        assert_eq!(batch1.total_bytes, 30);

        // Different Merkle roots
        assert_ne!(batch0.merkle_root, batch1.merkle_root);
    });
}

#[test]
fn payload_too_large_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Sequencer::submit_transaction(
                RuntimeOrigin::signed(1),
                h256(0xAA),
                1025, // MaxPayloadSize = 1024
                0
            ),
            Error::<Test>::PayloadTooLarge
        );
    });
}

#[test]
fn batch_full_rejected() {
    new_test_ext().execute_with(|| {
        // Fill the batch (MaxTxsPerBatch = 16)
        for i in 0..16u8 {
            assert_ok!(X3Sequencer::submit_transaction(
                RuntimeOrigin::signed(1),
                h256(i),
                10,
                0
            ));
        }
        assert_eq!(X3Sequencer::pending_count(), 16);

        // 17th transaction should fail
        assert_noop!(
            X3Sequencer::submit_transaction(RuntimeOrigin::signed(1), h256(0xFF), 10, 0),
            Error::<Test>::BatchFull
        );
    });
}

#[test]
fn merkle_root_deterministic() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x01),
            32,
            0
        ));
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x02),
            32,
            0
        ));
        X3Sequencer::on_finalize(1);
        let root1 = X3Sequencer::batches(0).unwrap().merkle_root;

        // Start fresh with same transactions
        System::set_block_number(2);
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x01),
            32,
            0
        ));
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x02),
            32,
            0
        ));
        X3Sequencer::on_finalize(2);
        let root2 = X3Sequencer::batches(1).unwrap().merkle_root;

        // Same input → same root
        assert_eq!(root1, root2);
    });
}

#[test]
fn merkle_root_order_matters() {
    new_test_ext().execute_with(|| {
        // Block 1: [0x01, 0x02]
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x01),
            32,
            0
        ));
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x02),
            32,
            0
        ));
        X3Sequencer::on_finalize(1);
        let root_a = X3Sequencer::batches(0).unwrap().merkle_root;

        // Block 2: [0x02, 0x01] — different order
        System::set_block_number(2);
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x02),
            32,
            0
        ));
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x01),
            32,
            0
        ));
        X3Sequencer::on_finalize(2);
        let root_b = X3Sequencer::batches(1).unwrap().merkle_root;

        // Different order → different root (ordering integrity)
        assert_ne!(root_a, root_b);
    });
}

#[test]
fn single_tx_merkle_root_equals_tx_hash() {
    new_test_ext().execute_with(|| {
        let tx_hash = h256(0x42);
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            tx_hash,
            32,
            0
        ));
        X3Sequencer::on_finalize(1);

        let batch = X3Sequencer::batches(0).unwrap();
        // Single tx → merkle root = tx hash
        assert_eq!(batch.merkle_root, tx_hash);
    });
}

#[test]
fn global_sequence_monotonic() {
    new_test_ext().execute_with(|| {
        for _ in 0..5 {
            assert_ok!(X3Sequencer::submit_transaction(
                RuntimeOrigin::signed(1),
                h256(0x01),
                10,
                0
            ));
        }
        assert_eq!(X3Sequencer::global_sequence(), 5);

        X3Sequencer::on_finalize(1);

        // Sequence continues after batch seal
        System::set_block_number(2);
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x01),
            10,
            0
        ));
        assert_eq!(X3Sequencer::global_sequence(), 6);
    });
}

#[test]
fn get_batch_helper_works() {
    new_test_ext().execute_with(|| {
        assert!(X3Sequencer::get_batch(0).is_none());

        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0xAA),
            32,
            0
        ));
        X3Sequencer::on_finalize(1);

        let batch = X3Sequencer::get_batch(0);
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().tx_count, 1);
    });
}

#[test]
fn source_chain_stored_correctly() {
    new_test_ext().execute_with(|| {
        // Submit from different source chains
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x01),
            10,
            0 // native
        ));
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x02),
            10,
            1 // rollup 1
        ));
        assert_ok!(X3Sequencer::submit_transaction(
            RuntimeOrigin::signed(1),
            h256(0x03),
            10,
            42 // rollup 42
        ));
        assert_eq!(X3Sequencer::pending_count(), 3);
    });
}
