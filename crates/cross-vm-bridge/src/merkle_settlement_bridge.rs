//! # Merkle Settlement Integration for Cross-VM Bridge
//!
//! This module extends the CrossVmBridge with merkle proof verification capabilities
//! for the 2PC (Two-Phase Commit) settlement phase, enabling cryptographically verified
//! atomic operations with optional merkle proof verification.

use crate::merkle_proof_validator::{MerkleProofSettlement, MerkleProofValidator};
use crate::{CrossVmBridge, DispatchError};
use alloc::collections::BTreeMap;

pub type Address = [u8; 32];
pub type Hash = [u8; 32];

/// Merkle-enabled settlement request for bridge operations
#[derive(Debug, Clone)]
pub struct MerkleEnabledSettlement {
    /// Nonce of the operation being settled
    pub nonce: u64,
    /// Merkle settlement proof for verification (optional)
    pub merkle_proof: Option<MerkleProofSettlement>,
    /// Finality threshold for Byzantine consensus (default: 2/3)
    pub finality_threshold: u32,
    /// Current finalized block height from the destination chain.
    /// Required when `merkle_proof` is present.
    pub current_finalized_block: Option<u64>,
    /// Maximum allowed proof age in blocks.
    pub max_proof_age_blocks: u64,
}

const DEFAULT_MAX_PROOF_AGE_BLOCKS: u64 = 256;

impl MerkleEnabledSettlement {
    /// Create a new merkle-enabled settlement
    pub fn new(nonce: u64, merkle_proof: Option<MerkleProofSettlement>) -> Self {
        Self {
            nonce,
            merkle_proof,
            finality_threshold: 2, // Default: require 2/3 consensus
            current_finalized_block: None,
            max_proof_age_blocks: DEFAULT_MAX_PROOF_AGE_BLOCKS,
        }
    }

    /// Set custom finality threshold
    pub fn with_finality_threshold(mut self, threshold: u32) -> Self {
        self.finality_threshold = threshold;
        self
    }

    /// Provide strict freshness bounds for merkle proof verification.
    pub fn with_freshness(
        mut self,
        current_finalized_block: u64,
        max_proof_age_blocks: u64,
    ) -> Self {
        self.current_finalized_block = Some(current_finalized_block);
        self.max_proof_age_blocks = max_proof_age_blocks;
        self
    }
}

/// Extension trait for merkle-aware settlement verification
pub trait MerkleSettlementExt {
    /// Verify merkle settlement before committing operation.
    ///
    /// `authorized_validators` maps each validator's 32-byte ID to its public
    /// key bytes. The caller must supply the current validator set — typically
    /// sourced from on-chain storage or bridge genesis configuration.
    fn verify_merkle_settlement(
        &self,
        settlement: &MerkleEnabledSettlement,
        validator: &dyn MerkleProofValidator,
        authorized_validators: &BTreeMap<Address, alloc::vec::Vec<u8>>,
    ) -> Result<bool, DispatchError>;
}

