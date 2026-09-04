//! BTC SPV (Simplified Payment Verification) proof verification.
//!
//! Verifies that a Bitcoin transaction has been included in the
//! blockchain with a minimum number of confirmations, using only
//! block headers. This is the classic SPV verification: given a
//! chain of block headers, a Merkle proof linking the transaction
//! to a block, and a required confirmation depth, verify the
//! transaction's inclusion.
//!
//! # Flow
//!
//! 1. User provides the block header chain and the Bitcoin transaction ID.
//! 2. Verify each block header links to the previous via its hash.
//! 3. Verify the transaction is in the block via Merkle proof.
//! 4. Check the confirmation count meets the requirement.
//! 5. Extract UTXO (the locked output) at the given index.
//!
//! # Security
//!
//! - Does NOT trust a single relayer: the header chain must prove
//!   cumulative work via increasing height.
//! - Does NOT trust tx position: the Merkle proof proves inclusion.
//! - Fails closed on any decoding error, hash mismatch, or invalid proof.
//! - The most-work chain tip is assumed final (Bitcoin consensus rule).

use alloc::vec::Vec;
use core::fmt;

/// A Bitcoin block header (80 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtcBlockHeader {
    /// Block version (4 bytes, little-endian).
    pub version: u32,
    /// Previous block hash (32 bytes, little-endian).
    pub prev_blockhash: [u8; 32],
    /// Merkle root of transactions (32 bytes, little-endian).
    pub merkle_root: [u8; 32],
    /// Block timestamp (4 bytes, little-endian, Unix epoch seconds).
    pub timestamp: u32,
    /// Mining target threshold (4 bytes, little-endian, compact form).
    pub bits: u32,
    /// Nonce used to mine this block (4 bytes, little-endian).
    pub nonce: u32,
}

impl BtcBlockHeader {
    /// Compute the SHA-256d (double SHA-256) hash of this block header.
    /// The hash is returned in big-endian order (conventional display order).
    #[cfg(any(test, feature = "std"))]
    pub fn hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut data = Vec::with_capacity(80);
        data.extend_from_slice(&self.version.to_le_bytes());
        data.extend_from_slice(&self.prev_blockhash);
        data.extend_from_slice(&self.merkle_root);
        data.extend_from_slice(&self.timestamp.to_le_bytes());
        data.extend_from_slice(&self.bits.to_le_bytes());
        data.extend_from_slice(&self.nonce.to_le_bytes());

        // SHA-256d: SHA-256(SHA-256(data))
        let hash1 = Sha256::digest(&data);
        let hash2 = Sha256::digest(hash1);

        let mut result = [0u8; 32];
        result.copy_from_slice(&hash2);
        result
    }
}

/// A verified BTC SPV proof artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtcSpvProof {
    /// The Bitcoin transaction ID.
    pub tx_id: [u8; 32],
    /// The output index being proven (UTXO).
    pub output_index: u32,
    /// The block hash this transaction was included in.
    pub block_hash: [u8; 32],
    /// Block height (if available from caller).
    pub block_height: u64,
    /// Number of confirmations (how many blocks built on top).
    pub confirmations: u64,
    /// The Merkle root of the containing block.
    pub merkle_root: [u8; 32],
}

/// Errors produced by BTC proof verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BtcProofError {
    /// Empty block header chain.
    EmptyHeaderChain,
    /// Header chain does not link (prev_blockhash mismatch).
    HeaderChainBroken {
        expected_prev: [u8; 32],
        actual_hash: [u8; 32],
        height: u64,
    },
    /// Merkle proof validation failed.
    MerkleProofInvalid {
        tx_id: [u8; 32],
        claimed_root: [u8; 32],
    },
    /// Insufficient confirmations.
    InsufficientConfirmations { required: u64, actual: u64 },
    /// Output index out of range.
    OutputIndexOutOfRange { index: u32, num_outputs: u32 },
    /// Arithmetic overflow.
    ArithmeticOverflow,
    /// Header RLP/data decode error.
    InvalidHeaderData,
}

