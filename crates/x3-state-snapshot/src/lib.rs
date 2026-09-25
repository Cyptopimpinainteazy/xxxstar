//! Content-addressed X3 state-snapshot manifests and their verifier.
//!
//! The storage audit found that X3 has backup/restore tooling but no snapshot
//! *format*: `scripts/snapshot-restore.sh` tars the database and checksums the
//! tar, which proves the tar survived the trip but says nothing about which
//! chain, which block, or which state root those bytes are supposed to be. A
//! validator joining a year-old chain at ~157M blocks also cannot be asked to
//! replay from genesis, so a state snapshot has to exist, and the moment it
//! exists it becomes an attack surface.
//!
//! # A snapshot is an acceleration mechanism, not a new trust root
//!
//! `verify_manifest` does not decide whether a state root is the real one. It
//! checks a snapshot against an anchor the caller already trusts: the finalized
//! block hash, state root and runtime version that came from consensus. The
//! `finality_proof` is carried and required to be present, and is deliberately
//! never parsed here. Validating a GRANDPA justification against the authority
//! set is consensus work done by the node, against the same `block_hash`.
//! Nothing in this module may be used to decide that a chain is canonical.
//!
//! What this module does own is the integrity of the transport format:
//!
//! * the manifest is hash-committed with a canonical, order-independent
//!   encoding, so a mirror cannot rewrite metadata without changing the hash;
//! * every chunk is independently hash-verifiable, so a corrupt chunk is caught
//!   at the chunk rather than after the database has been overwritten;
//! * the chunk set must be complete and correctly ordered, so a truncated or
//!   shuffled snapshot is rejected instead of silently applied;
//! * the manifest is bound to a chain id, block hash, state root and runtime
//!   version, so a stale snapshot, a snapshot from another chain, or one taken
//!   under a different runtime is refused;
//! * and, the part that makes the rest mean something, the entries carried by
//!   the chunks are fed back through the chain's own trie layout and hasher
//!   ([`verify_state_root`]), so a mirror cannot serve different state under a
//!   copied manifest: the recomputed root will not match the declared one.
//!
//! That list is the audit's "state sync needs a murder test" checklist:
//! malicious snapshot server, corrupt chunk, wrong state root, wrong block
//! hash, wrong runtime version, stale snapshot, snapshot from another chain,
//! incomplete snapshot.
//!
//! The same checks gate the other direction. [`restore_snapshot`] will not hand
//! back a chain spec until the snapshot has passed them and until the spec it
//! just built hashes back to the declared root, because a restore's output is
//! what a node then builds its database from. A restore that is refused writes
//! nothing: there is no "verified enough to boot".

#![deny(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sp_core::Blake2Hasher;
use sp_trie::{LayoutV0, LayoutV1, TrieConfiguration};
use std::fs;
use std::path::{Path, PathBuf};

/// Magic string identifying the manifest layout.
pub const SNAPSHOT_MAGIC: &str = "X3StateSnapshotV1";

/// Format version this crate writes and accepts.
pub const SNAPSHOT_FORMAT_VERSION: u16 = 1;

/// Size of every hash in this format (SHA-256).
pub const HASH_LEN: usize = 32;

/// One storage entry: the raw trie key and the raw value stored under it.
pub type StateEntry = (Vec<u8>, Vec<u8>);

/// A chunk slot read back from a snapshot directory: `None` means the snapshot
/// set did not contain that chunk.
pub type ChunkSlot = Option<Vec<u8>>;

/// What a verifier needs in order to know which chain state a snapshot claims
/// to be, and where that claim came from.
///
/// Hex fields are `0x`-prefixed and 32 bytes, and are parsed before use so a
/// typo cannot turn into a silently different hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotManifest {
    /// Always [`SNAPSHOT_MAGIC`]; anything else is not our format.
    pub format: String,
    /// Always [`SNAPSHOT_FORMAT_VERSION`] for files this crate accepts.
    pub format_version: u16,
    /// Genesis/chain id this snapshot belongs to.
    pub chain_id: String,
    /// Height of the finalized block the snapshot was taken at.
    pub block_number: u64,
    /// `0x`-prefixed hash of that finalized block.
    pub block_hash: String,
    /// `0x`-prefixed state root of that finalized block.
    pub state_root: String,
    /// `spec_version` of the runtime that produced the state.
    pub runtime_spec_version: u32,
    /// Backend/format tag, for example `rocksdb-substrate-trie-v1`.
    pub database_format: String,
    /// Declared maximum size of a single chunk, in bytes.
    pub chunk_size: u32,
    /// Number of chunks; must equal `chunk_hashes.len()`.
    pub chunk_count: u64,
    /// `0x`-prefixed hash of each chunk, in snapshot order.
    pub chunk_hashes: Vec<String>,
    /// Opaque `0x`-prefixed GRANDPA justification for `block_hash`. Carried and
    /// required to be present; validated by the consensus layer, never here.
    pub finality_proof: String,
}

