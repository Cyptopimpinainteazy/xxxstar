//! Execution proof verification surface.

use serde::{Deserialize, Serialize};

use crate::{OrchestratorError, Result};

/// Proof that a cross-VM message was committed on its source chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProof {
    pub source_chain: String,
    pub message_id: String,
    pub block_number: u64,
    pub state_root: Vec<u8>,
    pub proof_bytes: Vec<u8>,
}

pub trait ProofVerifier: Send + Sync {
    fn verify(&self, proof: &ExecutionProof) -> Result<bool>;
}

/// Mock verifier used by tests and the demo CLI. Real adapters plug in
/// their own verifier (light client, Merkle proof, attestation, etc.).
#[derive(Default)]
pub struct MockProofVerifier;

impl ProofVerifier for MockProofVerifier {
    fn verify(&self, proof: &ExecutionProof) -> Result<bool> {
        if proof.message_id.is_empty() || proof.proof_bytes.is_empty() {
            return Err(OrchestratorError::InvalidProof);
        }
        Ok(true)
    }
}
