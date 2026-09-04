//! # Proof Aggregator
//!
//! Manages collection, validation, and Byzantine consensus finalization of unified proofs.
//! Tracks which validators have attested to each proof and determines finality.

use crate::error::{SwarmError, SwarmResult};
use crate::unified_proof::UnifiedProof;
use std::collections::BTreeMap;
use tracing::{debug, info};

pub type Address = [u8; 32];
pub type Hash = [u8; 32];

/// Aggregation state for a proof
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregationState {
    /// Initial state: collecting attestations
    Collecting,
    /// Threshold reached: consensus achieved
    Finalized,
    /// Supermajority reached: Byzantine finality
    ByzantineFinalized,
    /// Failed to achieve consensus
    Failed,
}

/// Proof aggregation entry tracking consensus progress
#[derive(Debug, Clone)]
pub struct AggregationEntry {
    pub proof: UnifiedProof,
    pub state: AggregationState,
    pub first_seen: u64,
    pub last_updated: u64,
}

/// Proof aggregator for Byzantine finality
pub struct ProofAggregator {
    /// Map of proof_hash -> aggregation entry
    proofs: BTreeMap<Hash, AggregationEntry>,
    /// Validator identity -> ed25519 public key bytes, padded to the
    /// SignatureOutput verifier's 33-byte input shape.
    validator_pubkeys: BTreeMap<Address, [u8; 33]>,
    /// Total validators in system
    total_validators: u32,
    /// Byzantine finality threshold (2/3 + 1)
    finality_threshold: u32,
    /// Supermajority threshold (3/5)
    supermajority_threshold: u32,
}

impl ProofAggregator {
    /// Create a new proof aggregator
    pub fn new(total_validators: u32) -> Self {
        let finality_threshold = (total_validators * 2 / 3) + 1;
        // Supermajority: 3/4 consensus for enhanced finality
        let supermajority_threshold = (total_validators * 3 / 4) + 1;

        Self {
            proofs: BTreeMap::new(),
            validator_pubkeys: BTreeMap::new(),
            total_validators,
            finality_threshold,
            supermajority_threshold,
        }
    }

    /// Register the public key used to verify a validator's GPU attestations.
    pub fn register_validator_pubkey(&mut self, validator_id: Address, pubkey: [u8; 32]) {
        let mut padded = [0u8; 33];
        padded[..32].copy_from_slice(&pubkey);
        self.validator_pubkeys.insert(validator_id, padded);
    }