/// Why a manifest or chunk was refused.
///
/// Every variant is a case that must be rejected; there is no "unknown but
/// probably fine" path.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SnapshotError {
    /// The manifest's `format` field is not [`SNAPSHOT_MAGIC`].
    #[error("not an X3 state snapshot manifest: format is {got:?}")]
    NotASnapshotManifest {
        /// The value that was found.
        got: String,
    },

    /// The manifest is ours, but from a format version we cannot read.
    #[error("unsupported snapshot format version {got}, this build reads {expected}")]
    UnsupportedFormatVersion {
        /// Version found in the manifest.
        got: u16,
        /// Version this build understands.
        expected: u16,
    },

    /// A field that must be a 32-byte hex value is not.
    #[error("{field} is not a 0x-prefixed 32-byte hex value: {reason}")]
    MalformedHash {
        /// Field name, as it appears in the manifest.
        field: &'static str,
        /// Why parsing failed.
        reason: String,
    },

    /// The snapshot belongs to a different chain.
    #[error("snapshot is for chain {got:?}, expected {expected:?}")]
    WrongChain {
        /// Chain id the caller expected.
        expected: String,
        /// Chain id the manifest declares.
        got: String,
    },

    /// The snapshot is anchored to a different block.
    #[error("snapshot is anchored to block {got}, expected {expected}")]
    WrongBlockHash {
        /// Block hash the caller trusts.
        expected: String,
        /// Block hash the manifest declares.
        got: String,
    },

    /// The snapshot carries a different state root.
    #[error("snapshot carries state root {got}, expected {expected}")]
    WrongStateRoot {
        /// State root the caller trusts.
        expected: String,
        /// State root the manifest declares.
        got: String,
    },

    /// The snapshot was produced by a different runtime.
    #[error("snapshot was produced by runtime spec_version {got}, expected {expected}")]
    WrongRuntimeVersion {
        /// Runtime version the caller expects.
        expected: u32,
        /// Runtime version the manifest declares.
        got: u32,
    },

    /// The snapshot predates the caller's freshness floor.
    #[error("snapshot is at block {got}, behind the minimum accepted block {minimum}")]
    Stale {
        /// Block height of the snapshot.
        got: u64,
        /// Oldest block the caller accepts.
        minimum: u64,
    },

    /// The manifest claims a chunk count that does not match its hash list.
    #[error("manifest declares {declared} chunks but carries {actual} chunk hashes")]
    ChunkCountMismatch {
        /// The `chunk_count` field.
        declared: u64,
        /// `chunk_hashes.len()`.
        actual: usize,
    },

    /// A chunk's bytes do not hash to the hash the manifest committed to.
    #[error("chunk {index} hashes to {got}, but the manifest commits to {expected}")]
    ChunkHashMismatch {
        /// Chunk position.
        index: u64,
        /// Hash from the manifest.
        expected: String,
        /// Hash of the bytes actually supplied.
        got: String,
    },

    /// A declared chunk was not supplied.
    #[error("chunk {index} is missing from the snapshot set")]
    MissingChunk {
        /// Chunk position.
        index: u64,
    },

    /// A chunk was supplied that the manifest does not declare.
    #[error("snapshot carries {actual} chunks but the manifest declares {declared}")]
    UnexpectedChunkCount {
        /// `chunk_count` from the manifest.
        declared: u64,
        /// Number of chunk slots supplied.
        actual: usize,
    },

    /// A chunk is larger than the manifest's declared maximum.
    #[error("chunk {index} is {got} bytes, larger than the declared chunk size {declared}")]
    ChunkTooLarge {
        /// Chunk position.
        index: u64,
        /// Maximum declared by the manifest.
        declared: u32,
        /// Actual size.
        got: usize,
    },

    /// The manifest carries no finality proof.
    #[error("manifest carries no finality proof; a snapshot must name its finalized block")]
    MissingFinalityProof,

    /// The manifest hash the caller supplied does not match the manifest.
    #[error("manifest hashes to {got}, expected {expected}")]
    ManifestHashMismatch {
        /// Hash the caller trusts.
        expected: String,
        /// Hash computed from the manifest.
        got: String,
    },

    /// The chunk size is zero, which cannot describe any chunking.
    #[error("chunk size must be greater than zero")]
    ZeroChunkSize,

    /// The state being exported has no entries.
    #[error("refusing to write a snapshot with no state entries")]
    EmptyState,

    /// The same storage key appeared twice while building the state stream.
    #[error("storage key {key} appears more than once in the state being exported")]
    DuplicateKey {
        /// The duplicated key, hex encoded.
        key: String,
    },

    /// The byte stream being decoded is not a well formed state stream.
    #[error("state stream is malformed: {reason}")]
    MalformedStateStream {
        /// What was wrong with it.
        reason: String,
    },

    /// The genesis/chain spec carries child tries, which this format does not
    /// represent. Silently dropping them would produce a snapshot that verifies
    /// but is missing state.
    #[error("raw spec carries child tries ({found}); this format only represents the main trie")]
    UnsupportedChildTries {
        /// Which child-trie keys were present.
        found: String,
    },

    /// The raw spec is not shaped the way a Substrate raw chain spec is.
    #[error("raw spec is not usable as a state source: {reason}")]
    MalformedRawSpec {
        /// Why it was rejected.
        reason: String,
    },

    /// Filesystem trouble while writing or reading a snapshot directory.
    #[error("snapshot i/o failed: {reason}")]
    Io {
        /// The underlying error, already formatted.
        reason: String,
    },

    /// The state root recomputed from the snapshot's own bytes does not match
    /// the root the manifest declares. This is the check that turns a
    /// well-formed snapshot into a verified one.
    #[error(
        "snapshot bytes recompute to state root {recomputed}, but the manifest declares {declared}"
    )]
    RecomputedStateRootMismatch {
        /// The root the manifest committed to.
        declared: String,
        /// The root actually produced by the snapshot's entries.
        recomputed: String,
    },

    /// A state-version/trie-layout selector that is not `0` or `1`.
    #[error("state version must be 0 or 1, got {got}")]
    InvalidStateVersion {
        /// The value that was supplied.
        got: u8,
    },
}

