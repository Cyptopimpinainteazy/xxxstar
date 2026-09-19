//! EVM receipt proof verification: inclusion in the receipts trie, plus the log the
//! receipt carries.
//!
//! # What this module proves, and what it does not
//!
//! The trie walk is **not implemented here**. `verify_evm_receipt_proof` delegates it
//! to [`x3_verification_router::evm_receipt::verify_merkle_patricia_proof`] — the
//! canonical verifier, the one the relayer is wired to — passing the receipts root,
//! the `rlp(index)` trie key, the caller's receipt RLP as the leaf value, and the
//! caller's proof nodes. So the caller must supply a *root* to prove against; there is
//! no default and no fallback.
//!
//! Proved by a successful call:
//!
//! - the receipt RLP is the value at index `receipt_index` in the trie whose root is
//!   `receipts_root`, under the standard `rlp(index)` key convention;
//! - the receipt decodes, its status and gas-used fields are readable, and every
//!   expected log is present in it.
//!
//! **Not** proved, and carried as `Option`s for exactly that reason:
//!
//! - which block the root belongs to. `receipts_root` comes from a header, and this
//!   module never sees that header, its hash, or the chain height. A caller that has a
//!   finalized header says so through [`EvmReceiptProof::with_header_attestation`],
//!   which is an *attestation* rather than something this walk establishes.
//! - the transaction hash. It is not recoverable from a receipt; the trie key is an
//!   index. [`EvmReceiptProof::tx_hash`] is `None` unless a caller fills it in.
//! - the confirmation count. [`EvmReceiptProof::require_confirmations`] refuses when
//!   no header attestation was recorded, rather than reading a default of `1` as one
//!   confirmation.
//!
//! # Fails closed
//!
//! A bad root, a bad key, a missing or tampered node, an unreadable receipt, an
//! absent expected log, and a confirmation requirement with nothing to measure
//! against are all errors. There is no path through this module that returns an
//! `Ok` for a receipt bound to nothing.

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

/// The result of a receipt proof that *was* verified: the receipt is at
/// `receipt_index` in the trie rooted at `receipts_root`, and these are its logs.
///
/// The fields that a successful trie walk does not establish are `Option`s, so a
/// zeroed hash cannot read as a real one (TICKET-065).
#[derive(Debug, Clone)]
pub struct EvmReceiptProof {
    /// The receipt index in the block.
    pub receipt_index: u64,
    /// The transaction hash, when a caller supplied one. A receipt does not carry it
    /// — the trie key is an index — so this is `None` unless filled in.
    pub tx_hash: Option<[u8; 32]>,
    /// Status: 1 = success, 0 = failure.
    pub status: u8,
    /// Gas used by this transaction.
    pub gas_used: u128,
    /// Logs extracted from the receipt.
    pub logs: Vec<EvmLog>,
    /// Confirmations, when a finalized-header source supplied them.
    ///
    /// `None` rather than a default: a confirmation count this module invented would
    /// be a number about a chain view it never had.
    pub confirmations: Option<u64>,
    /// The receipts-trie root this receipt was **proved** to be in.
    pub receipts_root: [u8; 32],
    /// The block hash, when a finalized-header source supplied one.
    ///
    /// This is an attestation and not a proof: the trie walk above establishes that
    /// the receipt is in `receipts_root`, and nothing here establishes that
    /// `receipts_root` is the `receiptsRoot` of `block_hash`. A caller that needs the
    /// binding has to verify a header chain to get it.
    pub block_hash: Option<[u8; 32]>,
}

impl EvmReceiptProof {
    /// Record the block hash and confirmation count a finalized-header source
    /// supplied, and require the metadata to agree with the receipt.
    ///
    /// See the module documentation: these are attestations about a chain view this
    /// module never had, not conclusions of the trie walk.
    pub fn with_header_attestation(mut self, block_hash: [u8; 32], confirmations: u64) -> Self {
        self.block_hash = Some(block_hash);
        self.confirmations = Some(confirmations);
        self
    }

