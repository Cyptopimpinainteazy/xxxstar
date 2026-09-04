//! # X3 Proof of History Generator
//!
//! Provides a verifiable time-ordering primitive for X3 blocks, inspired by
//! Solana's PoH design but implemented as a Substrate *consensus digest item*.
//!
//! ## Design
//!
//! Solana's PoH is a sequential SHA-256 hash chain: `H[n] = SHA256(H[n-1])`.
//! Mixing a transaction hash into the chain anchors the transaction in time:
//! `H = SHA256(H[n-1] || tx_hash)`.
//!
//! We implement this as a Substrate **digest item** with engine ID `*b"poh0"`,
//! so PoH data lives in the block header alongside Aura and GRANDPA digests.
//!
//! ### Digest Payload
//! ```text
//! PoHDigest {
//!   tick:         u64     — monotonically increasing PoH tick counter
//!   poh_hash:     [u8;32] — current head of the PoH chain
//!   tx_mix_root:  [u8;32] — Merkle root of tx hashes mixed into PoH this slot
//! }
//! ```
//!
//! ### Validation
//! A validator receiving a block can verify:
//! 1. `poh_hash == SHA256(prev_poh_hash || tx_mix_root)` — PoH chain integrity.
//! 2. `tick == prev_tick + 1` — monotonicity.
//! 3. `tx_mix_root` commits to all txs in the proposed block.
//!
//! This lets light clients and finality proofs anchor ordering without
//! re-executing every transaction.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};

// ─── Constants ────────────────────────────────────────────────────────────────

/// Substrate consensus engine ID for the PoH digest item.
pub const POH_ENGINE_ID: [u8; 4] = *b"poh0";

/// Inherent identifier for PoH.
pub const INHERENT_IDENTIFIER: sp_inherents::InherentIdentifier = *b"poh____0";

// ─── Core Types ───────────────────────────────────────────────────────────────

/// The PoH digest payload embedded in a block header.
///
/// Encoded as SCALE (or JSON in tests) and placed in the block header's
/// `DigestItem::Consensus(POH_ENGINE_ID, payload)`.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode, DecodeWithMemTracking,
)]
pub struct PoHDigest {
    /// Monotonically increasing tick counter (one per block).
    pub tick: u64,
    /// Current head of the PoH hash chain.
    pub poh_hash: [u8; 32],
    /// Merkle root of ordered transaction hashes mixed into PoH this block.
    pub tx_mix_root: [u8; 32],
}

impl PoHDigest {
    /// Encode to bytes for embedding in a `DigestItem`.
    pub fn encode_payload(&self) -> Vec<u8> {
        self.encode()
    }

    /// Decode from the bytes stored in a `DigestItem`.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 72 {
            return None;
        }
        let tick = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let mut poh_hash = [0u8; 32];
        let mut tx_mix_root = [0u8; 32];
        poh_hash.copy_from_slice(&bytes[8..40]);
        tx_mix_root.copy_from_slice(&bytes[40..72]);
        Some(Self {
            tick,
            poh_hash,
            tx_mix_root,
        })
    }
}

/// Inherent data for PoH.
#[derive(Encode, Decode, DecodeWithMemTracking)]
pub struct PoHInherentData {
    pub digest: PoHDigest,
}

#[cfg(feature = "std")]
pub struct PoHInherentDataProvider {
    pub digest: PoHDigest,
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for PoHInherentDataProvider {
    async fn provide_inherent_data(
        &self,
        inherent_data: &mut sp_inherents::InherentData,
    ) -> Result<(), sp_inherents::Error> {
        inherent_data.put_data(INHERENT_IDENTIFIER, &self.digest)
    }

    async fn try_handle_error(
        &self,
        _identifier: &sp_inherents::InherentIdentifier,
        _error: &[u8],
    ) -> Option<Result<(), sp_inherents::Error>> {
        None
    }
}

// ─── PoH Chain State ──────────────────────────────────────────────────────────

/// Maintains the running PoH hash chain across blocks.
///
/// One `PoHState` per node. Updated atomically when a block is finalized.
#[derive(Debug, Clone)]
pub struct PoHState {
    current_tick: u64,
    current_hash: [u8; 32],
}

impl Default for PoHState {
    fn default() -> Self {
        // Genesis PoH seed: SHA256("X3-GENESIS-POH-SEED")
        let seed = Sha256::digest(b"X3-GENESIS-POH-SEED");
        Self {
            current_tick: 0,
            current_hash: seed.into(),
        }
    }
}

impl PoHState {
    /// Create PoH state with a specific genesis hash (useful for testnets).
    pub fn with_genesis(genesis_hash: [u8; 32]) -> Self {
        Self {
            current_tick: genesis_hash[0] as u64, // small initial tick offset from genesis
            current_hash: genesis_hash,
        }
    }

    /// Current PoH tick.
    pub fn tick(&self) -> u64 {
        self.current_tick
    }

