//! # Unified Proof Format for Atomic VM + GPU Validator System
//!
//! Combines PoAE (Proof of Atomic Execution) from the Atomic VM with GPU validator receipts
//! into a single format for end-to-end validation and Byzantine consensus finality.

use crate::gpu_receipt::{GpuReceipt, ProofType};
use crate::state_merkle_proof::StateMerkleProof;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub type Address = [u8; 32];
pub type Hash = [u8; 32];
pub type Signature = Vec<u8>;

/// Proof header metadata for unified proofs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofHeader {
    /// Unique bundle identifier
    pub bundle_id: Hash,
    /// Block number where bundle was finalized
    pub finalized_block: u64,
    /// Timestamp of proof generation (unix seconds)
    pub timestamp: u64,
    /// Version of unified proof format
    pub format_version: u32,
    /// Hash of all atomic VM legs
    pub legs_hash: Hash,
}

impl ProofHeader {
    pub fn new(bundle_id: Hash, finalized_block: u64, legs_hash: Hash) -> Self {
        Self {
            bundle_id,
            finalized_block,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            format_version: 1,
            legs_hash,
        }
    }

    /// Compute canonical hash of header for aggregation
    pub fn header_hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.bundle_id);
        hasher.update(&self.finalized_block.to_le_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.update(&self.format_version.to_le_bytes());
        hasher.update(&self.legs_hash);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

/// Atomic VM Proof component (PoAE)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtomicVmProof {
    /// Merkle root of execution receipts from X3 Kernel
    pub receipt_root: Hash,
    /// Hash of finality certificate (GRANDPA justification or Flash cert)
    pub finality_cert: Hash,
    /// Number of legs in bundle
    pub leg_count: u32,
    /// Raw finality certificate data (for verification)
    pub finality_cert_data: Vec<u8>,
}

impl AtomicVmProof {
    /// Validate structural consistency
    pub fn validate(&self) -> bool {
        self.receipt_root != [0u8; 32]
            && self.finality_cert != [0u8; 32]
            && self.leg_count > 0
            && !self.finality_cert_data.is_empty()
    }
}

/// Single GPU validator attestation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GpuValidatorAttestation {
    /// GPU validator identity
    pub validator_id: Address,
    /// GPU receipt from validation
    pub receipt: GpuReceipt,
    /// Validator signature over receipt and header
    pub signature: Signature,
    /// GPU device index (for multi-GPU tracking)
    pub device_index: u32,
    /// Proof type used (Recompute, Redundant, SpotCheck)
    pub proof_type: ProofType,
    /// Timestamp of attestation (unix seconds)
    pub timestamp: u64,
    /// GPU execution latency in milliseconds
    pub execution_latency_ms: u64,
}

impl GpuValidatorAttestation {
    pub fn new(
        validator_id: Address,
        receipt: GpuReceipt,
        signature: Signature,
        device_index: u32,
        proof_type: ProofType,
        execution_latency_ms: u64,
    ) -> Self {
        Self {
            validator_id,
            receipt,
            signature,
            device_index,
            proof_type,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            execution_latency_ms,
        }
    }

