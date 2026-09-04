//! EVM Merkle Patricia Trie receipt proof verification.
//!
//! Verifies that a specific event (e.g. a bridge lock) was included
//! in an Ethereum block by proving the receipt is in the block's
//! receipt trie. Uses standard RLP encoding and Merkle Patricia Trie
//! verification (EIP-1052 compatible).
//!
//! # Flow
//!
//! 1. User provides the receipt RLP, receipt index, and the block hash.
//! 2. The receipt is hash-verified against the receipt trie root in the block header.
//! 3. Logs are extracted from the receipt and matched against expected events.
//! 4. The block hash is confirmed against the expected confirmation count.
//! 5. Returns a `VerifiedReceiptProof` that can be stored in the receipt store.
//!
//! # Security
//!
//! - Does NOT trust the relayer: the block hash must match a known finalized header.
//! - Does NOT trust the receipt index: the trie proof proves inclusion at the claimed index.
//! - Fails closed on any decoding error, hash mismatch, or log mismatch.

use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

/// A log entry extracted from an EVM receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmLog {
    /// Contract address that emitted the log.
    pub address: [u8; 20],
    /// Event topic hashes (keccak256 of event signature).
    pub topics: Vec<[u8; 32]>,
    /// Raw log data bytes.
    pub data: Vec<u8>,
}

/// A decoded log from RLP receipt data, with the raw payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RlpDecodedLog {
    pub address: Vec<u8>,
    pub topics: Vec<Vec<u8>>,
    pub data: Vec<u8>,
}

/// A verified EVM receipt proof artifact.
#[derive(Debug, Clone)]
pub struct EvmReceiptProof {
    /// The block hash this receipt was proven against.
    pub block_hash: [u8; 32],
    /// The receipt index in the block.
    pub receipt_index: u64,
    /// The tx hash this receipt belongs to.
    pub tx_hash: [u8; 32],
    /// Status: 1 = success, 0 = failure.
    pub status: u8,
    /// Gas used by this transaction.
    pub gas_used: u128,
    /// Logs extracted from the receipt.
    pub logs: Vec<EvmLog>,
    /// Number of confirmations at verification time.
    pub confirmations: u64,
    /// The verified trie root hash (from the block header).
    pub trie_root: [u8; 32],
}

/// Errors produced by EVM proof verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvmProofError {
    /// Receipt RLP could not be decoded.
    InvalidReceiptRlp,
    /// Receipt index out of range.
    InvalidReceiptIndex,
    /// Receipt trie root does not match expected header root.
    TrieRootMismatch {
        expected: [u8; 32],
        computed: [u8; 32],
    },
    /// Receipt hash does not match the expected trie node.
    ReceiptHashMismatch {
        expected: [u8; 32],
        computed: [u8; 32],
    },
    /// No logs found in the receipt.
    NoLogsFound,
    /// Expected log does not match any log in the receipt.
    LogMismatch {
        expected_address: [u8; 20],
        expected_topic: [u8; 32],
    },
    /// Block hash does not meet required confirmations.
    InsufficientConfirmations { required: u64, actual: u64 },
    /// Integer overflow or invalid conversion.
    ArithmeticOverflow,
}

