//! Bitcoin consensus checks for an SPV verifier.
//!
//! An SPV proof says "this transaction is in a block buried under N blocks of real Bitcoin work".
//! Each of those words is a check, and a verifier that skips one accepts proofs Bitcoin would not:
//!
//! - **work**: a header's hash, read as a 256-bit little-endian number, must not exceed the target
//!   its `nBits` encodes ([`check_proof_of_work`]); the target must be a real one (not negative,
//!   zero or overflowing) and no easier than the network's `powLimit`;
//! - **real** work: `nBits` is chosen by whoever builds the header, so a verifier must require the
//!   difficulty Bitcoin requires at that height ([`next_work_required`], Bitcoin Core's
//!   `CalculateNextWorkRequired`, exactly) or a forger simply picks an easy target;
//! - **Bitcoin's** chain: the timestamp must exceed the median of the previous eleven
//!   ([`median_time_past`]) and must not be more than two hours past the verifier's clock, and the
//!   chain counted is the one with the most cumulative work ([`work_from_bits`]), not the longest;
//! - **in a block**: the Merkle path is applied left or right by the transaction's position, the
//!   position must fit the path's depth, and the duplicate-subtree malleation (CVE-2012-2459) and
//!   64-byte-transaction ambiguity are refused ([`merkle_root_from_path`], [`verify_transaction`]).
//!
//! [`LightClient`] puts those together: it starts from a checkpoint at a retarget boundary, admits
//! only headers that follow Bitcoin's rules from there, tracks the most-work tip, and answers
//! inclusion queries with confirmations counted on that tip's chain.
//!
//! Hashes are `[u8; 32]` in **internal (wire) byte order** — the bytes SHA-256d produces and the
//! bytes a header's `prev_blockhash` carries. Block explorers display the reverse
//! (`000000000019d668…` for the genesis block); [`display_order`] converts.
//!
//! Not supported, and refused rather than approximated: testnet3's 20-minute minimum-difficulty
//! rule and signet's block signatures. Their [`Params`] do not exist here, so a verifier cannot be
//! configured into checking a weaker rule than the network it claims to follow.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod client;
mod tx;
mod u256;

pub use client::{Checkpoint, Inclusion, LightClient, Submitted};
pub use tx::{transaction_outputs, TxError, TxOut};
pub use u256::U256;

use sha2::{Digest, Sha256};

/// A SHA-256d digest in internal byte order.
pub type Hash = [u8; 32];

/// Length of a serialized block header.
pub const HEADER_LEN: usize = 80;
/// How many ancestors the median-time-past rule reads.
pub const MEDIAN_TIME_SPAN: usize = 11;
/// Bitcoin refuses a header timestamped more than this far past the node's clock (two hours).
pub const MAX_FUTURE_BLOCK_TIME: u32 = 2 * 60 * 60;
/// Longest Merkle path accepted. A block of 2^32 transactions would need 32 levels; real blocks
/// need fewer than 16.
pub const MAX_MERKLE_DEPTH: usize = 32;

/// Why a header, chain or proof was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpvError {
    /// A serialized header was not 80 bytes.
    HeaderLength(usize),
    /// `nBits` decodes to a negative, zero or overflowing target.
    InvalidTarget(u32),
    /// `nBits` is easier than the network's proof-of-work limit.
    TargetAbovePowLimit(u32),
    /// The header's hash exceeds its target.
    InsufficientProofOfWork,
    /// `nBits` is not what Bitcoin requires at this height.
    UnexpectedDifficulty {
        height: u64,
        expected: u32,
        actual: u32,
    },
    /// The timestamp is not after the median of the previous eleven blocks.
    TimestampNotAfterMedian { time: u32, median: u32 },
    /// The timestamp is more than two hours past the verifier's clock.
    TimestampTooFarInFuture { time: u32, now: u32 },
    /// The header's parent is not a header this client has admitted.
    UnknownParent(Hash),
    /// The header was already admitted.
    DuplicateHeader(Hash),
    /// A checkpoint must sit on a retarget boundary so every later difficulty can be derived.
    CheckpointNotAtRetargetBoundary(u64),
    /// More than ten timestamps were supplied before a checkpoint.
    TooManyPriorTimestamps(usize),
    /// A block the query names is not one this client has admitted.
    UnknownBlock(Hash),
    /// The block is on a branch with less work than the best chain.
    NotInBestChain(Hash),
    /// Fewer confirmations than required.
    InsufficientConfirmations { required: u64, actual: u64 },
    /// The Merkle path is longer than [`MAX_MERKLE_DEPTH`].
    MerklePathTooLong(usize),
    /// The position needs more levels than the path has.
    MerkleIndexOutOfRange { index: u32, depth: usize },
    /// A right child equal to its left sibling: a duplicated subtree (CVE-2012-2459).
    MerkleDuplicateSubtree,
    /// The path does not lead to the block's Merkle root.
    MerkleRootMismatch,
    /// A 64-byte transaction is indistinguishable from an inner Merkle node and is refused.
    AmbiguousTransactionLength,
    /// Height arithmetic overflowed.
    HeightOverflow,
    /// A raw transaction could not be read.
    MalformedTransaction(TxError),
}

