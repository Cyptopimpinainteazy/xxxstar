// SPDX-License-Identifier: Apache-2.0
//
// supply_verification.rs — Runtime-level supply invariant verification (S0-1 fix).
//
// This module adds comprehensive supply verification that goes beyond the
// transaction-level checks in the supply ledger to provide block-level
// verification and merkle proof generation.
//
// SECURITY DESIGN (S0-1 Remediation):
//
// 1. Block-Level Verification:
//    - Verify supply invariants for ALL assets in `on_finalize`
//    - Catch any invariant violations that might slip through transaction checks
//    - Generate cryptographic proofs of supply correctness
//
// 2. Merkle Proof Generation:
//    - Build merkle tree of all asset supplies each block
//    - Enables external verification of supply state
//    - Supports light client verification
//
// 3. Audit Trail:
//    - Track all supply changes with event logs
//    - Enable forensic analysis of supply anomalies
//    - Support regulatory compliance requirements

use codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::{DispatchError, RuntimeDebug};
use sp_std::vec::Vec;
use x3_asset_kernel_types::{AssetId, Balance, InvariantError, SupplyLedger};

/// Supply verification proof for a single block.
///
/// This proof demonstrates that all asset supply invariants held at the
/// end of a block. External verifiers can validate the chain's economic
/// integrity without replaying all transactions.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, RuntimeDebug)]
pub struct SupplyProof {
    /// Block number this proof covers.
    pub block_number: u32,
    /// Merkle root of all asset supply states.
    pub supply_root: H256,
    /// Total number of assets verified.
    pub asset_count: u32,
    /// Total canonical supply across all assets.
    pub total_canonical: Balance,
    /// Total represented supply across all assets.
    pub total_represented: Balance,
    /// Individual asset proofs (for selective verification).
    pub asset_proofs: Vec<AssetSupplyProof>,
    /// Proof generation timestamp.
    pub timestamp: u64,
}

/// Supply proof for a single asset.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, RuntimeDebug)]
pub struct AssetSupplyProof {
    /// Asset identifier.
    pub asset_id: AssetId,
    /// Canonical supply ceiling.
    pub canonical_supply: Balance,
    /// Total represented supply.
    pub represented_supply: Balance,
    /// Breakdown by domain.
    pub native_supply: Balance,
    pub evm_supply: Balance,
    pub svm_supply: Balance,
    pub external_locked_supply: Balance,
    pub pending_supply: Balance,
    /// Leaf hash in the merkle tree.
    pub leaf_hash: H256,
    /// Merkle branch from leaf to root.
    pub merkle_branch: Vec<H256>,
    /// Index in the merkle tree.
    pub merkle_index: u32,
}

impl AssetSupplyProof {
    /// Create a proof from a supply ledger.
    pub fn from_ledger(asset_id: AssetId, ledger: &SupplyLedger, merkle_index: u32) -> Self {
        let leaf_hash = Self::compute_leaf_hash(asset_id, ledger);

        AssetSupplyProof {
            asset_id,
            canonical_supply: ledger.canonical_supply,
            represented_supply: ledger.represented().unwrap_or(Balance::MAX),
            native_supply: ledger.native_supply,
            evm_supply: ledger.evm_supply,
            svm_supply: ledger.svm_supply,
            external_locked_supply: ledger.external_locked_supply,
            pending_supply: ledger.pending_supply,
            leaf_hash,
            merkle_branch: Vec::new(), // Filled by merkle tree builder
            merkle_index,
        }
    }

    /// Compute the leaf hash for this asset's supply state.
    pub fn compute_leaf_hash(asset_id: AssetId, ledger: &SupplyLedger) -> H256 {
        let mut data = Vec::new();
        data.extend_from_slice(&asset_id.encode());
        data.extend_from_slice(&ledger.canonical_supply.to_le_bytes());
        data.extend_from_slice(&ledger.native_supply.to_le_bytes());
        data.extend_from_slice(&ledger.evm_supply.to_le_bytes());
        data.extend_from_slice(&ledger.svm_supply.to_le_bytes());
        data.extend_from_slice(&ledger.external_locked_supply.to_le_bytes());
        data.extend_from_slice(&ledger.pending_supply.to_le_bytes());

        H256::from(blake2_256(&data))
    }