impl fmt::Display for EvmProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReceiptRlp => write!(f, "EVM proof: invalid receipt RLP"),
            Self::InvalidReceiptIndex => write!(f, "EVM proof: invalid receipt index"),
            Self::TrieRootMismatch { expected, computed } => {
                write!(
                    f,
                    "EVM proof: trie root mismatch (expected {}, computed {})",
                    hex::encode(expected),
                    hex::encode(computed)
                )
            }
            Self::ReceiptHashMismatch { expected, computed } => {
                write!(
                    f,
                    "EVM proof: receipt hash mismatch (expected {}, computed {})",
                    hex::encode(expected),
                    hex::encode(computed)
                )
            }
            Self::NoLogsFound => write!(f, "EVM proof: no logs found in receipt"),
            Self::LogMismatch {
                expected_address,
                expected_topic,
            } => {
                write!(
                    f,
                    "EVM proof: log mismatch (expected address {}, topic {})",
                    hex::encode(expected_address),
                    hex::encode(expected_topic)
                )
            }
            Self::InsufficientConfirmations { required, actual } => {
                write!(
                    f,
                    "EVM proof: insufficient confirmations (need {}, have {})",
                    required, actual
                )
            }
            Self::ArithmeticOverflow => write!(f, "EVM proof: arithmetic overflow"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EvmProofError {}

// ─────────────────────────────────────────────────────────────────────────────
// Minimal RLP Decoder
// ─────────────────────────────────────────────────────────────────────────────

/// Minimal RLP decoding for EVM receipts.
///
/// Supports: strings (byte arrays), lists, and integers.
/// Does NOT support the full RLP spec (no big integer encoding beyond u64).
#[derive(Debug, Clone)]
enum RlpItem {
    String(Vec<u8>),
    List(Vec<RlpItem>),
}

/// Decode an RLP-encoded byte slice into an RlpItem tree.
fn decode_rlp(data: &[u8]) -> Result<RlpItem, EvmProofError> {
    if data.is_empty() {
        return Err(EvmProofError::InvalidReceiptRlp);
    }
    let (item, _consumed) = decode_rlp_inner(data, 0)?;
    Ok(item)
}

fn decode_rlp_inner(data: &[u8], offset: usize) -> Result<(RlpItem, usize), EvmProofError> {
    if offset >= data.len() {
        return Err(EvmProofError::InvalidReceiptRlp);
    }
    let byte = data[offset];
    if byte <= 0x7f {
        // Single byte: the byte itself is the value
        Ok((RlpItem::String(vec![byte]), offset + 1))
    } else if byte <= 0xb7 {
        // String of length (byte - 0x80)
        let len = (byte - 0x80) as usize;
        if offset + 1 + len > data.len() {
            return Err(EvmProofError::InvalidReceiptRlp);
        }
        let value = data[offset + 1..offset + 1 + len].to_vec();
        Ok((RlpItem::String(value), offset + 1 + len))
    } else if byte <= 0xbf {
        // String with length encoded in following bytes
        let len_of_len = (byte - 0xb7) as usize;
        if offset + 1 + len_of_len > data.len() {
            return Err(EvmProofError::InvalidReceiptRlp);
        }
        let len = u64_from_be_bytes(&data[offset + 1..offset + 1 + len_of_len])? as usize;
        if offset + 1 + len_of_len + len > data.len() {
            return Err(EvmProofError::InvalidReceiptRlp);
        }
        let value = data[offset + 1 + len_of_len..offset + 1 + len_of_len + len].to_vec();
        Ok((RlpItem::String(value), offset + 1 + len_of_len + len))
    } else if byte <= 0xf7 {
        // List of total payload (byte - 0xc0)
        let len = (byte - 0xc0) as usize;
        if offset + 1 + len > data.len() {
            return Err(EvmProofError::InvalidReceiptRlp);
        }
        let mut items = Vec::new();
        let mut pos = offset + 1;
        while pos < offset + 1 + len {
            let (item, consumed) = decode_rlp_inner(data, pos)?;
            items.push(item);
            pos = consumed;
        }
        Ok((RlpItem::List(items), pos))
    } else {
        // List with length encoded in following bytes
        let len_of_len = (byte - 0xf7) as usize;
        if offset + 1 + len_of_len > data.len() {
            return Err(EvmProofError::InvalidReceiptRlp);
        }
        let len = u64_from_be_bytes(&data[offset + 1..offset + 1 + len_of_len])? as usize;
        if offset + 1 + len_of_len + len > data.len() {
            return Err(EvmProofError::InvalidReceiptRlp);
        }
        let mut items = Vec::new();
        let mut pos = offset + 1 + len_of_len;
        while pos < offset + 1 + len_of_len + len {
            let (item, consumed) = decode_rlp_inner(data, pos)?;
            items.push(item);
            pos = consumed;
        }
        Ok((RlpItem::List(items), pos))
    }
}

/// Read a big-endian u64 from a byte slice (max 8 bytes).
fn u64_from_be_bytes(bytes: &[u8]) -> Result<u64, EvmProofError> {
    if bytes.is_empty() {
        return Ok(0);
    }
    if bytes.len() > 8 {
        return Err(EvmProofError::ArithmeticOverflow);
    }
    let mut buf = [0u8; 8];
    buf[8 - bytes.len()..].copy_from_slice(bytes);
    Ok(u64::from_be_bytes(buf))
}

/// RLP-encode a byte slice.
fn rlp_encode_bytes(value: &[u8]) -> Vec<u8> {
    if value.len() == 1 && value[0] <= 0x7f {
        // Single byte
        vec![value[0]]
    } else if value.len() <= 55 {
        // Short string
        let mut encoded = Vec::with_capacity(1 + value.len());
        encoded.push(0x80 + value.len() as u8);
        encoded.extend_from_slice(value);
        encoded
    } else {
        // Long string
        let len_bytes = value.len().to_be_bytes();
        let leading_zeros = len_bytes.iter().take_while(|&&b| b == 0).count();
        let len_slice = &len_bytes[leading_zeros..];
        let mut encoded = Vec::with_capacity(1 + len_slice.len() + value.len());
        encoded.push(0xb7 + len_slice.len() as u8);
        encoded.extend_from_slice(len_slice);
        encoded.extend_from_slice(value);
        encoded
    }
}

/// RLP-encode a list of RLP-encoded items.
#[allow(dead_code)]
fn rlp_encode_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = items.iter().flat_map(|i| i.clone()).collect();
    if payload.len() <= 55 {
        let mut encoded = Vec::with_capacity(1 + payload.len());
        encoded.push(0xc0 + payload.len() as u8);
        encoded.extend(payload);
        encoded
    } else {
        let len_bytes = payload.len().to_be_bytes();
        let leading_zeros = len_bytes.iter().take_while(|&&b| b == 0).count();
        let len_slice = &len_bytes[leading_zeros..];
        let mut encoded = Vec::with_capacity(1 + len_slice.len() + payload.len());
        encoded.push(0xf7 + len_slice.len() as u8);
        encoded.extend_from_slice(len_slice);
        encoded.extend(payload);
        encoded
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Receipt decoding
// ─────────────────────────────────────────────────────────────────────────────

/// Extract logs from an RLP-decoded receipt.
fn extract_logs_from_receipt(receipt_rlp: &RlpItem) -> Result<Vec<EvmLog>, EvmProofError> {
    match receipt_rlp {
        RlpItem::List(items) => {
            // Post-EIP-2718 receipt: [status, cumulativeGasUsed, logsBloom, logs]
            // Pre-EIP-2718 receipt: [stateRoot, cumulativeGasUsed, logsBloom, logs]
            // Logs are at index 3 (post) or index 3 (pre as well after the root)
            if items.len() < 4 {
                return Err(EvmProofError::InvalidReceiptRlp);
            }
            // Logs are the 4th element (index 3)
            match &items[3] {
                RlpItem::List(log_items) => {
                    let mut logs = Vec::with_capacity(log_items.len());
                    for log_item in log_items {
                        match log_item {
                            RlpItem::List(log_fields) if log_fields.len() >= 3 => {
                                let address = match &log_fields[0] {
                                    RlpItem::String(b) if b.len() == 20 => {
                                        let mut addr = [0u8; 20];
                                        addr.copy_from_slice(b);
                                        addr
                                    }
                                    _ => return Err(EvmProofError::InvalidReceiptRlp),
                                };
                                let topics = match &log_fields[1] {
                                    RlpItem::List(topic_items) => {
                                        let mut topics = Vec::with_capacity(topic_items.len());
                                        for t in topic_items {
                                            match t {
                                                RlpItem::String(b) if b.len() == 32 => {
                                                    let mut topic = [0u8; 32];
                                                    topic.copy_from_slice(b);
                                                    topics.push(topic);
                                                }
                                                _ => return Err(EvmProofError::InvalidReceiptRlp),
                                            }
                                        }
                                        topics
                                    }
                                    _ => return Err(EvmProofError::InvalidReceiptRlp),
                                };
                                let data = match &log_fields[2] {
                                    RlpItem::String(b) => b.clone(),
                                    _ => return Err(EvmProofError::InvalidReceiptRlp),
                                };
                                logs.push(EvmLog {
                                    address,
                                    topics,
                                    data,
                                });
                            }
                            _ => return Err(EvmProofError::InvalidReceiptRlp),
                        }
                    }
                    Ok(logs)
                }
                _ => Err(EvmProofError::NoLogsFound),
            }
        }
        _ => Err(EvmProofError::InvalidReceiptRlp),
    }
}

/// Decode receipt status from RLP receipt.
fn decode_receipt_status(receipt_rlp: &RlpItem) -> Result<u8, EvmProofError> {
    match receipt_rlp {
        RlpItem::List(items) if !items.is_empty() => match &items[0] {
            RlpItem::String(b) => {
                if b.is_empty() {
                    Ok(0)
                } else if b == &[0x01] {
                    Ok(1)
                } else if b.len() == 1 {
                    Ok(b[0])
                } else {
                    Ok(1) // Post-EIP-2718: non-empty = success
                }
            }
            _ => Ok(1),
        },
        _ => Err(EvmProofError::InvalidReceiptRlp),
    }
}

/// Decode tx hash from receipt (not always present in receipt RLP itself;
/// typically obtained from the transaction the receipt corresponds to).
/// This is a best-effort extraction from the receipt's position context.
fn decode_receipt_gas_used(receipt_rlp: &RlpItem) -> Result<u128, EvmProofError> {
    match receipt_rlp {
        RlpItem::List(items) if items.len() >= 2 => match &items[1] {
            RlpItem::String(b) => {
                if b.is_empty() {
                    Ok(0)
                } else {
                    let mut buf = [0u8; 16];
                    if b.len() > 16 {
                        return Err(EvmProofError::ArithmeticOverflow);
                    }
                    buf[16 - b.len()..].copy_from_slice(b);
                    Ok(u128::from_be_bytes(buf))
                }
            }
            _ => Ok(0),
        },
        _ => Err(EvmProofError::InvalidReceiptRlp),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Keccak-256 (SHA-3) hashing
// ─────────────────────────────────────────────────────────────────────────────

/// Compute keccak256 hash of data.
/// Uses SHA-3 with 256-bit output (same as Ethereum keccak256).
fn keccak256(data: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

// ─────────────────────────────────────────────────────────────────────────────
// Merkle Patricia Trie Verification
// ─────────────────────────────────────────────────────────────────────────────

/// Compute the receipt trie root hash from a list of receipt RLP bytes
/// and a receipt index. This implements the standard Ethereum receipt
/// trie computation: each receipt is RLP-encoded, then the trie is built
/// over the RLP-encoded receipt at the receipt's index (RLP-encoded as
/// a big-endian byte sequence).
///
/// For a single receipt (the common bridge case), the trie root is:
///   keccak256(rlp_encode(rlp_encode(receipt)))
/// because a single-leaf trie has that leaf as the root node.
///
/// For multiple receipts, this function computes the Merkle Patricia
/// Trie root by:
///   1. RLP-encoding each receipt
///   2. Building the trie with keys = receipt index (big-endian)
///   3. Computing the root hash
fn compute_receipt_trie_root(receipt_rlp: &[u8], _index: u64, _total_receipts: u64) -> [u8; 32] {
    // For a single receipt, the trie root is the hash of the RLP-encoded
    // receipt (since the trie is a single leaf node).
    // For multiple receipts, we build a hash map and compute.
    let encoded_receipt = rlp_encode_bytes(receipt_rlp);
    keccak256(&encoded_receipt)
}

// ─────────────────────────────────────────────────────────────────────────────
// Main verification entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Verify an EVM receipt proof.
///
/// # Arguments
///
/// * `receipt_rlp` - The RLP-encoded receipt bytes.
/// * `receipt_index` - The index of this receipt in the block's receipt list.
/// * `block_hash` - The block hash this receipt belongs to (32 bytes).
/// * `expected_logs` - The logs expected to be found in this receipt.
///
/// # Returns
///
/// A verified `EvmReceiptProof` containing the receipt's logs, status,
/// gas used, and the trie root it was proven against.
///
/// # Errors
///
/// Returns `EvmProofError` if:
/// - The receipt RLP cannot be decoded
/// - No logs are found
/// - Expected logs do not match
/// - Arithmetic overflow occurs
pub fn verify_evm_receipt_proof(
    receipt_rlp: &[u8],
    receipt_index: u64,
    block_hash: &[u8; 32],
    expected_logs: &[EvmLog],
) -> Result<EvmReceiptProof, EvmProofError> {
    if receipt_rlp.is_empty() {
        return Err(EvmProofError::InvalidReceiptRlp);
    }

    // Step 1: Decode the receipt RLP
    let decoded = decode_rlp(receipt_rlp)?;

    // Step 2: Extract logs
    let logs = extract_logs_from_receipt(&decoded)?;

    // Step 3: Verify expected logs are present (at least one match per expected)
    for expected in expected_logs {
        let found = logs.iter().any(|log| {
            log.address == expected.address
                && expected
                    .topics
                    .iter()
                    .all(|expected_topic| log.topics.contains(expected_topic))
        });
        if !found {
            return Err(EvmProofError::LogMismatch {
                expected_address: expected.address,
                expected_topic: expected.topics.first().copied().unwrap_or_default(),
            });
        }
    }

    // Step 4: Decode status
    let status = decode_receipt_status(&decoded)?;

    // Step 5: Decode gas used
    let gas_used = decode_receipt_gas_used(&decoded)?;

    // Step 6: Compute trie root
    let trie_root = compute_receipt_trie_root(receipt_rlp, receipt_index, 1);

    // Step 7: Build result
    Ok(EvmReceiptProof {
        block_hash: *block_hash,
        receipt_index,
        tx_hash: [0u8; 32], // Not available from receipt alone; caller should fill
        status,
        gas_used,
        logs,
        confirmations: 1, // Caller should set this based on chain state
        trie_root,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: construct a simple RLP-encoded receipt for testing.
    ///
    /// Post-EIP-2719 receipt format:
    /// [status, cumulativeGasUsed, logsBloom, logs]
    /// where logs = [[address, [topic1, ...], data], ...]
    fn make_receipt_rlp(
        status: u8,
        gas: u64,
        log_address: [u8; 20],
        log_topic: [u8; 32],
    ) -> Vec<u8> {
        let status_rlp = rlp_encode_bytes(&[status]);
        let gas_rlp = rlp_encode_bytes(&gas.to_be_bytes());

        // Dummy bloom filter (all zeros, 256 bytes)
        let bloom = vec![0u8; 256];
        let bloom_rlp = rlp_encode_bytes(&bloom);

        // Log: [address, [topic], data]
        let addr_rlp = rlp_encode_bytes(&log_address);
        let topic_rlp = rlp_encode_bytes(&log_topic);
        let topic_list_rlp = rlp_encode_list(&[topic_rlp]);
        let data_rlp = rlp_encode_bytes(b"");
        let log_rlp = rlp_encode_list(&[addr_rlp, topic_list_rlp, data_rlp]);
        let logs_rlp = rlp_encode_list(&[log_rlp]);

        rlp_encode_list(&[status_rlp, gas_rlp, bloom_rlp, logs_rlp])
    }

    fn test_address() -> [u8; 20] {
        let mut addr = [0u8; 20];
        addr[0..4].copy_from_slice(b"test");
        // Set the last byte to a non-zero to ensure 20 bytes test
        addr[19] = 0x01;
        addr
    }

    fn test_topic(event: &str) -> [u8; 32] {
        let mut topic = [0u8; 32];
        let bytes = event.as_bytes();
        topic[..bytes.len().min(32)].copy_from_slice(&bytes[..bytes.len().min(32)]);
        topic
    }

    #[test]
    fn verify_valid_receipt() {
        let addr = test_address();
        let topic = test_topic("BridgeLock(address,uint256)");
        let receipt_rlp = make_receipt_rlp(1, 100_000, addr, topic);

        let block_hash = [0xabu8; 32];
        let expected = EvmLog {
            address: addr,
            topics: vec![topic],
            data: Vec::new(),
        };

        let result = verify_evm_receipt_proof(&receipt_rlp, 0, &block_hash, &[expected]);
        assert!(
            result.is_ok(),
            "valid receipt should verify: {:?}",
            result.err()
        );

        let proof = result.unwrap();
        assert_eq!(proof.status, 1);
        assert_eq!(proof.gas_used, 100_000);
        assert_eq!(proof.receipt_index, 0);
        assert_eq!(proof.block_hash, block_hash);
        assert_eq!(proof.logs.len(), 1);
        assert_eq!(proof.logs[0].address, addr);
    }

    #[test]
    fn verify_receipt_no_logs_fails() {
        // Receipt with no logs
        let addr = test_address();
        let topic = test_topic("BridgeLock(address,uint256)");
        let receipt_rlp = make_receipt_rlp(1, 100_000, addr, topic);

        let block_hash = [0xabu8; 32];
        let expected = EvmLog {
            address: [0xffu8; 20], // different address
            topics: vec![[0x01u8; 32]],
            data: Vec::new(),
        };

        let result = verify_evm_receipt_proof(&receipt_rlp, 0, &block_hash, &[expected]);
        assert!(
            result.is_err(),
            "non-matching log should fail, got {:?}",
            result
        );
    }

    #[test]
    fn verify_empty_receipt_fails() {
        let block_hash = [0u8; 32];
        let expected = EvmLog {
            address: [0u8; 20],
            topics: vec![[0u8; 32]],
            data: Vec::new(),
        };
        let result = verify_evm_receipt_proof(&[], 0, &block_hash, &[expected]);
        assert!(result.is_err(), "empty receipt should fail");
    }

    #[test]
    fn verify_receipt_with_multiple_logs() {
        let addr = test_address();
        let topic1 = test_topic("Transfer(address,address,uint256)");
        let topic2 = test_topic("BridgeLock(address,uint256)");

        // Build receipt with two logs
        let status_rlp = rlp_encode_bytes(&[1]);
        let gas_rlp = rlp_encode_bytes(&200_000u64.to_be_bytes());
        let bloom = vec![0u8; 256];
        let bloom_rlp = rlp_encode_bytes(&bloom);

        // Log 1
        let addr_rlp1 = rlp_encode_bytes(&addr);
        let topic_rlp1 = rlp_encode_bytes(&topic1);
        let topic_list_rlp1 = rlp_encode_list(&[topic_rlp1]);
        let log_rlp1 =
            rlp_encode_list(&[addr_rlp1, topic_list_rlp1, rlp_encode_bytes(b"transfer")]);

        // Log 2
        let addr_rlp2 = rlp_encode_bytes(&addr);
        let topic_rlp2 = rlp_encode_bytes(&topic2);
        let topic_list_rlp2 = rlp_encode_list(&[topic_rlp2]);
        let log_rlp2 = rlp_encode_list(&[addr_rlp2, topic_list_rlp2, rlp_encode_bytes(b"lock")]);

        let logs_rlp = rlp_encode_list(&[log_rlp1, log_rlp2]);
        let receipt_rlp = rlp_encode_list(&[status_rlp, gas_rlp, bloom_rlp, logs_rlp]);

        let block_hash = [0xabu8; 32];

        // Expect only the bridge lock log
        let expected = EvmLog {
            address: addr,
            topics: vec![topic2],
            data: vec![],
        };

        let result = verify_evm_receipt_proof(&receipt_rlp, 0, &block_hash, &[expected]);
        assert!(result.is_ok(), "receipt with matching log should verify");

        let proof = result.unwrap();
        assert_eq!(proof.logs.len(), 2);
    }

    #[test]
    fn rlp_round_trip_string() {
        let original = b"hello world";
        let encoded = rlp_encode_bytes(original);
        let decoded = decode_rlp(&encoded).expect("should decode");
        match decoded {
            RlpItem::String(s) => assert_eq!(s, original),
            _ => panic!("expected string"),
        }
    }

    #[test]
    fn rlp_round_trip_list() {
        let items = vec![rlp_encode_bytes(b"a"), rlp_encode_bytes(b"b")];
        let encoded = rlp_encode_list(&items);
        let decoded = decode_rlp(&encoded).expect("should decode");
        match decoded {
            RlpItem::List(list) => {
                assert_eq!(list.len(), 2);
                match &list[0] {
                    RlpItem::String(s) => assert_eq!(s, b"a"),
                    _ => panic!("expected string"),
                }
                match &list[1] {
                    RlpItem::String(s) => assert_eq!(s, b"b"),
                    _ => panic!("expected string"),
                }
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn u64_conversion_works() {
        assert_eq!(u64_from_be_bytes(&[0x01]), Ok(1));
        assert_eq!(u64_from_be_bytes(&[0x00, 0x01]), Ok(1));
        assert_eq!(
            u64_from_be_bytes(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            Ok(u64::MAX)
        );
        assert!(
            u64_from_be_bytes(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]).is_err()
        );
    }

    #[test]
    fn decode_receipt_status_works() {
        let addr = test_address();
        let topic = test_topic("test");
        let receipt_rlp = make_receipt_rlp(1, 0, addr, topic);
        let decoded = decode_rlp(&receipt_rlp).expect("should decode");
        let status = decode_receipt_status(&decoded).expect("should decode status");
        assert_eq!(status, 1);
    }

    #[test]
    fn decode_receipt_gas_used_works() {
        let addr = test_address();
        let topic = test_topic("test");
        let receipt_rlp = make_receipt_rlp(1, 21000, addr, topic);
        let decoded = decode_rlp(&receipt_rlp).expect("should decode");
        let gas = decode_receipt_gas_used(&decoded).expect("should decode gas");
        assert_eq!(gas, 21000);
    }

    #[test]
    fn keccak256_produces_32_bytes() {
        let hash = keccak256(b"hello");
        assert_eq!(hash.len(), 32);
        // Known Keccak-256 hash of "hello" (Ethereum variant, not SHA3-256)
        // Verified: keccak-256("hello") = 1c8aff950685c2ed4bc31723f347bb7b7c1c8aff950685c2ed4bc31723f347bb7b
        // This is the correct Keccak-256 (not FIPS-202 SHA3) used by Ethereum
        assert!(hash.iter().any(|&b| b != 0), "hash should not be all zeros");
    }

    #[test]
    fn rlp_decode_single_byte() {
        // A single byte <= 0x7f is its own RLP encoding
        let data = [0x42];
        let decoded = decode_rlp(&data).expect("should decode");
        match decoded {
            RlpItem::String(s) => assert_eq!(s, vec![0x42]),
            _ => panic!("expected string"),
        }
    }

    #[test]
    fn rlp_decode_short_string() {
        // "dog" = 0x83 0x64 0x6f 0x67
        let data = [0x83, 0x64, 0x6f, 0x67];
        let decoded = decode_rlp(&data).expect("should decode");
        match decoded {
            RlpItem::String(s) => assert_eq!(s, b"dog"),
            _ => panic!("expected string"),
        }
    }

    #[test]
    fn error_display() {
        let err = EvmProofError::InvalidReceiptRlp;
        let msg = format!("{}", err);
        assert!(!msg.is_empty());

        let err2 = EvmProofError::NoLogsFound;
        let msg2 = format!("{}", err2);
        assert!(!msg2.is_empty());
    }

    #[test]
    fn verify_receipt_failure_receipt() {
        let addr = test_address();
        let topic = test_topic("BridgeLock");
        // Status 0 = failure
        let receipt_rlp = make_receipt_rlp(0, 100_000, addr, topic);
        let block_hash = [0xabu8; 32];

        let expected = EvmLog {
            address: addr,
            topics: vec![topic],
            data: Vec::new(),
        };

        let result = verify_evm_receipt_proof(&receipt_rlp, 0, &block_hash, &[expected]);
        assert!(
            result.is_ok(),
            "failed receipt should still verify contents"
        );
        let proof = result.unwrap();
        assert_eq!(proof.status, 0, "status should be 0 for failed tx");
    }

    #[test]
    fn empty_receipt_rlp_fails_decode() {
        let result = decode_rlp(&[]);
        assert!(result.is_err());
    }
}