    fn attestation_signing_message(
        receipt: &crate::gpu_receipt::GpuReceipt,
        bundle_id: Hash,
        finalized_block: u64,
        legs_hash: Hash,
    ) -> Vec<u8> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(b"x3-validator-attestation-v1");
        hasher.update(&receipt.kernel_hash);
        hasher.update(&receipt.input_commitment);
        hasher.update(&receipt.output_commitment);
        hasher.update(&receipt.executor);
        hasher.update(&receipt.gpu_cycles_used.to_le_bytes());
        hasher.update(&bundle_id);
        hasher.update(&finalized_block.to_le_bytes());
        hasher.update(&legs_hash);
        hasher.finalize().to_vec()
    }

    fn signature_from_bytes(signature: &[u8]) -> SwarmResult<crate::crypto::SignatureOutput> {
        if signature.len() != 65 {
            return Err(SwarmError::VerificationFailed(format!(
                "Invalid signature length: expected 65, got {}",
                signature.len()
            )));
        }

        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&signature[..32]);
        s.copy_from_slice(&signature[32..64]);
        Ok(crate::crypto::SignatureOutput::new(r, s, signature[64]))
    }

    fn verify_attestations(&self, proof: &UnifiedProof) -> SwarmResult<()> {
        for attestation in &proof.gpu_attestations {
            let pubkey = self
                .validator_pubkeys
                .get(&attestation.validator_id)
                .ok_or_else(|| {
                    SwarmError::VerificationFailed(format!(
                        "Missing public key for validator {:?}",
                        attestation.validator_id
                    ))
                })?;
            let signature = Self::signature_from_bytes(&attestation.signature)?;
            let msg = Self::attestation_signing_message(
                &attestation.receipt,
                proof.header.bundle_id,
                proof.header.finalized_block,
                proof.header.legs_hash,
            );

            if !signature.verify(&msg, pubkey) {
                return Err(SwarmError::VerificationFailed(format!(
                    "Invalid attestation signature for validator {:?}",
                    attestation.validator_id
                )));
            }
        }

        Ok(())
    }

    /// Submit a unified proof for aggregation
    pub fn submit_proof(&mut self, proof: UnifiedProof) -> SwarmResult<()> {
        let proof_hash = proof.proof_hash();

        // Validate proof structure
        let validation = proof.validate();
        if !validation.is_valid {
            return Err(SwarmError::VerificationFailed(format!(
                "Proof validation failed: {:?}",
                validation.errors
            )));
        }
        self.verify_attestations(&proof)?;

        if self.proofs.contains_key(&proof_hash) {
            return Err(SwarmError::DuplicateProof);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = AggregationEntry {
            proof,
            state: AggregationState::Collecting,
            first_seen: now,
            last_updated: now,
        };

        self.proofs.insert(proof_hash, entry);
        debug!("Proof submitted for aggregation");

        Ok(())
    }

    /// Update proof aggregation with new validator attestation
    pub fn add_attestation(
        &mut self,
        proof_hash: Hash,
        validator_id: Address,
        validator_signature: Vec<u8>,
        state_commitment: Hash,
    ) -> SwarmResult<()> {
        let entry = self
            .proofs
            .get_mut(&proof_hash)
            .ok_or(SwarmError::ProofNotFound)?;

        // Add vote to consensus
        let vote_added =
            entry
                .proof
                .add_validator_vote(validator_id, validator_signature, state_commitment);
        if !vote_added {
            return Err(SwarmError::DuplicateAttestation);
        }

        entry.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check if finality achieved
        let consensus_count = entry.proof.consensus.consensus_count;
        if consensus_count >= self.finality_threshold && entry.state == AggregationState::Collecting
        {
            entry.state = AggregationState::Finalized;
            info!(
                "Proof reached Byzantine finality ({}/{})",
                consensus_count, self.total_validators
            );
        }

        // Check if supermajority achieved
        if consensus_count >= self.supermajority_threshold
            && entry.state == AggregationState::Finalized
        {
            entry.state = AggregationState::ByzantineFinalized;
            info!(
                "Proof reached supermajority finality ({}/{})",
                consensus_count, self.total_validators
            );
        }

        Ok(())
    }

    /// Get proof by hash
    pub fn get_proof(&self, proof_hash: Hash) -> SwarmResult<UnifiedProof> {
        self.proofs
            .get(&proof_hash)
            .map(|entry| entry.proof.clone())
            .ok_or(SwarmError::ProofNotFound)
    }

    /// Return the most recently inserted proof, or `None` if the
    /// aggregator is empty. Intended for tests and read-only tooling
    /// that need to inspect the latest attestation without knowing its
    /// hash. Insertion order is preserved by `BTreeMap` only when keys
    /// are equal; for "latest" semantics we use the entry with the
    /// largest `proof_hash` so the result is stable.
    pub fn latest_proof(&self) -> Option<UnifiedProof> {
        self.proofs.values().next_back().map(|e| e.proof.clone())
    }

    /// Number of proofs currently held in the aggregator.
    pub fn proof_count(&self) -> usize {
        self.proofs.len()
    }

    /// Get aggregation state for proof
    pub fn get_aggregation_state(
        &self,
        proof_hash: Hash,
    ) -> SwarmResult<(AggregationState, u32, u64)> {
        self.proofs
            .get(&proof_hash)
            .map(|entry| {
                (
                    entry.state.clone(),
                    entry.proof.consensus.consensus_count,
                    entry.last_updated,
                )
            })
            .ok_or(SwarmError::ProofNotFound)
    }

    /// Check if proof is finalized
    pub fn is_finalized(&self, proof_hash: Hash) -> bool {
        self.proofs
            .get(&proof_hash)
            .map(|entry| {
                matches!(
                    entry.state,
                    AggregationState::Finalized | AggregationState::ByzantineFinalized
                )
            })
            .unwrap_or(false)
    }

    /// Check if proof is Byzantine finalized
    pub fn is_byzantine_finalized(&self, proof_hash: Hash) -> bool {
        self.proofs
            .get(&proof_hash)
            .map(|entry| entry.state == AggregationState::ByzantineFinalized)
            .unwrap_or(false)
    }

    /// Get divergent validators for a proof
    pub fn get_divergent_validators(&self, proof_hash: Hash) -> SwarmResult<Vec<Address>> {
        self.proofs
            .get(&proof_hash)
            .map(|entry| entry.proof.get_divergent_validators())
            .ok_or(SwarmError::ProofNotFound)
    }

    /// Cleanup old proofs (older than max_age_secs)
    pub fn cleanup_old_proofs(&mut self, max_age_secs: u64) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let before_count = self.proofs.len();
        self.proofs.retain(|_hash, entry| {
            let is_old = (now - entry.first_seen) >= max_age_secs;
            if is_old {
                debug!("Cleaning up old proof");
            }
            !is_old
        });

        before_count - self.proofs.len()
    }

    /// Get statistics about aggregation state
    pub fn get_stats(&self) -> AggregatorStats {
        let mut collecting = 0;
        let mut finalized = 0;
        let mut byzantine_finalized = 0;
        let mut failed = 0;
        let mut total_consensus_count = 0u64;

        for entry in self.proofs.values() {
            match entry.state {
                AggregationState::Collecting => collecting += 1,
                AggregationState::Finalized => finalized += 1,
                AggregationState::ByzantineFinalized => byzantine_finalized += 1,
                AggregationState::Failed => failed += 1,
            }
            total_consensus_count += entry.proof.consensus.consensus_count as u64;
        }

        AggregatorStats {
            total_proofs: self.proofs.len(),
            collecting,
            finalized,
            byzantine_finalized,
            failed,
            total_validators: self.total_validators,
            finality_threshold: self.finality_threshold,
            avg_consensus_count: if self.proofs.is_empty() {
                0
            } else {
                (total_consensus_count / self.proofs.len() as u64) as u32
            },
        }
    }
}

