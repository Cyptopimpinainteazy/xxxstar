//! Error types for the gateway.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

/// Gateway error types.
#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Upstream error: {0}")]
    Upstream(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias.
pub type Result<T> = std::result::Result<T, GatewayError>;

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            GatewayError::Database(err) => {
                tracing::error!(error = %err, "gateway database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database operation failed".to_string(),
                )
            }
            GatewayError::Serialization(err) => {
                tracing::error!(error = %err, "gateway serialization error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "serialization failed".to_string(),
                )
            }
            GatewayError::Config(err) => {
                tracing::error!(error = %err, "gateway configuration error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "gateway configuration error".to_string(),
                )
            }
            GatewayError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            GatewayError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            GatewayError::Upstream(err) => {
                tracing::error!(error = %err, "gateway upstream error");
                (
                    StatusCode::BAD_GATEWAY,
                    "upstream control-plane request failed".to_string(),
                )
            }
            GatewayError::Internal(err) => {
                tracing::error!(error = %err, "gateway internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        let body = serde_json::json!({
            "error": message
        });

        (status, axum::Json(body)).into_response()
    }
}