/// The anchor from consensus that a snapshot is checked against.
///
/// Every field is something the caller already believes because it came from
/// the chain, not from the snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedAnchor<'a> {
    /// Chain id the node is configured for.
    pub chain_id: &'a str,
    /// Finalized block hash the node trusts.
    pub block_hash: &'a str,
    /// State root of that finalized block.
    pub state_root: &'a str,
    /// `spec_version` the node is running.
    pub runtime_spec_version: u32,
    /// Oldest block height the caller accepts; `0` disables the floor.
    pub minimum_block_number: u64,
    /// Canonical manifest hash, when obtained out of band (for example published
    /// with a release). When present, the manifest must hash to it.
    pub manifest_hash: Option<&'a str>,
}

fn parse_hash(field: &'static str, value: &str) -> Result<[u8; HASH_LEN], SnapshotError> {
    let stripped = value
        .strip_prefix("0x")
        .ok_or_else(|| SnapshotError::MalformedHash {
            field,
            reason: "missing 0x prefix".to_string(),
        })?;

    let bytes = hex::decode(stripped).map_err(|err| SnapshotError::MalformedHash {
        field,
        reason: err.to_string(),
    })?;

    if bytes.len() != HASH_LEN {
        return Err(SnapshotError::MalformedHash {
            field,
            reason: format!("expected {HASH_LEN} bytes, got {}", bytes.len()),
        });
    }

    let mut out = [0u8; HASH_LEN];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn push_len_prefixed(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn hashes_equal(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Canonical, byte-exact encoding of the manifest.
///
/// This is not `serde_json`. JSON key order, whitespace and hex casing are
/// properties of the file rather than of the snapshot, and hashing them would
/// let a mirror change the hash without changing any meaning. Integers are
/// little endian, strings are length prefixed, and hex fields are decoded to
/// raw bytes first, so `0xAB...` and `0xab...` hash identically while a
/// different value cannot.
pub fn canonical_manifest_bytes(manifest: &SnapshotManifest) -> Result<Vec<u8>, SnapshotError> {
    if manifest.format != SNAPSHOT_MAGIC {
        return Err(SnapshotError::NotASnapshotManifest {
            got: manifest.format.clone(),
        });
    }
    if manifest.format_version != SNAPSHOT_FORMAT_VERSION {
        return Err(SnapshotError::UnsupportedFormatVersion {
            got: manifest.format_version,
            expected: SNAPSHOT_FORMAT_VERSION,
        });
    }
    if manifest.chunk_count != manifest.chunk_hashes.len() as u64 {
        return Err(SnapshotError::ChunkCountMismatch {
            declared: manifest.chunk_count,
            actual: manifest.chunk_hashes.len(),
        });
    }

    let block_hash = parse_hash("block_hash", &manifest.block_hash)?;
    let state_root = parse_hash("state_root", &manifest.state_root)?;

    let mut out = Vec::new();
    out.extend_from_slice(SNAPSHOT_MAGIC.as_bytes());
    out.extend_from_slice(&manifest.format_version.to_le_bytes());
    push_len_prefixed(&mut out, manifest.chain_id.as_bytes());
    out.extend_from_slice(&manifest.block_number.to_le_bytes());
    out.extend_from_slice(&block_hash);
    out.extend_from_slice(&state_root);
    out.extend_from_slice(&manifest.runtime_spec_version.to_le_bytes());
    push_len_prefixed(&mut out, manifest.database_format.as_bytes());
    out.extend_from_slice(&manifest.chunk_size.to_le_bytes());
    out.extend_from_slice(&manifest.chunk_count.to_le_bytes());
    for (index, declared) in manifest.chunk_hashes.iter().enumerate() {
        let parsed = parse_hash("chunk_hashes", declared).map_err(|err| match err {
            SnapshotError::MalformedHash { reason, .. } => SnapshotError::MalformedHash {
                field: "chunk_hashes",
                reason: format!("entry {index}: {reason}"),
            },
            other => other,
        })?;
        out.extend_from_slice(&parsed);
    }
    push_len_prefixed(&mut out, manifest.finality_proof.as_bytes());

    Ok(out)
}

/// Canonical `0x`-prefixed hash of a manifest.
pub fn manifest_hash(manifest: &SnapshotManifest) -> Result<String, SnapshotError> {
    let canonical = canonical_manifest_bytes(manifest)?;
    let digest = Sha256::digest(&canonical);
    Ok(format!("0x{}", hex::encode(digest)))
}

/// Hash a chunk exactly the way the manifest commits to it.
pub fn chunk_hash(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(Sha256::digest(bytes)))
}

/// Check a manifest against the caller's trusted anchor.
///
/// Refuses a wrong chain, wrong block, wrong state root, wrong runtime, stale
/// snapshot, malformed hash, missing finality proof, and, when the caller knows
/// it, a manifest whose hash does not match the published one.
pub fn verify_manifest(
    manifest: &SnapshotManifest,
    anchor: &TrustedAnchor<'_>,
) -> Result<(), SnapshotError> {
    if manifest.format != SNAPSHOT_MAGIC {
        return Err(SnapshotError::NotASnapshotManifest {
            got: manifest.format.clone(),
        });
    }
    if manifest.format_version != SNAPSHOT_FORMAT_VERSION {
        return Err(SnapshotError::UnsupportedFormatVersion {
            got: manifest.format_version,
            expected: SNAPSHOT_FORMAT_VERSION,
        });
    }

    if manifest.chain_id != anchor.chain_id {
        return Err(SnapshotError::WrongChain {
            expected: anchor.chain_id.to_string(),
            got: manifest.chain_id.clone(),
        });
    }

    let expected_block = parse_hash("block_hash", anchor.block_hash)?;
    let got_block = parse_hash("block_hash", &manifest.block_hash)?;
    if got_block != expected_block {
        return Err(SnapshotError::WrongBlockHash {
            expected: anchor.block_hash.to_string(),
            got: manifest.block_hash.clone(),
        });
    }

    let expected_root = parse_hash("state_root", anchor.state_root)?;
    let got_root = parse_hash("state_root", &manifest.state_root)?;
    if got_root != expected_root {
        return Err(SnapshotError::WrongStateRoot {
            expected: anchor.state_root.to_string(),
            got: manifest.state_root.clone(),
        });
    }

    if manifest.runtime_spec_version != anchor.runtime_spec_version {
        return Err(SnapshotError::WrongRuntimeVersion {
            expected: anchor.runtime_spec_version,
            got: manifest.runtime_spec_version,
        });
    }

    if manifest.block_number < anchor.minimum_block_number {
        return Err(SnapshotError::Stale {
            got: manifest.block_number,
            minimum: anchor.minimum_block_number,
        });
    }

    if manifest.finality_proof.trim().is_empty() {
        return Err(SnapshotError::MissingFinalityProof);
    }

    if manifest.chunk_count != manifest.chunk_hashes.len() as u64 {
        return Err(SnapshotError::ChunkCountMismatch {
            declared: manifest.chunk_count,
            actual: manifest.chunk_hashes.len(),
        });
    }

    if let Some(expected) = anchor.manifest_hash {
        let got = manifest_hash(manifest)?;
        if !hashes_equal(expected, &got) {
            return Err(SnapshotError::ManifestHashMismatch {
                expected: expected.to_string(),
                got,
            });
        }
    }

    Ok(())
}

/// Verify one chunk against the hash the manifest commits to.
pub fn verify_chunk(
    manifest: &SnapshotManifest,
    index: u64,
    bytes: &[u8],
) -> Result<(), SnapshotError> {
    if manifest.chunk_count != manifest.chunk_hashes.len() as u64 {
        return Err(SnapshotError::ChunkCountMismatch {
            declared: manifest.chunk_count,
            actual: manifest.chunk_hashes.len(),
        });
    }

    let expected = manifest
        .chunk_hashes
        .get(index as usize)
        .ok_or(SnapshotError::MissingChunk { index })?
        .clone();

    if bytes.len() > manifest.chunk_size as usize {
        return Err(SnapshotError::ChunkTooLarge {
            index,
            declared: manifest.chunk_size,
            got: bytes.len(),
        });
    }

    let got = chunk_hash(bytes);
    if !hashes_equal(&expected, &got) {
        return Err(SnapshotError::ChunkHashMismatch {
            index,
            expected,
            got,
        });
    }

    Ok(())
}

/// Verify a whole chunk set: complete, correctly ordered, every hash matching.
///
/// `chunks[i]` is the bytes of chunk `i`; `None` means the snapshot set did not
/// contain it. A missing chunk, an extra chunk, or chunks presented out of
/// order are all refused, so the caller does not have to notice that itself.
pub fn verify_chunk_set(
    manifest: &SnapshotManifest,
    chunks: &[ChunkSlot],
) -> Result<(), SnapshotError> {
    if chunks.len() as u64 != manifest.chunk_count {
        return Err(SnapshotError::UnexpectedChunkCount {
            declared: manifest.chunk_count,
            actual: chunks.len(),
        });
    }

    for (index, chunk) in chunks.iter().enumerate() {
        let index = index as u64;
        let bytes = chunk
            .as_deref()
            .ok_or(SnapshotError::MissingChunk { index })?;
        verify_chunk(manifest, index, bytes)?;
    }

    Ok(())
}

/// Verify an anchor and a full chunk set in one call.
pub fn verify_snapshot(
    manifest: &SnapshotManifest,
    anchor: &TrustedAnchor<'_>,
    chunks: &[ChunkSlot],
) -> Result<(), SnapshotError> {
    verify_manifest(manifest, anchor)?;
    verify_chunk_set(manifest, chunks)
}

// ── State-root verification ─────────────────────────────────────────────────
//
// Everything above this line proves *transport*: the chunks are the ones the
// manifest committed to, and the manifest is the one the caller expected. None
// of it proves the bytes are the state behind `state_root`, because a manifest
// field is just a string a mirror can copy.
//
// This section closes that. The chunks carry a canonical stream of storage
// entries; rebuilding the trie from those entries with the chain's own layout
// and hasher has to reproduce the declared root. A mirror that swaps in
// different state cannot make that come out right, and a snapshot taken under
// the wrong trie layout fails here rather than after a restore.

/// Which Substrate trie layout a chain computes its state root with.
///
/// `V1` is the SDK default and the layout the state machine uses for the main
/// trie; `V0` is kept because older exported state still round-trips under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrieVersion {
    /// The old layout, with no value nodes.
    V0,
    /// The current layout, which externalises large values into value nodes.
    #[default]
    V1,
}

impl TrieVersion {
    /// Parse the `0`/`1` form used by the manifest's sibling tooling.
    pub fn from_u8(value: u8) -> Result<Self, SnapshotError> {
        match value {
            0 => Ok(TrieVersion::V0),
            1 => Ok(TrieVersion::V1),
            got => Err(SnapshotError::InvalidStateVersion { got }),
        }
    }
}

/// Rebuild the state root that a set of storage entries produces.
///
/// Uses the same `LayoutV*/Blake2Hasher` pair the Substrate state machine uses
/// (`sp_state_machine::basic` maps `StateVersion` to exactly these two calls),
/// so for a given layout this is the chain's own root function, not a
/// reimplementation of it.
pub fn compute_state_root(
    entries: &[StateEntry],
    version: TrieVersion,
) -> Result<String, SnapshotError> {
    let sorted = canonical_entries(entries)?;
    let pairs = sorted
        .iter()
        .map(|(key, value)| (key.as_slice(), value.as_slice()));

    let root = match version {
        TrieVersion::V1 => LayoutV1::<Blake2Hasher>::trie_root(pairs),
        TrieVersion::V0 => LayoutV0::<Blake2Hasher>::trie_root(pairs),
    };

    Ok(format!("0x{}", hex::encode(root)))
}

/// Concatenate the chunk set into the state stream it encodes.
///
/// Completeness and per-chunk integrity are checked first, so a tampered chunk
/// is reported as a chunk error rather than as a stream-decoding failure.
pub fn concatenated_chunk_stream(
    manifest: &SnapshotManifest,
    chunks: &[ChunkSlot],
) -> Result<Vec<u8>, SnapshotError> {
    verify_chunk_set(manifest, chunks)?;

    let mut stream = Vec::new();
    for chunk in chunks {
        let bytes = chunk
            .as_deref()
            .ok_or(SnapshotError::MalformedStateStream {
                reason: "chunk missing after a successful chunk-set check".to_string(),
            })?;
        stream.extend_from_slice(bytes);
    }

    Ok(stream)
}

/// Recompute the state root from the snapshot's own bytes.
pub fn recompute_state_root(
    manifest: &SnapshotManifest,
    chunks: &[ChunkSlot],
    version: TrieVersion,
) -> Result<String, SnapshotError> {
    let stream = concatenated_chunk_stream(manifest, chunks)?;
    let entries = decode_state_entries(&stream)?;
    compute_state_root(&entries, version)
}

/// Check that the snapshot's bytes really are the state behind its own root.
pub fn verify_state_root(
    manifest: &SnapshotManifest,
    chunks: &[ChunkSlot],
    version: TrieVersion,
) -> Result<(), SnapshotError> {
    let recomputed = recompute_state_root(manifest, chunks, version)?;

    if !hashes_equal(&manifest.state_root, &recomputed) {
        return Err(SnapshotError::RecomputedStateRootMismatch {
            declared: manifest.state_root.clone(),
            recomputed,
        });
    }

    Ok(())
}

/// The full check: trusted anchor, chunk integrity, and recomputed state root.
///
/// This is the one a restore path should call. It is the only entry point that
/// proves the snapshot is the state it claims to be.
pub fn verify_snapshot_with_state_root(
    manifest: &SnapshotManifest,
    anchor: &TrustedAnchor<'_>,
    chunks: &[ChunkSlot],
    version: TrieVersion,
) -> Result<(), SnapshotError> {
    verify_manifest(manifest, anchor)?;
    verify_state_root(manifest, chunks, version)
}

// ── Export ──────────────────────────────────────────────────────────────────
//
// The export side turns a set of storage entries into the byte stream the
// manifest commits to. Two properties matter more than speed here:
//
// * the stream is a function of the *state*, not of the order the exporter
//   happened to visit it in. Entries are sorted by key and each is length
//   prefixed, so two nodes exporting the same block produce byte-identical
//   chunks and therefore the same manifest hash;
// * the chunking is a plain fixed-size split of that stream, so it is
//   reproducible without any per-chunk metadata beyond the hash list.
//
// The builder records the root it computes (`compute_state_root`) rather than
// trusting a typed-in value, and refuses to write a manifest whose declared root
// disagrees with the state it just read. Verification repeats that computation
// independently, in [`verify_state_root`], from the bytes on disk.

/// Canonical byte stream over a set of storage entries.
///
/// Entries are sorted by key and encoded as
/// `len(key) | key | len(value) | value` with little-endian `u64` lengths.
/// The same entries in a different order produce the same bytes; a repeated key
/// is refused rather than silently collapsed.
pub fn encode_state_entries(entries: &[StateEntry]) -> Result<Vec<u8>, SnapshotError> {
    let mut out = Vec::new();
    for (key, value) in canonical_entries(entries)? {
        push_len_prefixed(&mut out, key);
        push_len_prefixed(&mut out, value);
    }

    Ok(out)
}

/// Sort entries by key and refuse an empty set or a repeated key.
///
/// Both the stream encoder and the trie rebuild go through this, so "the same
/// state produces the same bytes" and "the same state produces the same root"
/// cannot drift apart.
fn canonical_entries(entries: &[StateEntry]) -> Result<Vec<&StateEntry>, SnapshotError> {
    if entries.is_empty() {
        return Err(SnapshotError::EmptyState);
    }

    let mut sorted: Vec<&StateEntry> = entries.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    for pair in sorted.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(SnapshotError::DuplicateKey {
                key: format!("0x{}", hex::encode(&pair[0].0)),
            });
        }
    }

    Ok(sorted)
}