/// Statistics about proof aggregation
#[derive(Debug, Clone)]
pub struct AggregatorStats {
    pub total_proofs: usize,
    pub collecting: usize,
    pub finalized: usize,
    pub byzantine_finalized: usize,
    pub failed: usize,
    pub total_validators: u32,
    pub finality_threshold: u32,
    pub avg_consensus_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::SigningKey;
    use crate::gpu_receipt::{GpuClass, GpuReceipt, ProofType};
    use crate::unified_proof::GpuValidatorAttestation;
    use crate::unified_proof::{AtomicVmProof, ProofHeader};

    fn create_test_proof(bundle_id: Hash, block: u64) -> UnifiedProof {
        let header = ProofHeader::new(bundle_id, block, [1u8; 32]);
        let atomic_proof = AtomicVmProof {
            receipt_root: [2u8; 32],
            finality_cert: [3u8; 32],
            leg_count: 1,
            finality_cert_data: vec![1, 2, 3],
        };
        UnifiedProof::new(header, atomic_proof, 10).unwrap()
    }

    fn create_signed_test_proof(
        key: &SigningKey,
        validator_id: Address,
        bundle_id: Hash,
        block: u64,
    ) -> UnifiedProof {
        let mut proof = create_test_proof(bundle_id, block);
        let receipt = GpuReceipt {
            kernel_hash: [9u8; 32],
            input_commitment: [8u8; 32],
            output_commitment: [7u8; 32],
            gpu_cycles_used: 42,
            device_class: GpuClass::DataCenter,
            executor: validator_id,
            proof_type: ProofType::RecomputeA,
        };
        let msg = ProofAggregator::attestation_signing_message(
            &receipt,
            proof.header.bundle_id,
            proof.header.finalized_block,
            proof.header.legs_hash,
        );
        let attestation = GpuValidatorAttestation::new(
            validator_id,
            receipt,
            key.sign(&msg).to_bytes(),
            0,
            ProofType::RecomputeA,
            1,
        );
        proof.add_attestation(attestation).unwrap();
        proof
    }

