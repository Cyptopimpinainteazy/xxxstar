//! Finality registry — enforces INV-R-002 (no double finalisation).
//!
//! Every proof hash that has been accepted into the finalised set is recorded
//! here. A second attempt to finalise the same hash is rejected with
//! [`FinalityError::AlreadyFinalised`]. This is a deterministic, stateful
//! guard that must sit at the settlement acceptance boundary.

use crate::types::{BlockHeight, Hash256};
use std::collections::HashMap;

/// Error returned by the finality registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinalityError {
    /// The proof hash was already finalised at the recorded block.
    AlreadyFinalised {
        proof_hash: Hash256,
        first_block: BlockHeight,
    },
    /// The provided proof hash is the zero hash, which is never valid.
    ZeroHash,
}

impl core::fmt::Display for FinalityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FinalityError::AlreadyFinalised {
                proof_hash,
                first_block,
            } => write!(
                f,
                "proof {} already finalised at block {}",
                hex_short(proof_hash),
                first_block
            ),
            FinalityError::ZeroHash => write!(f, "zero proof hash is invalid"),
        }
    }
}

/// A record of a finalised proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalityRecord {
    pub proof_hash: Hash256,
    pub finalised_at_block: BlockHeight,
}

/// Stateful registry that prevents double finalisation of the same proof.
///
/// Invariant: every `proof_hash` in `seen` was accepted exactly once.
/// Any subsequent call with the same hash returns `AlreadyFinalised`.
#[derive(Debug, Default)]
pub struct FinalityRegistry {
    /// Map from proof_hash to the block at which it was first finalised.
    seen: HashMap<Hash256, BlockHeight>,
}

impl FinalityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempt to register a proof as finalised at `block`.
    ///
    /// Returns `Ok(FinalityRecord)` on first acceptance.
    /// Returns `Err(FinalityError::AlreadyFinalised)` if the hash was previously
    /// registered (INV-R-002).
    pub fn accept(
        &mut self,
        proof_hash: Hash256,
        block: BlockHeight,
    ) -> Result<FinalityRecord, FinalityError> {
        if proof_hash == [0u8; 32] {
            return Err(FinalityError::ZeroHash);
        }
        if let Some(&first_block) = self.seen.get(&proof_hash) {
            return Err(FinalityError::AlreadyFinalised {
                proof_hash,
                first_block,
            });
        }
        self.seen.insert(proof_hash, block);
        Ok(FinalityRecord {
            proof_hash,
            finalised_at_block: block,
        })
    }

    /// Returns the block at which `proof_hash` was finalised, or `None`.
    pub fn lookup(&self, proof_hash: &Hash256) -> Option<BlockHeight> {
        self.seen.get(proof_hash).copied()
    }

    /// Total number of finalised proofs tracked.
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Returns `true` if no proofs have been finalised yet.
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

fn hex_short(bytes: &[u8]) -> String {
    bytes.iter().take(4).map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(seed: u8) -> Hash256 {
        let mut h = [0u8; 32];
        h[0] = seed;
        h[1] = 0xde;
        h[2] = 0xad;
        h
    }

    #[test]
    fn first_finalisation_accepted() {
        let mut reg = FinalityRegistry::new();
        let h = hash(1);
        let rec = reg.accept(h, 42).unwrap();
        assert_eq!(rec.proof_hash, h);
        assert_eq!(rec.finalised_at_block, 42);
    }

    #[test]
    fn second_finalisation_rejected() {
        let mut reg = FinalityRegistry::new();
        let h = hash(2);
        reg.accept(h, 10).unwrap();
        let err = reg.accept(h, 11).unwrap_err();
        assert_eq!(
            err,
            FinalityError::AlreadyFinalised {
                proof_hash: h,
                first_block: 10,
            }
        );
    }

    #[test]
    fn different_hashes_accepted_independently() {
        let mut reg = FinalityRegistry::new();
        reg.accept(hash(1), 1).unwrap();
        reg.accept(hash(2), 2).unwrap();
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn zero_hash_rejected() {
        let mut reg = FinalityRegistry::new();
        let err = reg.accept([0u8; 32], 1).unwrap_err();
        assert_eq!(err, FinalityError::ZeroHash);
    }

    #[test]
    fn lookup_returns_block() {
        let mut reg = FinalityRegistry::new();
        let h = hash(3);
        reg.accept(h, 99).unwrap();
        assert_eq!(reg.lookup(&h), Some(99));
    }

    #[test]
    fn lookup_returns_none_for_unseen() {
        let reg = FinalityRegistry::new();
        assert_eq!(reg.lookup(&hash(4)), None);
    }
}
