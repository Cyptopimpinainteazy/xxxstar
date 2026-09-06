//! Gateway configuration: validated process settings for the REST/GraphQL
//! server, its optional Postgres data source, optional Redis cache, and the
//! optional remote orchestra control-plane client the gateway relays to.

use crate::error::{GatewayError, Result};
use serde::Serialize;
use std::net::SocketAddr;

/// Postgres connectivity parameters consumed by [`crate::db::Database`].
#[derive(Debug, Clone, Serialize)]
pub struct DatabaseConfig {
    /// Postgres connection DSN. Present only when the operator opts into the
    /// indexed-data backend (see `X3_GATEWAY_DATABASE_URL`).
    pub url: String,
    /// Maximum number of connections in the pool.
    pub max_connections: u32,
    /// Minimum number of pooled connections.
    pub min_connections: u32,
}

impl DatabaseConfig {
    pub fn new(url: String) -> Self {
        Self {
            url,
            min_connections: 0,
            max_connections: 10,
        }
    }
}

/// Fully validated runtime settings for the gateway process.
#[derive(Debug, Clone)]
pub struct Config {
    /// Socket the HTTP server binds to.
    pub listen: SocketAddr,
    /// Optional Postgres backend. `None` runs the API surface in a degraded
    /// mode where DB-backed endpoints fail and `/readyz` reports the DB as
    /// down (health/liveness and DB-free endpoints still serve).
    pub db: Option<DatabaseConfig>,
    /// Optional Redis cache URL (e.g. `redis://127.0.0.1:6379`).
    pub redis_url: Option<String>,
    /// Base URL of the remote orchestra control-plane, if any.
    pub control_plane_base_url: Option<String>,
    /// Optional auth token for the orchestra control-plane client.
    pub control_plane_auth_token: Option<String>,
    /// Enables the transparent (in-memory) fallback DB used only when no real
    /// backend is configured. Controlled by `X3_GATEWAY_ALLOW_DB_FREE`.
    pub allow_db_free: bool,
}

/// Environment keys recognised by the gateway.
pub const ENV_DATABASE_URL: &str = "DATABASE_URL";
pub const ENV_GATEWAY_DATABASE_URL: &str = "X3_GATEWAY_DATABASE_URL";
pub const ENV_LISTEN: &str = "X3_GATEWAY_LISTEN";
pub const ENV_REDIS_URL: &str = "X3_GATEWAY_REDIS_URL";
pub const ENV_CONTROL_PLANE_URL: &str = "X3_GATEWAY_CONTROL_PLANE_URL";
pub const ENV_CONTROL_PLANE_TOKEN: &str = "X3_GATEWAY_CONTROL_PLANE_TOKEN";
pub const ENV_ALLOW_DB_FREE: &str = "X3_GATEWAY_ALLOW_DB_FREE";

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// Read configuration from the environment. Defaults are chosen so the
/// process is safe to start with zero configuration (binds localhost,
/// DB-free); provide the env keys documented on each field for production.
pub fn from_env() -> Result<Config> {
    let listen_raw = env(ENV_LISTEN).unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let listen: SocketAddr = listen_raw.parse().map_err(|e| {
        GatewayError::BadRequest(format!(
            "{ENV_LISTEN} must be a valid socket address, got `{listen_raw}`: {e}"
        ))
    })?;

    // Prefer the x3-specific key, fall back to the generic `DATABASE_URL`
    // used across the test suite.
    let database_url = env(ENV_GATEWAY_DATABASE_URL).or_else(|| env(ENV_DATABASE_URL));
    let db = database_url.map(DatabaseConfig::new);

    let allow_db_free = env(ENV_ALLOW_DB_FREE)
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false);

    if db.is_none() && !allow_db_free {
        tracing::warn!(
            "no {} set and {} not enabled — API will start in DB-free mode (readiness down)",
            ENV_GATEWAY_DATABASE_URL,
            ENV_ALLOW_DB_FREE
        );
    }

    Ok(Config {
        listen,
        db,
        redis_url: env(ENV_REDIS_URL),
        control_plane_base_url: env(ENV_CONTROL_PLANE_URL),
        control_plane_auth_token: env(ENV_CONTROL_PLANE_TOKEN),
        allow_db_free,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_loopback() {
        // Remove ambient env so the test is deterministic.
        std::env::remove_var(ENV_LISTEN);
        std::env::set_var(ENV_LISTEN, "");
        assert_eq!(
            from_env().expect("default config").listen,
            "127.0.0.1:8080".parse::<SocketAddr>().unwrap()
        );
    }

    #[test]
    fn rejects_malformed_listen_addr() {
        std::env::set_var(ENV_LISTEN, "not-an-addr");
        assert!(from_env().is_err());
        std::env::set_var(ENV_LISTEN, "");
    }
}