    #[test]
    fn test_aggregator_creation() {
        let aggregator = ProofAggregator::new(10);
        assert_eq!(aggregator.total_validators, 10);
        assert_eq!(aggregator.finality_threshold, 7); // 2/3 + 1
    }

    #[test]
    fn test_submit_proof() {
        let mut aggregator = ProofAggregator::new(10);
        let proof = create_test_proof([1u8; 32], 100);
        let proof_hash = proof.proof_hash();

        let result = aggregator.submit_proof(proof);
        assert!(result.is_ok());

        // Should not allow duplicate
        let proof2 = create_test_proof([1u8; 32], 100);
        let result2 = aggregator.submit_proof(proof2);
        assert!(result2.is_err());

        // Should be in collecting state
        assert!(aggregator.proofs.contains_key(&proof_hash));
    }

    #[test]
    fn test_submit_proof_requires_registered_attestation_pubkey() {
        let key = SigningKey::from_seed(b"registered attestation pubkey test seed 32b")
            .expect("test seed must be accepted");
        let validator_id = [11u8; 32];
        let mut aggregator = ProofAggregator::new(10);
        let proof = create_signed_test_proof(&key, validator_id, [12u8; 32], 100);

        let result = aggregator.submit_proof(proof);

        assert!(result.is_err());
    }

    #[test]
    fn test_submit_proof_rejects_invalid_attestation_signature() {
        let key = SigningKey::from_seed(b"valid signing key for rejection test 32b")
            .expect("test seed must be accepted");
        let wrong_key = SigningKey::from_seed(b"wrong signing key for rejection test 32b")
            .expect("test seed must be accepted");
        let validator_id = [13u8; 32];
        let mut aggregator = ProofAggregator::new(10);
        aggregator.register_validator_pubkey(validator_id, wrong_key.public_key_bytes());
        let proof = create_signed_test_proof(&key, validator_id, [14u8; 32], 100);

        let result = aggregator.submit_proof(proof);

        assert!(result.is_err());
    }

    #[test]
    fn test_submit_proof_accepts_registered_valid_attestation_signature() {
        let key = SigningKey::from_seed(b"valid registered attestation signature 32b")
            .expect("test seed must be accepted");
        let validator_id = [15u8; 32];
        let mut aggregator = ProofAggregator::new(10);
        aggregator.register_validator_pubkey(validator_id, key.public_key_bytes());
        let proof = create_signed_test_proof(&key, validator_id, [16u8; 32], 100);

        let result = aggregator.submit_proof(proof);

        assert!(result.is_ok());
    }

    #[test]
    fn test_attestation_aggregation() {
        let mut aggregator = ProofAggregator::new(10);
        let proof = create_test_proof([1u8; 32], 100);
        let proof_hash = proof.proof_hash();
        let commitment = [5u8; 32];

        aggregator.submit_proof(proof).unwrap();

        // Add attestations until finality
        for i in 0..7 {
            let mut validator = [0; 32];
            validator[0] = i as u8;
            aggregator
                .add_attestation(proof_hash, validator, vec![i as u8], commitment)
                .unwrap();
        }

        // Should be finalized
        assert!(aggregator.is_finalized(proof_hash));

        let (state, count, _) = aggregator.get_aggregation_state(proof_hash).unwrap();
        assert_eq!(state, AggregationState::Finalized);
        assert_eq!(count, 7);
    }

