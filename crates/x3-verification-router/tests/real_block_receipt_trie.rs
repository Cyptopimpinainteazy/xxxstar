//! The receipts-trie producer against a block a real EVM produced.
//!
//! A producer and a verifier that agree with each other can both be wrong: they
//! share the key convention, the hex-prefix flags and the hashing rule, so a
//! round-trip test proves only self-consistency. The check that settles it is the
//! header's own `receiptsRoot`, which the chain computed — a trie built from the
//! block's receipts reproduces it only if the construction is Ethereum's.
//!
//! The fixture (`tests/data/anvil_block1_receipts.hex`) is the output of a real
//! `anvil` node's `debug_getRawReceipts` for one of its blocks, one receipt per
//! line, exactly as the node returned the consensus encoding — not a re-encoding
//! of JSON receipts by the code under test. The block:
//!
//!   chain id  31337 (anvil)
//!   number    1
//!   hash      0xb230eac75cb1a09a42521a6a4c837cf460502617e8a0f7378608ec91495f45e6
//!   state     0xf15ebe0fd4624d7a4f81bc498eaee4e313e8c359dfbbc865e5216c214c4a6c14
//!   receipts  0x251f2cb798e965c5d9b11c882f37c69fd2c42b314fabe64d2b4998c76eb93ae8
//!   txs       3 legacy (type 0) transfers, so each `receipt_data` is the bare RLP
//!             list the settlement engine's structural check accepts
//!
//! Three transactions is the point: a one-receipt block's trie is a single leaf
//! at the root, which exercises none of the branch walk that multi-transaction
//! blocks depend on.

use x3_verification_router::evm_receipt::{
    receipt_trie_key, receipts_trie_proof, receipts_trie_root, verify_merkle_patricia_proof,
};

const HEADER_RECEIPTS_ROOT: &str =
    "251f2cb798e965c5d9b11c882f37c69fd2c42b314fabe64d2b4998c76eb93ae8";

/// One receipt per line, as `debug_getRawReceipts` returned them.
const RECEIPTS_HEX: &str = include_str!("data/anvil_block1_receipts.hex");

fn hex_decode(value: &str) -> Vec<u8> {
    assert!(value.len().is_multiple_of(2), "hex must be whole bytes");
    (0..value.len() / 2)
        .map(|i| u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).expect("hex"))
        .collect()
}

fn receipts() -> Vec<Vec<u8>> {
    RECEIPTS_HEX
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(hex_decode)
        .collect()
}

#[test]
fn the_fixture_is_a_multi_transaction_block_of_consensus_receipts() {
    let receipts = receipts();
    assert_eq!(receipts.len(), 3, "three receipts in this block");
    for (index, receipt) in receipts.iter().enumerate() {
        // A consensus receipt is an RLP list: a typed receipt would start with
        // its type byte instead, which the settlement engine refuses.
        assert!(
            receipt[0] >= 0xc0,
            "receipt {index} must be a bare RLP list, got first byte {:#x}",
            receipt[0]
        );
    }
}

#[test]
fn the_producer_reproduces_the_root_the_chain_computed() {
    let root = receipts_trie_root(&receipts()).expect("trie root");
    assert_eq!(
        hex_decode(HEADER_RECEIPTS_ROOT),
        root.to_vec(),
        "a trie built from this block's receipts must have the header's receiptsRoot"
    );
}

#[test]
fn every_receipt_of_a_real_block_has_a_verifying_proof() {
    let receipts = receipts();
    let root = receipts_trie_root(&receipts).expect("trie root");
    for (index, receipt) in receipts.iter().enumerate() {
        let proof = receipts_trie_proof(&receipts, index).expect("proof");
        assert!(
            proof.len() > 1,
            "receipt {index}: a proof is an RLP list of nodes"
        );
        assert_eq!(
            verify_merkle_patricia_proof(
                &root,
                &receipt_trie_key(index as u64),
                Some(receipt.as_slice()),
                &proof
            ),
            Ok(()),
            "receipt {index} must verify"
        );
    }
}

#[test]
fn a_real_blocks_proof_is_not_a_proof_for_another_index() {
    let receipts = receipts();
    let root = receipts_trie_root(&receipts).expect("trie root");
    let proof = receipts_trie_proof(&receipts, 1).expect("proof");
    assert_ne!(
        verify_merkle_patricia_proof(
            &root,
            &receipt_trie_key(2),
            Some(receipts[1].as_slice()),
            &proof
        ),
        Ok(()),
        "the proof for index 1 is not a proof for index 2"
    );
}
