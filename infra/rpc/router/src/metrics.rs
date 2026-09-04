//! Prometheus metrics for the X3 RPC Router.
//!
//! Exposes: request count, error count, latency histogram, upstream freshness,
//! block/slot lag, wrong-chain count, quorum mismatch count, healthy upstream count,
//! rate-limited count, blocked method count, WebSocket connection count.

use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec, register_int_counter, register_int_counter_vec,
    register_int_gauge, register_int_gauge_vec, Encoder, HistogramOpts,
    HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, TextEncoder,
};
use std::sync::Arc;

lazy_static! {
    // ── Request counters ──────────────────────────────────────────────────────
    static ref REQUESTS_TOTAL: IntCounter = register_int_counter!(
        "rpc_requests_total",
        "Total number of RPC requests received"
    )
    .unwrap();

    static ref REQUESTS_TOTAL_PER_CHAIN: IntCounterVec = register_int_counter_vec!(
        "rpc_requests_per_chain_total",
        "Total requests per chain",
        &["chain"]
    )
    .unwrap();

    static ref ERRORS_TOTAL: IntCounter = register_int_counter!(
        "rpc_errors_total",
        "Total number of RPC errors"
    )
    .unwrap();

    static ref ERRORS_TOTAL_PER_CHAIN: IntCounterVec = register_int_counter_vec!(
        "rpc_errors_per_chain_total",
        "Total errors per chain",
        &["chain"]
    )
    .unwrap();

    // ── Rate limiting ─────────────────────────────────────────────────────────
    static ref AUTH_FAILURES: IntCounter = register_int_counter!(
        "rpc_auth_failures_total",
        "Total auth failures"
    )
    .unwrap();

    static ref RATE_LIMITED: IntCounter = register_int_counter!(
        "rpc_rate_limited_total",
        "Total rate-limited requests"
    )
    .unwrap();

    static ref BLOCKED_METHODS: IntCounter = register_int_counter!(
        "rpc_blocked_methods_total",
        "Total blocked method calls"
    )
    .unwrap();

    // ── Latency ────────────────────────────────────────────────────────────────
    static ref REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        HistogramOpts::new(
            "rpc_request_duration_seconds",
            "RPC request duration in seconds"
        ),
        &["chain", "method"]
    )
    .unwrap();

    // ── Upstream health ────────────────────────────────────────────────────────
    static ref UPSTREAM_SCORE: IntGaugeVec = register_int_gauge_vec!(
        "rpc_upstream_score",
        "Health score (0-100) per upstream",
        &["upstream", "chain"]
    )
    .unwrap();

    static ref UPSTREAM_LATENCY_MS: IntGaugeVec = register_int_gauge_vec!(
        "rpc_upstream_latency_ms",
        "Latency in ms per upstream",
        &["upstream", "chain"]
    )
    .unwrap();

    static ref UPSTREAM_BLOCK_LAG: IntGaugeVec = register_int_gauge_vec!(
        "rpc_upstream_block_lag",
        "Block lag per upstream",
        &["upstream", "chain"]
    )
    .unwrap();

    static ref UPSTREAM_SLOT_LAG: IntGaugeVec = register_int_gauge_vec!(
        "rpc_upstream_slot_lag",
        "Slot lag per upstream",
        &["upstream", "chain"]
    )
    .unwrap();

    static ref UPSTREAM_ERROR_RATE_BPS: IntGaugeVec = register_int_gauge_vec!(
        "rpc_upstream_error_rate_bps",
        "Error rate in basis points per upstream",
        &["upstream", "chain"]
    )
    .unwrap();

    static ref HEALTHY_UPSTREAMS: IntGaugeVec = register_int_gauge_vec!(
        "rpc_healthy_upstreams",
        "Number of healthy upstreams per chain",
        &["chain"]
    )
    .unwrap();

    static ref WRONG_CHAIN_TOTAL: IntCounterVec = register_int_counter_vec!(
        "rpc_wrong_chain_total",
        "Total wrong-chain detections",
        &["upstream", "chain"]
    )
    .unwrap();

    static ref QUORUM_MISMATCH_TOTAL: IntCounterVec = register_int_counter_vec!(
        "rpc_quorum_mismatch_total",
        "Total quorum mismatches",
        &["chain", "method"]
    )
    .unwrap();

    static ref FAILOVER_TOTAL: IntCounterVec = register_int_counter_vec!(
        "rpc_failover_total",
        "Total failover events",
        &["chain", "from_upstream", "to_upstream"]
    )
    .unwrap();

    // ── WebSocket ──────────────────────────────────────────────────────────────
    static ref WS_CONNECTIONS: IntGauge = register_int_gauge!(
        "rpc_ws_connections",
        "Active WebSocket connections"
    )
    .unwrap();

    static ref WS_MESSAGES_TOTAL: IntCounter = register_int_counter!(
        "rpc_ws_messages_total",
        "Total WebSocket messages"
    )
    .unwrap();

    // ── Transaction broadcast ──────────────────────────────────────────────────
    static ref TX_BROADCAST_TOTAL: IntCounterVec = register_int_counter_vec!(
        "rpc_tx_broadcast_total",
        "Total transaction broadcast attempts",
        &["chain", "result"]
    )
    .unwrap();
}