    /// Compute attestation hash for consensus
    pub fn attestation_hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.validator_id);
        hasher.update(&self.receipt.kernel_hash);
        hasher.update(&self.receipt.output_commitment);
        hasher.update(&self.device_index.to_le_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

/// Byzantine consensus state for a unified proof
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ByzantineConsensus {
    /// Total validators in quorum
    pub total_validators: u32,
    /// Threshold for finality (typically 2/3 + 1 for Byzantine fault tolerance)
    pub finality_threshold: u32,
    /// Map of validator_id -> signature (for vote verification)
    pub validator_signatures: BTreeMap<Address, Signature>,
    /// Map of validator_id -> state_root_commitment (for divergence detection)
    pub state_commitments: BTreeMap<Address, Hash>,
    /// Number of validators who attested to consensus
    pub consensus_count: u32,
    /// Whether consensus is achieved
    pub is_finalized: bool,
    /// Timestamp of consensus achievement (unix seconds)
    pub consensus_timestamp: u64,
}

impl ByzantineConsensus {
    /// Create a new consensus tracker
    pub fn new(total_validators: u32) -> Self {
        // 2/3 + 1 threshold for Byzantine fault tolerance
        let finality_threshold = (total_validators * 2 / 3) + 1;

        Self {
            total_validators,
            finality_threshold,
            validator_signatures: BTreeMap::new(),
            state_commitments: BTreeMap::new(),
            consensus_count: 0,
            is_finalized: false,
            consensus_timestamp: 0,
        }
    }

    /// Add a validator vote
    pub fn add_vote(
        &mut self,
        validator_id: Address,
        signature: Signature,
        state_commitment: Hash,
    ) -> bool {
        if self.validator_signatures.contains_key(&validator_id) {
            return false; // Duplicate vote
        }

        self.validator_signatures.insert(validator_id, signature);
        self.state_commitments
            .insert(validator_id, state_commitment);
        self.consensus_count += 1;

        // Check if consensus reached
        if self.consensus_count >= self.finality_threshold && !self.is_finalized {
            self.is_finalized = true;
            self.consensus_timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }

        true
    }

    /// Check if consensus is finalized
    pub fn is_consensus_achieved(&self) -> bool {
        self.is_finalized
    }

    /// Get finality percentage
    pub fn finality_percentage(&self) -> u32 {
        if self.total_validators == 0 {
            0
        } else {
            (self.consensus_count * 100) / self.total_validators
        }
    }

    /// Detect divergent validators (state commitment mismatch)
    pub fn detect_divergent_validators(&self) -> Vec<Address> {
        if self.state_commitments.is_empty() {
            return Vec::new();
        }

        let mut commitment_counts: BTreeMap<Hash, Vec<Address>> = BTreeMap::new();
        for (validator_id, commitment) in &self.state_commitments {
            commitment_counts
                .entry(*commitment)
                .or_insert_with(Vec::new)
                .push(*validator_id);
        }

        // Find the majority commitment
        let majority_commitment = commitment_counts
            .iter()
            .max_by_key(|(_, validators)| validators.len())
            .map(|(commitment, _)| *commitment);

        // All validators NOT in the majority are divergent
        if let Some(majority) = majority_commitment {
            self.state_commitments
                .iter()
                .filter(|(_, commitment)| **commitment != majority)
                .map(|(validator_id, _)| *validator_id)
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// Unified Proof combining Atomic VM + GPU Validator System
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnifiedProof {
    /// Header metadata
    pub header: ProofHeader,
    /// Atomic VM proof (PoAE)
    pub atomic_vm_proof: AtomicVmProof,
    /// GPU validator attestations
    pub gpu_attestations: Vec<GpuValidatorAttestation>,
    /// Byzantine consensus state
    pub consensus: ByzantineConsensus,
    /// State merkle proof for cross-chain verification
    pub merkle_proof: Option<StateMerkleProof>,
}

impl UnifiedProof {
    /// Create a new unified proof
    pub fn new(
        header: ProofHeader,
        atomic_vm_proof: AtomicVmProof,
        total_validators: u32,
    ) -> Result<Self, String> {
        if !atomic_vm_proof.validate() {
            return Err("Invalid atomic VM proof".to_string());
        }

        Ok(Self {
            header,
            atomic_vm_proof,
            gpu_attestations: Vec::new(),
            consensus: ByzantineConsensus::new(total_validators),
            merkle_proof: None,
        })
    }

    /// Add a GPU validator attestation
    pub fn add_attestation(&mut self, attestation: GpuValidatorAttestation) -> Result<(), String> {
        // Check for duplicate validator
        if self
            .gpu_attestations
            .iter()
            .any(|a| a.validator_id == attestation.validator_id)
        {
            return Err("Duplicate validator attestation".to_string());
        }

        self.gpu_attestations.push(attestation);
        Ok(())
    }

    /// Add a validator vote to consensus
    pub fn add_validator_vote(
        &mut self,
        validator_id: Address,
        signature: Signature,
        state_commitment: Hash,
    ) -> bool {
        self.consensus
            .add_vote(validator_id, signature, state_commitment)
    }

    /// Set the state merkle proof
    pub fn set_merkle_proof(&mut self, proof: StateMerkleProof) {
        self.merkle_proof = Some(proof);
    }

    /// Compute canonical proof hash for verification
    pub fn proof_hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.header.header_hash());
        hasher.update(&self.atomic_vm_proof.receipt_root);
        hasher.update(&self.gpu_attestations.len().to_le_bytes());

        // Include all attestation hashes for deterministic ordering
        for attestation in &self.gpu_attestations {
            hasher.update(&attestation.attestation_hash());
        }

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Validate the complete unified proof
    pub fn validate(&self) -> ProofValidationResult {
        let mut result = ProofValidationResult::new();

        // Validate header
        if self.header.bundle_id == [0u8; 32] {
            result.add_error("Invalid bundle_id in header".to_string());
        }
        if self.header.finalized_block == 0 {
            result.add_error("Invalid finalized_block in header".to_string());
        }

        // Validate atomic VM proof
        if !self.atomic_vm_proof.validate() {
            result.add_error("Atomic VM proof validation failed".to_string());
        }

        // Validate attestations
        if self.gpu_attestations.is_empty() {
            result.add_warning("No GPU attestations in proof".to_string());
        }

        for attestation in &self.gpu_attestations {
            if attestation.validator_id == [0u8; 32] {
                result.add_error("Invalid validator_id in attestation".to_string());
            }
            if attestation.signature.is_empty() {
                result.add_error("Empty signature in attestation".to_string());
            }
        }

        // Validate merkle proof if present
        if let Some(merkle_proof) = &self.merkle_proof {
            if !merkle_proof.validate() {
                result.add_error("State merkle proof validation failed".to_string());
            }
        }

        // Validate consensus
        if !self.consensus.is_consensus_achieved() {
            result.add_warning(format!(
                "Consensus not finalized: {}/{}",
                self.consensus.consensus_count, self.consensus.total_validators
            ));
        }

        result.is_valid = result.errors.is_empty();
        result
    }

    /// Check for divergent validators
    pub fn get_divergent_validators(&self) -> Vec<Address> {
        self.consensus.detect_divergent_validators()
    }
}

/// Result of proof validation
#[derive(Debug, Clone, Default)]
pub struct ProofValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ProofValidationResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_header_creation() {
        let bundle_id = [1u8; 32];
        let legs_hash = [2u8; 32];
        let header = ProofHeader::new(bundle_id, 100, legs_hash);

        assert_eq!(header.bundle_id, bundle_id);
        assert_eq!(header.finalized_block, 100);
        assert_eq!(header.format_version, 1);
    }

    #[test]
    fn test_byzantine_consensus_threshold() {
        let mut consensus = ByzantineConsensus::new(10);
        assert_eq!(consensus.finality_threshold, 7); // 2/3 + 1 of 10

        let validator1 = [1; 32];
        let validator2 = [2; 32];
        let commitment = [1u8; 32];

        // Not finalized yet
        assert!(!consensus.is_consensus_achieved());

        // Add 6 votes (not enough)
        for i in 0..6 {
            let mut validator = [0; 32];
            validator[0] = i as u8;
            consensus.add_vote(validator, vec![i as u8], commitment);
        }
        assert!(!consensus.is_consensus_achieved());

        // Add 7th vote (reaches threshold)
        consensus.add_vote(validator1, vec![1], commitment);
        assert!(consensus.is_consensus_achieved());

        // Adding duplicate vote should return false
        assert!(!consensus.add_vote(validator1, vec![2], commitment));
    }

    #[test]
    fn test_divergent_validator_detection() {
        let mut consensus = ByzantineConsensus::new(10);
        let commitment1 = [1u8; 32];
        let commitment2 = [2u8; 32];

        // Add 7 validators with commitment1 (majority)
        for i in 0..7 {
            let mut validator = [0; 32];
            validator[0] = i as u8;
            consensus.add_vote(validator, vec![i as u8], commitment1);
        }

        // Add 3 validators with commitment2 (minority - divergent)
        let mut divergent = Vec::new();
        for i in 7..10 {
            let mut validator = [0; 32];
            validator[0] = i as u8;
            divergent.push(validator);
            consensus.add_vote(validator, vec![i as u8], commitment2);
        }

        let detected_divergent = consensus.detect_divergent_validators();
        assert_eq!(detected_divergent.len(), 3);
        assert_eq!(detected_divergent, divergent);
    }

    #[test]
    fn test_unified_proof_creation() {
        let bundle_id = [1u8; 32];
        let legs_hash = [2u8; 32];
        let header = ProofHeader::new(bundle_id, 100, legs_hash);

        let atomic_vm_proof = AtomicVmProof {
            receipt_root: [3u8; 32],
            finality_cert: [4u8; 32],
            leg_count: 1,
            finality_cert_data: vec![1, 2, 3],
        };

        let unified = UnifiedProof::new(header, atomic_vm_proof, 10);
        assert!(unified.is_ok());

        let proof = unified.unwrap();
        assert_eq!(proof.consensus.total_validators, 10);
    }

    #[test]
    fn test_unified_proof_with_merkle_proof() {
        use crate::state_merkle_proof::{generate_merkle_proof, MerkleProofPath, StateMerkleProof};

        let bundle_id = [1u8; 32];
        let legs_hash = [2u8; 32];
        let header = ProofHeader::new(bundle_id, 100, legs_hash);

        let atomic_vm_proof = AtomicVmProof {
            receipt_root: [3u8; 32],
            finality_cert: [4u8; 32],
            leg_count: 1,
            finality_cert_data: vec![1, 2, 3],
        };

        let mut proof = UnifiedProof::new(header, atomic_vm_proof, 10).unwrap();

        // Create a test merkle proof
        let leaves: Vec<[u8; 32]> = (0..4)
            .map(|i| {
                let mut leaf = [0u8; 32];
                leaf[0] = i as u8;
                leaf
            })
            .collect();

        let merkle_path = generate_merkle_proof(&leaves, 0).unwrap();
        let state_merkle_proof =
            StateMerkleProof::new(merkle_path, [99u8; 32], 100, leaves.len() as u64);

        proof.set_merkle_proof(state_merkle_proof);
        assert!(proof.merkle_proof.is_some());

        let result = proof.validate();
        // Should have warning about no consensus, error about merkle proof validation
        assert!(!result.errors.is_empty());
    }
}
