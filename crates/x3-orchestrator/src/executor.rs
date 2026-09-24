//! VM-level executor trait. Adapters that wrap a real virtual machine
//! (X3VM, EVM, SVM) implement this so the orchestrator can drive them.

use crate::{CrossVmMessage, MessageStatus, Result};

/// `Send + Sync` because adapters holding one must satisfy
/// [`crate::ChainAdapter`], which is itself `Send + Sync` — the same bound
/// [`crate::ProofVerifier`] carries.
pub trait VmExecutor: Send + Sync {
    fn execute_message(&self, msg: &CrossVmMessage) -> Result<MessageStatus>;
}

/// Executor that reports success without touching a chain.
///
/// The analogue of [`crate::MockProofVerifier`], and for the same reason: the
/// trait had no implementation anywhere in the workspace, so nothing could drive
/// an adapter even in a test. Real backends (node RPC, `x3-vm`) implement
/// [`VmExecutor`] and are supplied through the adapters' `with_executor`.
pub struct MockVmExecutor;

impl VmExecutor for MockVmExecutor {
    fn execute_message(&self, _msg: &CrossVmMessage) -> Result<MessageStatus> {
        Ok(MessageStatus::Executed)
    }
}