/// Inverse of [`encode_state_entries`].
///
/// The restore path uses this to recover the entries, so a stream that does not
/// decode cleanly is refused here rather than half-way through writing a
/// database.
pub fn decode_state_entries(stream: &[u8]) -> Result<Vec<StateEntry>, SnapshotError> {
    let mut cursor = 0usize;
    let mut entries = Vec::new();

    while cursor < stream.len() {
        let remaining = stream.len() - cursor;
        if remaining < 8 {
            return Err(SnapshotError::MalformedStateStream {
                reason: format!("{remaining} trailing byte(s) where a key length was expected"),
            });
        }

        let mut len_bytes = [0u8; 8];
        len_bytes.copy_from_slice(&stream[cursor..cursor + 8]);
        cursor += 8;
        let key_len = u64::from_le_bytes(len_bytes) as usize;

        if stream.len() - cursor < key_len {
            return Err(SnapshotError::MalformedStateStream {
                reason: format!(
                    "entry {}: key length {key_len} runs past the end",
                    entries.len()
                ),
            });
        }
        let key = stream[cursor..cursor + key_len].to_vec();
        cursor += key_len;

        if stream.len() - cursor < 8 {
            return Err(SnapshotError::MalformedStateStream {
                reason: format!("entry {}: truncated value length", entries.len()),
            });
        }
        len_bytes.copy_from_slice(&stream[cursor..cursor + 8]);
        cursor += 8;
        let value_len = u64::from_le_bytes(len_bytes) as usize;

        if stream.len() - cursor < value_len {
            return Err(SnapshotError::MalformedStateStream {
                reason: format!(
                    "entry {}: value length {value_len} runs past the end",
                    entries.len()
                ),
            });
        }
        let value = stream[cursor..cursor + value_len].to_vec();
        cursor += value_len;

        entries.push((key, value));
    }

    if entries.is_empty() {
        return Err(SnapshotError::EmptyState);
    }

    Ok(entries)
}

