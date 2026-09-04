//! Turbine Error Types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TurbineError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Invalid shred: {0}")]
    InvalidShred(String),

    #[error("Blockstore error: {0}")]
    BlockstoreError(String),

    #[error("Broadcast error: {0}")]
    BroadcastError(String),

    #[error("Recovery error: {0}")]
    RecoveryError(String),

    #[error("Peer error: {0}")]
    PeerError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Not started: {0}")]
    NotStarted(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type TurbineResult<T> = Result<T, TurbineError>;
