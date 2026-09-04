//! Routing engine: validates proofs, enforces replay protection, and
//! dispatches cross-VM messages to the target adapter.

use std::sync::Arc;

use crate::{
    AdapterRegistry, CrossVmMessage, ExecutionProof, OrchestratorError, ReplayGuard, Result,
};

pub struct OrchestratorRouter {
    registry: Arc<AdapterRegistry>,
    replay_guard: Arc<ReplayGuard>,
}

impl OrchestratorRouter {
    pub fn new(registry: Arc<AdapterRegistry>, replay_guard: Arc<ReplayGuard>) -> Self {
        Self {
            registry,
            replay_guard,
        }
    }

    /// Route a verified message from its source chain to its target chain.
    ///
    /// Returns the deterministic message id on success.
    pub fn route(&self, msg: &CrossVmMessage, proof: &ExecutionProof) -> Result<String> {
        let message_id = msg.id()?;

        self.replay_guard.check_and_mark(&message_id)?;

        let source = self
            .registry
            .get(&msg.source_chain)
            .ok_or_else(|| OrchestratorError::AdapterNotFound(msg.source_chain.0.clone()))?;

        let target = self
            .registry
            .get(&msg.target_chain)
            .ok_or_else(|| OrchestratorError::AdapterNotFound(msg.target_chain.0.clone()))?;

        if !source.verify(proof)? {
            return Err(OrchestratorError::InvalidProof);
        }

        target.execute(msg)?;

        Ok(message_id)
    }
}
