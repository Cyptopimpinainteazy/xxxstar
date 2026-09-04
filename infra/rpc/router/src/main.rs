//! X3 RPC Router — Protocol-aware multi-chain RPC gateway.
//!
//! Architecture:
//!   HAProxy edge → x3-rpc-router → upstreams (local nodes, paid providers, fallbacks)
//!
//! The router understands chain semantics (EVM, Solana, Bitcoin, X3 Substrate)
//! and routes each request to the best upstream based on health scores,
//! method policy, freshness requirements, and quorum rules.

mod auth;
mod bitcoin;
mod cache;
mod config;
mod evm;
mod health;
mod metrics;
mod quorum;
mod rate_limit;
mod scoring;
mod solana;
mod tx_broadcast;
mod x3;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn, Level};

use crate::auth::AuthValidator;
use crate::config::{AppConfig, ArcConfig};
use crate::health::HealthState;
use crate::metrics::{MetricsRecorder, ArcMetrics};
use crate::rate_limit::RateLimiter;
use crate::scoring::UpstreamPool;
use crate::tx_broadcast::TxBroadcastGuard;

/// Shared application state injected into every route handler.
#[derive(Clone)]
pub struct AppState {
    pub config: ArcConfig,
    pub pool: Arc<UpstreamPool>,
    pub metrics: ArcMetrics,
    pub health: Arc<HealthState>,
    pub rate_limiter: Arc<RateLimiter>,
    pub auth: Arc<AuthValidator>,
    pub tx_guard: Arc<TxBroadcastGuard>,
}

// ── HTTP Handlers ──────────────────────────────────────────────────────────

/// JSON-RPC dispatch — handles eth_*, solana_*, btc_*, x3_* methods.
async fn rpc_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    state.metrics.increment_requests();

    // Parse JSON-RPC envelope to extract method
    let request_id: Option<serde_json::Value> = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("id").cloned());

    let method = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("method").and_then(|m| m.as_str().map(String::from)))
        .unwrap_or_default();

    // ── Auth check ──────────────────────────────────────────────
    if !state.auth.validate_request(&headers, &method) {
        state.metrics.increment_auth_failures();
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "error": {"code": -32001, "message": "Unauthorized"},
                "id": request_id
            })),
        );
    }

    // ── Rate limit check ────────────────────────────────────────
    if !state.rate_limiter.check(addr.ip(), &method) {
        state.metrics.increment_rate_limited();
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "error": {"code": -32005, "message": "Rate limited"},
                "id": request_id
            })),
        );
    }

    // ── Method allow-list check ─────────────────────────────────
    match state.config.methods.classify(&method) {
        crate::config::MethodClass::Blocked => {
            state.metrics.increment_blocked_method();
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {"code": -32004, "message": "Method blocked"},
                    "id": request_id
                })),
            );
        }
        crate::config::MethodClass::AdminAuthenticated => {
            // only allowed with admin-level auth, verified above in auth check
        }
        _ => {} // safe_public, tx, archive — proceed to routing
    }

    // ── Route to the appropriate chain handler ──────────────────
    let chain = state.config.resolve_chain(&method);
    let result = match chain {
        config::ChainKind::Evm => evm::handle_request(&state, &method, &body).await,
        config::ChainKind::Solana => solana::handle_request(&state, &method, &body).await,
        config::ChainKind::Bitcoin => bitcoin::handle_request(&state, &method, &body).await,
        config::ChainKind::X3 => x3::handle_request(&state, &method, &body).await,
    };

    match result {
        Ok(response_body) => (
            StatusCode::OK,
            Json(serde_json::from_str::<serde_json::Value>(&response_body).unwrap_or(
                serde_json::json!({"jsonrpc": "2.0", "result": response_body, "id": request_id}),
            )),
        ),
        Err(e) => {
            state.metrics.increment_errors();
            error!(method = %method, error = %e, "RPC request failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {"code": -32603, "message": format!("Internal error: {}", e)},
                    "id": request_id
                })),
            )
        }
    }
}

