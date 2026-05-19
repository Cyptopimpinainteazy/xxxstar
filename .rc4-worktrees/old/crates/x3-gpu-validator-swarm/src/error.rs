//! X3 GPU Validator Swarm Error Types

use thiserror::Error;

/// Main error type for the X3 GPU Validator Swarm
#[derive(Error, Debug)]
pub enum SwarmError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    // Peer not found
    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    /// Crypto operation error
    #[error("Crypto error: {0}")]
    CryptoError(String),

    /// GPU operation error
    #[error("GPU error: {0}")]
    GpuError(String),

    /// Verification failed
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    /// Divergence detected between GPU and CPU results
    #[error("Divergence detected: {0}")]
    Divergence(String),

    /// Validator is quarantined
    #[error("Validator is quarantined: {0}")]
    Quarantined(String),

    /// Unauthorized operation
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Task timeout
    #[error("Task timeout: {0}")]
    Timeout(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),

    /// Proof not found
    #[error("Proof not found")]
    ProofNotFound,

    /// Duplicate proof
    #[error("Duplicate proof")]
    DuplicateProof,

    /// Duplicate attestation
    #[error("Duplicate attestation")]
    DuplicateAttestation,

    /// Invalid merkle proof
    #[error("Invalid merkle proof: {0}")]
    InvalidMerkleProof(String),

    /// Invalid state root
    #[error("Invalid state root: {0}")]
    InvalidStateRoot(String),

    /// Merkle path mismatch
    #[error("Merkle path mismatch: {0}")]
    MerklePathMismatch(String),

    /// Invalid merkle node
    #[error("Invalid merkle node: {0}")]
    InvalidMerkleNode(String),
}

impl From<std::io::Error> for SwarmError {
    fn from(e: std::io::Error) -> Self {
        SwarmError::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for SwarmError {
    fn from(e: serde_json::Error) -> Self {
        SwarmError::SerializationError(e.to_string())
    }
}

impl From<toml::de::Error> for SwarmError {
    fn from(e: toml::de::Error) -> Self {
        SwarmError::ConfigError(e.to_string())
    }
}

impl From<toml::ser::Error> for SwarmError {
    fn from(e: toml::ser::Error) -> Self {
        SwarmError::ConfigError(e.to_string())
    }
}

/// Result type alias
pub type SwarmResult<T> = Result<T, SwarmError>;