impl MerkleSettlementExt for CrossVmBridge {
    fn verify_merkle_settlement(
        &self,
        settlement: &MerkleEnabledSettlement,
        validator: &dyn MerkleProofValidator,
        authorized_validators: &BTreeMap<Address, alloc::vec::Vec<u8>>,
    ) -> Result<bool, DispatchError> {
        // If no merkle proof provided, settlement is not required
        let Some(ref proof) = settlement.merkle_proof else {
            return Ok(true); // Non-merkle settlement is always valid
        };

        // Reject if the validator set is empty — this means the bridge is
        // not yet configured and settlement cannot be safely verified.
        if authorized_validators.is_empty() {
            return Err(DispatchError::Other(
                "Merkle settlement rejected: no authorized validators configured",
            ));
        }

        // Enforce proof freshness against the caller-provided finalized tip.
        let current_finalized = settlement.current_finalized_block.ok_or({
            DispatchError::Other(
                "Merkle settlement rejected: missing current finalized block for freshness check",
            )
        })?;

        if proof.finalized_block > current_finalized {
            return Err(DispatchError::Other(
                "Merkle settlement rejected: proof block is in the future",
            ));
        }

        let proof_age = current_finalized.saturating_sub(proof.finalized_block);
        if proof_age > settlement.max_proof_age_blocks {
            return Err(DispatchError::Other(
                "Merkle settlement rejected: proof is stale",
            ));
        }

        // Verify merkle settlement through validator
        validator
            .verify_settlement_proof(proof, authorized_validators, settlement.finality_threshold)
            .map(|_| true)
            .map_err(|e| {
                // Propagate the specific error for diagnostics
                log::error!("Merkle settlement verification failed: {e}");
                DispatchError::Other("Merkle settlement verification failed")
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merkle_proof_validator::DefaultMerkleProofValidator;

    /// Empty validator set — used for non-merkle settlement tests.
    fn empty_validators() -> BTreeMap<Address, alloc::vec::Vec<u8>> {
        BTreeMap::new()
    }

    /// A test validator set with one fake validator entry.
    fn test_validators() -> BTreeMap<Address, alloc::vec::Vec<u8>> {
        let mut m = BTreeMap::new();
        m.insert([1u8; 32], alloc::vec![10u8; 32]); // validator 1 → fake 32-byte pubkey
        m
    }

    #[test]
    fn test_merkle_enabled_settlement_creation() {
        let settlement = MerkleEnabledSettlement::new(1, None);
        assert_eq!(settlement.nonce, 1);
        assert!(settlement.merkle_proof.is_none());
        assert_eq!(settlement.finality_threshold, 2);
        assert_eq!(settlement.current_finalized_block, None);
        assert_eq!(
            settlement.max_proof_age_blocks,
            DEFAULT_MAX_PROOF_AGE_BLOCKS
        );
    }

    #[test]
    fn test_merkle_enabled_settlement_custom_threshold() {
        let settlement = MerkleEnabledSettlement::new(1, None).with_finality_threshold(3);
        assert_eq!(settlement.finality_threshold, 3);
    }

    #[test]
    fn test_verify_non_merkle_settlement() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();
        let settlement = MerkleEnabledSettlement::new(1, None);

        // Non-merkle settlement should always verify (validator set irrelevant)
        let result = bridge.verify_merkle_settlement(&settlement, &validator, &empty_validators());
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_merkle_settlement_empty_validator_set_rejected() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();

        // Proof present but no authorized validators → must reject
        let proof = MerkleProofSettlement {
            state_root: [42u8; 32],
            finalized_block: 100,
            merkle_proof_bytes: alloc::vec![
                42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: BTreeMap::new(),
            execution_index: 1,
            metadata: None,
        };

        let settlement = MerkleEnabledSettlement::new(1, Some(proof))
            .with_finality_threshold(1)
            .with_freshness(100, 256);
        let result = bridge.verify_merkle_settlement(&settlement, &validator, &empty_validators());
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_merkle_settlement_invalid_proof() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();

        let proof = MerkleProofSettlement {
            state_root: [0u8; 32],
            finalized_block: 100,
            merkle_proof_bytes: alloc::vec![
                42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: BTreeMap::new(),
            execution_index: 1,
            metadata: None,
        };

        let settlement = MerkleEnabledSettlement::new(1, Some(proof))
            .with_finality_threshold(1)
            .with_freshness(100, 256);
        let result = bridge.verify_merkle_settlement(&settlement, &validator, &test_validators());
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_merkle_settlement_empty_signatures() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();

        let proof = MerkleProofSettlement {
            state_root: [42u8; 32],
            finalized_block: 100,
            merkle_proof_bytes: alloc::vec![
                42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: BTreeMap::new(),
            execution_index: 1,
            metadata: None,
        };

        let settlement = MerkleEnabledSettlement::new(1, Some(proof))
            .with_finality_threshold(1)
            .with_freshness(100, 256);
        let result = bridge.verify_merkle_settlement(&settlement, &validator, &test_validators());
        assert!(result.is_err());
    }

    #[test]
    fn test_merkle_settlement_with_signatures() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();

        let mut signatures = BTreeMap::new();
        signatures.insert([1; 32], alloc::vec![100, 101, 102]);
        signatures.insert([2; 32], alloc::vec![103, 104, 105]);

        let proof = MerkleProofSettlement {
            state_root: [42; 32],
            finalized_block: 100,
            merkle_proof_bytes: alloc::vec![
                100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: signatures,
            execution_index: 1,
            metadata: None,
        };

        let settlement = MerkleEnabledSettlement::new(1, Some(proof)).with_freshness(100, 256);
        let result = bridge.verify_merkle_settlement(&settlement, &validator, &test_validators());
        let _ = result;
    }

    #[test]
    fn test_verify_merkle_settlement_rejects_missing_freshness_context() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();

        let proof = MerkleProofSettlement {
            state_root: [42u8; 32],
            finalized_block: 100,
            merkle_proof_bytes: alloc::vec![
                42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: BTreeMap::new(),
            execution_index: 1,
            metadata: None,
        };

        let settlement = MerkleEnabledSettlement::new(6, Some(proof));
        let result = bridge.verify_merkle_settlement(&settlement, &validator, &test_validators());
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_merkle_settlement_rejects_stale_or_future_proof_block() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();

        let stale_proof = MerkleProofSettlement {
            state_root: [42u8; 32],
            finalized_block: 100,
            merkle_proof_bytes: alloc::vec![
                42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: BTreeMap::new(),
            execution_index: 1,
            metadata: None,
        };

        let stale_settlement =
            MerkleEnabledSettlement::new(7, Some(stale_proof)).with_freshness(400, 64);
        assert!(bridge
            .verify_merkle_settlement(&stale_settlement, &validator, &test_validators())
            .is_err());

        let future_proof = MerkleProofSettlement {
            state_root: [42u8; 32],
            finalized_block: 500,
            merkle_proof_bytes: alloc::vec![
                42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 244, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: BTreeMap::new(),
            execution_index: 1,
            metadata: None,
        };

        let future_settlement =
            MerkleEnabledSettlement::new(8, Some(future_proof)).with_freshness(450, 64);
        assert!(bridge
            .verify_merkle_settlement(&future_settlement, &validator, &test_validators())
            .is_err());
    }

    #[test]
    fn test_merkle_settlement_nonce_tracking() {
        let settlement1 = MerkleEnabledSettlement::new(1, None);
        let settlement2 = MerkleEnabledSettlement::new(2, None);

        assert_eq!(settlement1.nonce, 1);
        assert_eq!(settlement2.nonce, 2);
        assert_ne!(settlement1.nonce, settlement2.nonce);
    }

    // ===== E2E Integration Tests =====

    #[test]
    fn test_e2e_merkle_enabled_atomic_swap_simple() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();
        let settlement = MerkleEnabledSettlement::new(1, None);

        let result = bridge.verify_merkle_settlement(&settlement, &validator, &empty_validators());
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_e2e_merkle_enabled_settlement_with_proof() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();

        let mut signatures = BTreeMap::new();
        signatures.insert([1; 32], alloc::vec![100, 101, 102]);
        signatures.insert([2; 32], alloc::vec![103, 104, 105]);
        signatures.insert([3; 32], alloc::vec![106, 107, 108]);

        let proof = MerkleProofSettlement {
            state_root: [42; 32],
            finalized_block: 100,
            merkle_proof_bytes: alloc::vec![
                42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: signatures,
            execution_index: 1,
            metadata: None,
        };

        let settlement = MerkleEnabledSettlement::new(2, Some(proof));
        let result = bridge.verify_merkle_settlement(&settlement, &validator, &test_validators());
        let _ = result;
    }

    #[test]
    fn test_e2e_byzantine_consensus_varying_thresholds() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();

        let s1 = MerkleEnabledSettlement::new(1, None).with_finality_threshold(2);
        assert!(bridge
            .verify_merkle_settlement(&s1, &validator, &empty_validators())
            .is_ok());

        let s2 = MerkleEnabledSettlement::new(2, None).with_finality_threshold(3);
        assert!(bridge
            .verify_merkle_settlement(&s2, &validator, &empty_validators())
            .is_ok());

        let s3 = MerkleEnabledSettlement::new(3, None).with_finality_threshold(4);
        assert!(bridge
            .verify_merkle_settlement(&s3, &validator, &empty_validators())
            .is_ok());
    }

    #[test]
    fn test_e2e_finality_threshold_enforcement() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();

        let mut signatures = BTreeMap::new();
        signatures.insert([1; 32], alloc::vec![100, 101, 102]);

        let proof = MerkleProofSettlement {
            state_root: [42; 32],
            finalized_block: 100,
            merkle_proof_bytes: alloc::vec![
                42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: signatures,
            execution_index: 1,
            metadata: None,
        };

        let settlement = MerkleEnabledSettlement::new(4, Some(proof))
            .with_finality_threshold(5)
            .with_freshness(100, 256);
        let result = bridge.verify_merkle_settlement(&settlement, &validator, &test_validators());
        assert!(result.is_err());
    }

    #[test]
    fn test_e2e_merkle_settlement_sequence() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();
        let validators = test_validators();

        let prepare = MerkleEnabledSettlement::new(1, None);
        assert!(bridge
            .verify_merkle_settlement(&prepare, &validator, &validators)
            .is_ok());

        let mut signatures = BTreeMap::new();
        signatures.insert([1; 32], alloc::vec![200, 201]);
        signatures.insert([2; 32], alloc::vec![202, 203]);

        let proof = MerkleProofSettlement {
            state_root: [99; 32],
            finalized_block: 200,
            merkle_proof_bytes: alloc::vec![
                50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: signatures,
            execution_index: 1,
            metadata: None,
        };

        let commit = MerkleEnabledSettlement::new(2, Some(proof)).with_freshness(210, 256);
        let _ = bridge.verify_merkle_settlement(&commit, &validator, &validators);

        let finalize = MerkleEnabledSettlement::new(3, None);
        assert!(bridge
            .verify_merkle_settlement(&finalize, &validator, &validators)
            .is_ok());
    }

    #[test]
    fn test_e2e_backward_compatibility_no_merkle_proofs() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();

        for i in 1..=10 {
            let settlement = MerkleEnabledSettlement::new(i, None);
            let result =
                bridge.verify_merkle_settlement(&settlement, &validator, &empty_validators());
            assert!(result.is_ok(), "Settlement {i} should verify");
            assert!(result.unwrap(), "Settlement {i} result should be true");
        }
    }

    #[test]
    fn test_e2e_mixed_merkle_and_non_merkle_settlements() {
        let bridge = CrossVmBridge::new();
        let validator = DefaultMerkleProofValidator::new();
        let validators = test_validators();

        let s1 = MerkleEnabledSettlement::new(1, None);
        assert!(bridge
            .verify_merkle_settlement(&s1, &validator, &validators)
            .is_ok());

        let proof = MerkleProofSettlement {
            state_root: [111; 32],
            finalized_block: 150,
            merkle_proof_bytes: alloc::vec![
                60, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 60, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: BTreeMap::new(),
            execution_index: 1,
            metadata: None,
        };
        let s2 = MerkleEnabledSettlement::new(2, Some(proof)).with_freshness(160, 256);
        let _ = bridge.verify_merkle_settlement(&s2, &validator, &validators);

        let s3 = MerkleEnabledSettlement::new(3, None);
        assert!(bridge
            .verify_merkle_settlement(&s3, &validator, &validators)
            .is_ok());
    }

    #[test]
    fn test_e2e_settlement_metadata_preservation() {
        let metadata = alloc::vec![0xDE, 0xAD, 0xBE, 0xEF];

        let proof = MerkleProofSettlement {
            state_root: [77; 32],
            finalized_block: 250,
            merkle_proof_bytes: alloc::vec![
                70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ],
            validator_signatures: BTreeMap::new(),
            execution_index: 1,
            metadata: Some(metadata.clone()),
        };

        let settlement = MerkleEnabledSettlement::new(5, Some(proof));
        assert_eq!(
            settlement.merkle_proof.as_ref().unwrap().metadata,
            Some(metadata)
        );
    }
}