/// Health check endpoint — returns 200 if the router and all critical upstreams are healthy.
async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let overall = state.health.overall_status();
    let status_code = if overall.healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(serde_json::json!({
            "status": if overall.healthy { "ok" } else { "degraded" },
            "healthy_upstreams": overall.healthy_count,
            "total_upstreams": overall.total_count,
            "checks": overall.checks
        })),
    )
}

/// Prometheus metrics endpoint.
async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.metrics.render()
}

/// WebSocket upgrade handler for Solana and X3 subscriptions.
async fn ws_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ws: axum::extract::WebSocketUpgrade,
) -> impl IntoResponse {
    info!(client = %addr, "WebSocket upgrade requested");
    ws.on_upgrade(move |socket| handle_ws(socket, state, addr))
}

async fn handle_ws(
    mut socket: axum::extract::ws::WebSocket,
    state: Arc<AppState>,
    addr: SocketAddr,
) {
    use axum::extract::ws::Message;
    use futures::SinkExt;
    use futures::StreamExt;

    info!(client = %addr, "WebSocket connected");
    state.metrics.increment_ws_connections();

    // Determine chain from first subscription message
    let mut chain = None;
    let mut upstream_ws = None;

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Parse method, route to correct upstream
                        let method = serde_json::from_str::<serde_json::Value>(&text)
                            .ok()
                            .and_then(|v| v.get("method").and_then(|m| m.as_str().map(String::from)))
                            .unwrap_or_default();

                        if chain.is_none() {
                            chain = Some(state.config.resolve_chain(&method));
                        }

                        let result = match chain.unwrap_or(config::ChainKind::Solana) {
                            config::ChainKind::Solana => {
                                solana::ws_forward(&state, &method, &text).await
                            }
                            config::ChainKind::X3 => {
                                x3::ws_forward(&state, &method, &text).await
                            }
                            _ => Err(anyhow::anyhow!("WebSocket not supported for this chain")),
                        };

                        match result {
                            Ok(response) => {
                                let _ = socket.send(Message::Text(response.into())).await;
                            }
                            Err(e) => {
                                error!(error = %e, "WS forward failed");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!(client = %addr, "WebSocket closed");
                        break;
                    }
                    Some(Ok(_)) => {} // ping/pong/binary — handled by tungstenite
                    Some(Err(e)) => {
                        error!(client = %addr, error = %e, "WebSocket error");
                        break;
                    }
                    None => break,
                }
            }
        }
    }

    state.metrics.decrement_ws_connections();
    info!(client = %addr, "WebSocket disconnected");
}

// ── Main Entry Point ───────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "x3_rpc_router=info".into()),
        )
        .json()
        .init();

    info!("X3 RPC Router starting...");

    // Load configuration
    let config = Arc::new(AppConfig::load()?);
    info!("Configuration loaded: {} upstreams across {} chains",
        config.providers.len(),
        config.chains.len(),
    );

    // Initialize components
    let metrics = Arc::new(MetricsRecorder::new());
    let health = Arc::new(HealthState::new());
    let pool = Arc::new(UpstreamPool::new(config.clone(), metrics.clone(), health.clone()));
    let rate_limiter = Arc::new(RateLimiter::new(config.clone()));
    let auth = Arc::new(AuthValidator::new(config.clone()));
    let tx_guard = Arc::new(TxBroadcastGuard::new(config.clone()));

    let state = Arc::new(AppState {
        config,
        pool,
        metrics,
        health,
        rate_limiter,
        auth,
        tx_guard,
    });

    // Start background tasks
    {
        let s = state.clone();
        tokio::spawn(async move {
            loop {
                s.pool.score_all_upstreams().await;
                s.health.update_from_pool(&s.pool);
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            }
        });
    }

    // Build router
    let app = Router::new()
        .route("/", post(rpc_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/ws", get(ws_handler))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10 MB
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Bind HTTP server
    let http_addr: SocketAddr = "0.0.0.0:18545".parse()?;
    info!("HTTP RPC router listening on {}", http_addr);

    let listener = tokio::net::TcpListener::bind(http_addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}