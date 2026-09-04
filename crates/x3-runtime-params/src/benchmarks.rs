//! Benchmarks Module - Performance testing and benchmarking

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Load scenario for benchmarking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadScenario {
    /// Light load
    Light,
    /// Normal load
    Normal,
    /// Heavy load
    Heavy,
    /// Stress test
    Stress,
}

/// Performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Transactions per second
    pub tps: f64,
    /// Average latency in ms
    pub avg_latency_ms: f64,
    /// P99 latency in ms
    pub p99_latency_ms: f64,
    /// Block time in ms
    pub block_time_ms: f64,
    /// Finality time in ms
    pub finality_time_ms: f64,
    /// CPU utilization %
    pub cpu_utilization: f64,
    /// Memory utilization MB
    pub memory_mb: f64,
    /// Network throughput Mbps
    pub network_mbps: f64,
    /// Error rate %
    pub error_rate: f64,
}

impl PerformanceMetrics {
    /// Create new metrics
    pub fn new(tps: f64, avg_latency: f64) -> Self {
        Self {
            tps,
            avg_latency_ms: avg_latency,
            p99_latency_ms: avg_latency * 2.0,
            block_time_ms: 400.0,
            finality_time_ms: 12800.0,
            cpu_utilization: 50.0,
            memory_mb: 512.0,
            network_mbps: 100.0,
            error_rate: 0.01,
        }
    }
}

/// Benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    /// Scenario name
    pub scenario: String,
    /// Load scenario
    pub load: LoadScenario,
    /// Duration in seconds
    pub duration_secs: u64,
    /// Performance metrics
    pub metrics: PerformanceMetrics,
    /// Parameters used
    pub params: HashMap<String, String>,
    /// Success flag
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl BenchmarkResults {
    /// Create success result
    pub fn success(scenario: &str, load: LoadScenario, duration: u64, metrics: PerformanceMetrics) -> Self {
        Self {
            scenario: scenario.to_string(),
            load,
            duration_secs: duration,
            metrics,
            params: HashMap::new(),
            success: true,
            error: None,
        }
    }

    /// Create failure result
    pub fn failure(scenario: &str, load: LoadScenario, error: String) -> Self {
        Self {
            scenario: scenario.to_string(),
            load,
            duration_secs: 0,
            metrics: PerformanceMetrics::default(),
            params: HashMap::new(),
            success: false,
            error: Some(error),
        }
    }
}

/// Benchmark runner trait
pub trait BenchmarkRunner {
    /// Run benchmark
    fn run(&self, scenario: &str, load: LoadScenario) -> BenchmarkResults;
}

/// Load test configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    /// Initial TPS
    pub initial_tps: u64,
    /// Target TPS
    pub target_tps: u64,
    /// Ramp up time in seconds
    pub ramp_up_secs: u64,
    /// Steady state duration in seconds
    pub steady_state_secs: u64,
    /// Ramp down time in seconds
    pub ramp_down_secs: u64,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            initial_tps: 1000,
            target_tps: 65000,
            ramp_up_secs: 60,
            steady_state_secs: 300,
            ramp_down_secs: 60,
        }
    }
}

/// Get load scenario parameters
pub fn get_load_params(load: LoadScenario) -> LoadTestConfig {
    match load {
        LoadScenario::Light => LoadTestConfig {
            initial_tps: 100,
            target_tps: 5000,
            ramp_up_secs: 30,
            steady_state_secs: 60,
            ramp_down_secs: 30,
        },
        LoadScenario::Normal => LoadTestConfig {
            initial_tps: 1000,
            target_tps: 25000,
            ramp_up_secs: 60,
            steady_state_secs: 180,
            ramp_down_secs: 60,
        },
        LoadScenario::Heavy => LoadTestConfig {
            initial_tps: 5000,
            target_tps: 50000,
            ramp_up_secs: 120,
            steady_state_secs: 300,
            ramp_down_secs: 120,
        },
        LoadScenario::Stress => LoadTestConfig {
            initial_tps: 10000,
            target_tps: 100000,
            ramp_up_secs: 180,
            steady_state_secs: 600,
            ramp_down_secs: 180,
        },
    }
}