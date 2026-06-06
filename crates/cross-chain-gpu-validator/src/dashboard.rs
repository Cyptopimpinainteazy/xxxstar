//! Operator dashboard and metrics pipeline

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub tps: f64,
    pub atomic_success_rate: f64,
    pub rollback_count: u64,
    pub timeout_count: u64,
    pub gpu_health: bool,
    pub rpc_latency_ms: u64,
    pub active_swaps: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub total_swaps: u64,
    pub successful_commits: u64,
    pub rollbacks: u64,
    pub timeouts: u64,
    pub total_txs_processed: u64,
    pub avg_tps: f64,
    pub peak_tps: f64,
    pub gpu_enabled: bool,
    pub gpu_healthy: bool,
    pub avg_rpc_latency_ms: u64,
    pub snapshots: VecDeque<MetricsSnapshot>,
}

impl Default for DashboardMetrics {
    fn default() -> Self {
        Self {
            total_swaps: 0,
            successful_commits: 0,
            rollbacks: 0,
            timeouts: 0,
            total_txs_processed: 0,
            avg_tps: 0.0,
            peak_tps: 0.0,
            gpu_enabled: false,
            gpu_healthy: false,
            avg_rpc_latency_ms: 0,
            snapshots: VecDeque::with_capacity(1000),
        }
    }
}

/// Operator dashboard for live metrics and monitoring
pub struct OperatorDashboard {
    metrics: Arc<Mutex<DashboardMetrics>>,
    max_snapshots: usize,
}

impl OperatorDashboard {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            metrics: Arc::new(Mutex::new(DashboardMetrics::default())),
            max_snapshots,
        }
    }

    pub async fn record_swap_success(&self) {
        let mut metrics = self.metrics.lock().await;
        metrics.total_swaps += 1;
        metrics.successful_commits += 1;
    }

    pub async fn record_swap_rollback(&self) {
        let mut metrics = self.metrics.lock().await;
        metrics.total_swaps += 1;
        metrics.rollbacks += 1;
    }

    pub async fn record_swap_timeout(&self) {
        let mut metrics = self.metrics.lock().await;
        metrics.total_swaps += 1;
        metrics.timeouts += 1;
    }

    pub async fn record_txs_processed(&self, count: u64) {
        let mut metrics = self.metrics.lock().await;
        metrics.total_txs_processed += count;
    }

    pub async fn record_tps(&self, tps: f64, active_swaps: usize) {
        let mut metrics = self.metrics.lock().await;
        metrics.avg_tps = (metrics.avg_tps + tps) / 2.0;
        if tps > metrics.peak_tps {
            metrics.peak_tps = tps;
        }

        let snapshot = MetricsSnapshot {
            timestamp: Utc::now(),
            tps,
            atomic_success_rate: if metrics.total_swaps > 0 {
                metrics.successful_commits as f64 / metrics.total_swaps as f64
            } else {
                0.0
            },
            rollback_count: metrics.rollbacks,
            timeout_count: metrics.timeouts,
            gpu_health: metrics.gpu_healthy,
            rpc_latency_ms: metrics.avg_rpc_latency_ms,
            active_swaps,
        };

        metrics.snapshots.push_back(snapshot);
        if metrics.snapshots.len() > self.max_snapshots {
            metrics.snapshots.pop_front();
        }
    }

    pub async fn record_gpu_health(&self, healthy: bool) {
        let mut metrics = self.metrics.lock().await;
        metrics.gpu_healthy = healthy;
    }

    pub async fn record_rpc_latency(&self, latency_ms: u64) {
        let mut metrics = self.metrics.lock().await;
        metrics.avg_rpc_latency_ms = (metrics.avg_rpc_latency_ms + latency_ms) / 2;
    }

    pub async fn enable_gpu(&self, enabled: bool) {
        let mut metrics = self.metrics.lock().await;
        metrics.gpu_enabled = enabled;
    }

    pub async fn get_metrics(&self) -> DashboardMetrics {
        self.metrics.lock().await.clone()
    }

    pub async fn render_json(&self) -> String {
        let metrics = self.get_metrics().await;
        serde_json::to_string_pretty(&metrics).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dashboard_metrics_recording() {
        let dashboard = OperatorDashboard::new(100);

        dashboard.record_swap_success().await;
        dashboard.record_swap_success().await;
        dashboard.record_swap_rollback().await;
        dashboard.record_swap_timeout().await;

        let metrics = dashboard.get_metrics().await;
        assert_eq!(metrics.total_swaps, 4);
        assert_eq!(metrics.successful_commits, 2);
        assert_eq!(metrics.rollbacks, 1);
        assert_eq!(metrics.timeouts, 1);
    }

    #[tokio::test]
    async fn test_dashboard_tps_tracking() {
        let dashboard = OperatorDashboard::new(100);

        dashboard.record_tps(1000.0, 10).await;
        dashboard.record_tps(1500.0, 15).await;
        dashboard.record_tps(1200.0, 12).await;

        let metrics = dashboard.get_metrics().await;
        assert!(metrics.peak_tps >= 1500.0);
        assert!(!metrics.snapshots.is_empty());
    }

    #[tokio::test]
    async fn test_dashboard_gpu_health() {
        let dashboard = OperatorDashboard::new(100);

        dashboard.enable_gpu(true).await;
        dashboard.record_gpu_health(true).await;

        let metrics = dashboard.get_metrics().await;
        assert!(metrics.gpu_enabled);
        assert!(metrics.gpu_healthy);
    }

    #[tokio::test]
    async fn test_dashboard_json_render() {
        let dashboard = OperatorDashboard::new(100);
        dashboard.record_swap_success().await;

        let json = dashboard.render_json().await;
        assert!(json.contains("total_swaps"));
        assert!(json.contains("successful_commits"));
    }
}
