use serde::{Deserialize, Serialize};
use sp_core::{
    ed25519::{Public as Ed25519Public, Signature as Ed25519Signature},
    ByteArray, Pair as PairTrait,
};
use std::collections::HashSet;

pub type Hash = [u8; 32];
pub type ChainId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofVerificationError {
    EmptyValidatorSet,
    ValidatorSetHashMismatch,
    InvalidSignatureLength(u32),
    DuplicateValidatorSignature,
    ValidatorIndexOutOfBounds(usize),
    InvalidEd25519Signature,
    InsufficientSupermajoritySignatures,
    InvalidPrecommitLength,
    DuplicatePrecommit,
    InsufficientPrecommits,
    InvalidFinalityProof,
    InvalidStateCommitmentRoot,
    InvalidReceiptHash,
    MissingReceiptInclusionProof,
    InvalidIntentHash,
    InvalidIntentLockResources,
    InvalidSlashOffender,
    InvalidSlashAmount,
    ProofTypePayloadMismatch,
    ZkProofNotImplemented,
}

impl core::fmt::Display for ProofVerificationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyValidatorSet => write!(f, "Empty validator set"),
            Self::ValidatorSetHashMismatch => write!(f, "Validator set hash mismatch"),
            Self::InvalidSignatureLength(len) => write!(f, "Invalid signature length (expected 68 bytes, got {})", len),
            Self::DuplicateValidatorSignature => write!(f, "Duplicate validator signature detected"),
            Self::ValidatorIndexOutOfBounds(idx) => write!(f, "Validator index {} out of bounds", idx),
            Self::InvalidEd25519Signature => write!(f, "Invalid Ed25519 signature format"),
            Self::InsufficientSupermajoritySignatures => write!(f, "Insufficient valid signatures for supermajority"),
            Self::InvalidPrecommitLength => write!(f, "Invalid precommit length (expected 68 bytes)"),
            Self::DuplicatePrecommit => write!(f, "Duplicate precommit detected"),
            Self::InsufficientPrecommits => write!(f, "Insufficient valid precommits for supermajority"),
            Self::InvalidFinalityProof => write!(f, "Invalid finality proof"),
            Self::InvalidStateCommitmentRoot => write!(f, "Invalid state commitment root"),
            Self::InvalidReceiptHash => write!(f, "Invalid receipt hash"),
            Self::MissingReceiptInclusionProof => write!(f, "Missing receipt inclusion proof nodes"),
            Self::InvalidIntentHash => write!(f, "Invalid intent hash"),
            Self::InvalidIntentLockResources => write!(f, "Invalid intent lock resources"),
            Self::InvalidSlashOffender => write!(f, "Invalid slash offender"),
            Self::InvalidSlashAmount => write!(f, "Invalid slash amount"),
            Self::ProofTypePayloadMismatch => write!(f, "Proof type and payload mismatch"),
            Self::ZkProofNotImplemented => write!(f, "ZK proof verification not yet implemented; wire a Groth16/PLONK verifier and enable the `zk-proofs` feature"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofType {
    StateCommitment,
    ReceiptInclusion,
    IntentLock,
    SlashEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofPayload {
    StateCommitment(Hash),
    ReceiptInclusion {
        receipt_hash: Hash,
        merkle_proof: Vec<Hash>,
    },
    IntentLock {
        intent_hash: Hash,
        resources: Hash,
    },
    SlashEvent {
        offender: [u8; 32],
        amount: u128,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinalityProof {
    HotStuffQC {
        validator_set_hash: Hash,
        signatures: Vec<Vec<u8>>,
    },
    TendermintCommit {
        precommits: Vec<Vec<u8>>,
    },
    ZKProof {
        proof_data: Vec<u8>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainProof {
    pub source_chain: ChainId,
    pub block_hash: Hash,
    pub block_height: u64,
    pub proof_type: ProofType,
    pub payload: ProofPayload,
    pub finality_proof: FinalityProof,
}

/// Validator information for finality verification
#[derive(Clone, Debug)]
pub struct ValidatorInfo {
    pub account_id: Vec<u8>,
    pub grandpa_key: Ed25519Public,
}

pub struct ProofVerifier;

impl ProofVerifier {
    /// Verify cross-chain proof with finality verification
    ///
    /// # Arguments
    /// * `proof` - The cross-chain proof to verify
    /// * `validators` - Current validator set for signature verification
    pub fn verify(
        proof: &CrossChainProof,
        validators: &[ValidatorInfo],
    ) -> Result<bool, ProofVerificationError> {
        let is_final = match &proof.finality_proof {
            FinalityProof::HotStuffQC {
                validator_set_hash,
                signatures,
            } => Self::verify_hotstuff_qc(validator_set_hash, signatures, proof, validators)?,
            FinalityProof::TendermintCommit { precommits } => {
                Self::verify_tendermint_commit(precommits, proof, validators)?
            }
            FinalityProof::ZKProof { proof_data } => Self::verify_zk_proof(proof_data, proof)?,
        };

        if !is_final {
            return Err(ProofVerificationError::InvalidFinalityProof);
        }

        Self::verify_payload(proof)
    }

    /// Verify HotStuff QC with Ed25519 signatures
    fn verify_hotstuff_qc(
        validator_set_hash: &Hash,
        signatures: &[Vec<u8>],
        proof: &CrossChainProof,
        validators: &[ValidatorInfo],
    ) -> Result<bool, ProofVerificationError> {
        if validators.is_empty() {
            return Err(ProofVerificationError::EmptyValidatorSet);
        }

        let current_hash = Self::compute_validator_set_hash(validators);
        if validator_set_hash != &current_hash {
            return Err(ProofVerificationError::ValidatorSetHashMismatch);
        }

        let message_hash = Self::compute_finality_message_hash(proof);

        let mut valid_count = 0;
        let mut seen_validators = HashSet::new();

        for sig_bytes in signatures {
            if sig_bytes.len() != 68 {
                return Err(ProofVerificationError::InvalidSignatureLength(
                    sig_bytes.len() as u32,
                ));
            }

            let validator_index =
                u32::from_le_bytes([sig_bytes[0], sig_bytes[1], sig_bytes[2], sig_bytes[3]])
                    as usize;

            if !seen_validators.insert(validator_index) {
                return Err(ProofVerificationError::DuplicateValidatorSignature);
            }

            if validator_index >= validators.len() {
                return Err(ProofVerificationError::ValidatorIndexOutOfBounds(
                    validator_index,
                ));
            }
            let validator = &validators[validator_index];

            let sig_slice = &sig_bytes[4..68];
            let signature = Ed25519Signature::from_slice(sig_slice)
                .map_err(|_| ProofVerificationError::InvalidEd25519Signature)?;

            if sp_core::ed25519::Pair::verify(&signature, message_hash, &validator.grandpa_key) {
                valid_count += 1;
            }
        }

        let threshold = (validators.len() * 2 / 3) + 1;
        if valid_count < threshold {
            return Err(ProofVerificationError::InsufficientSupermajoritySignatures);
        }

        Ok(true)
    }

    /// Verify Tendermint precommits (similar to HotStuff QC)
    fn verify_tendermint_commit(
        precommits: &[Vec<u8>],
        proof: &CrossChainProof,
        validators: &[ValidatorInfo],
    ) -> Result<bool, ProofVerificationError> {
        if validators.is_empty() {
            return Err(ProofVerificationError::EmptyValidatorSet);
        }

        let message_hash = Self::compute_finality_message_hash(proof);

        let mut valid_count = 0;
        let mut seen_validators = HashSet::new();

        for precommit_bytes in precommits {
            if precommit_bytes.len() != 68 {
                return Err(ProofVerificationError::InvalidPrecommitLength);
            }

            let validator_index = u32::from_le_bytes([
                precommit_bytes[0],
                precommit_bytes[1],
                precommit_bytes[2],
                precommit_bytes[3],
            ]) as usize;

            if !seen_validators.insert(validator_index) {
                return Err(ProofVerificationError::DuplicatePrecommit);
            }

            if validator_index >= validators.len() {
                return Err(ProofVerificationError::ValidatorIndexOutOfBounds(
                    validator_index,
                ));
            }
            let validator = &validators[validator_index];

            let sig_slice = &precommit_bytes[4..68];
            let signature = Ed25519Signature::from_slice(sig_slice)
                .map_err(|_| ProofVerificationError::InvalidEd25519Signature)?;

            if sp_core::ed25519::Pair::verify(&signature, message_hash, &validator.grandpa_key) {
                valid_count += 1;
            }
        }

        let threshold = (validators.len() * 2 / 3) + 1;
        if valid_count < threshold {
            return Err(ProofVerificationError::InsufficientPrecommits);
        }

        Ok(true)
    }

    /// Verify ZK proof (feature-gated — requires a ZK verifier library).
    ///
    /// Without the `zk-proofs` feature, this returns an explicit error so
    /// production builds cannot accidentally route ZK proofs through an
    /// unverified path. With the feature enabled, a Groth16 / PLONK verifier
    /// is expected to be wired here by the consuming runtime.
    fn verify_zk_proof(
        _proof_data: &[u8],
        _proof: &CrossChainProof,
    ) -> Result<bool, ProofVerificationError> {
        Err(ProofVerificationError::ZkProofNotImplemented)
    }

    /// Compute validator set hash for verification
    fn compute_validator_set_hash(validators: &[ValidatorInfo]) -> Hash {
        use sp_core::hashing::blake2_256;

        // Concatenate all validator grandpa keys and hash
        let mut data = Vec::new();
        for validator in validators {
            data.extend_from_slice(validator.grandpa_key.as_ref());
        }
        blake2_256(&data)
    }

    /// Compute the finality message hash that validators sign
    fn compute_finality_message_hash(proof: &CrossChainProof) -> [u8; 32] {
        use sp_core::hashing::blake2_256;

        // Create canonical message for signing:
        // [source_chain: 8 bytes][block_hash: 32 bytes][block_height: 8 bytes]
        let mut message = Vec::new();
        message.extend_from_slice(&proof.source_chain.to_le_bytes());
        message.extend_from_slice(&proof.block_hash);
        message.extend_from_slice(&proof.block_height.to_le_bytes());

        blake2_256(&message)
    }

    fn verify_payload(proof: &CrossChainProof) -> Result<bool, ProofVerificationError> {
        match (&proof.proof_type, &proof.payload) {
            (ProofType::StateCommitment, ProofPayload::StateCommitment(root)) => {
                if *root == [0u8; 32] {
                    return Err(ProofVerificationError::InvalidStateCommitmentRoot);
                }
                Ok(true)
            }
            (
                ProofType::ReceiptInclusion,
                ProofPayload::ReceiptInclusion {
                    receipt_hash,
                    merkle_proof,
                },
            ) => {
                if *receipt_hash == [0u8; 32] {
                    return Err(ProofVerificationError::InvalidReceiptHash);
                }
                if merkle_proof.is_empty() {
                    return Err(ProofVerificationError::MissingReceiptInclusionProof);
                }
                Ok(true)
            }
            (
                ProofType::IntentLock,
                ProofPayload::IntentLock {
                    intent_hash,
                    resources,
                },
            ) => {
                if *intent_hash == [0u8; 32] {
                    return Err(ProofVerificationError::InvalidIntentHash);
                }
                if *resources == [0u8; 32] {
                    return Err(ProofVerificationError::InvalidIntentLockResources);
                }
                Ok(true)
            }
            (ProofType::SlashEvent, ProofPayload::SlashEvent { offender, amount }) => {
                if *offender == [0u8; 32] {
                    return Err(ProofVerificationError::InvalidSlashOffender);
                }
                if *amount == 0 {
                    return Err(ProofVerificationError::InvalidSlashAmount);
                }
                Ok(true)
            }
            _ => Err(ProofVerificationError::ProofTypePayloadMismatch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::{ed25519::Pair, Pair as PairTrait};

    /// Helper to create test validators
    fn create_test_validators(count: usize) -> (Vec<ValidatorInfo>, Vec<Pair>) {
        let mut validators = Vec::new();
        let mut keypairs = Vec::new();

        for i in 0..count {
            let seed = format!("//Validator{}", i);
            let keypair = Pair::from_string(&seed, None).unwrap();
            validators.push(ValidatorInfo {
                account_id: vec![i as u8; 32],
                grandpa_key: keypair.public(),
            });
            keypairs.push(keypair);
        }

        (validators, keypairs)
    }

    /// Helper to create test proof
    fn create_test_proof(finality_proof: FinalityProof) -> CrossChainProof {
        CrossChainProof {
            source_chain: 1,
            block_hash: [42u8; 32],
            block_height: 100,
            proof_type: ProofType::StateCommitment,
            payload: ProofPayload::StateCommitment([1u8; 32]),
            finality_proof,
        }
    }

    /// Helper to sign message and create signature bytes
    fn create_signature_bytes(keypair: &Pair, validator_index: u32, message: &[u8; 32]) -> Vec<u8> {
        let signature = keypair.sign(message);
        let mut sig_bytes = Vec::new();
        sig_bytes.extend_from_slice(&validator_index.to_le_bytes());
        sig_bytes.extend_from_slice(signature.as_ref());
        sig_bytes
    }

    #[test]
    fn test_valid_hotstuff_qc_with_supermajority() {
        // Create 7 validators (need 5 for supermajority: 7*2/3+1 = 5)
        let (validators, keypairs) = create_test_validators(7);
        let validator_set_hash = ProofVerifier::compute_validator_set_hash(&validators);

        // Create proof
        let proof = create_test_proof(FinalityProof::HotStuffQC {
            validator_set_hash,
            signatures: vec![],
        });

        // Compute message hash
        let message_hash = ProofVerifier::compute_finality_message_hash(&proof);

        // Create 5 valid signatures (exactly at threshold)
        let signatures: Vec<_> = keypairs[..5]
            .iter()
            .enumerate()
            .map(|(i, keypair)| create_signature_bytes(keypair, i as u32, &message_hash))
            .collect();

        let proof_with_sigs = CrossChainProof {
            finality_proof: FinalityProof::HotStuffQC {
                validator_set_hash,
                signatures,
            },
            ..proof
        };

        // Should accept with exactly supermajority threshold
        let result = ProofVerifier::verify(&proof_with_sigs, &validators);
        assert!(result.is_ok(), "Expected valid proof to pass");
    }

    #[test]
    fn test_insufficient_signatures_rejected() {
        // Create 7 validators (need 5 for supermajority)
        let (validators, keypairs) = create_test_validators(7);
        let validator_set_hash = ProofVerifier::compute_validator_set_hash(&validators);

        let proof = create_test_proof(FinalityProof::HotStuffQC {
            validator_set_hash,
            signatures: vec![],
        });

        let message_hash = ProofVerifier::compute_finality_message_hash(&proof);

        // Create only 4 signatures (below threshold of 5)
        let signatures: Vec<_> = keypairs[..4]
            .iter()
            .enumerate()
            .map(|(i, keypair)| create_signature_bytes(keypair, i as u32, &message_hash))
            .collect();

        let proof_with_sigs = CrossChainProof {
            finality_proof: FinalityProof::HotStuffQC {
                validator_set_hash,
                signatures,
            },
            ..proof
        };

        // Should reject insufficient signatures
        let result = ProofVerifier::verify(&proof_with_sigs, &validators);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ProofVerificationError::InsufficientSupermajoritySignatures
        );
    }

    #[test]
    fn test_duplicate_validator_signatures_rejected() {
        let (validators, keypairs) = create_test_validators(7);
        let validator_set_hash = ProofVerifier::compute_validator_set_hash(&validators);

        let proof = create_test_proof(FinalityProof::HotStuffQC {
            validator_set_hash,
            signatures: vec![],
        });

        let message_hash = ProofVerifier::compute_finality_message_hash(&proof);

        // Create signatures with validator 0 signing twice
        let signatures = vec![
            create_signature_bytes(&keypairs[0], 0, &message_hash),
            create_signature_bytes(&keypairs[1], 1, &message_hash),
            create_signature_bytes(&keypairs[0], 0, &message_hash), // Duplicate!
            create_signature_bytes(&keypairs[2], 2, &message_hash),
            create_signature_bytes(&keypairs[3], 3, &message_hash),
        ];

        let proof_with_sigs = CrossChainProof {
            finality_proof: FinalityProof::HotStuffQC {
                validator_set_hash,
                signatures,
            },
            ..proof
        };

        // Should reject duplicate validator
        let result = ProofVerifier::verify(&proof_with_sigs, &validators);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ProofVerificationError::DuplicateValidatorSignature
        );
    }

    #[test]
    fn test_invalid_validator_set_hash_rejected() {
        let (validators, keypairs) = create_test_validators(7);
        let _correct_hash = ProofVerifier::compute_validator_set_hash(&validators);
        let wrong_hash = [99u8; 32]; // Wrong hash

        let proof = create_test_proof(FinalityProof::HotStuffQC {
            validator_set_hash: wrong_hash,
            signatures: vec![],
        });

        let message_hash = ProofVerifier::compute_finality_message_hash(&proof);

        // Create valid signatures
        let signatures: Vec<_> = keypairs[..5]
            .iter()
            .enumerate()
            .map(|(i, keypair)| create_signature_bytes(keypair, i as u32, &message_hash))
            .collect();

        let proof_with_sigs = CrossChainProof {
            finality_proof: FinalityProof::HotStuffQC {
                validator_set_hash: wrong_hash,
                signatures,
            },
            ..proof
        };

        // Should reject mismatched validator set hash
        let result = ProofVerifier::verify(&proof_with_sigs, &validators);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ProofVerificationError::ValidatorSetHashMismatch
        );
    }

    #[test]
    fn test_invalid_ed25519_signature_rejected() {
        let (validators, keypairs) = create_test_validators(7);
        let validator_set_hash = ProofVerifier::compute_validator_set_hash(&validators);

        let proof = create_test_proof(FinalityProof::HotStuffQC {
            validator_set_hash,
            signatures: vec![],
        });

        let message_hash = ProofVerifier::compute_finality_message_hash(&proof);

        // Create signatures, but tamper with one
        let mut signatures = Vec::new();
        signatures.push(create_signature_bytes(&keypairs[0], 0, &message_hash));
        signatures.push(create_signature_bytes(&keypairs[1], 1, &message_hash));

        // Create invalid signature (wrong message)
        let wrong_message = [255u8; 32];
        signatures.push(create_signature_bytes(&keypairs[2], 2, &wrong_message));

        signatures.push(create_signature_bytes(&keypairs[3], 3, &message_hash));
        signatures.push(create_signature_bytes(&keypairs[4], 4, &message_hash));

        let proof_with_sigs = CrossChainProof {
            finality_proof: FinalityProof::HotStuffQC {
                validator_set_hash,
                signatures,
            },
            ..proof
        };

        // Should reject: only 4 valid signatures (need 5)
        let result = ProofVerifier::verify(&proof_with_sigs, &validators);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ProofVerificationError::InsufficientSupermajoritySignatures
        );
    }

    #[test]
    fn test_validator_index_out_of_bounds_rejected() {
        let (validators, keypairs) = create_test_validators(7);
        let validator_set_hash = ProofVerifier::compute_validator_set_hash(&validators);

        let proof = create_test_proof(FinalityProof::HotStuffQC {
            validator_set_hash,
            signatures: vec![],
        });

        let message_hash = ProofVerifier::compute_finality_message_hash(&proof);

        // Create signatures with out-of-bounds index
        let signatures = vec![
            create_signature_bytes(&keypairs[0], 0, &message_hash),
            create_signature_bytes(&keypairs[1], 1, &message_hash),
            create_signature_bytes(&keypairs[2], 99, &message_hash), // Out of bounds!
            create_signature_bytes(&keypairs[3], 3, &message_hash),
            create_signature_bytes(&keypairs[4], 4, &message_hash),
        ];

        let proof_with_sigs = CrossChainProof {
            finality_proof: FinalityProof::HotStuffQC {
                validator_set_hash,
                signatures,
            },
            ..proof
        };

        // Should reject out-of-bounds index
        let result = ProofVerifier::verify(&proof_with_sigs, &validators);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ProofVerificationError::ValidatorIndexOutOfBounds(99)
        );
    }

    #[test]
    fn test_empty_validator_set_rejected() {
        let validators = vec![]; // Empty set
        let validator_set_hash = [0u8; 32];

        let proof = create_test_proof(FinalityProof::HotStuffQC {
            validator_set_hash,
            signatures: vec![],
        });

        // Should reject empty validator set
        let result = ProofVerifier::verify(&proof, &validators);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ProofVerificationError::EmptyValidatorSet
        );
    }

    #[test]
    fn test_invalid_signature_length_rejected() {
        let (validators, _keypairs) = create_test_validators(7);
        let validator_set_hash = ProofVerifier::compute_validator_set_hash(&validators);

        // Create signature with wrong length
        let mut invalid_sig = Vec::new();
        invalid_sig.extend_from_slice(&0u32.to_le_bytes());
        invalid_sig.extend_from_slice(&[0u8; 32]); // Only 32 bytes instead of 64

        let proof = create_test_proof(FinalityProof::HotStuffQC {
            validator_set_hash,
            signatures: vec![invalid_sig],
        });

        // Should reject invalid length
        let result = ProofVerifier::verify(&proof, &validators);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ProofVerificationError::InvalidSignatureLength(36)
        );
    }

    #[test]
    fn test_tendermint_commit_verification() {
        // Create 7 validators
        let (validators, keypairs) = create_test_validators(7);

        let proof = create_test_proof(FinalityProof::TendermintCommit { precommits: vec![] });

        let message_hash = ProofVerifier::compute_finality_message_hash(&proof);

        // Create 5 valid precommits
        let precommits: Vec<_> = keypairs[..5]
            .iter()
            .enumerate()
            .map(|(i, keypair)| create_signature_bytes(keypair, i as u32, &message_hash))
            .collect();

        let proof_with_commits = CrossChainProof {
            finality_proof: FinalityProof::TendermintCommit { precommits },
            ..proof
        };

        // Should accept valid Tendermint commit
        let result = ProofVerifier::verify(&proof_with_commits, &validators);
        assert!(result.is_ok(), "Expected valid Tendermint commit to pass");
    }

    #[test]
    fn test_zk_proof_not_implemented() {
        let (validators, _keypairs) = create_test_validators(7);

        let proof = create_test_proof(FinalityProof::ZKProof {
            proof_data: vec![1, 2, 3, 4],
        });

        let result = ProofVerifier::verify(&proof, &validators);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ProofVerificationError::ZkProofNotImplemented
        );
    }

    #[test]
    fn test_exactly_at_supermajority_threshold() {
        // Test edge case: exactly 2/3 + 1 threshold
        // With 10 validators: threshold = (10 * 2 / 3) + 1 = 6 + 1 = 7
        let (validators, keypairs) = create_test_validators(10);
        let validator_set_hash = ProofVerifier::compute_validator_set_hash(&validators);

        let proof = create_test_proof(FinalityProof::HotStuffQC {
            validator_set_hash,
            signatures: vec![],
        });

        let message_hash = ProofVerifier::compute_finality_message_hash(&proof);

        // Create exactly 7 signatures (at threshold)
        let signatures: Vec<_> = keypairs[..7]
            .iter()
            .enumerate()
            .map(|(i, keypair)| create_signature_bytes(keypair, i as u32, &message_hash))
            .collect();

        let proof_with_sigs = CrossChainProof {
            finality_proof: FinalityProof::HotStuffQC {
                validator_set_hash,
                signatures,
            },
            ..proof
        };

        // Should accept exactly at threshold
        let result = ProofVerifier::verify(&proof_with_sigs, &validators);
        assert!(result.is_ok(), "Expected proof at exact threshold to pass");
    }

    #[test]
    fn test_one_below_threshold_rejected() {
        // With 10 validators: threshold = 7, test with 6 signatures
        let (validators, keypairs) = create_test_validators(10);
        let validator_set_hash = ProofVerifier::compute_validator_set_hash(&validators);

        let proof = create_test_proof(FinalityProof::HotStuffQC {
            validator_set_hash,
            signatures: vec![],
        });

        let message_hash = ProofVerifier::compute_finality_message_hash(&proof);

        // Create 6 signatures (one below threshold of 7)
        let signatures: Vec<_> = keypairs[..6]
            .iter()
            .enumerate()
            .map(|(i, keypair)| create_signature_bytes(keypair, i as u32, &message_hash))
            .collect();

        let proof_with_sigs = CrossChainProof {
            finality_proof: FinalityProof::HotStuffQC {
                validator_set_hash,
                signatures,
            },
            ..proof
        };

        // Should reject one below threshold
        let result = ProofVerifier::verify(&proof_with_sigs, &validators);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ProofVerificationError::InsufficientSupermajoritySignatures
        );
    }
}
