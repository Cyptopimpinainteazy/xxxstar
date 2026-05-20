//! Metrics and telemetry for X3 GPU Validator Swarm

use crate::error::SwarmResult;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Swarm-level metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwarmMetrics {
    /// Total validators
    pub total_validators: u64,
    /// Active validators
    pub active_validators: u64,
    /// Quarantined validators
    pub quarantined_validators: u64,
    /// Total tasks processed
    pub total_tasks: u64,
    /// Successful tasks
    pub successful_tasks: u64,
    /// Failed tasks
    pub failed_tasks: u64,
    /// Total tasks with divergence
    pub divergent_tasks: u64,
    /// CPU fallback count
    pub cpu_fallbacks: u64,
    /// Average task latency (ms)
    pub avg_task_latency_ms: f64,
    /// Tasks per second
    pub tasks_per_second: f64,
}

/// Validator-level metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidatorMetrics {
    /// Validator ID
    pub validator_id: String,
    /// Tasks completed
    pub tasks_completed: u64,
    /// Tasks failed
    pub tasks_failed: u64,
    /// Divergences detected
    pub divergences: u64,
    /// Last task timestamp
    pub last_task_at: Option<i64>,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
    /// Current stake
    pub stake: u64,
}

/// Real-time sliding window metrics for TPS measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlidingWindowMetrics {
    /// Window size in seconds
    pub window_size_secs: u64,
    /// Tasks completed in current window
    pub tasks_in_window: u64,
    /// Current TPS (within window)
    pub current_tps: f64,
    /// P50 latency (ms)
    pub p50_latency_ms: f64,
    /// P95 latency (ms)
    pub p95_latency_ms: f64,
    /// P99 latency (ms)
    pub p99_latency_ms: f64,
    /// Peak TPS (since start)
    pub peak_tps: f64,
    /// Average TPS (lifetime)
    pub avg_tps: f64,
    /// Last measurement timestamp
    pub last_measured_at: i64,
}

impl Default for SlidingWindowMetrics {
    fn default() -> Self {
        Self {
            window_size_secs: 10,
            tasks_in_window: 0,
            current_tps: 0.0,
            p50_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            peak_tps: 0.0,
            avg_tps: 0.0,
            last_measured_at: 0,
        }
    }
}

/// Internal sliding window buffer for efficient TPS calculation
struct LatencyWindow {
    /// Ring buffer of (timestamp, latency_ms) tuples
    buffer: VecDeque<(Instant, u64)>,
    /// Maximum buffer size
    max_size: usize,
}

impl LatencyWindow {
    fn new(window_size_secs: u64) -> Self {
        // Keep enough samples for percentile calculation + headroom
        let max_size = (window_size_secs as usize * 1000).max(10000);
        Self {
            buffer: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    fn push(&mut self, timestamp: Instant, latency_ms: u64) {
        self.buffer.push_back((timestamp, latency_ms));
        if self.buffer.len() > self.max_size {
            self.buffer.pop_front();
        }
    }

    fn prune_old(&mut self, window_duration: Duration) {
        let cutoff = Instant::now() - window_duration;
        while let Some((ts, _)) = self.buffer.front() {
            if *ts < cutoff {
                self.buffer.pop_front();
            } else {
                break;
            }
        }
    }

    fn calculate_percentile(&self, percentile: f64) -> f64 {
        if self.buffer.is_empty() {
            return 0.0;
        }

        let mut latencies: Vec<u64> = self.buffer.iter().map(|(_, lat)| *lat).collect();
        latencies.sort_unstable();

        let index =
            ((latencies.len() as f64 * percentile / 100.0).ceil() as usize).saturating_sub(1);
        latencies.get(index).copied().unwrap_or(0) as f64
    }

    fn count_in_window(&self, window_duration: Duration) -> u64 {
        let cutoff = Instant::now() - window_duration;
        self.buffer.iter().filter(|(ts, _)| *ts >= cutoff).count() as u64
    }
}

/// Metrics collector
pub struct MetricsCollector {
    /// Swarm metrics
    swarm: RwLock<SwarmMetrics>,
    /// Validator metrics
    validators: RwLock<HashMap<String, ValidatorMetrics>>,
    /// Counters
    counters: RwLock<HashMap<String, AtomicU64>>,
    /// Gauge values
    gauges: RwLock<HashMap<String, f64>>,
    /// Latency tracking
    latencies: RwLock<Vec<u64>>,
    /// Sliding window metrics
    sliding_window: RwLock<LatencyWindow>,
    /// Sliding window duration
    window_duration: Duration,
    /// Peak TPS tracker
    peak_tps: RwLock<f64>,
    /// Last window measurement timestamp
    last_window_measurement: RwLock<Instant>,
    /// Start time
    start_time: Instant,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self::with_window(Duration::from_secs(10))
    }

    /// Create a metrics collector with custom sliding window duration
    pub fn with_window(window_duration: Duration) -> Self {
        let window_size_secs = window_duration.as_secs();
        Self {
            swarm: RwLock::new(SwarmMetrics::default()),
            validators: RwLock::new(HashMap::new()),
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            latencies: RwLock::new(Vec::new()),
            sliding_window: RwLock::new(LatencyWindow::new(window_size_secs)),
            window_duration,
            peak_tps: RwLock::new(0.0),
            last_window_measurement: RwLock::new(Instant::now()),
            start_time: Instant::now(),
        }
    }

    /// Increment a counter
    pub fn increment(&self, name: &str, value: u64) {
        let mut counters = self.counters.write();
        let counter = counters
            .entry(name.to_string())
            .or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(value, Ordering::SeqCst);
    }

    /// Set a gauge value
    pub fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write();
        gauges.insert(name.to_string(), value);
    }

