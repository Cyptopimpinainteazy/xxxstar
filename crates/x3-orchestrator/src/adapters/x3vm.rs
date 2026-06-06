//! X3VM adapter (mock implementation; production wiring lives in
//! `x3-vm` + `x3-bridge-adapters`).

use crate::{ChainAdapter, ChainId, CrossVmMessage, ExecutionProof, OrchestratorError, Result};

pub struct X3VmAdapter {
    pub id: ChainId,
}

impl X3VmAdapter {
    pub fn new(id: ChainId) -> Self {
        Self { id }
    }
}

impl ChainAdapter for X3VmAdapter {
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
            "X3VM proof verification backend is not wired".into(),
        ))
    }

    fn execute(&self, _msg: &CrossVmMessage) -> Result<()> {
        Ok(())
    }
}
