//! Live Auditor for X3 Autonomic Core
//!
//! Monitors invariants and health metrics in real-time during block production.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub use x3_autonomic_types::*;

/// Auditor configuration
#[derive(Debug, Clone)]
pub struct AuditorConfig {
    pub check_interval_blocks: u32,
    pub alert_channel_capacity: usize,
    pub store_events: bool,
}

impl Default for AuditorConfig {
    fn default() -> Self {
        Self {
            check_interval_blocks: 1,
            alert_channel_capacity: 1000,
            store_events: true,
        }
    }
}

/// Live auditor state
pub struct LiveAuditor {
    config: AuditorConfig,
    current_block: u32,
    enabled_invariants: Arc<RwLock<Vec<InvariantDefinition>>>,
    enabled_metrics: Arc<RwLock<Vec<HealthMetricDefinition>>>,
}

impl LiveAuditor {
    pub fn new(config: AuditorConfig) -> Self {
        Self {
            config,
            current_block: 0,
            enabled_invariants: Arc::new(RwLock::new(Vec::new())),
            enabled_metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Enable an invariant for monitoring
    pub async fn enable_invariant(&self, invariant: InvariantDefinition) {
        let mut invariants = self.enabled_invariants.write().await;
        if invariant.enabled {
            info!("Enabling invariant: {} (ID: {})", invariant.name, invariant.id);
            invariants.push(invariant);
        }
    }

    /// Enable a health metric for monitoring
    pub async fn enable_metric(&self, metric: HealthMetricDefinition) {
        let mut metrics = self.enabled_metrics.write().await;
        if metric.enabled {
            info!("Enabling health metric: {}", metric.id);
            metrics.push(metric);
        }
    }

    /// Process a new block and run invariant checks
    pub async fn on_block(&self, block_number: u32) {
        self.current_block = block_number;
        let invariants = self.enabled_invariants.read().await;
        
        for invariant in invariants.iter() {
            if block_number % invariant.check_interval_blocks == 0 {
                info!("Checking invariant {} at block {}", invariant.name, block_number);
                // Invariant checking logic would be implemented here
                // Currently a placeholder - real implementation would query runtime state
            }
        }
    }

    /// Check all enabled invariants for a specific block
    pub async fn check_invariants(&self, block_number: u32) -> Vec<(u32, bool, Severity)> {
        let invariants = self.enabled_invariants.read().await;
        let mut results = Vec::new();
        
        for inv in invariants.iter() {
            let passed = true; // Placeholder - real implementation would check actual state
            results.push((inv.id, passed, inv.severity));
            
            if !passed && inv.severity == Severity::Critical || inv.severity == Severity::Emergency {
                error!("CRITICAL INVARIANT VIOLATION: {} at block {}", inv.name, block_number);
            }
        }
        
        results
    }

    /// Get current auditor health status
    pub fn health_status(&self) -> HealthStatus {
        if self.enabled_invariants.blocking_read().is_empty() {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auditor_creation() {
        let auditor = LiveAuditor::new(AuditorConfig::default());
        assert_eq!(auditor.current_block, 0);
    }

    #[tokio::test]
    async fn test_enable_invariant() {
        let auditor = LiveAuditor::new(AuditorConfig::default());
        let invariant = InvariantDefinition {
            id: 1,
            name: "test",
            description: "test invariant",
            severity: Severity::Warning,
            check_interval_blocks: 1,
            enabled: true,
        };
        auditor.enable_invariant(invariant).await;
        let invariants = auditor.enabled_invariants.read().await;
        assert_eq!(invariants.len(), 1);
    }
}