    /// Get a counter value
    pub fn get_counter(&self, name: &str) -> u64 {
        let counters = self.counters.read();
        counters
            .get(name)
            .map(|c| c.load(Ordering::SeqCst))
            .unwrap_or(0)
    }

    /// Get a gauge value
    pub fn get_gauge(&self, name: &str) -> f64 {
        let gauges = self.gauges.read();
        *gauges.get(name).unwrap_or(&0.0)
    }

    /// Record task completion
    pub fn record_task(&self, validator_id: &str, latency_ms: u64, success: bool, divergent: bool) {
        let now = Instant::now();

        // Update swarm metrics
        {
            let mut swarm = self.swarm.write();
            swarm.total_tasks += 1;
            if success {
                swarm.successful_tasks += 1;
            } else {
                swarm.failed_tasks += 1;
            }
            if divergent {
                swarm.divergent_tasks += 1;
            }
        }

        // Update validator metrics
        {
            let mut validators = self.validators.write();
            let metrics = validators
                .entry(validator_id.to_string())
                .or_insert_with(|| ValidatorMetrics {
                    validator_id: validator_id.to_string(),
                    ..Default::default()
                });
            metrics.tasks_completed += 1;
            if !success {
                metrics.tasks_failed += 1;
            }
            if divergent {
                metrics.divergences += 1;
            }
            metrics.last_task_at = Some(chrono::Utc::now().timestamp());

            // Update average latency
            let total_latency = metrics.avg_latency_ms * (metrics.tasks_completed - 1) as f64;
            metrics.avg_latency_ms =
                (total_latency + latency_ms as f64) / metrics.tasks_completed as f64;
        }

        // Record latency for swarm average
        {
            let mut latencies = self.latencies.write();
            latencies.push(latency_ms);
            if latencies.len() > 10000 {
                latencies.drain(0..1000);
            }
        }

        // Record in sliding window
        {
            let mut window = self.sliding_window.write();
            window.push(now, latency_ms);
        }

        // Increment counters
        self.increment("tasks_total", 1);
        if success {
            self.increment("tasks_success", 1);
        } else {
            self.increment("tasks_failed", 1);
        }
        if divergent {
            self.increment("tasks_divergent", 1);
        }
    }

    /// Record CPU fallback
    pub fn record_cpu_fallback(&self) {
        self.increment("cpu_fallbacks", 1);
        let mut swarm = self.swarm.write();
        swarm.cpu_fallbacks += 1;
    }

    /// Get swarm metrics
    pub fn get_swarm_metrics(&self) -> SwarmMetrics {
        let mut swarm = self.swarm.write();

        // Calculate average latency
        let latencies = self.latencies.read();
        swarm.avg_task_latency_ms = if latencies.is_empty() {
            0.0
        } else {
            latencies.iter().sum::<u64>() as f64 / latencies.len() as f64
        };

        // Calculate TPS
        let elapsed = self.start_time.elapsed().as_secs_f64();
        swarm.tasks_per_second = if elapsed > 0.0 {
            swarm.total_tasks as f64 / elapsed
        } else {
            0.0
        };

        swarm.clone()
    }