    /// Verify this proof's invariant holds.
    pub fn verify_invariant(&self) -> Result<(), InvariantError> {
        // Check: represented ≤ canonical
        if self.represented_supply > self.canonical_supply {
            return Err(InvariantError::SupplyCeilingExceeded);
        }

        // Check: represented == sum of parts
        let sum = self
            .native_supply
            .checked_add(self.evm_supply)
            .and_then(|s| s.checked_add(self.svm_supply))
            .and_then(|s| s.checked_add(self.external_locked_supply))
            .and_then(|s| s.checked_add(self.pending_supply))
            .ok_or(InvariantError::ArithmeticOverflow)?;

        if sum != self.represented_supply {
            return Err(InvariantError::SupplyCeilingExceeded);
        }

        Ok(())
    }

    /// Verify this proof's merkle branch leads to the claimed root.
    pub fn verify_merkle_branch(&self, expected_root: H256) -> bool {
        let ledger = SupplyLedger {
            canonical_supply: self.canonical_supply,
            native_supply: self.native_supply,
            evm_supply: self.evm_supply,
            svm_supply: self.svm_supply,
            external_locked_supply: self.external_locked_supply,
            pending_supply: self.pending_supply,
        };

        // Recompute the leaf from proof fields so tampering with proof payload
        // cannot pass by reusing a stale `leaf_hash`.
        let mut hash = Self::compute_leaf_hash(self.asset_id, &ledger);

        for (i, sibling) in self.merkle_branch.iter().enumerate() {
            let position = (self.merkle_index >> i) & 1;
            hash = if position == 0 {
                // Current node is left child
                Self::hash_pair(hash, *sibling)
            } else {
                // Current node is right child
                Self::hash_pair(*sibling, hash)
            };
        }

        hash == expected_root
    }

    fn hash_pair(left: H256, right: H256) -> H256 {
        let mut data = Vec::new();
        data.extend_from_slice(left.as_bytes());
        data.extend_from_slice(right.as_bytes());
        H256::from(blake2_256(&data))
    }
}

/// Merkle tree builder for asset supply proofs.
pub struct SupplyMerkleTree {
    leaves: Vec<H256>,
}

impl SupplyMerkleTree {
    /// Create a new merkle tree from asset proofs.
    pub fn new(proofs: &mut [AssetSupplyProof]) -> Self {
        let leaves: Vec<H256> = proofs.iter().map(|p| p.leaf_hash).collect();

        // Build merkle branches for each leaf
        if !leaves.is_empty() {
            let tree = Self::build_tree(&leaves);
            for (i, proof) in proofs.iter_mut().enumerate() {
                proof.merkle_branch = Self::get_branch(&tree, i, leaves.len());
            }
        }

        Self { leaves }
    }

    /// Get the merkle root of the tree.
    pub fn root(&self) -> H256 {
        if self.leaves.is_empty() {
            return H256::zero();
        }

        let tree = Self::build_tree(&self.leaves);
        tree[tree.len() - 1]
    }

    /// Build the full merkle tree structure.
    fn build_tree(leaves: &[H256]) -> Vec<H256> {
        if leaves.is_empty() {
            return Vec::new();
        }

        let mut tree = leaves.to_vec();
        let mut level_size = leaves.len();

        // Build tree bottom-up
        while level_size > 1 {
            let next_level_size = level_size.div_ceil(2);
            let level_start = tree.len() - level_size;

            for i in 0..next_level_size {
                let left_idx = level_start + (i * 2);
                let right_idx = left_idx + 1;

                let left = tree[left_idx];
                let right = if right_idx < level_start + level_size {
                    tree[right_idx]
                } else {
                    // Odd number of nodes, duplicate last node
                    left
                };

                tree.push(AssetSupplyProof::hash_pair(left, right));
            }

            level_size = next_level_size;
        }

        tree
    }

    /// Get the merkle branch for a leaf at the given index.
    fn get_branch(tree: &[H256], leaf_index: usize, leaf_count: usize) -> Vec<H256> {
        let mut branch = Vec::new();
        let mut index = leaf_index;
        let mut level_size = leaf_count;
        let mut level_start = 0;

        while level_size > 1 {
            let sibling_index = if index.is_multiple_of(2) {
                index + 1
            } else {
                index - 1
            };

            let sibling = if sibling_index < level_size {
                tree[level_start + sibling_index]
            } else {
                // No sibling, use own hash
                tree[level_start + index]
            };

            branch.push(sibling);

            // Move to next level
            level_start += level_size;
            level_size = level_size.div_ceil(2);
            index /= 2;
        }

        branch
    }
}