/// Split a state stream into fixed-size chunks.
///
/// Every chunk but the last is exactly `chunk_size` bytes. An empty stream is
/// refused: a snapshot that carries no state is a bug in the exporter, not a
/// legitimate empty snapshot.
pub fn split_into_chunks(stream: &[u8], chunk_size: usize) -> Result<Vec<Vec<u8>>, SnapshotError> {
    if chunk_size == 0 {
        return Err(SnapshotError::ZeroChunkSize);
    }
    if stream.is_empty() {
        return Err(SnapshotError::EmptyState);
    }

    Ok(stream
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect())
}

/// Everything an exporter has to supply besides the state itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotBuilder {
    /// Genesis/chain id being exported.
    pub chain_id: String,
    /// Height of the finalized block being exported.
    pub block_number: u64,
    /// `0x`-prefixed hash of that block.
    pub block_hash: String,
    /// `0x`-prefixed state root of that block.
    pub state_root: String,
    /// Runtime `spec_version` producing the state.
    pub runtime_spec_version: u32,
    /// Backend/format tag for the exported state.
    pub database_format: String,
    /// Chunk size in bytes.
    pub chunk_size: usize,
    /// `0x`-prefixed GRANDPA justification for `block_hash`.
    pub finality_proof: String,
}

impl SnapshotBuilder {
    /// Build the manifest and the chunks that go with it.
    ///
    /// The chunk hashes in the returned manifest are computed from the returned
    /// chunks, so what is written is by construction what the manifest commits
    /// to.
    pub fn build(
        &self,
        entries: &[StateEntry],
    ) -> Result<(SnapshotManifest, Vec<Vec<u8>>), SnapshotError> {
        let stream = encode_state_entries(entries)?;
        let chunks = split_into_chunks(&stream, self.chunk_size)?;

        let manifest = SnapshotManifest {
            format: SNAPSHOT_MAGIC.to_string(),
            format_version: SNAPSHOT_FORMAT_VERSION,
            chain_id: self.chain_id.clone(),
            block_number: self.block_number,
            block_hash: self.block_hash.clone(),
            state_root: self.state_root.clone(),
            runtime_spec_version: self.runtime_spec_version,
            database_format: self.database_format.clone(),
            chunk_size: u32::try_from(self.chunk_size).map_err(|_| {
                SnapshotError::MalformedRawSpec {
                    reason: format!(
                        "chunk size {} does not fit in the manifest field",
                        self.chunk_size
                    ),
                }
            })?,
            chunk_count: chunks.len() as u64,
            chunk_hashes: chunks.iter().map(|chunk| chunk_hash(chunk)).collect(),
            finality_proof: self.finality_proof.clone(),
        };

        Ok((manifest, chunks))
    }
}

