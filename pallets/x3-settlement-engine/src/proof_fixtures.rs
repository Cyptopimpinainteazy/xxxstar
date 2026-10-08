//! Proof fixtures shared by the pallet's tests and its benchmarks.
//!
//! The `submit_proof` benchmark needs the same evidence the tests use — a receipt that walks to a
//! real receipts root — and the fixture used to live in `tests.rs`, which the benchmark's CLI run
//! (a `--features runtime-benchmarks` build, not `cfg(test)`) cannot reach. Keeping one copy here,
//! compiled for either build, is what makes that benchmark runnable again: after PR #520 tightened
//! the EVM path to walk the trie, the benchmark still built a two-zero-root proof and failed with
//! `InvalidProof` on every run.

#![cfg(any(test, feature = "runtime-benchmarks"))]

use crate::types::{ProofType, SettlementProof};
use sp_core::H256;
use sp_runtime::DispatchError;
use sp_std::vec;
use sp_std::vec::Vec;

/// The height every fixture proof states (see `tests.rs` for why it is stated rather than derived).
pub(crate) const PROOF_HEIGHT: u64 = 18_000_000;

/// The index of the fixture receipt in its block: the trie key is `rlp(1)`.
pub(crate) const RECEIPT_INDEX: u32 = 1;

pub(crate) fn receipt_trie(receipt_rlp: &[u8], index: u32) -> (H256, Vec<u8>) {
    fn rlp_bytes(bytes: &[u8]) -> Vec<u8> {
        let mut stream = rlp::RlpStream::new();
        stream.append(&bytes.to_vec());
        stream.out().to_vec()
    }
    fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
        let mut stream = rlp::RlpStream::new_list(items.len());
        for item in items {
            stream.append_raw(item, 1);
        }
        stream.out().to_vec()
    }
    // The receipts-trie key is `rlp(index)`, the RLP of the integer.
    let key = {
        let be = index.to_be_bytes();
        let first = be.iter().position(|byte| *byte != 0);
        // No nonzero byte means the integer is zero. RLP encodes zero as 0x80;
        // the empty slice makes that case fall out of the length checks below.
        let significant: &[u8] = match first {
            None => &be[be.len()..],
            Some(first) => &be[first..],
        };
        if significant.is_empty() {
            vec![0x80]
        } else if significant.len() == 1 && significant[0] < 0x80 {
            vec![significant[0]]
        } else {
            let mut out = vec![0x80 + significant.len() as u8];
            out.extend_from_slice(significant);
            out
        }
    };
    // Hex-prefix leaf encoding of the key's nibbles (yellow paper appendix C).
    // The key has two nibbles per byte, so the nibble count is always even and
    // the leaf carries the 0x20 | length form; there is no odd-tail branch.
    let mut nibbles = Vec::new();
    for byte in &key {
        nibbles.push(byte >> 4);
        nibbles.push(byte & 0x0F);
    }
    let mut path = Vec::with_capacity(1 + nibbles.len() / 2);
    path.push(0x20 + (nibbles.len() / 2) as u8);
    for pair in nibbles.chunks(2) {
        path.push(pair[0] * 16 + pair[1]);
    }
    let leaf = rlp_list(&[rlp_bytes(&path), rlp_bytes(receipt_rlp)]);
    let root = H256::from(sp_io::hashing::keccak_256(&leaf));
    let proof = rlp_list(&[rlp_bytes(&leaf)]);
    (root, proof)
}

/// Build the receipt proof the EVM path accepts, or refuse.
///
/// Fallible on purpose: the three bounded-vector conversions below used to `unwrap`, which was
/// invisible while this code lived in `tests.rs` and became a production panic the moment it moved
/// into a module the benchmark build compiles. The tests unwrap this in their own `cfg(test)` wrapper.
pub(crate) fn create_evm_receipt_proof() -> Result<SettlementProof, DispatchError> {
    // RLP-encoded receipt: must be a valid list with at least 3 elements
    // Receipt format: [status/root, gas_used, logs, contractAddress?]
    // We create: [0x01 (status), 0x00 (0 gas), 0xc0 (empty logs list)]
    // RLP encoding: 0xc3 (list with 3 bytes) + 0x01 + 0x00 + 0xc0
    let receipt_data = vec![0xc3, 0x01, 0x00, 0xc0];

    // Compute Keccak256 hash of the receipt
    let tx_hash = H256::from(sp_io::hashing::keccak_256(&receipt_data));

    // The receipts root the proof is walked against, and the path that binds the
    // receipt to it: this is what makes the fixture evidence rather than a copy of
    // the header's public fields (TICKET-063).
    let (receipts_root, trie_proof) = receipt_trie(&receipt_data, RECEIPT_INDEX);

    Ok(SettlementProof {
        proof_type: ProofType::MerkleTrie,
        tx_hash,
        block_hash: H256::from([2u8; 32]),
        confirmations: 12,
        chain_height: Some(PROOF_HEIGHT),
        // Two entries, because the module verifies the proof against the
        // first two: a state root and the receipts root. A one-entry proof used to
        // have its second root invented as thirty-two zero bytes; see
        // `a_proof_that_does_not_carry_both_roots_is_refused`.
        merkle_proof: (vec![H256::from([3u8; 32]), receipts_root])
            .try_into()
            .map_err(|_| DispatchError::Other("fixture proof exceeds its bound"))?,
        receipt_data: receipt_data
            .try_into()
            .map_err(|_| DispatchError::Other("fixture receipt exceeds its bound"))?,
        receipt_index: Some(RECEIPT_INDEX),
        trie_proof: Some(
            trie_proof
                .try_into()
                .map_err(|_| DispatchError::Other("fixture trie proof exceeds its bound"))?,
        ),
    })
}