/// Supply verification error types.
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub enum SupplyVerificationError {
    /// Asset supply invariant violated.
    InvariantViolation { asset_id: AssetId },
    /// Merkle proof verification failed.
    InvalidMerkleProof { asset_id: AssetId },
    /// Arithmetic overflow during verification.
    ArithmeticOverflow,
    /// No assets to verify.
    NoAssets,
}

impl From<SupplyVerificationError> for DispatchError {
    fn from(err: SupplyVerificationError) -> Self {
        match err {
            SupplyVerificationError::InvariantViolation { .. } => {
                DispatchError::Other("Supply invariant violation")
            }
            SupplyVerificationError::InvalidMerkleProof { .. } => {
                DispatchError::Other("Invalid merkle proof")
            }
            SupplyVerificationError::ArithmeticOverflow => {
                DispatchError::Arithmetic(sp_runtime::ArithmeticError::Overflow)
            }
            SupplyVerificationError::NoAssets => DispatchError::Other("No assets to verify"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_ledger(
        canonical: Balance,
        native: Balance,
        evm: Balance,
        svm: Balance,
    ) -> SupplyLedger {
        SupplyLedger {
            canonical_supply: canonical,
            native_supply: native,
            evm_supply: evm,
            svm_supply: svm,
            external_locked_supply: 0,
            pending_supply: 0,
        }
    }

    #[test]
    fn test_asset_proof_creation() {
        let ledger = create_test_ledger(1000, 400, 300, 300);
        let proof = AssetSupplyProof::from_ledger(H256([1u8; 32]), &ledger, 0);

        assert_eq!(proof.canonical_supply, 1000);
        assert_eq!(proof.represented_supply, 1000);
        assert!(proof.verify_invariant().is_ok());
    }

    #[test]
    fn test_invariant_violation_detection() {
        let mut ledger = create_test_ledger(1000, 400, 300, 300);
        // Violate invariant: represented > canonical
        ledger.native_supply = 1001;

        let proof = AssetSupplyProof::from_ledger(H256([1u8; 32]), &ledger, 0);
        assert!(proof.verify_invariant().is_err());
    }

    #[test]
    fn test_merkle_tree_single_leaf() {
        let ledger = create_test_ledger(1000, 1000, 0, 0);
        let mut proofs = vec![AssetSupplyProof::from_ledger(H256([1u8; 32]), &ledger, 0)];

        let tree = SupplyMerkleTree::new(&mut proofs);
        let root = tree.root();

        assert_eq!(root, proofs[0].leaf_hash);
        assert!(proofs[0].verify_merkle_branch(root));
    }

    #[test]
    fn test_merkle_tree_multiple_leaves() {
        let ledger1 = create_test_ledger(1000, 1000, 0, 0);
        let ledger2 = create_test_ledger(2000, 1000, 1000, 0);
        let ledger3 = create_test_ledger(3000, 1000, 1000, 1000);

        let mut proofs = vec![
            AssetSupplyProof::from_ledger(H256([1u8; 32]), &ledger1, 0),
            AssetSupplyProof::from_ledger(H256([2u8; 32]), &ledger2, 1),
            AssetSupplyProof::from_ledger(H256([3u8; 32]), &ledger3, 2),
        ];

        let tree = SupplyMerkleTree::new(&mut proofs);
        let root = tree.root();

        // All proofs should verify against the root
        for proof in &proofs {
            assert!(proof.verify_merkle_branch(root));
        }
    }

    #[test]
    fn test_invalid_merkle_proof_detected() {
        let ledger = create_test_ledger(1000, 1000, 0, 0);
        let mut proofs = vec![AssetSupplyProof::from_ledger(H256([1u8; 32]), &ledger, 0)];

        let tree = SupplyMerkleTree::new(&mut proofs);
        let root = tree.root();

        // Tamper with the proof
        proofs[0].canonical_supply = 2000;
        let tampered_ledger = create_test_ledger(2000, 1000, 0, 0);
        proofs[0].leaf_hash =
            AssetSupplyProof::compute_leaf_hash(H256([1u8; 32]), &tampered_ledger);

        // Should fail verification
        assert!(!proofs[0].verify_merkle_branch(root));
    }
}