impl core::fmt::Display for SpvError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SpvError {}

/// A Bitcoin block header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockHeader {
    pub version: i32,
    pub prev_blockhash: Hash,
    pub merkle_root: Hash,
    pub time: u32,
    pub bits: u32,
    pub nonce: u32,
}

impl BlockHeader {
    /// Decode the 80-byte wire form.
    pub fn decode(bytes: &[u8]) -> Result<Self, SpvError> {
        let bytes: &[u8; HEADER_LEN] = bytes
            .try_into()
            .map_err(|_| SpvError::HeaderLength(bytes.len()))?;
        let word = |at: usize| [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]];
        let mut prev_blockhash = [0u8; 32];
        prev_blockhash.copy_from_slice(&bytes[4..36]);
        let mut merkle_root = [0u8; 32];
        merkle_root.copy_from_slice(&bytes[36..68]);
        Ok(Self {
            version: i32::from_le_bytes(word(0)),
            prev_blockhash,
            merkle_root,
            time: u32::from_le_bytes(word(68)),
            bits: u32::from_le_bytes(word(72)),
            nonce: u32::from_le_bytes(word(76)),
        })
    }

    /// The 80-byte wire form.
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut out = [0u8; HEADER_LEN];
        out[0..4].copy_from_slice(&self.version.to_le_bytes());
        out[4..36].copy_from_slice(&self.prev_blockhash);
        out[36..68].copy_from_slice(&self.merkle_root);
        out[68..72].copy_from_slice(&self.time.to_le_bytes());
        out[72..76].copy_from_slice(&self.bits.to_le_bytes());
        out[76..80].copy_from_slice(&self.nonce.to_le_bytes());
        out
    }

    /// The block hash, internal byte order.
    pub fn block_hash(&self) -> Hash {
        sha256d(&self.encode())
    }
}

/// Network rules a verifier follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    /// The easiest target any header may carry (Bitcoin Core's `powLimit`).
    pub pow_limit: U256,
    /// Blocks between difficulty adjustments.
    pub retarget_interval: u64,
    /// Seconds a retarget window is meant to take.
    pub target_timespan: u64,
    /// Regtest: difficulty never changes.
    pub no_retargeting: bool,
}

impl Params {
    /// Bitcoin mainnet.
    pub const fn mainnet() -> Self {
        Self {
            // 00000000ffffffff…ff
            pow_limit: U256::MAX.shr_const(32),
            retarget_interval: 2016,
            target_timespan: 14 * 24 * 60 * 60,
            no_retargeting: false,
        }
    }

    /// Bitcoin regtest: a trivial limit and no retargeting. For tests and local networks only.
    pub const fn regtest() -> Self {
        Self {
            // 7fffff…ff
            pow_limit: U256::MAX.shr_const(1),
            retarget_interval: 2016,
            target_timespan: 14 * 24 * 60 * 60,
            no_retargeting: true,
        }
    }
}

/// SHA-256 applied twice.
pub fn sha256d(data: &[u8]) -> Hash {
    let mut out = [0u8; 32];
    out.copy_from_slice(&Sha256::digest(Sha256::digest(data)));
    out
}

/// Reverse a hash between internal order and the order explorers display.
pub fn display_order(hash: &Hash) -> Hash {
    let mut out = *hash;
    out.reverse();
    out
}

/// Decode `nBits`, refusing what Bitcoin's `CheckProofOfWork` refuses: a negative, zero or
/// overflowing target.
pub fn target_from_bits(bits: u32) -> Result<U256, SpvError> {
    let size = bits >> 24;
    let word = bits & 0x007f_ffff;
    let negative = word != 0 && bits & 0x0080_0000 != 0;
    let overflow =
        word != 0 && (size > 34 || (word > 0xff && size > 33) || (word > 0xffff && size > 32));
    if negative || overflow {
        return Err(SpvError::InvalidTarget(bits));
    }
    let target = if size <= 3 {
        U256::from_u64(u64::from(word >> (8 * (3 - size))))
    } else {
        U256::from_u64(u64::from(word)).shl(8 * (size - 3))
    };
    if target.is_zero() {
        return Err(SpvError::InvalidTarget(bits));
    }
    Ok(target)
}

/// Encode a target as `nBits` (Bitcoin Core's `GetCompact`), rounding down to 24 bits of mantissa.
pub fn bits_from_target(target: &U256) -> u32 {
    let mut size = target.bits().div_ceil(8);
    let mut compact = if size <= 3 {
        (target.low_u64() << (8 * (3 - size))) as u32
    } else {
        target.shr(8 * (size - 3)).low_u64() as u32
    };
    if compact & 0x0080_0000 != 0 {
        compact >>= 8;
        size += 1;
    }
    compact | (size << 24)
}

/// The expected number of hashes to find a block at `bits`: `2^256 / (target + 1)`.
pub fn work_from_bits(bits: u32) -> Result<U256, SpvError> {
    let target = target_from_bits(bits)?;
    // 2^256 does not fit; (2^256 - target - 1) / (target + 1) + 1 is the same quotient.
    Ok(match target.checked_add(&U256::ONE) {
        Some(divisor) => target.not().div(&divisor).saturating_add(&U256::ONE),
        None => U256::ONE,
    })
}

