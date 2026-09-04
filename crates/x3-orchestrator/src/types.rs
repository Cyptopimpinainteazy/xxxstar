//! Shared types for the orchestrator.

use serde::{Deserialize, Serialize};

/// Execution domain a message originates from or targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VmKind {
    Evm,
    Svm,
    X3vm,
    Wasm,
    Bitcoin,
    Substrate,
}

/// Stable identifier for a chain participating in the orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChainId(pub String);

impl ChainId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Lifecycle status of a cross-VM message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageStatus {
    Pending,
    Verified,
    Executed,
    Failed(String),
}