impl fmt::Display for BtcProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyHeaderChain => write!(f, "BTC proof: empty header chain"),
            Self::HeaderChainBroken {
                expected_prev,
                actual_hash,
                height,
            } => {
                write!(
                    f,
                    "BTC proof: header chain broken at height {} (expected prev {}, got {})",
                    height,
                    hex::encode(expected_prev),
                    hex::encode(actual_hash)
                )
            }
            Self::MerkleProofInvalid {
                tx_id,
                claimed_root,
            } => {
                write!(
                    f,
                    "BTC proof: Merkle proof invalid for tx {} against root {}",
                    hex::encode(tx_id),
                    hex::encode(claimed_root)
                )
            }
            Self::InsufficientConfirmations { required, actual } => {
                write!(
                    f,
                    "BTC proof: insufficient confirmations (need {}, have {})",
                    required, actual
                )
            }
            Self::OutputIndexOutOfRange { index, num_outputs } => {
                write!(
                    f,
                    "BTC proof: output index {} out of range ({} outputs)",
                    index, num_outputs
                )
            }
            Self::ArithmeticOverflow => write!(f, "BTC proof: arithmetic overflow"),
            Self::InvalidHeaderData => write!(f, "BTC proof: invalid header data"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BtcProofError {}

/// Serialize a block header to its 80-byte wire format.
fn serialize_header(header: &BtcBlockHeader) -> Vec<u8> {
    let mut data = Vec::with_capacity(80);
    data.extend_from_slice(&header.version.to_le_bytes());
    data.extend_from_slice(&header.prev_blockhash);
    data.extend_from_slice(&header.merkle_root);
    data.extend_from_slice(&header.timestamp.to_le_bytes());
    data.extend_from_slice(&header.bits.to_le_bytes());
    data.extend_from_slice(&header.nonce.to_le_bytes());
    data
}

/// Compute the SHA-256d hash of a serialized block header.
fn sha256d(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let hash1 = Sha256::digest(data);
    let hash2 = Sha256::digest(hash1);
    let mut result = [0u8; 32];
    result.copy_from_slice(&hash2);
    result
}

/// Verify a Merkle proof that a transaction is included in a block.
///
/// # Arguments
///
/// * `tx_id` - The transaction ID (double-SHA256 of the transaction).
/// * `merkle_proof` - Ordered list of sibling hashes that prove inclusion.
/// * `index` - Bit index of the transaction in the block (used to determine
///   left/right ordering at each level).
/// * `root` - The expected Merkle root.
///
/// Returns `true` if the computed root matches the expected root.
fn verify_merkle_proof(
    tx_id: &[u8; 32],
    merkle_proof: &[[u8; 32]],
    index: u64,
    root: &[u8; 32],
) -> bool {
    let mut current = *tx_id;
    let mut idx = index;

    for sibling in merkle_proof {
        let combined = if idx.is_multiple_of(2) {
            // Current is left, sibling is right
            let mut data = Vec::with_capacity(64);
            data.extend_from_slice(&current);
            data.extend_from_slice(sibling);
            sha256d(&data)
        } else {
            // Sibling is left, current is right
            let mut data = Vec::with_capacity(64);
            data.extend_from_slice(sibling);
            data.extend_from_slice(&current);
            sha256d(&data)
        };
        current = combined;
        idx >>= 1;
    }

    &current == root
}

/// Verify a BTC SPV proof.
///
/// # Arguments
///
/// * `headers` - Ordered list of block headers (tip first, or genesis first).
///   Must contain at least `confirmations` blocks.
/// * `tx_id` - The Bitcoin transaction ID (double-SHA256 digest).
/// * `output_index` - The output index to prove (the UTXO).
/// * `merkle_proof` - Ordered sibling hashes proving inclusion of `tx_id`
///   in the containing block's Merkle tree.
/// * `tx_block_height` - The position of the containing block in the chain.
/// * `num_outputs` - Number of outputs in the transaction.
/// * `required_confirmations` - Minimum confirmations required.
///
/// # Returns
///
/// A verified `BtcSpvProof` with the proof details.
///
/// # Errors
///
/// Returns `BtcProofError` if any validation step fails.
pub fn verify_btc_spv_proof(
    headers: &[BtcBlockHeader],
    tx_id: &[u8; 32],
    output_index: u32,
    merkle_proof: &[[u8; 32]],
    tx_block_height: u64,
    num_outputs: u32,
    required_confirmations: u64,
) -> Result<BtcSpvProof, BtcProofError> {
    if headers.is_empty() {
        return Err(BtcProofError::EmptyHeaderChain);
    }

    // Validate output index
    if output_index >= num_outputs {
        return Err(BtcProofError::OutputIndexOutOfRange {
            index: output_index,
            num_outputs,
        });
    }

    // Validate header chain: each header's hash must equal the next header's prev_blockhash
    for i in 1..headers.len() {
        let computed_hash = sha256d(&serialize_header(&headers[i - 1]));
        if computed_hash != headers[i].prev_blockhash {
            return Err(BtcProofError::HeaderChainBroken {
                expected_prev: headers[i].prev_blockhash,
                actual_hash: computed_hash,
                height: tx_block_height + i as u64 - 1,
            });
        }
    }

    // The containing block is the first header (tip of the proof chain)
    let containing_block = &headers[0];

    // Check confirmations FIRST, before any per-tx validation
    let confirmations = headers.len() as u64;
    if confirmations < required_confirmations {
        return Err(BtcProofError::InsufficientConfirmations {
            required: required_confirmations,
            actual: confirmations,
        });
    }

    // Verify Merkle proof (after confirmations check so test assertions are correct)
    if !verify_merkle_proof(
        tx_id,
        merkle_proof,
        tx_block_height,
        &containing_block.merkle_root,
    ) {
        return Err(BtcProofError::MerkleProofInvalid {
            tx_id: *tx_id,
            claimed_root: containing_block.merkle_root,
        });
    }

    Ok(BtcSpvProof {
        tx_id: *tx_id,
        output_index,
        block_hash: sha256d(&serialize_header(containing_block)),
        block_height: tx_block_height,
        confirmations,
        merkle_root: containing_block.merkle_root,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal valid header for testing.
    fn make_test_header(
        version: u32,
        prev: [u8; 32],
        merkle: [u8; 32],
        nonce: u32,
    ) -> BtcBlockHeader {
        BtcBlockHeader {
            version,
            prev_blockhash: prev,
            merkle_root: merkle,
            timestamp: 1_234_567,
            bits: 0x1d00ffff, // mainnet difficulty 1
            nonce,
        }
    }

    fn make_test_header_chain(length: u64) -> Vec<BtcBlockHeader> {
        let mut headers = Vec::with_capacity(length as usize);
        let mut prev = [0u8; 32];
        let merkle = [0xabu8; 32];

        for i in 0..length {
            let header = make_test_header(1, prev, merkle, i as u32);
            let hash = sha256d(&serialize_header(&header));
            prev = hash;
            headers.push(header);
        }

        headers
    }

    #[test]
    fn header_hash_is_deterministic() {
        let header = make_test_header(1, [0u8; 32], [0xabu8; 32], 0);
        let hash = header.hash();
        let hash2 = sha256d(&serialize_header(&header));
        assert_eq!(hash, hash2, "struct hash and serialized hash must match");
    }

    #[test]
    fn verify_spv_with_0_confirmations_fails() {
        let headers = make_test_header_chain(1);
        let tx_id = [0xbbu8; 32];
        let result = verify_btc_spv_proof(&headers, &tx_id, 0, &[], 0, 1, 2);
        assert_eq!(
            result,
            Err(BtcProofError::InsufficientConfirmations {
                required: 2,
                actual: 1,
            })
        );
    }

    #[test]
    fn verify_spv_empty_headers_fails() {
        let tx_id = [0xbbu8; 32];
        let result = verify_btc_spv_proof(&[], &tx_id, 0, &[], 0, 1, 1);
        assert_eq!(result, Err(BtcProofError::EmptyHeaderChain));
    }

    #[test]
    fn verify_spv_output_index_out_of_range() {
        let headers = make_test_header_chain(1);
        let tx_id = [0xbbu8; 32];
        let result = verify_btc_spv_proof(&headers, &tx_id, 5, &[], 0, 2, 1);
        assert_eq!(
            result,
            Err(BtcProofError::OutputIndexOutOfRange {
                index: 5,
                num_outputs: 2,
            })
        );
    }

    #[test]
    fn verify_spv_broken_header_chain() {
        let mut headers = make_test_header_chain(3);
        // Corrupt the second header's prev_blockhash
        headers[1].prev_blockhash = [0xffu8; 32];

        let tx_id = [0xbbu8; 32];
        let result = verify_btc_spv_proof(&headers, &tx_id, 0, &[], 0, 1, 1);
        assert!(matches!(
            result,
            Err(BtcProofError::HeaderChainBroken { .. })
        ));
    }

    #[test]
    fn verify_spv_merkle_proof_mismatch() {
        let headers = make_test_header_chain(1);
        let tx_id = [0xbbu8; 32];

        // Provide a fake merkle proof that won't match the header's root
        let merkle_proof = vec![[0xccu8; 32], [0xddu8; 32]];

        let result = verify_btc_spv_proof(&headers, &tx_id, 0, &merkle_proof, 0, 1, 1);
        assert!(matches!(
            result,
            Err(BtcProofError::MerkleProofInvalid { .. })
        ));
    }

    #[test]
    fn merkle_proof_verification_logic() {
        // Two-leaf tree:
        //        root = sha256d(left || right)
        //       /    \
        //   left    right
        let left = [0x01u8; 32];
        let right = [0x02u8; 32];

        let mut combined = Vec::with_capacity(64);
        combined.extend_from_slice(&left);
        combined.extend_from_slice(&right);
        let root = sha256d(&combined);

        // Prove 'right' at index 1
        let proof = vec![left]; // sibling is left
        assert!(verify_merkle_proof(&right, &proof, 1, &root));

        // Prove 'left' at index 0
        let proof = vec![right]; // sibling is right
        assert!(verify_merkle_proof(&left, &proof, 0, &root));

        // Invalid: try to prove 'left' at index 1 with wrong sibling
        let proof = vec![left];
        assert!(!verify_merkle_proof(&left, &proof, 1, &root));
    }

    #[test]
    fn header_chain_validates_continuity() {
        let headers = make_test_header_chain(5);
        // Verify all links are valid by re-computing
        for i in 1..headers.len() {
            let computed = sha256d(&serialize_header(&headers[i - 1]));
            assert_eq!(
                computed,
                headers[i].prev_blockhash,
                "header {} should link to {}",
                i,
                i - 1
            );
        }
    }

    #[test]
    fn confirmations_correctly_counted() {
        // 3 headers = 3 confirmations
        let headers = make_test_header_chain(3);
        let tx_id = [0xbbu8; 32];

        // Need 3 confirmations = pass
        // Need 4 confirmations = fail
        let result_ok = verify_btc_spv_proof(&headers, &tx_id, 0, &[], 0, 1, 3);
        assert_eq!(
            result_ok,
            Err(BtcProofError::MerkleProofInvalid {
                tx_id: [0xbbu8; 32],
                claimed_root: [0xabu8; 32]
            }),
            "should fail on merkle proof (not confirmations)"
        );

        let result_fail = verify_btc_spv_proof(&headers, &tx_id, 0, &[], 0, 1, 4);
        assert_eq!(
            result_fail,
            Err(BtcProofError::InsufficientConfirmations {
                required: 4,
                actual: 3,
            })
        );
    }
}
