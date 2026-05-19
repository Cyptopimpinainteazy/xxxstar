//! Constitutional error types.

use crate::invariants::CoreInvariant;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ConstitutionError {
    #[error("invariant violation ({0:?}): {1}")]
    InvariantViolation(CoreInvariant, String),

    #[error("amendment rejected: {0}")]
    AmendmentRejected(String),

    #[error("governance proof required but not provided (Article IV)")]
    ProofRequired,

    #[error("governance proof commitment does not match on-chain record")]
    ProofMismatch,

    #[error("execution is non-deterministic (Article II): {0}")]
    NonDeterministic(String),

    #[error("execution is unbounded (Article II): {0}")]
    Unbounded(String),

    #[error("proposal depth {0} exceeds constitutional maximum {1} (Article IV)")]
    ProposalDepthExceeded(u8, u8),
}