    /// Current PoH hash (head of chain).
    pub fn hash(&self) -> [u8; 32] {
        self.current_hash
    }

    /// Advance the PoH chain by one block, incorporating ordered transaction hashes.
    ///
    /// Algorithm:
    /// 1. Compute `tx_mix_root = merkle_root(sorted tx_hashes)`
    /// 2. `new_poh_hash = SHA256(prev_poh_hash || tx_mix_root)`
    /// 3. Increment tick.
    ///
    /// Returns the [`PoHDigest`] to embed in the block header.
    pub fn advance(&mut self, tx_hashes: &[[u8; 32]]) -> PoHDigest {
        let tx_mix_root = merkle_root(tx_hashes);

        // Chain: new_hash = SHA256(prev_hash || tx_mix_root)
        let mut h = Sha256::new();
        h.update(self.current_hash);
        h.update(tx_mix_root);
        let new_hash: [u8; 32] = h.finalize().into();

        self.current_tick += 1;
        self.current_hash = new_hash;

        debug!(
            "[PoH] Tick {} — hash={} tx_count={}",
            self.current_tick,
            hex::encode(&new_hash[..8]),
            tx_hashes.len(),
        );

        PoHDigest {
            tick: self.current_tick,
            poh_hash: new_hash,
            tx_mix_root,
        }
    }
}

// ─── PoH Verifier ─────────────────────────────────────────────────────────────

/// Verifies a PoH digest against the previous state.
///
/// Called during block import to ensure the proposer's PoH digest is valid.
pub struct PoHVerifier;

impl PoHVerifier {
    /// Verify a [`PoHDigest`] against the previous block's PoH hash and tick.
    ///
    /// Returns `Ok(())` if the digest is valid, `Err(reason)` otherwise.
    pub fn verify(
        digest: &PoHDigest,
        prev_tick: u64,
        prev_hash: &[u8; 32],
        tx_hashes: &[[u8; 32]],
    ) -> Result<(), PoHVerifyError> {
        // 1. Monotonicity check
        if digest.tick != prev_tick + 1 {
            return Err(PoHVerifyError::NonMonotonicTick {
                expected: prev_tick + 1,
                got: digest.tick,
            });
        }

        // 2. Recompute tx_mix_root
        let expected_root = merkle_root(tx_hashes);
        if digest.tx_mix_root != expected_root {
            warn!(
                "[PoH] tx_mix_root mismatch at tick {}: expected={} got={}",
                digest.tick,
                hex::encode(expected_root),
                hex::encode(digest.tx_mix_root),
            );
            return Err(PoHVerifyError::TxMixRootMismatch);
        }

        // 3. Recompute PoH hash
        let mut h = Sha256::new();
        h.update(prev_hash);
        h.update(digest.tx_mix_root);
        let expected_hash: [u8; 32] = h.finalize().into();

        if digest.poh_hash != expected_hash {
            warn!(
                "[PoH] Hash chain broken at tick {}: expected={} got={}",
                digest.tick,
                hex::encode(expected_hash),
                hex::encode(digest.poh_hash),
            );
            return Err(PoHVerifyError::HashChainBroken);
        }

        Ok(())
    }
}

/// Verification errors for PoH digests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PoHVerifyError {
    NonMonotonicTick { expected: u64, got: u64 },
    TxMixRootMismatch,
    HashChainBroken,
    MissingDigest,
}

impl std::fmt::Display for PoHVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonMonotonicTick { expected, got } => write!(
                f,
                "PoH tick not monotonic: expected {}, got {}",
                expected, got
            ),
            Self::TxMixRootMismatch => {
                write!(f, "PoH tx_mix_root does not match block transactions")
            }
            Self::HashChainBroken => write!(f, "PoH hash chain is broken"),
            Self::MissingDigest => write!(f, "PoH digest missing from block header"),
        }
    }
}

// ─── Merkle Root Helper ───────────────────────────────────────────────────────

