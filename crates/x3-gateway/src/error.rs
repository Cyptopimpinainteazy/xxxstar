//! Gateway error type shared across the REST, GraphQL and database layers.
//!
//! `GatewayError` is the single error type surfaced by the x3-gateway crate.
//! Handlers return `Result<T, GatewayError>`; because `GatewayError`
//! implements [`IntoResponse`], axum turns failures into consistent JSON
//! error bodies without per-handler conversion boilerplate.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// Convenience alias used throughout `crate::db` and `crate::rest`.
pub type Result<T> = std::result::Result<T, GatewayError>;

/// Error taxonomy for the gateway service.
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    /// Client sent an invalid request (validation or shape failure).
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Requested resource does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// Unexpected internal failure (DB, cache, control-plane, config, io).
    #[error("internal error: {0}")]
    Internal(String),
}

/// Wire representation returned by [`GatewayError::into_response`].
#[derive(Debug, Serialize)]
struct ErrorEnvelope<'a> {
    code: &'a str,
    message: &'a str,
}

impl GatewayError {
    /// Short numeric-independent machine code for HTTP consumers.
    fn code(&self) -> &'static str {
        match self {
            GatewayError::BadRequest(_) => "bad_request",
            GatewayError::NotFound(_) => "not_found",
            GatewayError::Internal(_) => "internal_error",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            GatewayError::BadRequest(_) => StatusCode::BAD_REQUEST,
            GatewayError::NotFound(_) => StatusCode::NOT_FOUND,
            GatewayError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let status = self.status();
        let envelope = ErrorEnvelope {
            code: self.code(),
            message: &self.to_string(),
        };
        (status, Json(envelope)).into_response()
    }
}

// `?` propagation from lower-level fallible layers into `GatewayError`.
impl From<sqlx::Error> for GatewayError {
    fn from(err: sqlx::Error) -> Self {
        GatewayError::Internal(format!("database error: {err}"))
    }
}

impl From<serde_json::Error> for GatewayError {
    fn from(err: serde_json::Error) -> Self {
        GatewayError::Internal(format!("json error: {err}"))
    }
}

impl From<std::io::Error> for GatewayError {
    fn from(err: std::io::Error) -> Self {
        GatewayError::Internal(format!("io error: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_variants_to_http_status() {
        assert_eq!(
            GatewayError::BadRequest("nope".into()).status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            GatewayError::NotFound("missing".into()).status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            GatewayError::Internal("boom".into()).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn message_includes_detail() {
        let msg = GatewayError::BadRequest("empty tenant".into()).to_string();
        assert!(msg.contains("empty tenant"));
    }
}
