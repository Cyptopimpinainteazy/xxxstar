use std::fmt;
use thiserror::Error;

/// FoundryError represents all possible errors in the X3 Foundry system.
#[derive(Error, Debug, Clone)]
pub enum FoundryError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Generation failed: {0}")]
    GenerationFailed(String),

    #[error("Compliance check failed: {0}")]
    ComplianceFailed(String),

    #[error("Security audit failed: {0}")]
    SecurityAuditFailed(String),

    #[error("Simulation failed: {0}")]
    SimulationFailed(String),

    #[error("Deployment failed: {0}")]
    DeploymentFailed(String),

    #[error("Revenue tracking error: {0}")]
    RevenueError(String),

    #[error("No revenue to claim for {0}")]
    NoRevenueToClaim(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Invalid prompt: {0}")]
    InvalidPrompt(String),

    #[error("Chain not supported: {0}")]
    ChainNotSupported(String),

    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<serde_json::Error> for FoundryError {
    fn from(err: serde_json::Error) -> Self {
        FoundryError::SerializationError(err.to_string())
    }
}

impl From<std::io::Error> for FoundryError {
    fn from(err: std::io::Error) -> Self {
        FoundryError::Internal(err.to_string())
    }
}

impl From<reqwest::Error> for FoundryError {
    fn from(err: reqwest::Error) -> Self {
        FoundryError::NetworkError(err.to_string())
    }
}

impl From<anyhow::Error> for FoundryError {
    fn from(err: anyhow::Error) -> Self {
        FoundryError::Internal(err.to_string())
    }
}

impl From<uuid::Error> for FoundryError {
    fn from(err: uuid::Error) -> Self {
        FoundryError::Internal(err.to_string())
    }
}

impl From<chrono::ParseError> for FoundryError {
    fn from(err: chrono::ParseError) -> Self {
        FoundryError::Internal(err.to_string())
    }
}

impl From<hex::FromHexError> for FoundryError {
    fn from(err: hex::FromHexError) -> Self {
        FoundryError::Internal(err.to_string())
    }
}

impl From<Box<dyn std::error::Error>> for FoundryError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        FoundryError::Unknown(err.to_string())
    }
}

/// Result type alias for FoundryError.
pub type FoundryResult<T> = Result<T, FoundryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = FoundryError::TemplateNotFound("test".into());
        assert_eq!(format!("{}", err), "Template not found: test");
    }

    #[test]
    fn test_error_from_serde() {
        let serde_err = serde_json::from_str::<String>("invalid").unwrap_err();
        let foundry_err: FoundryError = serde_err.into();
        assert!(matches!(foundry_err, FoundryError::SerializationError(_)));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let foundry_err: FoundryError = io_err.into();
        assert!(matches!(foundry_err, FoundryError::Internal(_)));
    }

    #[test]
    fn test_foundry_result() {
        let ok: FoundryResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: FoundryResult<i32> = Err(FoundryError::NotFound("test".into()));
        assert!(err.is_err());
    }
}
