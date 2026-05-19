//! Error types for cross-chain GPU validator

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorError {
    #[error("GPU initialization failed: {0}")]
    GpuInitError(String),

    #[error("GPU validation failed: {0}")]
    GpuValidationError(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("CPU validation fallback failed: {0}")]
    CpuValidationError(String),

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Atomic registry error: {0}")]
    RegistryError(String),

    #[error("Redis connection failed: {0}")]
    RedisError(String),

    #[error("Swap timeout exceeded")]
    SwapTimeout,

    #[error("Swap state invalid: {0}")]
    InvalidSwapState(String),

    #[error("EVM validation failed: {0}")]
    EvmValidationFailed(String),

    #[error("SVM validation failed: {0}")]
    SvmValidationFailed(String),

    #[error("Atomic violation detected: {0}")]
    AtomicViolation(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ValidatorError>;
