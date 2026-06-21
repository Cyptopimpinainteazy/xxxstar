//! SVM (Solana-style) chain adapter (mock implementation).

use crate::{ChainAdapter, ChainId, CrossVmMessage, ExecutionProof, OrchestratorError, Result};

pub struct SvmAdapter {
    pub id: ChainId,
}

impl SvmAdapter {
    pub fn new(id: ChainId) -> Self {
        Self { id }
    }
}

impl ChainAdapter for SvmAdapter {
    fn chain_id(&self) -> ChainId {
        self.id.clone()
    }

    fn send(&self, _msg: &CrossVmMessage) -> Result<String> {
        Err(OrchestratorError::ExecutionFailed(
            "SVM adapter: send not yet wired to node RPC backend".into(),
        ))
    }

    fn verify(&self, proof: &ExecutionProof) -> Result<bool> {
        if proof.proof_bytes.is_empty() {
            return Err(OrchestratorError::InvalidProof);
        }
        Err(OrchestratorError::ExecutionFailed(
            "SVM proof verification backend is not wired".into(),
        ))
    }

    fn execute(&self, _msg: &CrossVmMessage) -> Result<()> {
        Err(OrchestratorError::ExecutionFailed(
            "SVM adapter: execute not yet wired to node RPC backend".into(),
        ))
    }
}
