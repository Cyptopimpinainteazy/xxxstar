//! Health monitoring for X3 GPU Validator Swarm

use crate::metrics::{HealthCheck, HealthStatus};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Check interval
    pub check_interval: Duration,
    /// Timeout for checks
    pub timeout: Duration,
    /// Max consecutive failures before unhealthy
    pub max_consecutive_failures: u32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            max_consecutive_failures: 3,
        }
    }
}

/// Health monitor
pub struct HealthMonitor {
    /// Registered health checks
    checks: RwLock<HashMap<String, Box<dyn Fn() -> HealthCheck + Send + Sync>>>,
    /// Last check results
    last_results: RwLock<HashMap<String, HealthCheck>>,
    /// Config
    _config: HealthConfig,
    /// Start time
    start_time: Instant,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(config: HealthConfig) -> Self {
        Self {
            checks: RwLock::new(HashMap::new()),
            last_results: RwLock::new(HashMap::new()),
            _config: config,
            start_time: Instant::now(),
        }
    }

    /// Register a health check
    pub fn register<F>(&self, name: String, check_fn: F)
    where
        F: Fn() -> HealthCheck + Send + Sync + 'static,
    {
        self.checks.write().insert(name, Box::new(check_fn));
    }

    /// Run all health checks
    pub fn check_all(&self) -> Vec<HealthCheck> {
        let checks = self.checks.read();
        let mut results = Vec::new();

        for (name, check_fn) in checks.iter() {
            let result = check_fn();
            self.last_results
                .write()
                .insert(name.clone(), result.clone());
            results.push(result);
        }

        results
    }

    /// Get overall health status
    pub fn get_overall_status(&self) -> HealthStatus {
        let results = self.last_results.read();

        if results.is_empty() {
            return HealthStatus::Healthy;
        }

        let mut has_unhealthy = false;
        let mut has_degraded = false;

        for (_, check) in results.iter() {
            match check.status {
                HealthStatus::Unhealthy => {
                    has_unhealthy = true;
                    break;
                }
                HealthStatus::Degraded => {
                    has_degraded = true;
                }
                _ => {}
            }
        }

        if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Get a specific check result
    pub fn get_check(&self, name: &str) -> Option<HealthCheck> {
        self.last_results.read().get(name).cloned()
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get all checks
    pub fn get_all_checks(&self) -> HashMap<String, HealthCheck> {
        self.last_results.read().clone()
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new(HealthConfig::default())
    }
}

/// Validator health tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorHealthTracker {
    /// Validator ID
    pub validator_id: String,
    /// Last heartbeat
    pub last_heartbeat: Option<i64>,
    /// Consecutive failures
    pub consecutive_failures: u32,
    /// Total tasks
    pub total_tasks: u64,
    /// Failed tasks
    pub failed_tasks: u64,
    /// Is healthy
    pub is_healthy: bool,
}

impl ValidatorHealthTracker {
    /// Create a new tracker
    pub fn new(validator_id: String) -> Self {
        Self {
            validator_id,
            last_heartbeat: None,
            consecutive_failures: 0,
            total_tasks: 0,
            failed_tasks: 0,
            is_healthy: true,
        }
    }

    /// Record a heartbeat
    pub fn record_heartbeat(&mut self) {
        self.last_heartbeat = Some(chrono::Utc::now().timestamp());
        self.consecutive_failures = 0;
        self.is_healthy = true;
    }

    /// Record task completion
    pub fn record_task(&mut self, success: bool) {
        self.total_tasks += 1;
        if !success {
            self.failed_tasks += 1;
            self.consecutive_failures += 1;

            if self.consecutive_failures >= 3 {
                self.is_healthy = false;
            }
        }
    }

    /// Check if alive (recent heartbeat)
    pub fn is_alive(&self, max_age: Duration) -> bool {
        if let Some(heartbeat_ts) = self.last_heartbeat {
            (chrono::Utc::now().timestamp() - heartbeat_ts) < max_age.as_secs() as i64
        } else {
            false
        }
    }

    /// Get error rate
    pub fn error_rate(&self) -> f64 {
        if self.total_tasks == 0 {
            0.0
        } else {
            self.failed_tasks as f64 / self.total_tasks as f64
        }
    }
}
