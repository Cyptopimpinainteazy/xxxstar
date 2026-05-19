//! EVM chain adapter (mock implementation, real RPC integration plugs into
//! the same trait).

use crate::{ChainAdapter, ChainId, CrossVmMessage, ExecutionProof, OrchestratorError, Result};

pub struct EvmAdapter {
    pub id: ChainId,
}

impl EvmAdapter {
    pub fn new(id: ChainId) -> Self {
        Self { id }
    }
}

impl ChainAdapter for EvmAdapter {
    fn chain_id(&self) -> ChainId {
        self.id.clone()
    }

    fn send(&self, msg: &CrossVmMessage) -> Result<String> {
        msg.id()
    }

    fn verify(&self, proof: &ExecutionProof) -> Result<bool> {
        if proof.proof_bytes.is_empty() {
            return Err(OrchestratorError::InvalidProof);
        }
        Err(OrchestratorError::ExecutionFailed(
            "EVM proof verification backend is not wired".into(),
        ))
    }

    fn execute(&self, _msg: &CrossVmMessage) -> Result<()> {
        Ok(())
    }
}