    /// Get sliding window metrics (real-time TPS measurement)
    pub fn get_sliding_window_metrics(&self) -> SlidingWindowMetrics {
        let mut window = self.sliding_window.write();
        let mut last_measurement = self.last_window_measurement.write();

        // Prune old entries outside the window
        window.prune_old(self.window_duration);

        // Count tasks in current window
        let tasks_in_window = window.count_in_window(self.window_duration);

        // Calculate current TPS
        let window_secs = self.window_duration.as_secs_f64();
        let current_tps = tasks_in_window as f64 / window_secs;

        // Update peak TPS
        let mut peak_tps = self.peak_tps.write();
        if current_tps > *peak_tps {
            *peak_tps = current_tps;
        }

        // Calculate percentiles
        let p50 = window.calculate_percentile(50.0);
        let p95 = window.calculate_percentile(95.0);
        let p99 = window.calculate_percentile(99.0);

        // Calculate lifetime average TPS
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let swarm = self.swarm.read();
        let avg_tps = if elapsed > 0.0 {
            swarm.total_tasks as f64 / elapsed
        } else {
            0.0
        };

        *last_measurement = Instant::now();

        SlidingWindowMetrics {
            window_size_secs: self.window_duration.as_secs(),
            tasks_in_window,
            current_tps,
            p50_latency_ms: p50,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
            peak_tps: *peak_tps,
            avg_tps,
            last_measured_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Format sliding window metrics as a readable string
    pub fn format_sliding_window_metrics(&self) -> String {
        let metrics = self.get_sliding_window_metrics();
        format!(
            "TPS: {:.1} (peak: {:.1}, avg: {:.1}) | Window: {}s | Latency p50/p95/p99: {:.2}/{:.2}/{:.2}ms | Tasks: {}",
            metrics.current_tps,
            metrics.peak_tps,
            metrics.avg_tps,
            metrics.window_size_secs,
            metrics.p50_latency_ms,
            metrics.p95_latency_ms,
            metrics.p99_latency_ms,
            metrics.tasks_in_window,
        )
    }

    /// Get validator metrics
    pub fn get_validator_metrics(&self, validator_id: &str) -> Option<ValidatorMetrics> {
        let validators = self.validators.read();
        validators.get(validator_id).cloned()
    }

    /// Get all validator metrics
    pub fn get_all_validator_metrics(&self) -> Vec<ValidatorMetrics> {
        let validators = self.validators.read();
        validators.values().cloned().collect()
    }

    /// Update validator count
    pub fn update_validator_count(&self, total: u64, active: u64, quarantined: u64) {
        let mut swarm = self.swarm.write();
        swarm.total_validators = total;
        swarm.active_validators = active;
        swarm.quarantined_validators = quarantined;
        self.set_gauge("validators_total", total as f64);
        self.set_gauge("validators_active", active as f64);
        self.set_gauge("validators_quarantined", quarantined as f64);
    }

    /// Reset metrics
    pub fn reset(&self) {
        *self.swarm.write() = SwarmMetrics::default();
        self.validators.write().clear();
        self.counters.write().clear();
        self.gauges.write().clear();
        self.latencies.write().clear();
        *self.sliding_window.write() = LatencyWindow::new(self.window_duration.as_secs());
        *self.peak_tps.write() = 0.0;
        *self.last_window_measurement.write() = Instant::now();
    }

    /// Export all metrics as JSON
    pub fn export_json(&self) -> SwarmResult<String> {
        #[derive(Serialize)]
        struct ExportData {
            swarm: SwarmMetrics,
            sliding_window: SlidingWindowMetrics,
            validators: Vec<ValidatorMetrics>,
            counters: HashMap<String, u64>,
            gauges: HashMap<String, f64>,
            uptime_seconds: f64,
        }

        let data = ExportData {
            swarm: self.get_swarm_metrics(),
            sliding_window: self.get_sliding_window_metrics(),
            validators: self.get_all_validator_metrics(),
            counters: self
                .counters
                .read()
                .iter()
                .map(|(k, v)| (k.clone(), v.load(Ordering::SeqCst)))
                .collect(),
            gauges: self.gauges.read().clone(),
            uptime_seconds: self.start_time.elapsed().as_secs_f64(),
        };

        serde_json::to_string_pretty(&data).map_err(|e| e.into())
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Service name
    pub service: String,
    /// Health status
    pub status: HealthStatus,
    /// Message
    pub message: Option<String>,
    /// Timestamp
    pub timestamp: i64,
    /// Details
    pub details: HashMap<String, String>,
}

/// Health status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Healthy
    Healthy,
    /// Degraded
    Degraded,
    /// Unhealthy
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Validator health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorHealth {
    pub validator_id: String,
    pub status: HealthStatus,
    pub last_heartbeat: Option<i64>,
    pub tasks_recent: u64,
    pub error_rate: f64,
    pub divergence_rate: f64,
}
