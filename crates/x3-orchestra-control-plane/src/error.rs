use thiserror::Error;

pub type Result<T> = std::result::Result<T, ControlPlaneError>;

#[derive(Debug, Error)]
pub enum ControlPlaneError {
    #[error("{0} not found")]
    NotFound(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("approval required before dispatch")]
    ApprovalRequired,
    #[error("intent is not ready for dispatch")]
    IntentNotDispatchable,
    #[error("vote window is not open")]
    VoteWindowNotOpen,
    #[error("vote window cannot close before its deadline")]
    VoteWindowStillOpen,
    #[error("voter is not eligible for this vote window")]
    IneligibleVoter,
    #[error("vote already recorded for this voter")]
    DuplicateVote,
    #[error("crm adapter failure: {0}")]
    Crm(String),
    #[error("persistence failure: {0}")]
    Persistence(String),
}
