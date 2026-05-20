//! # Proof Integration for Validator System
//!
//! Bridges GPU validator execution results with unified proof generation and aggregation.
//! Converts validator task execution results into unified proofs for Byzantine consensus finality.

use crate::deterministic::ExecutionResult;
use crate::error::SwarmResult;
use crate::gpu_receipt::{GpuClass, GpuReceipt, ProofType};
use crate::state_merkle_proof::{generate_merkle_proof, StateMerkleProof};
use crate::unified_proof::{AtomicVmProof, GpuValidatorAttestation, ProofHeader, UnifiedProof};
use crate::{crypto::HashOutput, deterministic::ExecutionMode};
use sha2::{Digest, Sha256};
use x3_orchestra_control_plane::EvidenceBundle;

pub type Hash = [u8; 32];

/// Converts validator execution result to GPU receipt
pub fn execution_result_to_receipt(
    result: &ExecutionResult,
    executor_address: [u8; 32],
    _device_index: u32,
) -> SwarmResult<GpuReceipt> {
    let kernel_hash = compute_kernel_hash(&result.task_id);
    let input_commitment = compute_input_hash(&[]);
    let output_commitment = if result.outputs.is_empty() {
        [0u8; 32]
    } else {
        let outputs: Vec<Vec<u8>> = result
            .outputs
            .iter()
            .map(|o| o.as_bytes().to_vec())
            .collect();
        compute_output_hash(&outputs)
    };

    Ok(GpuReceipt {
        kernel_hash,
        input_commitment,
        output_commitment,
        gpu_cycles_used: 0, // Not available in ExecutionResult
        device_class: GpuClass::DataCenter,
        executor: executor_address,
        proof_type: ProofType::RecomputeA,
    })
}

/// Creates unified proof from validator execution result
pub fn create_unified_proof(
    result: &ExecutionResult,
    receipt: GpuReceipt,
    validator_signature: Vec<u8>,
    bundle_id: [u8; 32],
    finalized_block: u64,
    total_validators: u32,
) -> SwarmResult<UnifiedProof> {
    let header = ProofHeader::new(
        bundle_id,
        finalized_block,
        compute_legs_hash(&result.task_id),
    );

    // Create atomic VM proof component
    let finality_cert = compute_finality_cert(&result.task_id);
    let finality_cert_data = finality_cert.to_vec();

    let atomic_proof = AtomicVmProof {
        receipt_root: compute_receipt_root(&result.task_id),
        finality_cert,
        leg_count: 1,
        finality_cert_data,
    };

    // Create GPU attestation
    let attestation = GpuValidatorAttestation {
        validator_id: receipt.executor,
        receipt: receipt.clone(),
        signature: validator_signature,
        device_index: 0,
        proof_type: receipt.proof_type.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        execution_latency_ms: (result.execution_time_us / 1000) as u64,
    };

    let mut proof = UnifiedProof::new(header, atomic_proof, total_validators)
        .map_err(|e| crate::error::SwarmError::VerificationFailed(e))?;

    proof
        .add_attestation(attestation)
        .map_err(|e| crate::error::SwarmError::VerificationFailed(e))?;

    // Generate and attach merkle proof
    if let Ok(merkle_proof) = create_merkle_proof(result, finalized_block) {
        proof.set_merkle_proof(merkle_proof);
    }

    Ok(proof)
}

/// Converts orchestra control-plane evidence into a unified proof that can enter the
/// existing validator aggregation pipeline.
pub fn orchestra_evidence_to_unified_proof(
    evidence: &EvidenceBundle,
    validator_id: [u8; 32],
    validator_signature: Vec<u8>,
    bundle_id: [u8; 32],
    finalized_block: u64,
    total_validators: u32,
) -> SwarmResult<UnifiedProof> {
    let result = orchestra_evidence_to_execution_result(evidence);
    let receipt = GpuReceipt {
        kernel_hash: compute_kernel_hash(&format!("orchestra:{}", evidence.bundle_id)),
        input_commitment: compute_input_hash(&[
            evidence.artifact_uri.as_bytes().to_vec(),
            evidence.digest.as_bytes().to_vec(),
        ]),
        output_commitment: compute_output_hash(&[
            serde_json::to_vec(&evidence.summary.detail).unwrap_or_default()
        ]),
        gpu_cycles_used: 0,
        device_class: GpuClass::DataCenter,
        executor: validator_id,
        proof_type: ProofType::RecomputeA,
    };

    create_unified_proof(
        &result,
        receipt,
        validator_signature,
        bundle_id,
        finalized_block,
        total_validators,
    )
}

fn orchestra_evidence_to_execution_result(evidence: &EvidenceBundle) -> ExecutionResult {
    let mut digest_hasher = Sha256::new();
    digest_hasher.update(evidence.digest.as_bytes());
    digest_hasher.update(evidence.artifact_uri.as_bytes());
    digest_hasher.update(serde_json::to_vec(&evidence.summary.detail).unwrap_or_default());
    let digest = digest_hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&digest);

    ExecutionResult {
        task_id: format!("orchestra:{}", evidence.bundle_id),
        outputs: vec![HashOutput::new(output)],
        verification: crate::crypto::VerificationResult::Valid,
        execution_mode: ExecutionMode::GpuWithCpuVerification,
        execution_time_us: 0,
        divergence_detected: false,
        cpu_fallback_used: false,
        error: None,
    }
}

