//! SVM (Solana-style) chain adapter.
//!
//! `send`/`execute` remain unwired and say so; `verify` delegates to the
//! [`ProofVerifier`] the adapter was given. Constructing one with
//! [`SvmAdapter::new`] supplies no verifier, so it refuses every proof.

use crate::{
    ChainAdapter, ChainId, CrossVmMessage, ExecutionProof, OrchestratorError, ProofVerifier,
    Result, VmExecutor,
};
use std::sync::Arc;

pub struct SvmAdapter {
    pub id: ChainId,
    verifier: Option<Arc<dyn ProofVerifier>>,
    executor: Option<Arc<dyn VmExecutor>>,
}

impl SvmAdapter {
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
        match &self.verifier {
            Some(verifier) => verifier.verify(proof),
            None => Err(OrchestratorError::ExecutionFailed(
                "SVM proof verification backend is not wired".into(),
            )),
        }
    }

    fn execute(&self, msg: &CrossVmMessage) -> Result<()> {
        match &self.executor {
            Some(executor) => executor.execute_message(msg).map(|_| ()),
            None => Err(OrchestratorError::ExecutionFailed(
                "SVM adapter: execute not yet wired to node RPC backend".into(),
            )),
        }
    }
}