/// Compute a simple binary Merkle root over ordered transaction hashes.
///
/// An empty slice returns the SHA256 of 64 zero bytes (defined empty root).
/// Otherwise pairs are hashed until one root remains.
pub fn merkle_root(hashes: &[[u8; 32]]) -> [u8; 32] {
    if hashes.is_empty() {
        // Defined empty root — deterministic
        return Sha256::digest([0u8; 64]).into();
    }

    if hashes.len() == 1 {
        return hashes[0];
    }

    // Work up the tree level by level
    let mut current: Vec<[u8; 32]> = hashes.to_vec();
    while current.len() > 1 {
        // Pad odd-length levels by duplicating last hash (standard Bitcoin Merkle behavior)
        if current.len() % 2 == 1 {
            let last = *current.last().unwrap();
            current.push(last);
        }

        current = current
            .chunks(2)
            .map(|pair| {
                let mut h = Sha256::new();
                h.update(pair[0]);
                h.update(pair[1]);
                h.finalize().into()
            })
            .collect();
    }

    current[0]
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tx_hash(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    #[test]
    fn test_poh_advances_monotonically() {
        let mut state = PoHState::default();
        assert_eq!(state.tick(), 0);

        let d1 = state.advance(&[make_tx_hash(0x01)]);
        assert_eq!(d1.tick, 1);

        let d2 = state.advance(&[make_tx_hash(0x02)]);
        assert_eq!(d2.tick, 2);
        assert_ne!(d1.poh_hash, d2.poh_hash);
    }

    #[test]
    fn test_poh_is_deterministic() {
        let txs = vec![make_tx_hash(0xAA), make_tx_hash(0xBB)];

        let mut state_a = PoHState::default();
        let d_a = state_a.advance(&txs);

        let mut state_b = PoHState::default();
        let d_b = state_b.advance(&txs);

        // Same inputs → same output
        assert_eq!(d_a, d_b);
    }

    #[test]
    fn test_poh_different_txs_different_hash() {
        let mut state = PoHState::default();
        let d1 = state.clone().advance(&[make_tx_hash(0x01)]);
        let d2 = state.advance(&[make_tx_hash(0x02)]);
        assert_ne!(d1.poh_hash, d2.poh_hash);
    }

    #[test]
    fn test_verification_roundtrip() {
        let mut state = PoHState::default();
        let prev_hash = state.hash();
        let prev_tick = state.tick();

        let txs = vec![make_tx_hash(0x10), make_tx_hash(0x20), make_tx_hash(0x30)];
        let digest = state.advance(&txs);

        let result = PoHVerifier::verify(&digest, prev_tick, &prev_hash, &txs);
        assert!(result.is_ok(), "Verification failed: {:?}", result);
    }

    #[test]
    fn test_verification_rejects_nonmonotonic_tick() {
        let mut state = PoHState::default();
        let prev_hash = state.hash();

        let txs = vec![make_tx_hash(0x10)];
        let mut digest = state.advance(&txs);
        digest.tick = 99; // tamper

        let result = PoHVerifier::verify(&digest, 0, &prev_hash, &txs);
        assert_eq!(
            result,
            Err(PoHVerifyError::NonMonotonicTick {
                expected: 1,
                got: 99
            })
        );
    }

    #[test]
    fn test_verification_rejects_tampered_hash() {
        let mut state = PoHState::default();
        let prev_hash = state.hash();

        let txs = vec![make_tx_hash(0x10)];
        let mut digest = state.advance(&txs);
        digest.poh_hash[0] ^= 0xFF; // tamper

        let result = PoHVerifier::verify(&digest, 0, &prev_hash, &txs);
        assert_eq!(result, Err(PoHVerifyError::HashChainBroken));
    }

    #[test]
    fn test_verification_rejects_wrong_txs() {
        let mut state = PoHState::default();
        let prev_hash = state.hash();

        let txs = vec![make_tx_hash(0x10)];
        let digest = state.advance(&txs);

        // Verify with different tx set
        let wrong_txs = vec![make_tx_hash(0xFF)];
        let result = PoHVerifier::verify(&digest, 0, &prev_hash, &wrong_txs);
        assert_eq!(result, Err(PoHVerifyError::TxMixRootMismatch));
    }

    #[test]
    fn test_digest_encode_decode_roundtrip() {
        let digest = PoHDigest {
            tick: 42,
            poh_hash: [0xAB; 32],
            tx_mix_root: [0xCD; 32],
        };
        let encoded = digest.encode();
        assert_eq!(encoded.len(), 72);
        let decoded = PoHDigest::decode(&encoded).unwrap();
        assert_eq!(digest, decoded);
    }

    #[test]
    fn test_merkle_root_empty() {
        // Empty → defined constant
        let r1 = merkle_root(&[]);
        let r2 = merkle_root(&[]);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_merkle_root_single() {
        let h = [0xAB; 32];
        assert_eq!(merkle_root(&[h]), h);
    }

    #[test]
    fn test_merkle_root_order_matters() {
        let h1 = [0x01; 32];
        let h2 = [0x02; 32];
        assert_ne!(merkle_root(&[h1, h2]), merkle_root(&[h2, h1]));
    }

    #[test]
    fn test_multi_block_chain_verification() {
        let mut state = PoHState::default();
        let mut prev_hash = state.hash();
        let mut prev_tick = state.tick();

        for i in 0u8..10 {
            let txs: Vec<[u8; 32]> = (0..5).map(|j| [i * 10 + j; 32]).collect();
            let digest = state.advance(&txs);

            let result = PoHVerifier::verify(&digest, prev_tick, &prev_hash, &txs);
            assert!(result.is_ok(), "Block {} failed: {:?}", i, result);

            prev_hash = digest.poh_hash;
            prev_tick = digest.tick;
        }
    }
}