/// A header's own proof of work: a valid target no easier than `powLimit`, met by its hash.
pub fn check_proof_of_work(header: &BlockHeader, params: &Params) -> Result<(), SpvError> {
    let target = target_from_bits(header.bits)?;
    if target > params.pow_limit {
        return Err(SpvError::TargetAbovePowLimit(header.bits));
    }
    if U256::from_le_bytes(&header.block_hash()) > target {
        return Err(SpvError::InsufficientProofOfWork);
    }
    Ok(())
}

/// Bitcoin Core's `CalculateNextWorkRequired`: the `nBits` for the first block of a new window,
/// from the last block of the previous one (`last_bits`, `last_time`) and the first
/// (`first_time`). As in Bitcoin, the window measured is 2015 intervals, not 2016.
pub fn next_work_required(params: &Params, last_bits: u32, last_time: u32, first_time: u32) -> u32 {
    if params.no_retargeting {
        return last_bits;
    }
    let timespan = params.target_timespan;
    let actual = (i64::from(last_time) - i64::from(first_time))
        .clamp((timespan / 4) as i64, (timespan * 4) as i64) as u64;
    let Ok(previous) = target_from_bits(last_bits) else {
        // Unreachable for a chain this crate admitted; the limit is the conservative answer.
        return bits_from_target(&params.pow_limit);
    };
    // Bitcoin Core multiplies then divides in 256 bits, which cannot overflow under mainnet's limit.
    // `mul_div_u64` gives the same quotient without that precondition; a result that does not fit
    // in 256 bits is above any limit, so it becomes the limit.
    let next = previous
        .mul_div_u64(actual, timespan)
        .unwrap_or(params.pow_limit);
    bits_from_target(if next > params.pow_limit {
        &params.pow_limit
    } else {
        &next
    })
}

/// The median of up to eleven timestamps (Bitcoin's `GetMedianTimePast`).
pub fn median_time_past(timestamps: &[u32]) -> Option<u32> {
    if timestamps.is_empty() {
        return None;
    }
    let mut sorted = [0u32; MEDIAN_TIME_SPAN];
    let n = timestamps.len().min(MEDIAN_TIME_SPAN);
    sorted[..n].copy_from_slice(&timestamps[timestamps.len() - n..]);
    sorted[..n].sort_unstable();
    Some(sorted[n / 2])
}

/// The root a Merkle path leads to from `leaf` at position `index`.
///
/// At each level the position's low bit says whether the running hash is the left (0) or right (1)
/// child. Refused: a path longer than [`MAX_MERKLE_DEPTH`], a position that needs more levels than
/// the path has, and a right child equal to its sibling — Bitcoin pads an odd level by repeating its
/// last node as a *left* child's sibling, so an equal pair with the leaf on the right is a
/// duplicated subtree, the malleation behind CVE-2012-2459. An empty path is a one-transaction
/// block, whose root is the transaction itself.
pub fn merkle_root_from_path(leaf: &Hash, index: u32, path: &[Hash]) -> Result<Hash, SpvError> {
    if path.len() > MAX_MERKLE_DEPTH {
        return Err(SpvError::MerklePathTooLong(path.len()));
    }
    if path.len() < 32 && (index >> path.len()) != 0 {
        return Err(SpvError::MerkleIndexOutOfRange {
            index,
            depth: path.len(),
        });
    }
    let mut current = *leaf;
    let mut position = index;
    let mut buf = [0u8; 64];
    for sibling in path {
        if position & 1 == 1 {
            if *sibling == current {
                return Err(SpvError::MerkleDuplicateSubtree);
            }
            buf[..32].copy_from_slice(sibling);
            buf[32..].copy_from_slice(&current);
        } else {
            buf[..32].copy_from_slice(&current);
            buf[32..].copy_from_slice(sibling);
        }
        current = sha256d(&buf);
        position >>= 1;
    }
    Ok(current)
}

/// Check that `txid` sits at `index` in a block whose Merkle root is `root`.
pub fn verify_merkle_inclusion(
    txid: &Hash,
    index: u32,
    path: &[Hash],
    root: &Hash,
) -> Result<(), SpvError> {
    if merkle_root_from_path(txid, index, path)? == *root {
        Ok(())
    } else {
        Err(SpvError::MerkleRootMismatch)
    }
}

/// Check a raw transaction (legacy, non-witness serialization) against a block's Merkle root and
/// return its txid. A 64-byte transaction is refused: its txid is the hash of 64 bytes, which is
/// exactly what an inner Merkle node is, so inclusion of one cannot be told from a forged one.
pub fn verify_transaction(
    raw_tx: &[u8],
    index: u32,
    path: &[Hash],
    root: &Hash,
) -> Result<Hash, SpvError> {
    if raw_tx.len() == 64 {
        return Err(SpvError::AmbiguousTransactionLength);
    }
    let txid = sha256d(raw_tx);
    verify_merkle_inclusion(&txid, index, path, root)?;
    Ok(txid)
}
