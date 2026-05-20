//! # State Merkle Proof Verification for Atomic VM
//!
//! Provides state merkle proof types and verification logic for validating state commitments
//! in the Atomic VM system. Merkle proofs enable cryptographic verification of state updates
//! without storing full state trees, essential for cross-chain settlement verification.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub type Hash = [u8; 32];

/// A single node in a merkle tree path
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MerkleNode {
    /// Hash value of this node
    pub hash: Hash,
    /// Whether this node is on the left or right of its parent
    pub is_left: bool,
}

impl MerkleNode {
    /// Create a new merkle node
    pub fn new(hash: Hash, is_left: bool) -> Self {
        Self { hash, is_left }
    }

    /// Compute the parent hash given a sibling hash
    pub fn parent_hash(&self, sibling: &MerkleNode) -> Hash {
        let mut hasher = Sha256::new();

        // Hash left then right for consistent ordering
        if self.is_left {
            hasher.update(&self.hash);
            hasher.update(&sibling.hash);
        } else {
            hasher.update(&sibling.hash);
            hasher.update(&self.hash);
        }

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

/// Merkle proof path from a leaf to the root
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MerkleProofPath {
    /// Leaf hash being proven
    pub leaf_hash: Hash,
    /// Path from leaf to root (siblings to hash upward)
    pub path: Vec<MerkleNode>,
    /// Expected root hash this path should lead to
    pub expected_root: Hash,
}

impl MerkleProofPath {
    /// Create a new merkle proof path
    pub fn new(leaf_hash: Hash, path: Vec<MerkleNode>, expected_root: Hash) -> Self {
        Self {
            leaf_hash,
            path,
            expected_root,
        }
    }

    /// Verify that the path is valid (leads to expected root)
    pub fn verify(&self) -> bool {
        if self.path.is_empty() {
            // Empty path means leaf IS the root
            return self.leaf_hash == self.expected_root;
        }

        let mut current_hash = self.leaf_hash;

        for node in self.path.iter() {
            // Determine current position based on sibling position
            // If sibling is on left, we're on right (current_is_left=false)
            // If sibling is on right, we're on left (current_is_left=true)
            let current_is_left = !node.is_left;

            // Create a temporary node for the current level
            let current_node = MerkleNode::new(current_hash, current_is_left);

            // Compute parent
            current_hash = current_node.parent_hash(node);
        }

        current_hash == self.expected_root
    }

    /// Get the number of levels in this proof
    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// Get the leaf index this proof corresponds to (for multi-proof trees)
    pub fn leaf_index(&self) -> u64 {
        // Reconstruct leaf index from path structure
        // If is_left is true, sibling is on left, so we're on right (bit = 1)
        // If is_left is false, sibling is on right, so we're on left (bit = 0)
        let mut index = 0u64;
        for (level, node) in self.path.iter().enumerate() {
            if !node.is_left {
                // Sibling is on right, so we're on left (bit stays 0)
                // Do nothing for bit 0
            } else {
                // Sibling is on left, so we're on right (bit = 1)
                index |= 1 << level;
            }
        }
        index
    }
}

/// State root verification information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateRootVerification {
    /// State root hash
    pub state_root: Hash,
    /// Block number this state root is from
    pub block_number: u64,
    /// Timestamp of state (unix seconds)
    pub timestamp: u64,
    /// Verification status
    pub verified: bool,
}

impl StateRootVerification {
    /// Create a new state root verification entry
    pub fn new(state_root: Hash, block_number: u64) -> Self {
        Self {
            state_root,
            block_number,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            verified: false,
        }
    }

    /// Mark this state root as verified
    pub fn mark_verified(&mut self) {
        self.verified = true;
    }
}

/// Complete state merkle proof with verification data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateMerkleProof {
    /// The merkle proof path
    pub proof_path: MerkleProofPath,
    /// State being proven
    pub state_root_verification: StateRootVerification,
    /// Number of leaves in the complete tree (for range proofs)
    pub tree_size: u64,
    /// Optional metadata for cross-chain verification
    pub metadata: Option<Vec<u8>>,
}

impl StateMerkleProof {
    /// Create a new state merkle proof
    pub fn new(
        proof_path: MerkleProofPath,
        state_root: Hash,
        block_number: u64,
        tree_size: u64,
    ) -> Self {
        let state_root_verification = StateRootVerification::new(state_root, block_number);

        Self {
            proof_path,
            state_root_verification,
            tree_size,
            metadata: None,
        }
    }

    /// Validate the entire state merkle proof
    pub fn validate(&self) -> bool {
        // Verify the path leads to the expected root
        if !self.proof_path.verify() {
            return false;
        }

        // Verify the expected root matches the state root being proven
        if self.proof_path.expected_root != self.state_root_verification.state_root {
            return false;
        }

        // Verify tree size is reasonable (non-zero, not excessively large)
        if self.tree_size == 0 || self.tree_size > 1_000_000 {
            return false;
        }

        // Verify leaf index is within tree bounds
        let leaf_index = self.proof_path.leaf_index();
        if leaf_index >= self.tree_size {
            return false;
        }

        true
    }

