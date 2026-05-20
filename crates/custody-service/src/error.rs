/// Custody service error types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CustodyError {
    #[error("vault not found: {0}")]
    VaultNotFound(String),

    #[error("insufficient vault balance: required {required}, available {available}")]
    InsufficientBalance { required: u128, available: u128 },

    #[error("operation authorization failed: {0}")]
    AuthorizationFailed(String),

    #[error("insufficient authorization tier: required {required}, provided {provided}")]
    InsufficientTier { required: String, provided: String },

    #[error("policy violation: {0}")]
    PolicyViolation(String),

    #[error("vault is frozen: {0}")]
    VaultFrozen(String),

    #[error("operation not found: {0}")]
    OperationNotFound(String),

    #[error("operation already exists (idempotency): {0}")]
    OperationExists(String),

    #[error("HSM operation failed: {0}")]
    HSMError(String),

    #[error("key not found in HSM: {0}")]
    KeyNotFound(String),

    #[error("invalid destination: {0}")]
    InvalidDestination(String),

    #[error("operation expired")]
    OperationExpired,

    #[error("invalid amount: {0}")]
    InvalidAmount(String),

    #[error("settlement linkage failed: {0}")]
    SettlementLinkageFailed(String),

    #[error("audit trail corrupted")]
    AuditTrailCorrupted,

    #[error("signer policy violation: {0}")]
    SignerPolicyViolation(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, CustodyError>;
