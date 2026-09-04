//! Error types for the court system.

use crate::types::DisputeId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CourtError {
    #[error("dispute not found: {0:?}")]
    DisputeNotFound(DisputeId),

    #[error("challenger bond too small")]
    BondTooSmall,

    #[error("dispute {0:?} is not in a fileable state")]
    DisputeNotFileable(DisputeId),

    #[error("dispute {0:?} exceeded its finality deadline")]
    DeadlineExceeded(DisputeId),

    #[error("duplicate dispute ID: {0:?}")]
    DuplicateDispute(DisputeId),

    #[error("replay engine failure: {0}")]
    ReplayFailed(String),

    #[error("proof chain verification failed")]
    ProofVerificationFailed,
}