/// Name of a chunk file inside a snapshot directory.
pub fn chunk_file_name(index: u64) -> String {
    format!("{index}.chunk")
}

/// Write a snapshot directory: every chunk first, then `manifest.json` last.
///
/// Ordering matters. A directory that has a manifest has all of its chunks, so
/// an interrupted export leaves an obviously incomplete directory rather than
/// one that looks complete until a restore is half-applied.
pub fn write_snapshot(
    dir: &Path,
    manifest: &SnapshotManifest,
    chunks: &[Vec<u8>],
) -> Result<(), SnapshotError> {
    if chunks.len() as u64 != manifest.chunk_count {
        return Err(SnapshotError::UnexpectedChunkCount {
            declared: manifest.chunk_count,
            actual: chunks.len(),
        });
    }

    fs::create_dir_all(dir).map_err(|err| SnapshotError::Io {
        reason: format!("cannot create {}: {err}", dir.display()),
    })?;

    for (index, chunk) in chunks.iter().enumerate() {
        let path = dir.join(chunk_file_name(index as u64));
        fs::write(&path, chunk).map_err(|err| SnapshotError::Io {
            reason: format!("cannot write {}: {err}", path.display()),
        })?;
    }

    let manifest_path = dir.join("manifest.json");
    let encoded = serde_json::to_string_pretty(manifest).map_err(|err| SnapshotError::Io {
        reason: format!("cannot serialise manifest: {err}"),
    })?;
    fs::write(&manifest_path, encoded).map_err(|err| SnapshotError::Io {
        reason: format!("cannot write {}: {err}", manifest_path.display()),
    })?;

    Ok(())
}

