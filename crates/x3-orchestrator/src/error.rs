//! Orchestrator error types.

use thiserror::Error;

/// Errors returned by orchestrator routing, replay, proof, and invariant
/// checks. All runtime paths return `Result<T>` rather than panicking.
#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("chain adapter not found: {0}")]
    AdapterNotFound(String),

    #[error("message already executed: {0}")]
    ReplayDetected(String),

    #[error("invalid proof")]
    InvalidProof,

    #[error("routing failed: {0}")]
    RoutingFailed(String),

    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    #[error("canonical supply invariant failed")]
    InvariantFailed,
}

pub type Result<T> = std::result::Result<T, OrchestratorError>;
