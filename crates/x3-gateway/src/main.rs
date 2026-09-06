//! x3-gateway binary entry point.
//!
//! Loads validated process configuration, connects the optional Postgres
//! backend (running migrations when a real DSN is configured), opens the
//! optional Redis cache and orchestra control-plane client, builds the
//! REST+GraphQL axum router and serves it with graceful shutdown.
//!
//! Production wiring: provide `X3_GATEWAY_DATABASE_URL` (or `DATABASE_URL`)
//! so the gateway boots fully connected (schema/migrations applied on
//! connect). Without a database DSN the server still starts in a documented
//! degraded mode — liveness and DB-free endpoints serve, `/readyz` and the
//! GraphQL `dbReachable` field report the backend is down, and DB-backed
//! endpoints return 500 — so operators can run the API edge while the
//! indexer/DB is being provisioned. See `x3_gateway::config` for the full
//! list of environment keys.

use std::sync::Arc;
use x3_gateway::cache::RedisCache;
use x3_gateway::config::{self, DatabaseConfig};
use x3_gateway::db::Database;
use x3_gateway::error::Result;
use x3_gateway::{graphql, rest};
use x3_orchestra_control_plane::ControlPlaneClient;

/// Default DSN used only to construct a lazily-connected pool in degraded
/// (no-DB) mode; points at nothing reachable so `healthy()` reports down.
const DEGRADED_DB_URL: &str = "postgres://user:pass@127.0.0.1:5432/x3_gateway";

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let cfg = config::from_env()?;

    tracing::info!(listen = %cfg.listen, "x3-gateway starting");

    // 1. Database: real connect+migrate when configured, otherwise a lazy
    //    pool so the API edge can still serve its DB-free surface.
    let db = match &cfg.db {
        Some(db_cfg) => {
            tracing::info!("connecting to database and applying migrations");
            Database::connect(db_cfg).await?
        }
        None => {
            tracing::warn!(
                "no database configured; running in degraded DB-free mode \
                 (readiness down, DB-backed endpoints return 500)"
            );
            Database::connect_lazy(&DatabaseConfig::new(DEGRADED_DB_URL.to_string()))?
        }
    };

    // 2. Optional Redis response cache.
    let redis = match &cfg.redis_url {
        Some(url) => match RedisCache::connect(url).await {
            Ok(cache) => {
                tracing::info!("redis cache enabled");
                Some(cache)
            }
            Err(err) => {
                tracing::warn!(error = %err, "redis unavailable; running without cache");
                None
            }
        },
        None => None,
    };

    // 3. Optional orchestra control-plane client.
    let orchestra_client = cfg.control_plane_base_url.as_ref().map(|url| {
        Arc::new(ControlPlaneClient::new(
            url.clone(),
            cfg.control_plane_auth_token.clone(),
        ))
    });

    // 4. Build schema + router from the same real modules the tests drive.
    let schema = graphql::create_schema(db.clone(), orchestra_client.clone());
    let app = rest::create_router(db, schema, orchestra_client, redis);

    // 5. Bind and serve with graceful shutdown on Ctrl-C / SIGTERM.
    let listener = tokio::net::TcpListener::bind(cfg.listen)
        .await
        .map_err(|e| x3_gateway::error::GatewayError::Internal(format!(
            "failed to bind {}: {e}",
            cfg.listen
        )))?;
    let local = listener.local_addr()?;
    tracing::info!(addr = %local, "x3-gateway listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| {
            x3_gateway::error::GatewayError::Internal(format!("http server error: {e}"))
        })?;

    tracing::info!("x3-gateway shut down cleanly");
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,x3_gateway=debug"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
