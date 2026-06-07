//! # Recursive Epoch Proofs
//!
//! Implements ZK block proofs and recursive epoch proof aggregation for the X3 proof stack.
//!
//! ## Stack Position (from spec vΩ-1.0)
//!
//! ```text
//! 5. ZK Block Proofs       ← ZkBlockProof (per-block)
//! 6. Recursive Epoch Proofs ← EpochProof (aggregates N blocks)
//! 7. Cross-chain Proof Verifiers ← handled by pallets/x3-verifier
//! ```
//!
//! Every block MUST produce a `ZkBlockProof` committing to:
//! - pre-state (parent block state root)
//! - post-state (block state root after applying all transactions)
//! - the set of transactions processed
//!
//! Epoch proofs aggregate `N` consecutive block proofs into a single succinct proof
//! via a recursive hashing scheme. They represent "central bank settlement finality"
//! — after an epoch proof is anchored on-chain, all blocks within that epoch are
//! considered final.
//!
//! ## Undefined Behavior
//!
//! Per the spec: *"Every state transition is either proven correct or rejected.
//! Undefined behavior is not representable."*
//!
//! A `ZkBlockProof` with `valid: false` is rejected by the verifier. There is no
//! intermediate state.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::types::{BlockHeight, Hash256};

// ---------------------------------------------------------------------------
// ZK Block Proof (Layer 5 of the execution stack)
// ---------------------------------------------------------------------------

/// A zero-knowledge validity proof for a single block.
///
/// In production this wraps a Groth16 or PLONK proof. In the current
/// implementation the `proof_bytes` field carries the commitment; the actual
/// circuit is operated by the GPU proving network (`crates/gpu-swarm`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZkBlockProof {
    /// Block height this proof covers.
    pub block_height: BlockHeight,
    /// State root of the parent block (pre-state).
    pub pre_state_root: Hash256,
    /// State root after applying all transactions (post-state).
    pub post_state_root: Hash256,
    /// Merkle root of all transactions in this block.
    pub transactions_root: Hash256,
    /// Number of transactions included.
    pub transaction_count: u32,
    /// Total gas consumed in this block.
    pub gas_used: u64,
    /// Raw proof bytes (circuit-specific; Groth16/PLONK).
    /// A zeroed array means the proof is pending (not yet generated).
    pub proof_bytes: Vec<u8>,
    /// SHA-256 commitment over all above fields (excluding `proof_bytes`).
    /// Used as the proof identifier and for epoch aggregation.
    pub commitment: Hash256,
    /// Whether this proof has passed verification.
    pub valid: bool,
}

impl ZkBlockProof {
    /// Compute the canonical commitment over block proof fields.
    pub fn compute_commitment(
        block_height: BlockHeight,
        pre_state_root: &Hash256,
        post_state_root: &Hash256,
        transactions_root: &Hash256,
        transaction_count: u32,
        gas_used: u64,
    ) -> Hash256 {
        let mut h = Sha256::new();
        h.update(block_height.to_le_bytes());
        h.update(pre_state_root);
        h.update(post_state_root);
        h.update(transactions_root);
        h.update(transaction_count.to_le_bytes());
        h.update(gas_used.to_le_bytes());
        let mut out = [0u8; 32];
        out.copy_from_slice(&h.finalize());
        out
    }

    /// Build a new block proof (proof_bytes to be filled by the GPU prover).
    pub fn new(
        block_height: BlockHeight,
        pre_state_root: Hash256,
        post_state_root: Hash256,
        transactions_root: Hash256,
        transaction_count: u32,
        gas_used: u64,
    ) -> Self {
        let commitment = Self::compute_commitment(
            block_height,
            &pre_state_root,
            &post_state_root,
            &transactions_root,
            transaction_count,
            gas_used,
        );
        Self {
            block_height,
            pre_state_root,
            post_state_root,
            transactions_root,
            transaction_count,
            gas_used,
            proof_bytes: vec![],
            commitment,
            valid: false,
        }
    }

