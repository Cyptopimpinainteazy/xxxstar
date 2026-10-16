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
//! This is a **stateless** check: the prover supplies the headers, so what it proves is
//! that someone did real work on top of the transaction — every header must meet its own
//! target, no target may be easier than the caller's `max_target_bits` (the difficulty the
//! caller knows Bitcoin to be at), and the targets may not change between consecutive
//! headers off a retarget boundary. That bounds the cost of a forgery at the caller's
//! difficulty times the confirmations required; it does **not** show the headers are on
//! Bitcoin's best chain. Settlement that moves value verifies against an anchored header
//! store instead (`x3_btc_spv::LightClient`, the settlement engine's checkpointed chain).
//!
//! The previous version checked only that headers linked, never that any of them carried
//! proof of work, and applied the Merkle path at the *block height* rather than the
//! transaction's position in the block. Linked headers with no work cost nothing to make.

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
    /// The same header in `x3-btc-spv`'s form, which hashes and checks it.
    pub fn to_spv(&self) -> x3_btc_spv::BlockHeader {
        x3_btc_spv::BlockHeader {
            version: self.version as i32,
            prev_blockhash: self.prev_blockhash,
            merkle_root: self.merkle_root,
            time: self.timestamp,
            bits: self.bits,
            nonce: self.nonce,
        }
    }

    /// Compute the SHA-256d (double SHA-256) hash of this block header.
    ///
    /// The bytes returned are the digest in **wire (internal) order** — the same bytes
    /// `x3-settlement-engine`'s `compute_btc_block_hash` and `x3-bitcoin-vault`'s
    /// `BitcoinBlockHeader::block_hash` return, and the same bytes a header's `prev_blockhash` field
    /// carries. Bitcoin *displays* a block hash as the reverse of this (`000000000019d668…` for the
    /// genesis block); anything that prints a hash has to reverse it, and anything that compares one
    /// must not. The previous doc comment here claimed display order, which the code has never
    /// returned — a comment that would have had a reader reverse a hash before comparing it.
    pub fn hash(&self) -> [u8; 32] {
        self.to_spv().block_hash()
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
    /// A header's hash does not meet its own target, or its target exceeds Bitcoin's limit.
    InsufficientWork { height: u64 },
    /// A header's target is easier than the caller's floor, or changes off a retarget boundary.
    DifficultyTooLow { height: u64, bits: u32 },
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
            Self::InsufficientWork { height } => {
                write!(
                    f,
                    "BTC proof: header at height {} lacks proof of work",
                    height
                )
            }
            Self::DifficultyTooLow { height, bits } => write!(
                f,
                "BTC proof: header at height {} has target {:#010x}, easier than allowed",
                height, bits
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BtcProofError {}

/// Verify a BTC SPV proof.
///
/// # Arguments
///
/// * `headers` - Consecutive block headers, oldest first. `headers[0]` contains the
///   transaction; each later header confirms it once more.
/// * `tx_id` - The Bitcoin transaction ID (double-SHA256 digest, internal byte order).
/// * `tx_index` - The transaction's position in its block.
/// * `output_index` - The output index to prove (the UTXO).
/// * `merkle_proof` - Sibling hashes from the transaction up to the Merkle root.
/// * `tx_block_height` - The containing block's height.
/// * `num_outputs` - Number of outputs in the transaction.
/// * `required_confirmations` - Minimum confirmations required.
/// * `max_target_bits` - The easiest target (as `nBits`) a header may carry: the difficulty
///   the caller knows Bitcoin to be at.
///
/// # Errors
///
/// Returns `BtcProofError` if any validation step fails.
#[allow(clippy::too_many_arguments)]
pub fn verify_btc_spv_proof(
    headers: &[BtcBlockHeader],
    tx_id: &[u8; 32],
    tx_index: u32,
    output_index: u32,
    merkle_proof: &[[u8; 32]],
    tx_block_height: u64,
    num_outputs: u32,
    required_confirmations: u64,
    max_target_bits: u32,
) -> Result<BtcSpvProof, BtcProofError> {
    if headers.is_empty() {
        return Err(BtcProofError::EmptyHeaderChain);
    }
    if output_index >= num_outputs {
        return Err(BtcProofError::OutputIndexOutOfRange {
            index: output_index,
            num_outputs,
        });
    }
    let confirmations = headers.len() as u64;
    if confirmations < required_confirmations {
        return Err(BtcProofError::InsufficientConfirmations {
            required: required_confirmations,
            actual: confirmations,
        });
    }

    let floor = x3_btc_spv::target_from_bits(max_target_bits)
        .map_err(|_| BtcProofError::InvalidHeaderData)?;
    let mainnet = x3_btc_spv::Params::mainnet();
    for (i, header) in headers.iter().enumerate() {
        let height = tx_block_height
            .checked_add(i as u64)
            .ok_or(BtcProofError::ArithmeticOverflow)?;
        let spv = header.to_spv();
        x3_btc_spv::check_proof_of_work(&spv, &mainnet)
            .map_err(|_| BtcProofError::InsufficientWork { height })?;
        let target = x3_btc_spv::target_from_bits(header.bits)
            .map_err(|_| BtcProofError::InvalidHeaderData)?;
        if target > floor {
            return Err(BtcProofError::DifficultyTooLow {
                height,
                bits: header.bits,
            });
        }
        if i > 0 {
            let parent = &headers[i - 1];
            let computed_hash = parent.to_spv().block_hash();
            if computed_hash != header.prev_blockhash {
                return Err(BtcProofError::HeaderChainBroken {
                    expected_prev: header.prev_blockhash,
                    actual_hash: computed_hash,
                    height: height - 1,
                });
            }
            if !height.is_multiple_of(mainnet.retarget_interval) && header.bits != parent.bits {
                return Err(BtcProofError::DifficultyTooLow {
                    height,
                    bits: header.bits,
                });
            }
        }
    }

    let containing_block = &headers[0];
    x3_btc_spv::verify_merkle_inclusion(
        tx_id,
        tx_index,
        merkle_proof,
        &containing_block.merkle_root,
    )
    .map_err(|_| BtcProofError::MerkleProofInvalid {
        tx_id: *tx_id,
        claimed_root: containing_block.merkle_root,
    })?;

    Ok(BtcSpvProof {
        tx_id: *tx_id,
        output_index,
        block_hash: containing_block.to_spv().block_hash(),
        block_height: tx_block_height,
        confirmations,
        merkle_root: containing_block.merkle_root,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    fn display(hex: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        out.reverse();
        out
    }

    /// Bitcoin mainnet blocks 0, 1 and 2. Each is a one-transaction block, so its Merkle
    /// root is its coinbase's txid.
    fn mainnet_0_to_2() -> Vec<BtcBlockHeader> {
        let genesis = BtcBlockHeader {
            version: 1,
            prev_blockhash: [0; 32],
            merkle_root: display(
                "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
            ),
            timestamp: 1231006505,
            bits: 0x1d00ffff,
            nonce: 2083236893,
        };
        let one = BtcBlockHeader {
            version: 1,
            prev_blockhash: genesis.to_spv().block_hash(),
            merkle_root: display(
                "0e3e2357e806b6cdb1f70b54c3a3a17b6714ee1f0e68bebb44a74b1efd512098",
            ),
            timestamp: 1231469665,
            bits: 0x1d00ffff,
            nonce: 2573394689,
        };
        let two = BtcBlockHeader {
            version: 1,
            prev_blockhash: one.to_spv().block_hash(),
            merkle_root: display(
                "9b0fc92260312ce44e74ef369f5c66bbb85848f2eddd5a7a1cde251e54ccfdd5",
            ),
            timestamp: 1231469744,
            bits: 0x1d00ffff,
            nonce: 1639830024,
        };
        vec![genesis, one, two]
    }

    const DIFFICULTY_1: u32 = 0x1d00ffff;

    #[test]
    fn header_hash_matches_the_published_genesis_id() {
        let headers = mainnet_0_to_2();
        assert_eq!(
            headers[0].hash(),
            display("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f")
        );
    }

    #[test]
    fn the_genesis_coinbase_is_proven_under_two_real_confirmations() {
        let headers = mainnet_0_to_2();
        let coinbase = headers[0].merkle_root;
        let proof = verify_btc_spv_proof(&headers, &coinbase, 0, 0, &[], 0, 1, 3, DIFFICULTY_1)
            .expect("real blocks prove a real transaction");
        assert_eq!(proof.confirmations, 3);
        assert_eq!(proof.block_hash, headers[0].hash());
    }

    #[test]
    fn too_few_headers_are_refused() {
        let headers = mainnet_0_to_2();
        let coinbase = headers[0].merkle_root;
        assert_eq!(
            verify_btc_spv_proof(&headers, &coinbase, 0, 0, &[], 0, 1, 4, DIFFICULTY_1),
            Err(BtcProofError::InsufficientConfirmations {
                required: 4,
                actual: 3
            })
        );
        assert_eq!(
            verify_btc_spv_proof(&[], &coinbase, 0, 0, &[], 0, 1, 1, DIFFICULTY_1),
            Err(BtcProofError::EmptyHeaderChain)
        );
    }

    #[test]
    fn linked_headers_without_work_are_refused() {
        // What the previous version accepted: headers that link and carry no work.
        let mut headers = Vec::new();
        let mut prev = [0u8; 32];
        for nonce in 0..3 {
            let header = BtcBlockHeader {
                version: 1,
                prev_blockhash: prev,
                merkle_root: [0xab; 32],
                timestamp: 1_234_567,
                bits: DIFFICULTY_1,
                nonce,
            };
            prev = header.hash();
            headers.push(header);
        }
        assert_eq!(
            verify_btc_spv_proof(&headers, &[0xab; 32], 0, 0, &[], 0, 1, 3, DIFFICULTY_1),
            Err(BtcProofError::InsufficientWork { height: 0 })
        );
    }

    #[test]
    fn a_floor_harder_than_the_headers_refuses_them() {
        let headers = mainnet_0_to_2();
        assert_eq!(
            verify_btc_spv_proof(
                &headers,
                &headers[0].merkle_root,
                0,
                0,
                &[],
                0,
                1,
                1,
                0x1b0404cb
            ),
            Err(BtcProofError::DifficultyTooLow {
                height: 0,
                bits: DIFFICULTY_1
            })
        );
    }

    #[test]
    fn a_broken_link_and_a_wrong_position_are_refused() {
        let mut headers = mainnet_0_to_2();
        let coinbase = headers[0].merkle_root;
        assert!(matches!(
            verify_btc_spv_proof(&headers, &coinbase, 1, 0, &[], 0, 1, 1, DIFFICULTY_1),
            Err(BtcProofError::MerkleProofInvalid { .. })
        ));
        headers.swap(1, 2);
        assert!(matches!(
            verify_btc_spv_proof(&headers, &coinbase, 0, 0, &[], 0, 1, 1, DIFFICULTY_1),
            Err(BtcProofError::HeaderChainBroken { height: 0, .. })
        ));
    }

    #[test]
    fn output_index_out_of_range_is_refused() {
        let headers = mainnet_0_to_2();
        assert_eq!(
            verify_btc_spv_proof(
                &headers,
                &headers[0].merkle_root,
                0,
                5,
                &[],
                0,
                2,
                1,
                DIFFICULTY_1
            ),
            Err(BtcProofError::OutputIndexOutOfRange {
                index: 5,
                num_outputs: 2
            })
        );
    }
}
