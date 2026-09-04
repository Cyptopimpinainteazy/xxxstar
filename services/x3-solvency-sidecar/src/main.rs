//! X3 Solvency Sidecar — entry point.
//!
//! Starts three independent tokio tasks:
//!
//! 1. **Subscriber** — connects to the X3 node WebSocket and updates the
//!    shared dashboard on every new block.
//! 2. **Metrics server** — serves `GET /metrics` on `metrics_port` using the
//!    Prometheus text exposition format.
//! 3. **API server** — serves the REST API on `api_port`.
//!
//! `tokio::select!` monitors all three tasks; if any exits the event is logged
//! (the process itself is kept running because the remaining two tasks may
//! still be serving requests).

#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod api;
mod metrics;
mod state;
mod subscriber;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use state::new_dashboard;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

/// X3 Phase 4.5 solvency telemetry sidecar.
///
/// Connects to an X3/Substrate node WebSocket, tracks block-level solvency
/// data, exposes Prometheus metrics, and serves a REST API for operator tooling.
#[derive(Parser, Debug)]
#[command(name = "x3-solvency-sidecar", version, about)]
struct Cli {
    /// WebSocket URL of the X3/Substrate node.
    #[arg(long, default_value = "ws://127.0.0.1:9944")]
    node_ws: String,

    /// TCP port for the Prometheus `/metrics` scrape endpoint.
    #[arg(long, default_value_t = 9615)]
    metrics_port: u16,

    /// TCP port for the REST API (`/api/v1/...`).
    #[arg(long, default_value_t = 9616)]
    api_port: u16,

    /// Optional HTTP URL to POST alert JSON to when frozen transitions occur.
    #[arg(long)]
    alert_webhook: Option<String>,

    /// Subscriber poll interval in milliseconds (reserved for future use;
    /// the subscriber is subscription-driven, not polled).
    #[arg(long, default_value_t = 2000)]
    poll_interval_ms: u64,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise structured logging.  Respects RUST_LOG; falls back to INFO.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    let cli = Cli::parse();

    tracing::info!(
        node_ws = %cli.node_ws,
        metrics_port = cli.metrics_port,
        api_port = cli.api_port,
        alert_webhook = ?cli.alert_webhook,
        "X3 Solvency Sidecar starting"
    );

    // Shared state — created once, cloned cheaply (Arc).
    let dashboard = new_dashboard();

    // ------------------------------------------------------------------
    // Task 1: Subscriber
    // ------------------------------------------------------------------
    let sub_dashboard = dashboard.clone();
    let sub_node_ws = cli.node_ws.clone();
    let sub_webhook = cli.alert_webhook.clone();
    let sub_interval = cli.poll_interval_ms;

    let subscriber_handle = tokio::spawn(async move {
        subscriber::run_subscriber(sub_node_ws, sub_dashboard, sub_webhook, sub_interval).await;
    });

    // ------------------------------------------------------------------
    // Task 2: Prometheus metrics server
    // ------------------------------------------------------------------
    let metrics_addr = format!("0.0.0.0:{}", cli.metrics_port);
    let metrics_dashboard = dashboard.clone();

    let metrics_handle =
        tokio::spawn(async move { serve_metrics(metrics_addr, metrics_dashboard).await });

    // ------------------------------------------------------------------
    // Task 3: REST API server
    // ------------------------------------------------------------------
    let api_addr = format!("0.0.0.0:{}", cli.api_port);
    let api_dashboard = dashboard.clone();

    let api_handle = tokio::spawn(async move { serve_api(api_addr, api_dashboard).await });

    // ------------------------------------------------------------------
    // Supervision: log if any task exits unexpectedly.
    // ------------------------------------------------------------------
    tokio::select! {
        result = subscriber_handle => {
            match result {
                Ok(()) => tracing::error!("Subscriber task exited unexpectedly (returned)"),
                Err(e) => tracing::error!("Subscriber task panicked: {e}"),
            }
        }
        result = metrics_handle => {
            match result {
                Ok(Ok(())) => tracing::error!("Metrics server exited unexpectedly"),
                Ok(Err(e)) => tracing::error!("Metrics server error: {e}"),
                Err(e) => tracing::error!("Metrics server task panicked: {e}"),
            }
        }
        result = api_handle => {
            match result {
                Ok(Ok(())) => tracing::error!("API server exited unexpectedly"),
                Ok(Err(e)) => tracing::error!("API server error: {e}"),
                Err(e) => tracing::error!("API server task panicked: {e}"),
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Server helpers
// ---------------------------------------------------------------------------

/// Serve `GET /metrics` on `addr`.
///
/// On each request, re-reads the dashboard snapshot to ensure metrics are
/// up-to-date even if the subscriber has not ticked recently.
async fn serve_metrics(
    addr: String,
    dashboard: state::SharedDashboard,
) -> anyhow::Result<()> {
    use axum::{routing::get, Router};

    let router = Router::new()
        .route("/metrics", get(metrics_scrape_handler))
        .with_state(dashboard);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind metrics server on {addr}: {e}"))?;

    tracing::info!(addr = %addr, "Metrics server listening");

    axum::serve(listener, router)
        .await
        .map_err(|e| anyhow::anyhow!("Metrics server error: {e}"))
}

/// Handler for `GET /metrics`.
///
/// Pulls a fresh snapshot from the dashboard, pushes it into the Prometheus
/// gauges, then renders the text exposition.
async fn metrics_scrape_handler(
    axum::extract::State(dashboard): axum::extract::State<state::SharedDashboard>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let snap = match dashboard.read() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    metrics::update_metrics(&snap);

    let body = metrics::render_metrics();
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

/// Serve the REST API on `addr`.
async fn serve_api(addr: String, dashboard: state::SharedDashboard) -> anyhow::Result<()> {
    let router = api::router(dashboard);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind API server on {addr}: {e}"))?;

    tracing::info!(addr = %addr, "API server listening");

    axum::serve(listener, router)
        .await
        .map_err(|e| anyhow::anyhow!("API server error: {e}"))
}