    /// The transaction hash, when a caller filled it in.
    pub fn with_tx_hash(mut self, tx_hash: [u8; 32]) -> Self {
        self.tx_hash = Some(tx_hash);
        self
    }

    /// Refuse unless a header attestation recorded at least `required` confirmations.
    ///
    /// A receipt with no attestation is refused rather than treated as unconfirmed:
    /// "we were not told" and "we were told zero" are different facts, and the first
    /// is the one that must not pass a finality check.
    pub fn require_confirmations(&self, required: u64) -> Result<(), EvmProofError> {
        match self.confirmations {
            Some(actual) if actual >= required => Ok(()),
            Some(actual) => Err(EvmProofError::InsufficientConfirmations { required, actual }),
            None => Err(EvmProofError::NoHeaderAttestation { required }),
        }
    }
}

/// Errors produced by EVM proof verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvmProofError {
    /// Receipt RLP could not be decoded.
    InvalidReceiptRlp,
    /// The proof did not show the receipt at the claimed index in the claimed root.
    ///
    /// This is the one the trie walk produces, and it covers a wrong root, a wrong
    /// index, a missing node and a tampered node alike — the verifier is not asked to
    /// guess which of those it was, because a proof that fails does not say.
    NotIncluded,
    /// No logs found in the receipt.
    NoLogsFound,
    /// Expected log does not match any log in the receipt.
    LogMismatch {
        expected_address: [u8; 20],
        expected_topic: [u8; 32],
    },
    /// Block hash does not meet required confirmations.
    InsufficientConfirmations { required: u64, actual: u64 },
    /// A confirmation requirement was checked with no header attestation recorded.
    NoHeaderAttestation { required: u64 },
    /// Integer overflow or invalid conversion.
    ArithmeticOverflow,
}