pub type ArcMetrics = Arc<MetricsRecorder>;

#[derive(Clone)]
pub struct MetricsRecorder;

impl MetricsRecorder {
    pub fn new() -> Self {
        Self
    }

    // ── Request tracking ─────────────────────────────────────────────

    pub fn increment_requests(&self) {
        REQUESTS_TOTAL.inc();
    }

    pub fn increment_errors(&self) {
        ERRORS_TOTAL.inc();
    }

    pub fn increment_auth_failures(&self) {
        AUTH_FAILURES.inc();
    }

    pub fn increment_rate_limited(&self) {
        RATE_LIMITED.inc();
    }

    pub fn increment_blocked_method(&self) {
        BLOCKED_METHODS.inc();
    }

    pub fn record_request_duration(&self, chain: &str, method: &str, duration_secs: f64) {
        REQUEST_DURATION
            .with_label_values(&[chain, method])
            .observe(duration_secs);
    }

    // ── Upstream metrics ─────────────────────────────────────────────

    pub fn update_upstream_metrics(
        &self,
        upstream_id: &str,
        chain: &str,
        score: u8,
        latency_ms: u64,
        block_lag: u32,
        slot_lag: u32,
        error_rate_bps: u32,
    ) {
        UPSTREAM_SCORE
            .with_label_values(&[upstream_id, chain])
            .set(score as i64);
        UPSTREAM_LATENCY_MS
            .with_label_values(&[upstream_id, chain])
            .set(latency_ms as i64);
        UPSTREAM_BLOCK_LAG
            .with_label_values(&[upstream_id, chain])
            .set(block_lag as i64);
        UPSTREAM_SLOT_LAG
            .with_label_values(&[upstream_id, chain])
            .set(slot_lag as i64);
        UPSTREAM_ERROR_RATE_BPS
            .with_label_values(&[upstream_id, chain])
            .set(error_rate_bps as i64);
    }

    pub fn set_healthy_upstreams(&self, chain: &str, count: u32) {
        HEALTHY_UPSTREAMS
            .with_label_values(&[chain])
            .set(count as i64);
    }

    pub fn increment_wrong_chain(&self, upstream: &str, chain: &str) {
        WRONG_CHAIN_TOTAL
            .with_label_values(&[upstream, chain])
            .inc();
    }

    pub fn increment_quorum_mismatch(&self, chain: &str, method: &str) {
        QUORUM_MISMATCH_TOTAL
            .with_label_values(&[chain, method])
            .inc();
    }

    pub fn increment_failover(&self, chain: &str, from: &str, to: &str) {
        FAILOVER_TOTAL
            .with_label_values(&[chain, from, to])
            .inc();
    }

    // ── WebSocket ────────────────────────────────────────────────────

    pub fn increment_ws_connections(&self) {
        WS_CONNECTIONS.inc();
    }

    pub fn decrement_ws_connections(&self) {
        WS_CONNECTIONS.dec();
    }

    pub fn increment_ws_messages(&self) {
        WS_MESSAGES_TOTAL.inc();
    }

    // ── Transaction broadcast ─────────────────────────────────────────

    pub fn record_tx_broadcast(&self, chain: &str, result: &str) {
        TX_BROADCAST_TOTAL
            .with_label_values(&[chain, result])
            .inc();
    }

    // ── Render ────────────────────────────────────────────────────────

    /// Render all metrics in Prometheus text format.
    pub fn render(&self) -> String {
        let metric_families = prometheus::gather();
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}