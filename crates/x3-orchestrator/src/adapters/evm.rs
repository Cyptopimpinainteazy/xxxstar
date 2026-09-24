//! EVM chain adapter.
//!
//! `send` and `execute` are still not wired to a node RPC backend and say so.
//! Verification is now wired: the adapter holds the [`ProofVerifier`] it was
//! given and delegates to it, rather than hardcoding "not wired". The trait and
//! its `MockProofVerifier` shipped in this crate with no caller at all, so even a
//! test that wanted to exercise routing had no way to supply one.
//!
//! [`EvmAdapter::new`] deliberately still has no verifier and refuses every
//! proof: nothing about constructing an adapter implies a proof is trustworthy.

use crate::{
    ChainAdapter, ChainId, CrossVmMessage, ExecutionProof, OrchestratorError, ProofVerifier,
    Result, VmExecutor,
};
use std::sync::Arc;

pub struct EvmAdapter {
    pub id: ChainId,
    verifier: Option<Arc<dyn ProofVerifier>>,
    executor: Option<Arc<dyn VmExecutor>>,
}

impl EvmAdapter {
    /// An adapter with neither backend: `verify` and `execute` both refuse.
    pub fn new(id: ChainId) -> Self {
        Self {
            id,
            verifier: None,
            executor: None,
        }
    }

    /// An adapter that delegates proof verification to `verifier`.
    pub fn with_verifier(mut self, verifier: Arc<dyn ProofVerifier>) -> Self {
        self.verifier = Some(verifier);
        self
    }

    /// An adapter that delegates message execution to `executor`.
    pub fn with_executor(mut self, executor: Arc<dyn VmExecutor>) -> Self {
        self.executor = Some(executor);
        self
    }
}

impl ChainAdapter for EvmAdapter {
    fn chain_id(&self) -> ChainId {
        self.id.clone()
    }

    fn send(&self, _msg: &CrossVmMessage) -> Result<String> {
        Err(OrchestratorError::ExecutionFailed(
            "EVM adapter: send not yet wired to node RPC backend".into(),
        ))
    }

    fn verify(&self, proof: &ExecutionProof) -> Result<bool> {
        if proof.proof_bytes.is_empty() {
            return Err(OrchestratorError::InvalidProof);
        }
        match &self.verifier {
            Some(verifier) => verifier.verify(proof),
            None => Err(OrchestratorError::ExecutionFailed(
                "EVM proof verification backend is not wired".into(),
            )),
        }
    }

    fn execute(&self, msg: &CrossVmMessage) -> Result<()> {
        match &self.executor {
            Some(executor) => executor.execute_message(msg).map(|_| ()),
            None => Err(OrchestratorError::ExecutionFailed(
                "EVM adapter: execute not yet wired to node RPC backend".into(),
            )),
        }
    }
}
