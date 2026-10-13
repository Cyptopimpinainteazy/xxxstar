/// Comprehensive error types for the Quantum Kernel Voyager backend.
/// All errors are serializable for IPC transport to the frontend.
use serde::Serialize;
use std::fmt;

/// Top-level application error enum.
/// Each variant maps to a specific error domain — no raw strings.
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    ChainError(String),
    KeystoreError(String),
    IoError(String),
    SerializationError(String),
    ValidationError(String),
    RateLimitError(String),
    NotFound(String),
    Unauthorized(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChainError(msg) => write!(f, "Chain error: {msg}"),
            Self::KeystoreError(msg) => write!(f, "Keystore error: {msg}"),
            Self::IoError(msg) => write!(f, "IO error: {msg}"),
            Self::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            Self::ValidationError(msg) => write!(f, "Validation error: {msg}"),
            Self::RateLimitError(msg) => write!(f, "Rate limit: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::Unauthorized(msg) => write!(f, "Unauthorized: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerializationError(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = AppError::ChainError("connection refused".into());
        assert!(e.to_string().contains("connection refused"));
    }

    #[test]
    fn error_serialize() {
        let e = AppError::ValidationError("bad input".into());
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("ValidationError"));
        assert!(json.contains("bad input"));
    }
}