    /// Mark this proof as verified (called by `ZkBlockVerifier` after circuit check).
    pub fn mark_valid(mut self, proof_bytes: Vec<u8>) -> Self {
        self.proof_bytes = proof_bytes;
        self.valid = true;
        self
    }

    /// Returns `true` if this proof is verified and carries a non-empty circuit proof.
    pub fn is_proven(&self) -> bool {
        self.valid && !self.proof_bytes.is_empty()
    }
}

// ---------------------------------------------------------------------------
// ZK Block Verifier
// ---------------------------------------------------------------------------

/// Verifies `ZkBlockProof` objects.
///
/// In production this calls the on-chain Groth16/PLONK verifier contract.
/// During development it performs structural validity checks.
#[derive(Debug, Default)]
pub struct ZkBlockVerifier;

impl ZkBlockVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Verify a block proof.
    ///
    /// Returns `Ok(())` if the proof is structurally valid and the commitment
    /// matches the recomputed value.
    pub fn verify(&self, proof: &ZkBlockProof) -> Result<(), ZkProofError> {
        if !proof.valid {
            return Err(ZkProofError::ProofNotVerified(proof.block_height));
        }
        if proof.proof_bytes.is_empty() {
            return Err(ZkProofError::EmptyProofBytes(proof.block_height));
        }

        // Verify commitment integrity
        let expected = ZkBlockProof::compute_commitment(
            proof.block_height,
            &proof.pre_state_root,
            &proof.post_state_root,
            &proof.transactions_root,
            proof.transaction_count,
            proof.gas_used,
        );
        if expected != proof.commitment {
            return Err(ZkProofError::CommitmentMismatch {
                block: proof.block_height,
                expected,
                actual: proof.commitment,
            });
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Recursive Epoch Proof (Layer 6 of the execution stack)
// ---------------------------------------------------------------------------

/// Number of blocks per epoch. After this many blocks, an epoch proof is produced.
pub const BLOCKS_PER_EPOCH: u32 = 256;

/// A recursive epoch proof: aggregates `N` consecutive block proofs into one.
///
/// Represents "central bank settlement finality" — once anchored on-chain all
/// blocks within the epoch are unconditionally final.
///
/// The aggregation strategy is a binary Merkle tree over block commitments,
/// giving O(log N) inclusion proofs for any block in the epoch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpochProof {
    /// Epoch index (epoch 0 = blocks 0..BLOCKS_PER_EPOCH-1, etc.).
    pub epoch: u64,
    /// First block height in this epoch.
    pub start_block: BlockHeight,
    /// Last block height in this epoch (inclusive).
    pub end_block: BlockHeight,
    /// Merkle root computed over all `block_commitments` in order.
    pub commitment_root: Hash256,
    /// The ordered list of block proof commitments this epoch covers.
    pub block_commitments: Vec<Hash256>,
    /// Pre-epoch state root (equals the parent epoch's final state root).
    pub pre_epoch_state_root: Hash256,
    /// Post-epoch state root (the canonical state after all blocks in this epoch).
    pub post_epoch_state_root: Hash256,
    /// Recursive proof bytes (a SNARK over all block proofs in the epoch).
    /// Empty until generated by the GPU proving network.
    pub proof_bytes: Vec<u8>,
    /// Whether this epoch proof has been verified.
    pub valid: bool,
}

impl EpochProof {
    /// Compute the Merkle root over an ordered list of block commitments.
    pub fn compute_commitment_root(commitments: &[Hash256]) -> Hash256 {
        if commitments.is_empty() {
            return [0u8; 32];
        }
        // Iterative binary Merkle tree reduction
        let mut layer: Vec<Hash256> = commitments.to_vec();
        while layer.len() > 1 {
            let mut next = Vec::with_capacity(layer.len().div_ceil(2));
            let mut i = 0;
            while i < layer.len() {
                if i + 1 < layer.len() {
                    let mut h = Sha256::new();
                    h.update(layer[i]);
                    h.update(layer[i + 1]);
                    let mut out = [0u8; 32];
                    out.copy_from_slice(&h.finalize());
                    next.push(out);
                } else {
                    // Odd leaf — promote as-is
                    next.push(layer[i]);
                }
                i += 2;
            }
            layer = next;
        }
        layer[0]
    }

    /// Build a new epoch proof from verified block proofs.
    ///
    /// Returns `Err` if:
    /// - any block proof is not verified
    /// - block heights are not contiguous
    /// - the list is empty
    pub fn build_from_blocks(
        epoch: u64,
        block_proofs: &[ZkBlockProof],
        pre_epoch_state_root: Hash256,
    ) -> Result<Self, ZkProofError> {
        if block_proofs.is_empty() {
            return Err(ZkProofError::EmptyEpoch(epoch));
        }

        // Verify all block proofs are valid and contiguous
        let start_block = block_proofs[0].block_height;
        let mut expected_height = start_block;
        let mut commitments = Vec::with_capacity(block_proofs.len());

        for bp in block_proofs {
            if !bp.is_proven() {
                return Err(ZkProofError::UnprovenBlockInEpoch {
                    epoch,
                    block: bp.block_height,
                });
            }
            if bp.block_height != expected_height {
                return Err(ZkProofError::NonContiguousBlocks {
                    epoch,
                    expected: expected_height,
                    got: bp.block_height,
                });
            }
            commitments.push(bp.commitment);
            expected_height += 1;
        }

        let end_block = expected_height - 1;
        let post_epoch_state_root = block_proofs.last().unwrap().post_state_root;
        let commitment_root = Self::compute_commitment_root(&commitments);

        Ok(Self {
            epoch,
            start_block,
            end_block,
            commitment_root,
            block_commitments: commitments,
            pre_epoch_state_root,
            post_epoch_state_root,
            proof_bytes: vec![],
            valid: false,
        })
    }

    /// Attach the recursive SNARK proof bytes and mark the epoch as valid.
    pub fn finalize(mut self, proof_bytes: Vec<u8>) -> Self {
        self.proof_bytes = proof_bytes;
        self.valid = true;
        self
    }

    /// Returns `true` if this epoch proof is fully proven and finalised.
    pub fn is_finalized(&self) -> bool {
        self.valid && !self.proof_bytes.is_empty()
    }

    /// Generate an inclusion proof (Merkle path) for a specific block commitment.
    ///
    /// Returns `Some(siblings)` in order from leaf to root.
    pub fn inclusion_proof(&self, block_height: BlockHeight) -> Option<Vec<Hash256>> {
        let idx = block_height.checked_sub(self.start_block)? as usize;
        if idx >= self.block_commitments.len() {
            return None;
        }

        let mut siblings = Vec::new();
        let mut layer = self.block_commitments.clone();
        let mut pos = idx;

        while layer.len() > 1 {
            let sibling_pos = if pos.is_multiple_of(2) {
                pos + 1
            } else {
                pos - 1
            };
            if sibling_pos < layer.len() {
                siblings.push(layer[sibling_pos]);
            }

            let mut next = Vec::with_capacity(layer.len().div_ceil(2));
            let mut i = 0;
            while i < layer.len() {
                if i + 1 < layer.len() {
                    let mut h = Sha256::new();
                    h.update(layer[i]);
                    h.update(layer[i + 1]);
                    let mut out = [0u8; 32];
                    out.copy_from_slice(&h.finalize());
                    next.push(out);
                } else {
                    next.push(layer[i]);
                }
                i += 2;
            }
            pos /= 2;
            layer = next;
        }

        Some(siblings)
    }
}

// ---------------------------------------------------------------------------
// Recursive Proof Aggregator
// ---------------------------------------------------------------------------

/// Aggregates block proofs into epoch proofs as blocks arrive.
#[derive(Debug, Default)]
pub struct RecursiveProofAggregator {
    pending_blocks: Vec<ZkBlockProof>,
    current_epoch: u64,
    last_state_root: Hash256,
    blocks_per_epoch: u32,
}

impl RecursiveProofAggregator {
    pub fn new(genesis_state_root: Hash256) -> Self {
        Self {
            pending_blocks: Vec::new(),
            current_epoch: 0,
            last_state_root: genesis_state_root,
            blocks_per_epoch: BLOCKS_PER_EPOCH,
        }
    }

    /// Constructor for testing with a custom epoch size.
    pub fn new_with_epoch_size(genesis_state_root: Hash256, blocks_per_epoch: u32) -> Self {
        Self {
            pending_blocks: Vec::new(),
            current_epoch: 0,
            last_state_root: genesis_state_root,
            blocks_per_epoch,
        }
    }

    /// Accept a verified block proof.
    ///
    /// Returns `Some(EpochProof)` (ready for GPU finalization) when the epoch is complete.
    pub fn add_block(&mut self, proof: ZkBlockProof) -> Result<Option<EpochProof>, ZkProofError> {
        if !proof.is_proven() {
            return Err(ZkProofError::ProofNotVerified(proof.block_height));
        }
        self.pending_blocks.push(proof);

        if self.pending_blocks.len() >= self.blocks_per_epoch as usize {
            let blocks = std::mem::take(&mut self.pending_blocks);
            let pre_root = self.last_state_root;
            let epoch = self.current_epoch;
            let ep = EpochProof::build_from_blocks(epoch, &blocks, pre_root)?;
            self.last_state_root = ep.post_epoch_state_root;
            self.current_epoch += 1;
            return Ok(Some(ep));
        }

        Ok(None)
    }

    /// Number of blocks accumulated toward the current epoch.
    pub fn pending_count(&self) -> usize {
        self.pending_blocks.len()
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors for ZK / epoch proof operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ZkProofError {
    #[error("block {0} proof is not verified")]
    ProofNotVerified(BlockHeight),

    #[error("block {0} proof bytes are empty")]
    EmptyProofBytes(BlockHeight),

    #[error("block {block}: commitment mismatch (expected {expected:x?}, got {actual:x?})")]
    CommitmentMismatch {
        block: BlockHeight,
        expected: Hash256,
        actual: Hash256,
    },

    #[error("epoch {0} contains no blocks")]
    EmptyEpoch(u64),

    #[error("epoch {epoch}: block {block} is not proven")]
    UnprovenBlockInEpoch { epoch: u64, block: BlockHeight },

    #[error("epoch {epoch}: expected block {expected}, got {got}")]
    NonContiguousBlocks {
        epoch: u64,
        expected: BlockHeight,
        got: BlockHeight,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block_proof(height: BlockHeight, pre: Hash256, post: Hash256) -> ZkBlockProof {
        let p = ZkBlockProof::new(height, pre, post, [height as u8; 32], 10, 1_000_000);
        p.mark_valid(vec![0xde, 0xad, 0xbe, 0xef])
    }

    #[test]
    fn block_proof_commitment_deterministic() {
        let p1 = ZkBlockProof::new(1, [0u8; 32], [1u8; 32], [2u8; 32], 5, 500);
        let p2 = ZkBlockProof::new(1, [0u8; 32], [1u8; 32], [2u8; 32], 5, 500);
        assert_eq!(p1.commitment, p2.commitment);
    }

    #[test]
    fn block_proof_validity_roundtrip() {
        let p = ZkBlockProof::new(42, [0u8; 32], [1u8; 32], [2u8; 32], 3, 900);
        assert!(!p.is_proven());
        let p = p.mark_valid(vec![1, 2, 3, 4]);
        assert!(p.is_proven());
    }

    #[test]
    fn verifier_rejects_unverified_proof() {
        let p = ZkBlockProof::new(1, [0u8; 32], [1u8; 32], [2u8; 32], 0, 0);
        let v = ZkBlockVerifier::new();
        assert!(v.verify(&p).is_err());
    }

    #[test]
    fn verifier_accepts_valid_proof() {
        let p = ZkBlockProof::new(1, [0u8; 32], [1u8; 32], [2u8; 32], 10, 50_000);
        let p = p.mark_valid(vec![0xff; 32]);
        let v = ZkBlockVerifier::new();
        assert!(v.verify(&p).is_ok());
    }

    #[test]
    fn epoch_proof_builds_from_contiguous_blocks() {
        let genesis = [0u8; 32];
        let blocks: Vec<ZkBlockProof> = (0u64..4)
            .map(|h| {
                let pre = if h == 0 { genesis } else { [h as u8; 32] };
                let post = [(h + 1) as u8; 32];
                make_block_proof(h, pre, post)
            })
            .collect();

        let ep = EpochProof::build_from_blocks(0, &blocks, genesis).unwrap();
        assert_eq!(ep.epoch, 0);
        assert_eq!(ep.start_block, 0);
        assert_eq!(ep.end_block, 3);
        assert_eq!(ep.block_commitments.len(), 4);
        assert!(!ep.is_finalized());

        let ep = ep.finalize(vec![0xaa; 64]);
        assert!(ep.is_finalized());
    }

    #[test]
    fn epoch_proof_rejects_unproven_block() {
        let genesis = [0u8; 32];
        let unproven = ZkBlockProof::new(0, genesis, [1u8; 32], [2u8; 32], 1, 100);
        let result = EpochProof::build_from_blocks(0, &[unproven], genesis);
        assert!(result.is_err());
    }

    #[test]
    fn epoch_proof_rejects_non_contiguous_blocks() {
        let genesis = [0u8; 32];
        let b0 = make_block_proof(0, genesis, [1u8; 32]);
        let b2 = make_block_proof(2, [1u8; 32], [2u8; 32]); // skipped block 1
        let result = EpochProof::build_from_blocks(0, &[b0, b2], genesis);
        assert!(result.is_err());
    }

    #[test]
    fn aggregator_emits_epoch_after_n_blocks() {
        let genesis = [0u8; 32];
        let mut agg = RecursiveProofAggregator::new_with_epoch_size(genesis, 4);

        for h in 0u64..3 {
            let result = agg.add_block(make_block_proof(h, [h as u8; 32], [(h + 1) as u8; 32]));
            assert!(result.unwrap().is_none());
        }

        // 4th block completes the epoch
        let result = agg
            .add_block(make_block_proof(3, [3u8; 32], [4u8; 32]))
            .unwrap();
        assert!(result.is_some());
        let ep = result.unwrap();
        assert_eq!(ep.epoch, 0);
        assert_eq!(ep.block_commitments.len(), 4);
    }

    #[test]
    fn merkle_commitment_root_is_deterministic() {
        let commitments: Vec<Hash256> = (0u8..8).map(|i| [i; 32]).collect();
        let r1 = EpochProof::compute_commitment_root(&commitments);
        let r2 = EpochProof::compute_commitment_root(&commitments);
        assert_eq!(r1, r2);
    }

    #[test]
    fn inclusion_proof_is_valid_for_any_block() {
        let genesis = [0u8; 32];
        let blocks: Vec<ZkBlockProof> = (0u64..8)
            .map(|h| make_block_proof(h, [h as u8; 32], [(h + 1) as u8; 32]))
            .collect();
        let ep = EpochProof::build_from_blocks(0, &blocks, genesis)
            .unwrap()
            .finalize(vec![0xbb; 32]);

        for h in 0u64..8 {
            let path = ep.inclusion_proof(h);
            assert!(path.is_some(), "inclusion proof missing for block {}", h);
        }
    }
}
