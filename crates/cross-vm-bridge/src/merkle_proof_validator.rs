//! # Merkle Proof Validator for Cross-VM Bridge Settlement
//!
//! Provides merkle proof validation for cross-VM bridge settlement operations.
//! This module integrates the state merkle proofs (from Gap #2) into the cross-VM
//! bridge settlement process, enabling cryptographically verified atomic settlement
//! without relying on external trust.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

/// Type alias for addresses (32-byte keys)
pub type Address = [u8; 32];

/// Type alias for hashes (32-byte values)
pub type Hash = [u8; 32];

/// Type alias for signatures (variable length)
pub type Signature = Vec<u8>;

/// Result type for merkle proof validation
pub type MerkleValidationResult = Result<(), MerkleProofValidationError>;

/// Errors that can occur during merkle proof validation
#[derive(Debug, Clone)]
pub enum MerkleProofValidationError {
    /// Merkle proof path is invalid
    InvalidMerkleProof(String),
    /// Insufficient validator signatures for Byzantine consensus
    InsufficientValidatorSignatures { have: u32, need: u32 },
    /// Validator signature verification failed
    SignatureVerificationFailed { validator_id: Address },
    /// Validator is not in the authorized set
    UnauthorizedValidator { validator_id: Address },
    /// State root mismatch between proof and claimed state
    StateRootMismatch { expected: Hash, actual: Hash },
    /// Block number in proof is invalid
    InvalidBlockNumber { block_number: u64 },
    /// Tree size is out of bounds
    InvalidTreeSize { size: u64 },
    /// Internal error
    InternalError(String),
}

