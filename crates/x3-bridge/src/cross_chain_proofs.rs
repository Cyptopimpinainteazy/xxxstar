use serde::{Deserialize, Serialize};
use sp_core::{
    ed25519::{Public as Ed25519Public, Signature as Ed25519Signature},
    ByteArray, Pair as PairTrait,
};
use std::collections::HashSet;

pub type Hash = [u8; 32];
pub type ChainId = u64;

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
    ) -> Result<bool, &'static str> {
        // Implement Court VM on-chain client logic (analogous to IBC)
        // 1. Check finality proof based on chain ID
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
            return Err("Invalid finality proof");
        }

        // 2. Validate payload/type consistency and basic structure.
        Self::verify_payload(proof)
    }

    /// Verify HotStuff QC with Ed25519 signatures
    fn verify_hotstuff_qc(
        validator_set_hash: &Hash,
        signatures: &[Vec<u8>],
        proof: &CrossChainProof,
        validators: &[ValidatorInfo],
    ) -> Result<bool, &'static str> {
        if validators.is_empty() {
            return Err("Empty validator set");
        }

        // 1. Verify validator set hash matches
        let current_hash = Self::compute_validator_set_hash(validators);
        if validator_set_hash != &current_hash {
            return Err("Validator set hash mismatch");
        }

        // 2. Compute the finality message hash that was signed
        let message_hash = Self::compute_finality_message_hash(proof);

        // 3. Verify each signature
        let mut valid_count = 0;
        let mut seen_validators = HashSet::new();

        for sig_bytes in signatures {
            // Each signature is encoded as: [validator_index: 4 bytes][signature: 64 bytes]
            if sig_bytes.len() != 68 {
                return Err("Invalid signature length (expected 68 bytes)");
            }

            // Parse validator index (first 4 bytes, little-endian)
            let validator_index =
                u32::from_le_bytes([sig_bytes[0], sig_bytes[1], sig_bytes[2], sig_bytes[3]])
                    as usize;

            // Check for duplicate validator votes
            if !seen_validators.insert(validator_index) {
                return Err("Duplicate validator signature detected");
            }

            // Get validator from set
            if validator_index >= validators.len() {
                return Err("Validator index out of bounds");
            }
            let validator = &validators[validator_index];

            // Parse Ed25519 signature (next 64 bytes)
            let sig_slice = &sig_bytes[4..68];
            let signature = Ed25519Signature::from_slice(sig_slice)
                .map_err(|_| "Invalid Ed25519 signature format")?;

            // Verify Ed25519 signature against validator's public key
            if sp_core::ed25519::Pair::verify(&signature, &message_hash, &validator.grandpa_key) {
                valid_count += 1;
            }
        }

        // 4. Check supermajority threshold: (total * 2 / 3) + 1
        let threshold = (validators.len() * 2 / 3) + 1;
        if valid_count < threshold {
            return Err("Insufficient valid signatures for supermajority");
        }

        Ok(true)
    }

    /// Verify Tendermint precommits (similar to HotStuff QC)
    fn verify_tendermint_commit(
        precommits: &[Vec<u8>],
        proof: &CrossChainProof,
        validators: &[ValidatorInfo],
    ) -> Result<bool, &'static str> {
        if validators.is_empty() {
            return Err("Empty validator set");
        }

        // Compute message hash
        let message_hash = Self::compute_finality_message_hash(proof);

        // Verify precommit signatures
        let mut valid_count = 0;
        let mut seen_validators = HashSet::new();

        for precommit_bytes in precommits {
            if precommit_bytes.len() != 68 {
                return Err("Invalid precommit length");
            }

            let validator_index = u32::from_le_bytes([
                precommit_bytes[0],
                precommit_bytes[1],
                precommit_bytes[2],
                precommit_bytes[3],
            ]) as usize;

            if !seen_validators.insert(validator_index) {
                return Err("Duplicate precommit detected");
            }

            if validator_index >= validators.len() {
                return Err("Validator index out of bounds");
            }
            let validator = &validators[validator_index];

            let sig_slice = &precommit_bytes[4..68];
            let signature = Ed25519Signature::from_slice(sig_slice)
                .map_err(|_| "Invalid Ed25519 signature format")?;

            if sp_core::ed25519::Pair::verify(&signature, &message_hash, &validator.grandpa_key) {
                valid_count += 1;
            }
        }

        let threshold = (validators.len() * 2 / 3) + 1;
        if valid_count < threshold {
            return Err("Insufficient valid precommits for supermajority");
        }

        Ok(true)
    }

    /// Verify ZK proof (placeholder - requires ZK verifier library)
    fn verify_zk_proof(_proof_data: &[u8], _proof: &CrossChainProof) -> Result<bool, &'static str> {
        // TODO: Implement ZK proof verification
        // This requires integration with a ZK proof system (e.g., Groth16, PLONK)
        // For now, reject ZK proofs until verification is implemented
        Err("ZK proof verification not yet implemented")
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

    fn verify_payload(proof: &CrossChainProof) -> Result<bool, &'static str> {
        match (&proof.proof_type, &proof.payload) {
            (ProofType::StateCommitment, ProofPayload::StateCommitment(root)) => {
                if *root == [0u8; 32] {
                    return Err("Invalid state commitment root");
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
                    return Err("Invalid receipt hash");
                }
                if merkle_proof.is_empty() {
                    return Err("Missing receipt inclusion proof nodes");
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
                    return Err("Invalid intent hash");
                }
                if *resources == [0u8; 32] {
                    return Err("Invalid intent lock resources");
                }
                Ok(true)
            }
            (ProofType::SlashEvent, ProofPayload::SlashEvent { offender, amount }) => {
                if *offender == [0u8; 32] {
                    return Err("Invalid slash offender");
                }
                if *amount == 0 {
                    return Err("Invalid slash amount");
                }
                Ok(true)
            }
            _ => Err("Proof type and payload mismatch"),
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
        let mut signatures = Vec::new();
        for i in 0..5 {
            signatures.push(create_signature_bytes(
                &keypairs[i],
                i as u32,
                &message_hash,
            ));
        }

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
        let mut signatures = Vec::new();
        for i in 0..4 {
            signatures.push(create_signature_bytes(
                &keypairs[i],
                i as u32,
                &message_hash,
            ));
        }

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
            "Insufficient valid signatures for supermajority"
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
        let mut signatures = Vec::new();
        signatures.push(create_signature_bytes(&keypairs[0], 0, &message_hash));
        signatures.push(create_signature_bytes(&keypairs[1], 1, &message_hash));
        signatures.push(create_signature_bytes(&keypairs[0], 0, &message_hash)); // Duplicate!
        signatures.push(create_signature_bytes(&keypairs[2], 2, &message_hash));
        signatures.push(create_signature_bytes(&keypairs[3], 3, &message_hash));

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
            "Duplicate validator signature detected"
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
        let mut signatures = Vec::new();
        for i in 0..5 {
            signatures.push(create_signature_bytes(
                &keypairs[i],
                i as u32,
                &message_hash,
            ));
        }

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
        assert_eq!(result.unwrap_err(), "Validator set hash mismatch");
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
            "Insufficient valid signatures for supermajority"
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
        let mut signatures = Vec::new();
        signatures.push(create_signature_bytes(&keypairs[0], 0, &message_hash));
        signatures.push(create_signature_bytes(&keypairs[1], 1, &message_hash));
        signatures.push(create_signature_bytes(&keypairs[2], 99, &message_hash)); // Out of bounds!
        signatures.push(create_signature_bytes(&keypairs[3], 3, &message_hash));
        signatures.push(create_signature_bytes(&keypairs[4], 4, &message_hash));

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
        assert_eq!(result.unwrap_err(), "Validator index out of bounds");
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
        assert_eq!(result.unwrap_err(), "Empty validator set");
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
            "Invalid signature length (expected 68 bytes)"
        );
    }

    #[test]
    fn test_tendermint_commit_verification() {
        // Create 7 validators
        let (validators, keypairs) = create_test_validators(7);

        let proof = create_test_proof(FinalityProof::TendermintCommit { precommits: vec![] });

        let message_hash = ProofVerifier::compute_finality_message_hash(&proof);

        // Create 5 valid precommits
        let mut precommits = Vec::new();
        for i in 0..5 {
            precommits.push(create_signature_bytes(
                &keypairs[i],
                i as u32,
                &message_hash,
            ));
        }

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

        // Should reject ZK proof (not yet implemented)
        let result = ProofVerifier::verify(&proof, &validators);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "ZK proof verification not yet implemented"
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
        let mut signatures = Vec::new();
        for i in 0..7 {
            signatures.push(create_signature_bytes(
                &keypairs[i],
                i as u32,
                &message_hash,
            ));
        }

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
        let mut signatures = Vec::new();
        for i in 0..6 {
            signatures.push(create_signature_bytes(
                &keypairs[i],
                i as u32,
                &message_hash,
            ));
        }

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
            "Insufficient valid signatures for supermajority"
        );
    }
}