    /// Compute the state commitment hash for this proof
    pub fn compute_commitment(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.state_root_verification.state_root);
        hasher.update(&self.state_root_verification.block_number.to_le_bytes());
        hasher.update(&self.tree_size.to_le_bytes());
        hasher.update(&self.proof_path.leaf_hash);

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Set metadata for cross-chain verification
    pub fn set_metadata(&mut self, metadata: Vec<u8>) {
        self.metadata = Some(metadata);
    }
}

/// Helper function to compute merkle root from a list of leaf hashes
pub fn compute_merkle_root(leaves: &[Hash]) -> Hash {
    if leaves.is_empty() {
        return [0u8; 32];
    }

    if leaves.len() == 1 {
        return leaves[0];
    }

    // Build tree level by level
    let mut current_level = leaves.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for i in (0..current_level.len()).step_by(2) {
            let left = current_level[i];
            let right = if i + 1 < current_level.len() {
                current_level[i + 1]
            } else {
                // Hash odd node with itself
                current_level[i]
            };

            let mut hasher = Sha256::new();
            hasher.update(&left);
            hasher.update(&right);
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            next_level.push(hash);
        }

        current_level = next_level;
    }

    current_level[0]
}

/// Helper function to generate a merkle proof path for a leaf at a given index
pub fn generate_merkle_proof(
    leaves: &[Hash],
    leaf_index: usize,
) -> Result<MerkleProofPath, String> {
    if leaf_index >= leaves.len() {
        return Err("Leaf index out of bounds".to_string());
    }

    if leaves.is_empty() {
        return Err("No leaves provided".to_string());
    }

    let leaf_hash = leaves[leaf_index];

    // Build the tree and collect proof path
    let mut levels = vec![leaves.to_vec()];
    let mut current_level = leaves.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for i in (0..current_level.len()).step_by(2) {
            let left = current_level[i];
            let right = if i + 1 < current_level.len() {
                current_level[i + 1]
            } else {
                current_level[i]
            };

            let mut hasher = Sha256::new();
            hasher.update(&left);
            hasher.update(&right);
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            next_level.push(hash);
        }

        levels.push(next_level.clone());
        current_level = next_level;
    }

    // Extract proof path from tree levels
    let mut path = Vec::new();
    let mut current_index = leaf_index;

    for level in levels.iter().take(levels.len() - 1) {
        let sibling_index = if current_index % 2 == 0 {
            current_index + 1
        } else {
            current_index - 1
        };

        if sibling_index < level.len() {
            let sibling_hash = level[sibling_index];
            // is_left indicates if the SIBLING is on the left
            let is_left = sibling_index < current_index;
            path.push(MerkleNode::new(sibling_hash, is_left));
        }

        current_index /= 2;
    }

    let expected_root = levels[levels.len() - 1][0];

    Ok(MerkleProofPath::new(leaf_hash, path, expected_root))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_leaves(count: usize) -> Vec<Hash> {
        (0..count)
            .map(|i| {
                let mut hash = [0u8; 32];
                hash[0] = i as u8;
                hash
            })
            .collect()
    }

    #[test]
    fn test_merkle_node_creation() {
        let hash = [1u8; 32];
        let node = MerkleNode::new(hash, true);

        assert_eq!(node.hash, hash);
        assert!(node.is_left);
    }

    #[test]
    fn test_merkle_node_parent_hash() {
        let left = MerkleNode::new([1u8; 32], true);
        let right = MerkleNode::new([2u8; 32], false);

        let parent_hash = left.parent_hash(&right);

        // Hash should be deterministic
        let mut hasher = Sha256::new();
        hasher.update(&[1u8; 32]);
        hasher.update(&[2u8; 32]);
        let result = hasher.finalize();
        let mut expected = [0u8; 32];
        expected.copy_from_slice(&result);

        assert_eq!(parent_hash, expected);
    }

    #[test]
    fn test_merkle_proof_path_verify_single_leaf() {
        let leaf_hash = [1u8; 32];
        let path = MerkleProofPath::new(leaf_hash, vec![], leaf_hash);

        assert!(path.verify());
    }

    #[test]
    fn test_merkle_proof_path_verify_valid() {
        let leaves = create_test_leaves(4);
        let proof = generate_merkle_proof(&leaves, 0).unwrap();

        assert!(proof.verify());
    }

    #[test]
    fn test_merkle_proof_path_verify_invalid_root() {
        let leaf_hash = [1u8; 32];
        let wrong_root = [99u8; 32];
        let path = MerkleProofPath::new(leaf_hash, vec![], wrong_root);

        assert!(!path.verify());
    }

    #[test]
    fn test_compute_merkle_root_empty() {
        let root = compute_merkle_root(&[]);
        assert_eq!(root, [0u8; 32]);
    }

    #[test]
    fn test_compute_merkle_root_single_leaf() {
        let leaf = [42u8; 32];
        let root = compute_merkle_root(&[leaf]);
        assert_eq!(root, leaf);
    }

    #[test]
    fn test_compute_merkle_root_two_leaves() {
        let leaf1 = [1u8; 32];
        let leaf2 = [2u8; 32];

        let mut hasher = Sha256::new();
        hasher.update(&leaf1);
        hasher.update(&leaf2);
        let result = hasher.finalize();
        let mut expected = [0u8; 32];
        expected.copy_from_slice(&result);

        let root = compute_merkle_root(&[leaf1, leaf2]);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_compute_merkle_root_odd_leaves() {
        let leaves = create_test_leaves(5);
        let root = compute_merkle_root(&leaves);

        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_generate_merkle_proof_leaf_index() {
        let leaves = create_test_leaves(8);

        for i in 0..leaves.len() {
            let proof = generate_merkle_proof(&leaves, i).unwrap();
            assert_eq!(proof.leaf_index() as usize, i);
        }
    }

    #[test]
    fn test_generate_merkle_proof_verify_multiple() {
        let leaves = create_test_leaves(8);

        for i in 0..leaves.len() {
            let proof = generate_merkle_proof(&leaves, i).unwrap();
            assert!(proof.verify(), "Proof for leaf {} failed verification", i);
        }
    }

    #[test]
    fn test_generate_merkle_proof_invalid_index() {
        let leaves = create_test_leaves(4);
        let result = generate_merkle_proof(&leaves, 10);

        assert!(result.is_err());
    }

    #[test]
    fn test_state_root_verification_creation() {
        let state_root = [1u8; 32];
        let block_number = 100;

        let verification = StateRootVerification::new(state_root, block_number);

        assert_eq!(verification.state_root, state_root);
        assert_eq!(verification.block_number, block_number);
        assert!(!verification.verified);
    }

    #[test]
    fn test_state_root_verification_mark_verified() {
        let mut verification = StateRootVerification::new([1u8; 32], 100);

        assert!(!verification.verified);
        verification.mark_verified();
        assert!(verification.verified);
    }

    #[test]
    fn test_state_merkle_proof_creation() {
        let leaves = create_test_leaves(4);
        let proof_path = generate_merkle_proof(&leaves, 0).unwrap();
        let state_root = proof_path.expected_root;

        let proof = StateMerkleProof::new(proof_path, state_root, 100, leaves.len() as u64);

        assert_eq!(proof.state_root_verification.block_number, 100);
        assert_eq!(proof.tree_size, 4);
    }

    #[test]
    fn test_state_merkle_proof_validate() {
        let leaves = create_test_leaves(4);
        let proof_path = generate_merkle_proof(&leaves, 1).unwrap();
        let state_root = proof_path.expected_root;

        let proof = StateMerkleProof::new(proof_path, state_root, 100, leaves.len() as u64);

        assert!(proof.validate());
    }

    #[test]
    fn test_state_merkle_proof_validate_invalid_state_root() {
        let leaves = create_test_leaves(4);
        let proof_path = generate_merkle_proof(&leaves, 0).unwrap();
        let wrong_state_root = [99u8; 32];

        let proof = StateMerkleProof::new(proof_path, wrong_state_root, 100, leaves.len() as u64);

        assert!(!proof.validate());
    }

    #[test]
    fn test_state_merkle_proof_validate_invalid_tree_size() {
        let leaves = create_test_leaves(4);
        let proof_path = generate_merkle_proof(&leaves, 0).unwrap();
        let state_root = proof_path.expected_root;

        // Tree size is 0
        let proof = StateMerkleProof::new(proof_path.clone(), state_root, 100, 0);
        assert!(!proof.validate());
    }

    #[test]
    fn test_state_merkle_proof_compute_commitment() {
        let leaves = create_test_leaves(4);
        let proof_path = generate_merkle_proof(&leaves, 0).unwrap();
        let state_root = proof_path.expected_root;

        let proof = StateMerkleProof::new(proof_path, state_root, 100, leaves.len() as u64);
        let commitment = proof.compute_commitment();

        assert_ne!(commitment, [0u8; 32]);
    }

    #[test]
    fn test_state_merkle_proof_set_metadata() {
        let leaves = create_test_leaves(4);
        let proof_path = generate_merkle_proof(&leaves, 0).unwrap();
        let state_root = proof_path.expected_root;

        let mut proof = StateMerkleProof::new(proof_path, state_root, 100, leaves.len() as u64);
        assert!(proof.metadata.is_none());

        let metadata = vec![1, 2, 3, 4];
        proof.set_metadata(metadata.clone());
        assert_eq!(proof.metadata, Some(metadata));
    }
}