/// Helper: compute kernel hash from task ID
fn compute_kernel_hash(task_id: &str) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(task_id.as_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Helper: compute input hash from inputs
fn compute_input_hash(inputs: &[Vec<u8>]) -> Hash {
    let mut hasher = Sha256::new();
    for input in inputs {
        hasher.update(input);
    }
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Helper: compute output hash from outputs
fn compute_output_hash(outputs: &[Vec<u8>]) -> Hash {
    let mut hasher = Sha256::new();
    for output in outputs {
        hasher.update(output);
    }
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Helper: compute legs hash
fn compute_legs_hash(task_id: &str) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(b"legs:");
    hasher.update(task_id.as_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Helper: compute receipt root
fn compute_receipt_root(task_id: &str) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(b"receipt_root:");
    hasher.update(task_id.as_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Helper: compute finality certificate
fn compute_finality_cert(task_id: &str) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(b"finality_cert:");
    hasher.update(task_id.as_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Helper: create state merkle proof from execution result
fn create_merkle_proof(
    result: &ExecutionResult,
    block_number: u64,
) -> SwarmResult<StateMerkleProof> {
    // Generate merkle tree from execution outputs
    let output_hashes: Vec<Hash> = result
        .outputs
        .iter()
        .map(|output| {
            let mut hasher = Sha256::new();
            hasher.update(output.as_bytes());
            let digest = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&digest);
            hash
        })
        .collect();

    // If no outputs, create a single hash from task_id
    let leaves = if output_hashes.is_empty() {
        let hash = compute_kernel_hash(&result.task_id);
        vec![hash]
    } else {
        output_hashes
    };

    // Generate merkle proof for the first output (state root)
    let proof_path = generate_merkle_proof(&leaves, 0)
        .map_err(|e| crate::error::SwarmError::InvalidMerkleProof(e))?;

    let state_root = proof_path.expected_root;
    let tree_size = leaves.len() as u64;

    Ok(StateMerkleProof::new(
        proof_path,
        state_root,
        block_number,
        tree_size,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::HashOutput;

    #[test]
    fn test_execution_to_receipt_conversion() {
        let result = ExecutionResult {
            task_id: "task_123".to_string(),
            outputs: vec![HashOutput::new([4u8; 32])],
            verification: crate::crypto::VerificationResult::Valid,
            execution_mode: crate::deterministic::ExecutionMode::GpuWithCpuVerification,
            execution_time_us: 50000,
            divergence_detected: false,
            cpu_fallback_used: false,
            error: None,
        };

        let executor = [42u8; 32];
        let receipt = execution_result_to_receipt(&result, executor, 0).unwrap();

        assert_eq!(receipt.executor, executor);
        assert_eq!(receipt.device_class, GpuClass::DataCenter);
        assert_eq!(receipt.proof_type, ProofType::RecomputeA);
    }

    #[test]
    fn test_unified_proof_creation() {
        let result = ExecutionResult {
            task_id: "task_456".to_string(),
            outputs: vec![],
            verification: crate::crypto::VerificationResult::Valid,
            execution_mode: crate::deterministic::ExecutionMode::GpuWithCpuVerification,
            execution_time_us: 25000,
            divergence_detected: false,
            cpu_fallback_used: false,
            error: None,
        };

        let executor = [99u8; 32];
        let receipt = execution_result_to_receipt(&result, executor, 0).unwrap();
        let bundle_id = [11u8; 32];
        let signature = vec![1, 2, 3, 4];

        let proof = create_unified_proof(&result, receipt, signature, bundle_id, 100, 10).unwrap();

        assert_eq!(proof.header.bundle_id, bundle_id);
        assert_eq!(proof.header.finalized_block, 100);
        assert!(proof.validate().is_valid);
    }

    #[test]
    fn orchestra_evidence_converts_to_unified_proof() {
        let evidence = EvidenceBundle {
            bundle_id: "orchestra-bundle-1".to_string(),
            intent_id: Some("intent-1".to_string()),
            approval_case_id: Some("case-1".to_string()),
            vote_window_id: Some("window-1".to_string()),
            artifact_uri: "orchestra://vote-windows/window-1/closure".to_string(),
            digest: "sha256:orchestra".to_string(),
            summary: x3_orchestra_control_plane::EvidenceSummary {
                action: "vote_window_closed".to_string(),
                detail: serde_json::json!({"approved": true}),
            },
            created_at_unix: 123,
        };

        let proof = orchestra_evidence_to_unified_proof(
            &evidence,
            [7u8; 32],
            vec![9, 9, 9],
            [3u8; 32],
            42,
            5,
        )
        .expect("orchestra proof");

        assert_eq!(proof.header.bundle_id, [3u8; 32]);
        assert_eq!(proof.header.finalized_block, 42);
        assert!(proof.validate().is_valid);
    }
}