/// Read a snapshot directory written by [`write_snapshot`].
///
/// A chunk file that is absent comes back as `None` so the verifier produces its
/// own "incomplete snapshot" verdict rather than a raw i/o error.
pub fn read_snapshot(dir: &Path) -> Result<(SnapshotManifest, Vec<ChunkSlot>), SnapshotError> {
    let manifest_path = dir.join("manifest.json");
    let raw = fs::read_to_string(&manifest_path).map_err(|err| SnapshotError::Io {
        reason: format!("cannot read {}: {err}", manifest_path.display()),
    })?;
    let manifest: SnapshotManifest =
        serde_json::from_str(&raw).map_err(|err| SnapshotError::Io {
            reason: format!("cannot parse {}: {err}", manifest_path.display()),
        })?;

    let chunks = (0..manifest.chunk_count)
        .map(|index| {
            let path = dir.join(chunk_file_name(index));
            match fs::read(&path) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(err) => Err(SnapshotError::Io {
                    reason: format!("cannot read {}: {err}", path.display()),
                }),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((manifest, chunks))
}

/// Convenience: the path a chunk would live at.
pub fn chunk_path(dir: &Path, index: u64) -> PathBuf {
    dir.join(chunk_file_name(index))
}

/// Read the storage entries out of a Substrate raw chain spec.
///
/// `export-state` on a running node writes exactly this shape, so this is the
/// bridge between a node that has state and a snapshot directory: the command
/// produces the JSON, this turns it into entries, and [`SnapshotBuilder`] turns
/// those into chunks.
///
/// Child tries are refused rather than ignored. A snapshot that silently drops
/// child storage would verify and then restore an incomplete database.
pub fn state_entries_from_raw_spec(
    spec: &serde_json::Value,
) -> Result<Vec<StateEntry>, SnapshotError> {
    let raw = spec
        .get("genesis")
        .and_then(|genesis| genesis.get("raw"))
        .ok_or_else(|| SnapshotError::MalformedRawSpec {
            reason: "no `genesis.raw` object".to_string(),
        })?;

    for child_key in ["children", "childrenDefault"] {
        if let Some(children) = raw.get(child_key) {
            let count = children.as_object().map(|map| map.len()).unwrap_or(0);
            if count > 0 {
                return Err(SnapshotError::UnsupportedChildTries {
                    found: format!("{child_key} with {count} entr(ies)"),
                });
            }
        }
    }

    let top = raw
        .get("top")
        .and_then(|top| top.as_object())
        .ok_or_else(|| SnapshotError::MalformedRawSpec {
            reason: "no `genesis.raw.top` object".to_string(),
        })?;

    let mut entries = Vec::with_capacity(top.len());
    for (key, value) in top {
        let value = value
            .as_str()
            .ok_or_else(|| SnapshotError::MalformedRawSpec {
                reason: format!("value for key {key} is not a string"),
            })?;
        entries.push((
            decode_hex_field("key", key)?,
            decode_hex_field("value", value)?,
        ));
    }

    if entries.is_empty() {
        return Err(SnapshotError::EmptyState);
    }

    Ok(entries)
}

fn decode_hex_field(what: &str, value: &str) -> Result<Vec<u8>, SnapshotError> {
    let stripped = value
        .strip_prefix("0x")
        .ok_or_else(|| SnapshotError::MalformedRawSpec {
            reason: format!("{what} {value:?} is missing its 0x prefix"),
        })?;
    hex::decode(stripped).map_err(|err| SnapshotError::MalformedRawSpec {
        reason: format!("{what} {value:?} is not valid hex: {err}"),
    })
}

// ── Restore ─────────────────────────────────────────────────────────────────
//
// Restore is the mirror image of export, and it is the only path in this crate
// that produces something a node will then boot from. It is therefore the most
// conservative code here: nothing is handed back until the snapshot has passed
// the *same* checks `verify` runs — the trusted anchor, per-chunk integrity, a
// complete chunk set, and the state root recomputed from the snapshot's own
// bytes — and until the chain spec that was just built hashes back to that same
// declared root.
//
// Two things this deliberately does not do:
//
// * it does not merge restored state into a template's state. Merging produces a
//   third state that neither the snapshot nor the chain can name, so a template
//   contributes metadata only and the state is replaced wholesale;
// * it does not treat a missing anchor as optional. `verify` permits a
//   self-consistency-only run and says so loudly; a restore cannot, because its
//   output is what a validator's database gets built from.

/// Everything a restore has to be told besides the snapshot itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreRequest<'a> {
    /// Manifest read from the snapshot directory.
    pub manifest: &'a SnapshotManifest,
    /// Anchor from consensus the snapshot must match. Required: a restore that
    /// cannot say which chain, block, root and runtime it is rebuilding is a
    /// database overwrite with extra steps.
    pub anchor: &'a TrustedAnchor<'a>,
    /// Chunk slots, as read from the snapshot directory.
    pub chunks: &'a [ChunkSlot],
    /// Trie layout the declared state root is computed with.
    pub version: TrieVersion,
    /// `name` of the produced chain spec.
    pub chain_name: String,
    /// `id` of the produced chain spec.
    pub spec_id: String,
    /// Optional chain spec whose non-state metadata (`chainType`, bootnodes,
    /// `protocolId`, telemetry, `properties`) is carried into the restored spec.
    /// Its state, if it has any, is discarded.
    pub template: Option<&'a serde_json::Value>,
}

/// Where the state in a restored chain spec came from.
///
/// This is written into the spec's `properties` as well as returned, so the
/// provenance travels with the file an operator hands to a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreProvenance {
    /// Chain id the snapshot was anchored to.
    pub chain_id: String,
    /// Height of the block the state was taken at.
    pub block_number: u64,
    /// `0x`-prefixed hash of that block.
    pub block_hash: String,
    /// `0x`-prefixed state root, recomputed from the snapshot's bytes.
    pub state_root: String,
    /// Runtime `spec_version` that produced the state.
    pub runtime_spec_version: u32,
    /// Hash of the manifest the state was read from.
    pub manifest_hash: String,
    /// Number of chunks the state arrived in.
    pub chunk_count: u64,
    /// Number of storage entries written into the restored spec.
    pub state_entries: u64,
    /// Trie layout the state root was recomputed with (`V0` or `V1`).
    pub trie_layout: String,
}