    #[test]
    fn test_cleanup_old_proofs() {
        let mut aggregator = ProofAggregator::new(10);
        let proof = create_test_proof([1u8; 32], 100);

        aggregator.submit_proof(proof).unwrap();
        assert_eq!(aggregator.proofs.len(), 1);

        // Cleanup with 0 age should remove all
        let removed = aggregator.cleanup_old_proofs(0);
        assert_eq!(removed, 1);
        assert_eq!(aggregator.proofs.len(), 0);
    }

    #[test]
    fn test_e2e_proof_aggregation_workflow() {
        // End-to-end test showing full proof aggregation workflow
        let total_validators = 10;
        let mut aggregator = ProofAggregator::new(total_validators);

        // Create and submit proof
        let proof = create_test_proof([99u8; 32], 500);
        let proof_hash = proof.proof_hash();
        aggregator.submit_proof(proof).unwrap();

        // Verify initial state
        let (state, count, _) = aggregator.get_aggregation_state(proof_hash).unwrap();
        assert_eq!(state, AggregationState::Collecting);
        assert_eq!(count, 0);

        // Add 6 attestations (below finality threshold of 7 for 10-validator system)
        let commitment_a = [10u8; 32];
        for i in 0..6 {
            let mut validator = [0; 32];
            validator[0] = i;
            aggregator
                .add_attestation(proof_hash, validator, vec![i], commitment_a)
                .unwrap();
        }

        let (state, count, _) = aggregator.get_aggregation_state(proof_hash).unwrap();
        assert_eq!(state, AggregationState::Collecting); // Still collecting
        assert_eq!(count, 6);
        assert!(!aggregator.is_finalized(proof_hash));

        // Add 7th attestation to reach finality threshold
        let mut validator = [0; 32];
        validator[0] = 6;
        aggregator
            .add_attestation(proof_hash, validator, vec![6], commitment_a)
            .unwrap();

        let (state, count, _) = aggregator.get_aggregation_state(proof_hash).unwrap();
        assert_eq!(state, AggregationState::Finalized); // Finalized!
        assert_eq!(count, 7);
        assert!(aggregator.is_finalized(proof_hash));
        assert!(!aggregator.is_byzantine_finalized(proof_hash));

        // Add more attestations to reach supermajority (3/4 + 1 = 8 for 10)
        for i in 7..8 {
            let mut validator = [0; 32];
            validator[0] = i;
            aggregator
                .add_attestation(proof_hash, validator, vec![i], commitment_a)
                .unwrap();
        }

        let (state, count, _) = aggregator.get_aggregation_state(proof_hash).unwrap();
        assert_eq!(state, AggregationState::ByzantineFinalized); // Byzantine finalized!
        assert_eq!(count, 8);
        assert!(aggregator.is_byzantine_finalized(proof_hash));

        // Detect divergent validators (add attestations with different commitment)
        let commitment_b = [20u8; 32];
        for i in 8..10 {
            let mut validator = [0; 32];
            validator[0] = i;
            aggregator
                .add_attestation(proof_hash, validator, vec![i], commitment_b)
                .unwrap();
        }

        // Check stats
        let stats = aggregator.get_stats();
        assert_eq!(stats.total_proofs, 1);
        assert_eq!(stats.byzantine_finalized, 1);
        assert_eq!(stats.total_validators, 10);

        // Query divergent validators
        let divergent = aggregator.get_divergent_validators(proof_hash).unwrap();
        assert_eq!(divergent.len(), 2); // Validators 8 and 9

        // Retrieve finalized proof
        let finalized_proof = aggregator.get_proof(proof_hash).unwrap();
        assert!(finalized_proof.validate().is_valid);
    }
}
