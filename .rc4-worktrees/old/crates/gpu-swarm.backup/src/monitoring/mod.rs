// crates/gpu-swarm/src/monitoring/mod.rs
// GPU Swarm Monitoring Module - Prometheus metrics, OpenTelemetry integration, logging

pub mod logging;
pub mod metrics;
pub mod tracing;

pub use logging::setup_logging;
pub use metrics::MetricsCollector;
pub use tracing::setup_tracing;

use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;

/// Global metrics registry
pub static METRICS_REGISTRY: once_cell::sync::Lazy<prometheus::Registry> =
    once_cell::sync::Lazy::new(|| prometheus::Registry::new());

/// Initialize all monitoring subsystems
pub async fn init_monitoring() -> Result<Arc<MetricsCollector>, Box<dyn std::error::Error>> {
    // Setup tracing (OpenTelemetry/Jaeger)
    setup_tracing()?;

    // Setup structured logging
    setup_logging()?;

    // Initialize metrics collector
    let collector = MetricsCollector::new_with_registry(METRICS_REGISTRY.clone())?;

    ::tracing::info!("✅ Monitoring subsystems initialized");

    Ok(Arc::new(collector))
}

/// Expose metrics endpoint for Prometheus
pub async fn metrics_handler() -> Result<String, Box<dyn std::error::Error>> {
    let encoder = TextEncoder::new();
    let metric_families = METRICS_REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}

// --- Lightweight monitoring helper types used by tests ---
/// Simple trace/span context for tests
#[derive(Clone, Debug)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
}

impl TraceContext {
    pub fn new() -> Self {
        TraceContext {
            trace_id: format!("t-{}", uuid::Uuid::new_v4()),
            span_id: format!("s-{}", uuid::Uuid::new_v4()),
            parent_span_id: None,
        }
    }

    pub fn child_span(&self) -> Self {
        TraceContext {
            trace_id: self.trace_id.clone(),
            span_id: format!("s-{}", uuid::Uuid::new_v4()),
            parent_span_id: Some(self.span_id.clone()),
        }
    }
}

/// Health check response structure used in tests
#[derive(Clone, Debug)]
pub struct HealthCheckResponse {
    pub status: String,
    pub uptime_seconds: u64,
    pub connected_peers: u32,
    pub task_queue_size: u32,
    pub gpu_devices_available: u32,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub network_sync_status: String,
    pub last_block_number: u64,
    pub is_synced: bool,
    pub timestamp: u64,
}

impl HealthCheckResponse {
    pub fn is_synced(&self) -> bool {
        self.is_synced
    }
}

/// Simple alert rule used by monitoring tests
#[derive(Clone, Debug)]
pub struct AlertRule {
    pub name: String,
    pub metric: String,
    pub condition: String,
    pub threshold: f64,
    pub duration_seconds: u64,
    pub severity: String,
    pub description: String,
}

impl AlertRule {
    pub fn should_trigger(&self, value: f64) -> bool {
        match self.condition.as_str() {
            "gt" => value > self.threshold,
            "lt" => value < self.threshold,
            "ge" => value >= self.threshold,
            "le" => value <= self.threshold,
            _ => false,
        }
    }
}
