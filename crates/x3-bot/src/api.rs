use crate::telemetry::{ATOMIC_SWAPS_FAILED, ATOMIC_SWAPS_SUCCESS, REGISTRY, TRADES_EXECUTED};
use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tracing::info;

/// Global uptime tracker - set when bot starts
static START_TIME: AtomicU64 = AtomicU64::new(0);

/// Set the start time for uptime tracking
pub fn set_start_time() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    START_TIME.store(now, Ordering::SeqCst);
}

/// Get current uptime in seconds
fn get_uptime_sec() -> u64 {
    let start = START_TIME.load(Ordering::SeqCst);
    if start == 0 {
        return 0;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    now.saturating_sub(start)
}

pub async fn start_metrics_server(port: u16) {
    // Initialize uptime tracking
    set_start_time();

    let app = Router::new()
        .route("/metrics.json", get(get_metrics_json))
        .route("/health", get(|| async { "OK" }));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("📈 Metrics server listening on http://{}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_metrics_json() -> Json<Value> {
    // Collect Prometheus metrics and format for the Dashboard index.html
    let success = ATOMIC_SWAPS_SUCCESS.get();
    let failed = ATOMIC_SWAPS_FAILED.get();
    let total_swaps = success + failed;
    let success_rate = if total_swaps > 0.0 {
        success / total_swaps
    } else {
        1.0
    };

    // Get actual uptime
    let uptime_sec = get_uptime_sec();

    Json(json!({
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "svm_tps": 1850000.0, // Mocked live TPS from P4
        "evm_tps": 850000.0,  // Mocked live TPS from P5
        "total_tx": TRADES_EXECUTED.get(),
        "chains_active": 2,
        "gpu_count": 3,
        "gpu_health": "perfect",
        "atomic_success_rate": success_rate,
        "atomic_rollbacks": failed as u64,
        "pending_swaps": 0,
        "uptime_sec": uptime_sec,
    }))
}