impl core::fmt::Display for MerkleProofValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidMerkleProof(msg) => write!(f, "Invalid merkle proof: {msg}"),
            Self::InsufficientValidatorSignatures { have, need } => {
                write!(f, "Insufficient validator signatures: {have}/{need}")
            }
            Self::SignatureVerificationFailed { validator_id } => {
                write!(
                    f,
                    "Signature verification failed for validator {validator_id:?}"
                )
            }
            Self::UnauthorizedValidator { validator_id } => {
                write!(f, "Unauthorized validator: {validator_id:?}")
            }
            Self::StateRootMismatch { expected, actual } => {
                write!(
                    f,
                    "State root mismatch: expected {expected:?}, got {actual:?}"
                )
            }
            Self::InvalidBlockNumber { block_number } => {
                write!(f, "Invalid block number: {block_number}")
            }
            Self::InvalidTreeSize { size } => {
                write!(f, "Invalid tree size: {size}")
            }
            Self::InternalError(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

/// Settlement merkle proof data
#[derive(Debug, Clone)]
pub struct MerkleProofSettlement {
    /// The state root being proven
    pub state_root: Hash,
    /// Block number where state was finalized
    pub finalized_block: u64,
    /// Merkle proof path bytes
    pub merkle_proof_bytes: Vec<u8>,
    /// Validator signatures attesting to this settlement
    pub validator_signatures: BTreeMap<Address, Signature>,
    /// Execution index (for ordering multiple settlements)
    pub execution_index: u64,
    /// Optional metadata for cross-chain context
    pub metadata: Option<Vec<u8>>,
}

impl MerkleProofSettlement {
    /// Create a new merkle proof settlement
    pub fn new(
        state_root: Hash,
        finalized_block: u64,
        merkle_proof_bytes: Vec<u8>,
        execution_index: u64,
    ) -> Self {
        Self {
            state_root,
            finalized_block,
            merkle_proof_bytes,
            validator_signatures: BTreeMap::new(),
            execution_index,
            metadata: None,
        }
    }

    /// Add a validator signature to the settlement
    pub fn add_validator_signature(&mut self, validator_id: Address, signature: Signature) -> bool {
        if self.validator_signatures.contains_key(&validator_id) {
            return false; // Duplicate validator
        }
        self.validator_signatures.insert(validator_id, signature);
        true
    }

    /// Set metadata for cross-chain context
    pub fn set_metadata(&mut self, metadata: Vec<u8>) {
        self.metadata = Some(metadata);
    }

    /// Get the number of validator signatures
    pub fn validator_signature_count(&self) -> u32 {
        self.validator_signatures.len() as u32
    }

    /// Compute settlement hash for canonical ordering
    pub fn settlement_hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.state_root);
        hasher.update(self.finalized_block.to_le_bytes());
        hasher.update(self.execution_index.to_le_bytes());
        hasher.update(&self.merkle_proof_bytes);

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

/// Merkle proof validator trait for bridge settlement
pub trait MerkleProofValidator {
    /// Verify a settlement merkle proof
    ///
    /// # Arguments
    /// * `settlement` - The merkle proof settlement to verify
    /// * `authorized_validators` - Map of authorized validator IDs to their public keys
    /// * `finality_threshold` - Minimum number of validator signatures required for finality
    ///
    /// # Returns
    /// OK if settlement is valid and has sufficient Byzantine consensus, Err otherwise
    fn verify_settlement_proof(
        &self,
        settlement: &MerkleProofSettlement,
        authorized_validators: &BTreeMap<Address, Vec<u8>>,
        finality_threshold: u32,
    ) -> MerkleValidationResult;

    /// Verify merkle proof path bytes
    fn verify_merkle_path(
        &self,
        merkle_proof_bytes: &[u8],
        state_root: Hash,
        finalized_block: u64,
    ) -> MerkleValidationResult;

    /// Verify validator consensus on settlement
    fn verify_validator_consensus(
        &self,
        settlement: &MerkleProofSettlement,
        authorized_validators: &BTreeMap<Address, Vec<u8>>,
        finality_threshold: u32,
    ) -> MerkleValidationResult;
}

/// Default implementation of merkle proof validator
pub struct DefaultMerkleProofValidator;

impl DefaultMerkleProofValidator {
    /// Create a new default merkle proof validator
    pub fn new() -> Self {
        Self
    }

    /// Verify a single validator's signature using ed25519.
    ///
    /// The public key is stored as `authorized_validators[validator_id]` (32 bytes).
    /// The signature is a 64-byte ed25519 signature over the settlement hash.
    fn verify_validator_signature(
        &self,
        validator_id: &Address,
        settlement_hash: Hash,
        signature: &Signature,
        authorized_validators: &BTreeMap<Address, Vec<u8>>,
    ) -> MerkleValidationResult {
        // Check validator is authorized
        let pubkey_bytes = authorized_validators.get(validator_id).ok_or(
            MerkleProofValidationError::UnauthorizedValidator {
                validator_id: *validator_id,
            },
        )?;

        // Check signature is 64 bytes (ed25519 signature length)
        if signature.len() != 64 {
            return Err(MerkleProofValidationError::SignatureVerificationFailed {
                validator_id: *validator_id,
            });
        }

        // Check public key is 32 bytes
        if pubkey_bytes.len() != 32 {
            return Err(MerkleProofValidationError::SignatureVerificationFailed {
                validator_id: *validator_id,
            });
        }

        // Build ed25519 types from raw bytes
        let pubkey = sp_core::ed25519::Public::from_raw({
            let mut buf = [0u8; 32];
            buf.copy_from_slice(pubkey_bytes);
            buf
        });
        let sig = sp_core::ed25519::Signature::from_raw({
            let mut buf = [0u8; 64];
            buf.copy_from_slice(signature);
            buf
        });

        // Verify the ed25519 signature against the settlement hash
        if !sp_io::crypto::ed25519_verify(&sig, &settlement_hash, &pubkey) {
            return Err(MerkleProofValidationError::SignatureVerificationFailed {
                validator_id: *validator_id,
            });
        }

        Ok(())
    }
}

impl Default for DefaultMerkleProofValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl MerkleProofValidator for DefaultMerkleProofValidator {
    fn verify_settlement_proof(
        &self,
        settlement: &MerkleProofSettlement,
        authorized_validators: &BTreeMap<Address, Vec<u8>>,
        finality_threshold: u32,
    ) -> MerkleValidationResult {
        // Validate block number
        if settlement.finalized_block == 0 {
            return Err(MerkleProofValidationError::InvalidBlockNumber {
                block_number: settlement.finalized_block,
            });
        }

        // Validate state root is not all zeros
        if settlement.state_root == [0u8; 32] {
            return Err(MerkleProofValidationError::StateRootMismatch {
                expected: settlement.state_root,
                actual: [0u8; 32],
            });
        }

        // Verify merkle proof bytes are valid
        self.verify_merkle_path(
            &settlement.merkle_proof_bytes,
            settlement.state_root,
            settlement.finalized_block,
        )?;

        // Verify validator consensus
        self.verify_validator_consensus(settlement, authorized_validators, finality_threshold)?;

        Ok(())
    }

    fn verify_merkle_path(
        &self,
        merkle_proof_bytes: &[u8],
        state_root: Hash,
        finalized_block: u64,
    ) -> MerkleValidationResult {
        // CRITICAL-002/007 FIX: Validate state_root and finalized_block are embedded in proof bytes.
        // Format: [state_root: 32][finalized_block: u64 LE][leaf_index: u64 LE][leaf_hash: 32][sibling_0: 32]...
        // Minimum size: 32 (root) + 8 (block) + 8 (index) + 32 (leaf) = 80 bytes

        if merkle_proof_bytes.is_empty() {
            return Err(MerkleProofValidationError::InvalidMerkleProof(
                "Empty merkle proof bytes".into(),
            ));
        }

        if merkle_proof_bytes.len() < 80 {
            return Err(MerkleProofValidationError::InvalidMerkleProof(
                "Proof too short: need at least 80 bytes (state_root + finalized_block + index + leaf hash)".into(),
            ));
        }

        // Extract and validate embedded state_root matches parameter
        let mut embedded_root = [0u8; 32];
        embedded_root.copy_from_slice(&merkle_proof_bytes[0..32]);
        if embedded_root != state_root {
            return Err(MerkleProofValidationError::StateRootMismatch {
                expected: state_root,
                actual: embedded_root,
            });
        }

        // Extract and validate embedded finalized_block matches settlement metadata
        let embedded_block = u64::from_le_bytes(
            merkle_proof_bytes[32..40]
                .try_into()
                .map_err(|_| MerkleProofValidationError::InternalError("parse error".into()))?,
        );
        if embedded_block != finalized_block {
            return Err(MerkleProofValidationError::InvalidBlockNumber {
                block_number: embedded_block,
            });
        }

        // Remaining bytes after root+block+index+leaf must be a multiple of 32 (sibling hashes)
        let sibling_bytes_len = merkle_proof_bytes.len() - 80;
        if !sibling_bytes_len.is_multiple_of(32) {
            return Err(MerkleProofValidationError::InvalidMerkleProof(
                "Sibling hashes not aligned to 32 bytes".into(),
            ));
        }

        let leaf_index = u64::from_le_bytes(
            merkle_proof_bytes[40..48]
                .try_into()
                .map_err(|_| MerkleProofValidationError::InternalError("parse error".into()))?,
        );

        let mut current_hash = [0u8; 32];
        current_hash.copy_from_slice(&merkle_proof_bytes[48..80]);

        let num_siblings = sibling_bytes_len / 32;
        let mut idx = leaf_index;

        for i in 0..num_siblings {
            let sib_start = 80 + i * 32;
            let sibling = &merkle_proof_bytes[sib_start..sib_start + 32];

            let mut hasher = Sha256::new();
            if idx & 1 == 0 {
                // Current node is left child
                hasher.update(current_hash);
                hasher.update(sibling);
            } else {
                // Current node is right child
                hasher.update(sibling);
                hasher.update(current_hash);
            }
            let result = hasher.finalize();
            current_hash.copy_from_slice(&result);
            idx >>= 1;
        }

        // After walking the path, current_hash should equal state_root
        if current_hash != state_root {
            return Err(MerkleProofValidationError::StateRootMismatch {
                expected: state_root,
                actual: current_hash,
            });
        }

        Ok(())
    }

    fn verify_validator_consensus(
        &self,
        settlement: &MerkleProofSettlement,
        authorized_validators: &BTreeMap<Address, Vec<u8>>,
        finality_threshold: u32,
    ) -> MerkleValidationResult {
        // Check we have at least finality_threshold signatures
        let signature_count = settlement.validator_signature_count();
        if signature_count < finality_threshold {
            return Err(
                MerkleProofValidationError::InsufficientValidatorSignatures {
                    have: signature_count,
                    need: finality_threshold,
                },
            );
        }

        // Compute settlement hash for signature verification
        let settlement_hash = settlement.settlement_hash();

        // Verify each signature
        for (validator_id, signature) in &settlement.validator_signatures {
            self.verify_validator_signature(
                validator_id,
                settlement_hash,
                signature,
                authorized_validators,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a valid single-leaf merkle proof.
    /// With zero siblings, the leaf hash IS the state root.
    fn build_single_leaf_proof(leaf_data: &[u8], finalized_block: u64) -> (Hash, Vec<u8>) {
        let mut hasher = Sha256::new();
        hasher.update(leaf_data);
        let mut leaf_hash = [0u8; 32];
        leaf_hash.copy_from_slice(&hasher.finalize());

        // CRITICAL-002/007 FIX: Proof now includes state_root and finalized_block binding
        // Format: [state_root: 32 bytes][finalized_block: u64 LE][leaf_index: u64 LE][leaf_hash: 32 bytes]
        let mut proof = Vec::new();
        proof.extend_from_slice(&leaf_hash); // state_root is the leaf hash for single-leaf tree
        proof.extend_from_slice(&finalized_block.to_le_bytes());
        proof.extend_from_slice(&0u64.to_le_bytes());
        proof.extend_from_slice(&leaf_hash);

        (leaf_hash, proof)
    }

    /// Build a two-leaf merkle tree and return the root + proof for the left leaf.
    fn build_two_leaf_proof(finalized_block: u64) -> (Hash, Vec<u8>) {
        let mut h0 = Sha256::new();
        h0.update(b"leaf0");
        let mut leaf0 = [0u8; 32];
        leaf0.copy_from_slice(&h0.finalize());

        let mut h1 = Sha256::new();
        h1.update(b"leaf1");
        let mut leaf1 = [0u8; 32];
        leaf1.copy_from_slice(&h1.finalize());

        // root = SHA256(leaf0 || leaf1)
        let mut hr = Sha256::new();
        hr.update(leaf0);
        hr.update(leaf1);
        let mut root = [0u8; 32];
        root.copy_from_slice(&hr.finalize());

        // CRITICAL-002/007 FIX: proof includes state_root and finalized_block binding
        // Proof for leaf0 (index=0): [root][finalized_block][0u64][leaf0][leaf1 as sibling]
        let mut proof = Vec::new();
        proof.extend_from_slice(&root); // state_root embedded
        proof.extend_from_slice(&finalized_block.to_le_bytes());
        proof.extend_from_slice(&0u64.to_le_bytes());
        proof.extend_from_slice(&leaf0);
        proof.extend_from_slice(&leaf1);

        (root, proof)
    }

    #[test]
    fn test_settlement_creation() {
        let state_root = [1u8; 32];
        let settlement = MerkleProofSettlement::new(state_root, 100, vec![1, 2, 3, 4], 0);

        assert_eq!(settlement.state_root, state_root);
        assert_eq!(settlement.finalized_block, 100);
        assert_eq!(settlement.execution_index, 0);
        assert_eq!(settlement.validator_signature_count(), 0);
    }

    #[test]
    fn test_add_validator_signature() {
        let mut settlement = MerkleProofSettlement::new([1u8; 32], 100, vec![1, 2, 3, 4], 0);
        let validator_id = [2u8; 32];
        let signature = vec![1, 2, 3];

        assert!(settlement.add_validator_signature(validator_id, signature.clone()));
        assert_eq!(settlement.validator_signature_count(), 1);

        // Duplicate should be rejected
        assert!(!settlement.add_validator_signature(validator_id, signature));
        assert_eq!(settlement.validator_signature_count(), 1);
    }

    #[test]
    fn test_settlement_hash_deterministic() {
        let settlement1 = MerkleProofSettlement::new([1u8; 32], 100, vec![1, 2, 3, 4], 0);
        let settlement2 = MerkleProofSettlement::new([1u8; 32], 100, vec![1, 2, 3, 4], 0);

        assert_eq!(settlement1.settlement_hash(), settlement2.settlement_hash());
    }

    #[test]
    fn test_settlement_hash_changes_with_state_root() {
        let settlement1 = MerkleProofSettlement::new([1u8; 32], 100, vec![1, 2, 3, 4], 0);
        let settlement2 = MerkleProofSettlement::new([2u8; 32], 100, vec![1, 2, 3, 4], 0);

        assert_ne!(settlement1.settlement_hash(), settlement2.settlement_hash());
    }

    #[test]
    fn test_validator_creation() {
        let _validator = DefaultMerkleProofValidator::new();
    }

    #[test]
    fn test_verify_empty_merkle_proof() {
        let validator = DefaultMerkleProofValidator::new();
        let result = validator.verify_merkle_path(&[], [0u8; 32], 100);

        match result {
            Err(MerkleProofValidationError::InvalidMerkleProof(_)) => (),
            other => panic!("Expected InvalidMerkleProof, got {other:?}"),
        }
    }

    #[test]
    fn test_verify_too_short_merkle_proof() {
        let validator = DefaultMerkleProofValidator::new();
        let result = validator.verify_merkle_path(&[1, 2, 3, 4], [0u8; 32], 100);

        match result {
            Err(MerkleProofValidationError::InvalidMerkleProof(_)) => (),
            other => panic!("Expected InvalidMerkleProof, got {other:?}"),
        }
    }

    #[test]
    fn test_verify_single_leaf_merkle_proof() {
        let validator = DefaultMerkleProofValidator::new();
        let (root, proof) = build_single_leaf_proof(b"hello world", 100);
        assert!(validator.verify_merkle_path(&proof, root, 100).is_ok());
    }

    #[test]
    fn test_verify_two_leaf_merkle_proof() {
        let validator = DefaultMerkleProofValidator::new();
        let (root, proof) = build_two_leaf_proof(100);
        assert!(validator.verify_merkle_path(&proof, root, 100).is_ok());
    }

    #[test]
    fn test_merkle_proof_wrong_root_rejected() {
        let validator = DefaultMerkleProofValidator::new();
        let (_root, proof) = build_two_leaf_proof(100);
        let wrong_root = [0xffu8; 32];
        match validator.verify_merkle_path(&proof, wrong_root, 100) {
            Err(MerkleProofValidationError::StateRootMismatch { .. }) => (),
            other => panic!("Expected StateRootMismatch, got {other:?}"),
        }
    }

    #[test]
    fn test_merkle_proof_wrong_embedded_block_rejected() {
        let validator = DefaultMerkleProofValidator::new();
        let (root, proof) = build_single_leaf_proof(b"hello world", 100);
        match validator.verify_merkle_path(&proof, root, 101) {
            Err(MerkleProofValidationError::InvalidBlockNumber { block_number: 100 }) => (),
            other => panic!("Expected InvalidBlockNumber, got {other:?}"),
        }
    }

    #[test]
    fn test_verify_invalid_block_number() {
        let validator = DefaultMerkleProofValidator::new();
        let settlement = MerkleProofSettlement::new([1u8; 32], 0, vec![1, 2, 3, 4], 0);
        let authorized = BTreeMap::new();

        let result = validator.verify_settlement_proof(&settlement, &authorized, 1);

        match result {
            Err(MerkleProofValidationError::InvalidBlockNumber { block_number: 0 }) => (),
            other => panic!("Expected InvalidBlockNumber, got {other:?}"),
        }
    }

    #[test]
    fn test_verify_insufficient_validators() {
        let validator = DefaultMerkleProofValidator::new();
        let (root, proof) = build_single_leaf_proof(b"settle", 100);
        let mut settlement = MerkleProofSettlement::new(root, 100, proof, 0);

        // Add one signature (will fail count check before sig verification)
        let vid = [2u8; 32];
        settlement.add_validator_signature(vid, vec![0u8; 64]);

        // Require 3 validators — test directly via verify_validator_consensus
        let mut authorized = BTreeMap::new();
        authorized.insert(vid, vec![0u8; 32]);

        let result = validator.verify_validator_consensus(&settlement, &authorized, 3);
        match result {
            Err(MerkleProofValidationError::InsufficientValidatorSignatures {
                have: 1,
                need: 3,
            }) => (),
            other => panic!("Expected InsufficientValidatorSignatures, got {other:?}"),
        }
    }

    #[test]
    fn test_verify_unauthorized_validator() {
        let validator = DefaultMerkleProofValidator::new();
        let (root, proof) = build_single_leaf_proof(b"settle", 100);
        let mut settlement = MerkleProofSettlement::new(root, 100, proof, 0);
        let authorized = BTreeMap::new();

        // Add validator signature but don't authorize it
        let validator_id = [2u8; 32];
        settlement.add_validator_signature(validator_id, vec![0u8; 64]);

        // Test via verify_validator_consensus directly
        let result = validator.verify_validator_consensus(&settlement, &authorized, 1);
        match result {
            Err(MerkleProofValidationError::UnauthorizedValidator { validator_id: id })
                if id == validator_id => {}
            other => panic!("Expected UnauthorizedValidator, got {other:?}"),
        }
    }

    #[test]
    fn test_verify_valid_settlement_with_real_signatures() {
        use sp_core::{ed25519::Pair, Pair as PairT};

        let validator = DefaultMerkleProofValidator::new();
        let (root, proof) = build_single_leaf_proof(b"atomic settlement data", 100);
        let mut settlement = MerkleProofSettlement::new(root, 100, proof, 0);

        let settlement_hash = settlement.settlement_hash();
        let mut authorized = BTreeMap::new();

        // Generate 3 ed25519 keypairs, sign the settlement hash, and add
        for i in 0u8..3 {
            let seed = [i + 10; 32];
            let pair = Pair::from_seed(&seed);
            let pubkey = pair.public();

            let sig = pair.sign(&settlement_hash);

            let mut vid = [0u8; 32];
            vid.copy_from_slice(pubkey.as_ref());

            settlement.add_validator_signature(vid, sig.0.to_vec());
            authorized.insert(vid, pubkey.0.to_vec());
        }

        let result = validator.verify_settlement_proof(&settlement, &authorized, 2);
        assert!(
            result.is_ok(),
            "Valid settlement should pass: {:?}",
            result.err()
        );
    }
}