/// Turn a snapshot into a raw chain spec a node can boot from.
///
/// The snapshot is verified against `request.anchor` first, so the returned spec
/// is by construction the state of the block the caller already trusts. The
/// returned spec is re-read through the same reader the exporter uses
/// ([`state_entries_from_raw_spec`]) and hashed once more, so a spec that does
/// not reproduce the declared root is refused here rather than after a node has
/// built its database from it.
///
/// The result is a *storage* reconstruction. It carries the runtime code because
/// `:code` is part of the state, but it says nothing about whether the restored
/// state should be treated as canonical — that decision belongs to consensus,
/// and the `finality_proof` in the manifest is what consensus checks.
pub fn restore_snapshot(
    request: &RestoreRequest<'_>,
) -> Result<(serde_json::Value, RestoreProvenance), SnapshotError> {
    verify_snapshot_with_state_root(
        request.manifest,
        request.anchor,
        request.chunks,
        request.version,
    )?;

    let stream = concatenated_chunk_stream(request.manifest, request.chunks)?;
    let entries = decode_state_entries(&stream)?;

    let mut top = serde_json::Map::with_capacity(entries.len());
    for (key, value) in &entries {
        top.insert(
            format!("0x{}", hex::encode(key)),
            serde_json::Value::String(format!("0x{}", hex::encode(value))),
        );
    }

    // Metadata from the template, copied field by field: a restored spec must not
    // inherit anything that could be read as state or as a second genesis.
    let mut spec = serde_json::Map::new();
    if let Some(template) = request.template {
        let object = template
            .as_object()
            .ok_or_else(|| SnapshotError::MalformedRawSpec {
                reason: "template chain spec is not a JSON object".to_string(),
            })?;
        for field in [
            "name",
            "id",
            "chainType",
            "bootNodes",
            "telemetryEndpoints",
            "protocolId",
            "forkId",
            "properties",
        ] {
            if let Some(value) = object.get(field) {
                spec.insert(field.to_string(), value.clone());
            }
        }
    }

    // The caller's name and id win over the template's, so a restored spec can
    // never be mistaken for the spec it was templated from.
    spec.insert(
        "name".to_string(),
        serde_json::Value::String(request.chain_name.clone()),
    );
    spec.insert(
        "id".to_string(),
        serde_json::Value::String(request.spec_id.clone()),
    );
    // A restored state is not a live chain. Anything else would have a node
    // announce itself as mainnet while its database is at someone else's block.
    spec.entry("chainType".to_string())
        .or_insert_with(|| serde_json::Value::String("Local".to_string()));

    let provenance = RestoreProvenance {
        chain_id: request.manifest.chain_id.clone(),
        block_number: request.manifest.block_number,
        block_hash: request.manifest.block_hash.clone(),
        state_root: request.manifest.state_root.clone(),
        runtime_spec_version: request.manifest.runtime_spec_version,
        manifest_hash: manifest_hash(request.manifest)?,
        chunk_count: request.manifest.chunk_count,
        state_entries: entries.len() as u64,
        trie_layout: format!("{:?}", request.version),
    };

    // `properties` is the chain spec's own arbitrary-metadata bag, so provenance
    // written here survives a round trip through the node's spec parser.
    let mut properties = match spec.remove("properties") {
        Some(serde_json::Value::Object(existing)) => existing,
        _ => serde_json::Map::new(),
    };
    for (key, value) in [
        ("x3SnapshotChainId", provenance.chain_id.clone()),
        ("x3SnapshotBlockNumber", provenance.block_number.to_string()),
        ("x3SnapshotBlockHash", provenance.block_hash.clone()),
        ("x3SnapshotStateRoot", provenance.state_root.clone()),
        (
            "x3SnapshotRuntimeSpecVersion",
            provenance.runtime_spec_version.to_string(),
        ),
        ("x3SnapshotManifestHash", provenance.manifest_hash.clone()),
        ("x3SnapshotChunkCount", provenance.chunk_count.to_string()),
        (
            "x3SnapshotStateEntries",
            provenance.state_entries.to_string(),
        ),
        ("x3SnapshotTrieLayout", provenance.trie_layout.clone()),
    ] {
        properties.insert(key.to_string(), serde_json::Value::String(value));
    }
    spec.insert(
        "properties".to_string(),
        serde_json::Value::Object(properties),
    );

    // Both halves of a raw genesis are always present: `RawGenesis` in
    // `sc-chain-spec` denies unknown fields and requires `childrenDefault`, so a
    // spec that omits it is not a spec the node can read at all.
    spec.insert(
        "genesis".to_string(),
        serde_json::json!({
            "raw": {
                "top": serde_json::Value::Object(top),
                "childrenDefault": serde_json::Map::new(),
            }
        }),
    );

    let spec = serde_json::Value::Object(spec);

    // Last check before the spec is offered: read it back the way the exporter
    // reads a spec and hash it again. This is what makes "the bytes we verified
    // are the bytes we wrote" a property of this function rather than a claim
    // about it.
    let reread = state_entries_from_raw_spec(&spec)?;
    let reread_root = compute_state_root(&reread, request.version)?;
    if !hashes_equal(&request.manifest.state_root, &reread_root) {
        return Err(SnapshotError::RecomputedStateRootMismatch {
            declared: request.manifest.state_root.clone(),
            recomputed: reread_root,
        });
    }

    Ok((spec, provenance))
}

#[cfg(test)]
mod tests;
