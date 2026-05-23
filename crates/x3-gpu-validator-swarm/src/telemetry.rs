//! Telemetry for X3 GPU Validator Swarm

use crate::error::SwarmResult;
use crate::metrics::{HealthStatus, SwarmMetrics};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Telemetry configuration
pub use crate::config::TelemetryConfig;

/// Telemetry event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Event type
    pub event_type: String,
    /// Validator ID
    pub validator_id: String,
    /// Timestamp
    pub timestamp: i64,
    /// Data
    pub data: HashMap<String, serde_json::Value>,
}

/// Telemetry sink
pub struct TelemetrySink {
    /// Config
    config: TelemetryConfig,
    /// Pending events
    events: RwLock<Vec<TelemetryEvent>>,
    /// Last send time
    last_send: RwLock<Option<Instant>>,
    /// Session ID
    session_id: String,
}

impl TelemetrySink {
    /// Create a new telemetry sink
    pub fn new(config: TelemetryConfig, validator_id: String) -> Self {
        Self {
            config,
            events: RwLock::new(Vec::new()),
            last_send: RwLock::new(None),
            session_id: format!("{}_{}", validator_id, chrono::Utc::now().timestamp()),
        }
    }

    /// Record an event
    pub fn record(
        &self,
        event_type: String,
        validator_id: String,
        data: HashMap<String, serde_json::Value>,
    ) {
        if !self.config.enabled {
            return;
        }

        let event = TelemetryEvent {
            event_type,
            validator_id,
            timestamp: chrono::Utc::now().timestamp(),
            data,
        };

        self.events.write().push(event);
    }

    /// Record metrics
    pub fn record_metrics(&self, validator_id: String, metrics: &SwarmMetrics) {
        let mut data = HashMap::new();
        data.insert(
            "total_tasks".to_string(),
            serde_json::json!(metrics.total_tasks),
        );
        data.insert(
            "successful_tasks".to_string(),
            serde_json::json!(metrics.successful_tasks),
        );
        data.insert(
            "failed_tasks".to_string(),
            serde_json::json!(metrics.failed_tasks),
        );
        data.insert(
            "divergent_tasks".to_string(),
            serde_json::json!(metrics.divergent_tasks),
        );
        data.insert(
            "cpu_fallbacks".to_string(),
            serde_json::json!(metrics.cpu_fallbacks),
        );
        data.insert(
            "avg_task_latency_ms".to_string(),
            serde_json::json!(metrics.avg_task_latency_ms),
        );
        data.insert(
            "tasks_per_second".to_string(),
            serde_json::json!(metrics.tasks_per_second),
        );

        self.record("metrics".to_string(), validator_id, data);
    }

    /// Record health status
    pub fn record_health(&self, validator_id: String, status: HealthStatus) {
        let mut data = HashMap::new();
        data.insert("status".to_string(), serde_json::json!(status.to_string()));

        self.record("health".to_string(), validator_id, data);
    }

    /// Record task completion
    pub fn record_task(&self, validator_id: String, task_id: &str, latency_ms: u64, success: bool) {
        let mut data = HashMap::new();
        data.insert("task_id".to_string(), serde_json::json!(task_id));
        data.insert("latency_ms".to_string(), serde_json::json!(latency_ms));
        data.insert("success".to_string(), serde_json::json!(success));

        self.record("task_completed".to_string(), validator_id, data);
    }

    /// Record divergence
    pub fn record_divergence(&self, validator_id: String, task_id: &str, details: &str) {
        let mut data = HashMap::new();
        data.insert("task_id".to_string(), serde_json::json!(task_id));
        data.insert("details".to_string(), serde_json::json!(details));

        self.record("divergence".to_string(), validator_id, data);
    }

    /// Get pending events
    pub fn get_pending_events(&self) -> Vec<TelemetryEvent> {
        self.events.read().clone()
    }

    /// Clear pending events
    pub fn clear_events(&self) {
        self.events.write().clear();
        *self.last_send.write() = Some(Instant::now());
    }

    /// Check if should send
    pub fn should_send(&self) -> bool {
        if !self.config.enabled {
            return false;
        }

        if let Some(last) = *self.last_send.read() {
            last.elapsed() > Duration::from_secs(self.config.interval_secs)
        } else {
            true
        }
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Export events as JSON
    pub fn export_json(&self) -> SwarmResult<String> {
        serde_json::to_string_pretty(&*self.events.read()).map_err(|e| e.into())
    }
}
