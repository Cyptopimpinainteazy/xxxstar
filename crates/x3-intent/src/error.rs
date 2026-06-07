//! Error types for the intent system.

use crate::types::IntentState;
use thiserror::Error;
use x3_proof::types::IntentId;

#[derive(Debug, Error)]
pub enum IntentError {
    #[error("intent bond must be non-zero for slashable execution")]
    ZeroBond,

    #[error("intent fee cap must be non-zero")]
    ZeroFeeCap,

    #[error("invalid state transition from {from:?} to {to:?}")]
    InvalidTransition { from: IntentState, to: IntentState },

    #[error("intent {0:?} has expired")]
    Expired(IntentId),

    #[error("intent {0:?} has not expired yet")]
    NotExpired(IntentId),

    #[error("intent {0:?} is already in a terminal state")]
    AlreadyTerminal(IntentId),

    #[error("route must have at least one leg")]
    EmptyRoute,

    #[error("fee {fee} exceeds cap {cap}")]
    FeeCapExceeded { fee: u128, cap: u128 },

    #[error("no execution result available")]
    NoExecutionResult,

    #[error("execution failed for intent {intent_id:?} (slashable: {slashable})")]
    ExecutionFailed {
        intent_id: IntentId,
        slashable: bool,
    },

    #[error("duplicate intent ID: {0:?}")]
    DuplicateId(IntentId),
}
