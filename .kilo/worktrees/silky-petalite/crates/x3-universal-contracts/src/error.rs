//! Error type for the Universal Contracts SDK.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UcError {
    #[error("action list is empty")]
    EmptyActionList,

    #[error("Abort action must be the last action in the sequence")]
    AbortNotLast,

    #[error("amount cannot be zero")]
    ZeroAmount,

    #[error("source and destination asset IDs are identical")]
    SameAsset,

    #[error("fee_cap cannot be zero")]
    ZeroFeeCap,

    #[error("intent error: {0}")]
    IntentError(String),

    #[error("packet error: {0}")]
    PacketError(String),
}