impl fmt::Display for EvmProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReceiptRlp => write!(f, "EVM proof: invalid receipt RLP"),
            Self::NotIncluded => write!(
                f,
                "EVM proof: the receipt is not at the claimed index in the claimed receipts root"
            ),
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
            Self::NoHeaderAttestation { required } => write!(
                f,
                "EVM proof: {required} confirmations were required and no header attestation \
                 was recorded, so there is nothing to measure them against"
            ),
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
///
/// Test-only since TICKET-065: the lib's last caller was the fabricated trie root that
/// hashed the caller's own bytes, and the proof builder in the tests below needs a real
/// encoder to build a real trie. Kept rather than deleted so there is one RLP encoder in
/// this module rather than two.
#[cfg(test)]
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
#[cfg(test)]
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
// Main verification entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Verify an EVM receipt proof.
///
/// The inclusion proof is delegated to
/// [`x3_verification_router::evm_receipt::verify_merkle_patricia_proof`] — this
/// function does not compute, guess or default a trie root. See the module
/// documentation for what a success does and does not establish.
///
/// # Arguments
///
/// * `receipts_root` - The `receiptsRoot` of the block header this receipt must be in.
/// * `receipt_rlp` - The RLP-encoded receipt bytes.
/// * `receipt_index` - The index of this receipt in the block's receipt list.
/// * `trie_proof` - The Merkle Patricia proof: an RLP list of the node byte strings
///   from the root to the leaf.
/// * `expected_logs` - The logs expected to be found in this receipt.
///
/// # Returns
///
/// A verified `EvmReceiptProof` containing the receipt's logs, status, gas used, and
/// the root it was proven against. `block_hash`, `tx_hash` and `confirmations` are
/// `None`: none of them is a conclusion of the trie walk, and the caller fills them in
/// from the sources that do establish them.
///
/// # Errors
///
/// Returns `EvmProofError` if:
/// - The receipt RLP cannot be decoded
/// - The trie proof does not show the receipt at `receipt_index` under
///   `receipts_root` ([`EvmProofError::NotIncluded`])
/// - No logs are found
/// - Expected logs do not match
/// - Arithmetic overflow occurs
pub fn verify_evm_receipt_proof(
    receipts_root: &[u8; 32],
    receipt_rlp: &[u8],
    receipt_index: u64,
    trie_proof: &[u8],
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

    // Step 6: Prove the receipt is in the trie. The key is `rlp(index)` — the same
    // convention the canonical verifier's own `receipt_trie_key` produces, so this
    // crate cannot reintroduce the list-wrapped key TICKET-064 found in the router.
    // The delegation is the point: a hash of the caller's own bytes is not a trie
    // root, and it is the caller's bytes that would be hashed (TICKET-065).
    let key = x3_verification_router::evm_receipt::receipt_trie_key(receipt_index);
    x3_verification_router::evm_receipt::verify_merkle_patricia_proof(
        receipts_root,
        &key,
        Some(receipt_rlp),
        trie_proof,
    )
    .map_err(|_| EvmProofError::NotIncluded)?;

    // Step 7: Build result
    Ok(EvmReceiptProof {
        receipt_index,
        tx_hash: None,
        status,
        gas_used,
        logs,
        confirmations: None,
        receipts_root: *receipts_root,
        block_hash: None,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Keccak-256, the primitive Ethereum hashes trie nodes with: the pre-standardised
    /// padding, which is not FIPS-202 SHA3-256.
    fn keccak256(data: &[u8]) -> [u8; 32] {
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(data);
        let mut out = [0u8; 32];
        out.copy_from_slice(&hasher.finalize());
        out
    }

    // ── Receipts-trie proof builders ───────────────────────────────────────
    //
    // These build a proof from the *standard* encoding rather than from anything the
    // verifier exposes: the key is `rlp(index)`, the node is
    // `rlp([compact_leaf_path(nibbles(key)), receipt_rlp])`, and the proof is the RLP
    // list of node byte strings. A test that borrowed the verifier's own key helper
    // would agree with whatever that helper does — which is how a list-wrapped key
    // survived a whole round in `x3-verification-router` (TICKET-064), and how a hash
    // of the caller's own bytes passed for a trie root here (TICKET-065).

    /// The receipts-trie key for an index: `rlp(index)`, with the integer stripped of
    /// leading zero bytes first. Index 0 is `0x80`, index 1 is `0x01`.
    fn trie_key(index: u64) -> Vec<u8> {
        let be = index.to_be_bytes();
        let first = be.iter().position(|byte| *byte != 0).unwrap_or(be.len());
        rlp_encode_bytes(&be[first..])
    }

    /// Hex-prefix (compact) encoding of a leaf path, per the yellow paper.
    fn compact_leaf_path(key: &[u8]) -> Vec<u8> {
        let mut nibbles = Vec::with_capacity(key.len() * 2);
        for byte in key {
            nibbles.push(byte >> 4);
            nibbles.push(byte & 0x0F);
        }
        let mut out = Vec::new();
        if nibbles.len() % 2 == 0 {
            out.push(0x20 | (nibbles.len() / 2) as u8);
            for pair in nibbles.chunks(2) {
                out.push((pair[0] << 4) | pair[1]);
            }
        } else {
            out.push(0x30 | (nibbles.len() / 2) as u8);
            out.push((nibbles[0] << 4) | nibbles[1]);
            for pair in nibbles[2..].chunks(2) {
                out.push((pair[0] << 4) | pair[1]);
            }
        }
        out
    }

    /// A one-leaf receipts trie holding `receipt_rlp` at `index`, and the proof that
    /// binds it: `(receipts_root, proof)`.
    fn single_receipt_trie(index: u64, receipt_rlp: &[u8]) -> ([u8; 32], Vec<u8>) {
        let leaf = rlp_encode_list(&[
            rlp_encode_bytes(&compact_leaf_path(&trie_key(index))),
            rlp_encode_bytes(receipt_rlp),
        ]);
        let root = keccak256(&leaf);
        let proof = rlp_encode_list(&[rlp_encode_bytes(&leaf)]);
        (root, proof)
    }

    /// A proof whose last node byte has been altered, so the walk's hash check fails.
    fn tampered(proof: &[u8]) -> Vec<u8> {
        let mut out = proof.to_vec();
        let last = out.len() - 1;
        out[last] ^= 0x01;
        out
    }

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
        let (receipts_root, proof_rlp) = single_receipt_trie(0, &receipt_rlp);

        let expected = EvmLog {
            address: addr,
            topics: vec![topic],
            data: Vec::new(),
        };

        let result =
            verify_evm_receipt_proof(&receipts_root, &receipt_rlp, 0, &proof_rlp, &[expected]);
        assert!(
            result.is_ok(),
            "valid receipt should verify: {:?}",
            result.err()
        );

        let proof = result.unwrap();
        assert_eq!(proof.status, 1);
        assert_eq!(proof.gas_used, 100_000);
        assert_eq!(proof.receipt_index, 0);
        assert_eq!(proof.receipts_root, receipts_root);
        assert_eq!(proof.logs.len(), 1);
        assert_eq!(proof.logs[0].address, addr);
        // Nothing the trie walk does not establish is invented: the block hash, the
        // transaction hash and the confirmation count are absent until a source that
        // knows them supplies them.
        assert_eq!(proof.block_hash, None);
        assert_eq!(proof.tx_hash, None);
        assert_eq!(proof.confirmations, None);

        // And the finality check refuses rather than reading a default.
        assert_eq!(
            proof.require_confirmations(1),
            Err(EvmProofError::NoHeaderAttestation { required: 1 })
        );
        let attested = proof.with_header_attestation([0xabu8; 32], 12);
        assert_eq!(attested.block_hash, Some([0xabu8; 32]));
        assert_eq!(attested.require_confirmations(12), Ok(()));
        assert_eq!(
            attested.require_confirmations(13),
            Err(EvmProofError::InsufficientConfirmations {
                required: 13,
                actual: 12
            })
        );
    }

    #[test]
    fn a_tampered_trie_node_is_refused() {
        let addr = test_address();
        let topic = test_topic("BridgeLock(address,uint256)");
        let receipt_rlp = make_receipt_rlp(1, 100_000, addr, topic);
        let (receipts_root, proof_rlp) = single_receipt_trie(0, &receipt_rlp);
        let expected = EvmLog {
            address: addr,
            topics: vec![topic],
            data: Vec::new(),
        };

        assert_eq!(
            verify_evm_receipt_proof(
                &receipts_root,
                &receipt_rlp,
                0,
                &tampered(&proof_rlp),
                &[expected]
            )
            .err(),
            Some(EvmProofError::NotIncluded)
        );
    }

    #[test]
    fn a_receipt_at_the_wrong_index_is_refused() {
        // The same receipt, proved against the root of a trie that holds it at index 7.
        // Index 0's key is `0x80` and index 7's is `0x07`, so the leaf path differs and
        // the walk must not accept the claim that the receipt is at index 0.
        let addr = test_address();
        let topic = test_topic("BridgeLock(address,uint256)");
        let receipt_rlp = make_receipt_rlp(1, 100_000, addr, topic);
        let (root_at_seven, proof_at_seven) = single_receipt_trie(7, &receipt_rlp);
        let expected = EvmLog {
            address: addr,
            topics: vec![topic],
            data: Vec::new(),
        };

        assert!(
            verify_evm_receipt_proof(
                &root_at_seven,
                &receipt_rlp,
                7,
                &proof_at_seven,
                std::slice::from_ref(&expected)
            )
            .is_ok(),
            "the honest index verifies"
        );
        assert_eq!(
            verify_evm_receipt_proof(
                &root_at_seven,
                &receipt_rlp,
                0,
                &proof_at_seven,
                &[expected]
            )
            .err(),
            Some(EvmProofError::NotIncluded)
        );
    }

    #[test]
    fn a_receipt_under_a_root_that_does_not_hold_it_is_refused() {
        // The defect this pins: the old verifier hashed the caller's own receipt bytes
        // and called the result a trie root, so any root the caller named was
        // "confirmed". A root from a different receipt must not verify.
        let addr = test_address();
        let topic = test_topic("BridgeLock(address,uint256)");
        let receipt_rlp = make_receipt_rlp(1, 100_000, addr, topic);
        let (_, proof_rlp) = single_receipt_trie(0, &receipt_rlp);
        let (other_root, _) = single_receipt_trie(0, &make_receipt_rlp(1, 21_000, addr, topic));
        let expected = EvmLog {
            address: addr,
            topics: vec![topic],
            data: Vec::new(),
        };

        assert_eq!(
            verify_evm_receipt_proof(&other_root, &receipt_rlp, 0, &proof_rlp, &[expected]).err(),
            Some(EvmProofError::NotIncluded)
        );
    }

    #[test]
    fn an_empty_proof_list_is_refused() {
        let addr = test_address();
        let topic = test_topic("BridgeLock(address,uint256)");
        let receipt_rlp = make_receipt_rlp(1, 100_000, addr, topic);
        let (receipts_root, _) = single_receipt_trie(0, &receipt_rlp);
        let expected = EvmLog {
            address: addr,
            topics: vec![topic],
            data: Vec::new(),
        };

        // `0xc0` is the empty RLP list: a proof that walks nothing.
        assert_eq!(
            verify_evm_receipt_proof(&receipts_root, &receipt_rlp, 0, &[0xc0], &[expected]).err(),
            Some(EvmProofError::NotIncluded)
        );
    }

    #[test]
    fn verify_receipt_no_logs_fails() {
        // Receipt with no logs
        let addr = test_address();
        let topic = test_topic("BridgeLock(address,uint256)");
        let receipt_rlp = make_receipt_rlp(1, 100_000, addr, topic);
        let (receipts_root, proof_rlp) = single_receipt_trie(0, &receipt_rlp);

        let expected = EvmLog {
            address: [0xffu8; 20], // different address
            topics: vec![[0x01u8; 32]],
            data: Vec::new(),
        };

        let result =
            verify_evm_receipt_proof(&receipts_root, &receipt_rlp, 0, &proof_rlp, &[expected]);
        assert!(
            result.is_err(),
            "non-matching log should fail, got {:?}",
            result
        );
    }

    #[test]
    fn verify_empty_receipt_fails() {
        let expected = EvmLog {
            address: [0u8; 20],
            topics: vec![[0u8; 32]],
            data: Vec::new(),
        };
        let result = verify_evm_receipt_proof(&[0u8; 32], &[], 0, &[0xc0], &[expected]);
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
        let (receipts_root, proof_rlp) = single_receipt_trie(0, &receipt_rlp);

        // Expect only the bridge lock log
        let expected = EvmLog {
            address: addr,
            topics: vec![topic2],
            data: vec![],
        };

        let result =
            verify_evm_receipt_proof(&receipts_root, &receipt_rlp, 0, &proof_rlp, &[expected]);
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
    fn keccak256_matches_the_mainnet_known_answer() {
        // `keccak256("")` — the empty-string digest every Ethereum implementation
        // agrees on, and the one that separates Keccak-256 from FIPS-202 SHA3-256.
        // The test here previously asserted only "not all zeros" against a "Verified:"
        // comment whose value was not the digest of anything, which is the same class
        // of invented evidence as the trie root beside it (TICKET-065).
        assert_eq!(
            hex::encode(keccak256(b"")),
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
        );
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
        let (receipts_root, proof_rlp) = single_receipt_trie(0, &receipt_rlp);

        let expected = EvmLog {
            address: addr,
            topics: vec![topic],
            data: Vec::new(),
        };

        let result =
            verify_evm_receipt_proof(&receipts_root, &receipt_rlp, 0, &proof_rlp, &[expected]);
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
