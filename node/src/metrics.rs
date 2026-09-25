use std::sync::atomic::{AtomicU64, Ordering};
/// Metrics and Monitoring for X3 Chain
///
/// Production-ready Prometheus metrics for observability.
/// Uses Substrate's prometheus registry when available.
use std::sync::Arc;

use substrate_prometheus_endpoint::prometheus;

/// Prometheus registry type alias (use substrate_prometheus_endpoint's Registry)
pub type PrometheusRegistry = substrate_prometheus_endpoint::Registry;

/// X3 Chain metrics collector with Prometheus integration
pub struct MetricsCollector {
    /// Prometheus registry reference
    registry: Option<Arc<PrometheusRegistry>>,
    /// Blocks produced counter
    blocks_produced: Arc<AtomicU64>,
    /// Transactions received counter
    transactions_received: Arc<AtomicU64>,
    /// Comit transactions submitted
    comits_submitted: Arc<AtomicU64>,
    /// Comit transactions confirmed
    comits_confirmed: Arc<AtomicU64>,
    /// Comit transactions failed
    comits_failed: Arc<AtomicU64>,
    /// EVM executions counter
    evm_executions: Arc<AtomicU64>,
    /// SVM executions counter
    svm_executions: Arc<AtomicU64>,
    /// Cross-VM (dual) executions
    dual_vm_executions: Arc<AtomicU64>,
    /// Canonical ledger updates
    canonical_ledger_updates: Arc<AtomicU64>,
    /// Cross-VM operations prepared
    cross_vm_prepared: Arc<AtomicU64>,
    /// Cross-VM operations committed
    cross_vm_committed: Arc<AtomicU64>,
    /// Cross-VM operations aborted
    cross_vm_aborted: Arc<AtomicU64>,
    /// Fee deductions
    fee_deductions: Arc<AtomicU64>,
}

/// Prometheus metrics wrapper for proper registration
pub struct X3PrometheusMetrics {
    /// Blocks produced
    pub blocks_produced: prometheus::Counter,
    /// Transactions received
    pub transactions_received: prometheus::Counter,
    /// Comit transactions submitted
    pub comits_submitted: prometheus::Counter,
    /// Comit transactions confirmed
    pub comits_confirmed: prometheus::Counter,
    /// Comit transactions failed
    pub comits_failed: prometheus::Counter,
    /// EVM executions
    pub evm_executions: prometheus::Counter,
    /// SVM executions
    pub svm_executions: prometheus::Counter,
    /// Cross-VM executions
    pub dual_vm_executions: prometheus::Counter,
    /// Canonical ledger updates
    pub canonical_ledger_updates: prometheus::Counter,
    /// Cross-VM operations prepared
    pub cross_vm_prepared: prometheus::Counter,
    /// Cross-VM operations committed
    pub cross_vm_committed: prometheus::Counter,
    /// Cross-VM operations aborted
    pub cross_vm_aborted: prometheus::Counter,
    /// Fee deductions
    pub fee_deductions: prometheus::Counter,
    /// Serialised size of each imported block, in bytes.
    ///
    /// The storage audit's per-block metric list asks for `block_bytes`, and it
    /// is what turns "blocks are small" into the disk arithmetic the audit
    /// depends on: at 200 ms slots the chain produces ~157.7M blocks/year, so a
    /// 10 KB block is 1.58 TB/year and a 50 KB block is 7.9 TB/year before any
    /// state growth.
    pub imported_block_bytes: prometheus::Histogram,
    /// Extrinsics per imported block.
    pub imported_block_extrinsics: prometheus::Histogram,
    /// Wall-clock seconds between imported blocks.
    ///
    /// Distinct from the configured slot time: a chain that targets 200 ms and
    /// lands on 900 ms has a throughput problem that block-height metrics alone
    /// cannot show.
    pub block_interval_seconds: prometheus::Histogram,
    /// Transactions waiting in the pool's ready queue.
    ///
    /// This is the number that says whether the *chain* is the limit. During the
    /// throughput sweep the only way to tell "the chain kept up" from "the client
    /// could not offer more" was to snapshot two cumulative counters around a load
    /// run and diff them. A live ready-queue gauge answers it with one scrape, and
    /// a ready queue that *climbs and stays up* is the signal that block space has
    /// become the constraint.
    pub txpool_ready: prometheus::IntGauge,
    /// Bytes of ready transaction encodings.
    pub txpool_ready_bytes: prometheus::IntGauge,
    /// Transactions in the future queue, i.e. held back by a nonce gap.
    pub txpool_future: prometheus::IntGauge,
    /// Bytes of future transaction encodings.
    pub txpool_future_bytes: prometheus::IntGauge,
}

impl X3PrometheusMetrics {
    /// Register metrics with the Prometheus registry
    pub fn register(registry: &PrometheusRegistry) -> Result<Self, prometheus::Error> {
        let blocks_produced = prometheus::Counter::new(
            "x3_blocks_produced_total",
            "Total number of blocks produced by this node",
        )?;
        let transactions_received = prometheus::Counter::new(
            "x3_transactions_received_total",
            "Total number of transactions received",
        )?;
        let comits_submitted = prometheus::Counter::new(
            "x3_comits_submitted_total",
            "Total number of Comit transactions submitted",
        )?;
        let comits_confirmed = prometheus::Counter::new(
            "x3_comits_confirmed_total",
            "Total number of Comit transactions confirmed",
        )?;
        let comits_failed = prometheus::Counter::new(
            "x3_comits_failed_total",
            "Total number of Comit transactions failed",
        )?;
        let evm_executions =
            prometheus::Counter::new("x3_evm_executions_total", "Total number of EVM executions")?;
        let svm_executions =
            prometheus::Counter::new("x3_svm_executions_total", "Total number of SVM executions")?;
        let dual_vm_executions = prometheus::Counter::new(
            "x3_dual_vm_executions_total",
            "Total number of cross-VM executions",
        )?;
        let canonical_ledger_updates = prometheus::Counter::new(
            "x3_canonical_ledger_updates_total",
            "Total number of canonical ledger updates",
        )?;
        let cross_vm_prepared = prometheus::Counter::new(
            "x3_cross_vm_prepared_total",
            "Total number of cross-VM operations prepared",
        )?;
        let cross_vm_committed = prometheus::Counter::new(
            "x3_cross_vm_committed_total",
            "Total number of cross-VM operations committed",
        )?;
        let cross_vm_aborted = prometheus::Counter::new(
            "x3_cross_vm_aborted_total",
            "Total number of cross-VM operations aborted",
        )?;
        let fee_deductions =
            prometheus::Counter::new("x3_fee_deductions_total", "Total number of fee deductions")?;

        // Buckets are chosen for the questions the storage audit asks: "how big
        // is a block, really" (1 KB .. 4 MB), "how full is a block" (0 .. 512
        // extrinsics), and "what cadence does the chain actually hold" (10 ms ..
        // 10 s).
        let imported_block_bytes = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "x3_imported_block_bytes",
                "Serialised size of each imported block (header + extrinsics) in bytes",
            )
            .buckets(vec![
                1_024.0,
                4_096.0,
                16_384.0,
                65_536.0,
                262_144.0,
                1_048_576.0,
                4_194_304.0,
            ]),
        )?;
        let imported_block_extrinsics = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "x3_imported_block_extrinsics",
                "Number of extrinsics in each imported block",
            )
            .buckets(vec![0.0, 1.0, 8.0, 32.0, 128.0, 512.0, 2_048.0]),
        )?;
        let block_interval_seconds = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "x3_block_interval_seconds",
                "Wall-clock seconds between imported blocks",
            )
            .buckets(vec![0.01, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0]),
        )?;
        let txpool_ready = prometheus::IntGauge::new(
            "x3_txpool_ready",
            "Transactions waiting in the pool's ready queue",
        )?;
        let txpool_ready_bytes = prometheus::IntGauge::new(
            "x3_txpool_ready_bytes",
            "Bytes of ready transaction encodings in the pool",
        )?;
        let txpool_future = prometheus::IntGauge::new(
            "x3_txpool_future",
            "Transactions in the pool's future queue (held by a nonce gap)",
        )?;
        let txpool_future_bytes = prometheus::IntGauge::new(
            "x3_txpool_future_bytes",
            "Bytes of future transaction encodings in the pool",
        )?;

        registry.register(Box::new(blocks_produced.clone()))?;
        registry.register(Box::new(transactions_received.clone()))?;
        registry.register(Box::new(comits_submitted.clone()))?;
        registry.register(Box::new(comits_confirmed.clone()))?;
        registry.register(Box::new(comits_failed.clone()))?;
        registry.register(Box::new(evm_executions.clone()))?;
        registry.register(Box::new(svm_executions.clone()))?;
        registry.register(Box::new(dual_vm_executions.clone()))?;
        registry.register(Box::new(canonical_ledger_updates.clone()))?;
        registry.register(Box::new(cross_vm_prepared.clone()))?;
        registry.register(Box::new(cross_vm_committed.clone()))?;
        registry.register(Box::new(cross_vm_aborted.clone()))?;
        registry.register(Box::new(fee_deductions.clone()))?;
        registry.register(Box::new(imported_block_bytes.clone()))?;
        registry.register(Box::new(imported_block_extrinsics.clone()))?;
        registry.register(Box::new(block_interval_seconds.clone()))?;
        registry.register(Box::new(txpool_ready.clone()))?;
        registry.register(Box::new(txpool_ready_bytes.clone()))?;
        registry.register(Box::new(txpool_future.clone()))?;
        registry.register(Box::new(txpool_future_bytes.clone()))?;

        Ok(Self {
            blocks_produced,
            transactions_received,
            comits_submitted,
            comits_confirmed,
            comits_failed,
            evm_executions,
            svm_executions,
            dual_vm_executions,
            canonical_ledger_updates,
            cross_vm_prepared,
            cross_vm_committed,
            cross_vm_aborted,
            fee_deductions,
            imported_block_bytes,
            imported_block_extrinsics,
            block_interval_seconds,
            txpool_ready,
            txpool_ready_bytes,
            txpool_future,
            txpool_future_bytes,
        })
    }
}

impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            blocks_produced: self.blocks_produced.clone(),
            transactions_received: self.transactions_received.clone(),
            comits_submitted: self.comits_submitted.clone(),
            comits_confirmed: self.comits_confirmed.clone(),
            comits_failed: self.comits_failed.clone(),
            evm_executions: self.evm_executions.clone(),
            svm_executions: self.svm_executions.clone(),
            dual_vm_executions: self.dual_vm_executions.clone(),
            canonical_ledger_updates: self.canonical_ledger_updates.clone(),
            cross_vm_prepared: self.cross_vm_prepared.clone(),
            cross_vm_committed: self.cross_vm_committed.clone(),
            cross_vm_aborted: self.cross_vm_aborted.clone(),
            fee_deductions: self.fee_deductions.clone(),
        }
    }
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            registry: None,
            blocks_produced: Arc::new(AtomicU64::new(0)),
            transactions_received: Arc::new(AtomicU64::new(0)),
            comits_submitted: Arc::new(AtomicU64::new(0)),
            comits_confirmed: Arc::new(AtomicU64::new(0)),
            comits_failed: Arc::new(AtomicU64::new(0)),
            evm_executions: Arc::new(AtomicU64::new(0)),
            svm_executions: Arc::new(AtomicU64::new(0)),
            dual_vm_executions: Arc::new(AtomicU64::new(0)),
            canonical_ledger_updates: Arc::new(AtomicU64::new(0)),
            cross_vm_prepared: Arc::new(AtomicU64::new(0)),
            cross_vm_committed: Arc::new(AtomicU64::new(0)),
            cross_vm_aborted: Arc::new(AtomicU64::new(0)),
            fee_deductions: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create with Prometheus registry
    pub fn with_registry(registry: Arc<PrometheusRegistry>) -> Self {
        // Register custom metrics with the registry
        // The actual prometheus Counter/Gauge registration happens through
        // Substrate's prometheus endpoint which exposes these counters
        Self {
            registry: Some(registry),
            blocks_produced: Arc::new(AtomicU64::new(0)),
            transactions_received: Arc::new(AtomicU64::new(0)),
            comits_submitted: Arc::new(AtomicU64::new(0)),
            comits_confirmed: Arc::new(AtomicU64::new(0)),
            comits_failed: Arc::new(AtomicU64::new(0)),
            evm_executions: Arc::new(AtomicU64::new(0)),
            svm_executions: Arc::new(AtomicU64::new(0)),
            dual_vm_executions: Arc::new(AtomicU64::new(0)),
            canonical_ledger_updates: Arc::new(AtomicU64::new(0)),
            cross_vm_prepared: Arc::new(AtomicU64::new(0)),
            cross_vm_committed: Arc::new(AtomicU64::new(0)),
            cross_vm_aborted: Arc::new(AtomicU64::new(0)),
            fee_deductions: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a block created event
    pub fn block_created(&self) {
        self.blocks_produced.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a transaction received
    pub fn transaction_received(&self) {
        self.transactions_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a Comit transaction submitted
    pub fn comit_submitted(&self) {
        self.comits_submitted.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a Comit transaction confirmed
    pub fn comit_confirmed(&self) {
        self.comits_confirmed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a Comit transaction failed
    pub fn comit_failed(&self) {
        self.comits_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record EVM execution
    pub fn evm_execution(&self) {
        self.evm_executions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record SVM execution
    pub fn svm_execution(&self) {
        self.svm_executions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record dual-VM (cross-VM) execution
    pub fn dual_vm_execution(&self) {
        self.dual_vm_executions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a canonical ledger update
    pub fn canonical_ledger_update(&self) {
        self.canonical_ledger_updates
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cross-VM operation prepared
    pub fn cross_vm_prepared(&self) {
        self.cross_vm_prepared.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cross-VM operation committed
    pub fn cross_vm_committed(&self) {
        self.cross_vm_committed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cross-VM operation aborted
    pub fn cross_vm_aborted(&self) {
        self.cross_vm_aborted.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a fee deduction
    pub fn fee_deduction(&self) {
        self.fee_deductions.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            blocks_produced: self.blocks_produced.load(Ordering::Relaxed),
            transactions_received: self.transactions_received.load(Ordering::Relaxed),
            comits_submitted: self.comits_submitted.load(Ordering::Relaxed),
            comits_confirmed: self.comits_confirmed.load(Ordering::Relaxed),
            comits_failed: self.comits_failed.load(Ordering::Relaxed),
            evm_executions: self.evm_executions.load(Ordering::Relaxed),
            svm_executions: self.svm_executions.load(Ordering::Relaxed),
            dual_vm_executions: self.dual_vm_executions.load(Ordering::Relaxed),
        }
    }

    /// Check if registry is attached
    pub fn has_registry(&self) -> bool {
        self.registry.is_some()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of current metrics values
#[derive(Clone, Debug, Default)]
pub struct MetricsSnapshot {
    /// Number of blocks the node has produced.
    pub blocks_produced: u64,
    /// Transactions received by the node (from RPC or P2P).
    pub transactions_received: u64,
    /// Comit submissions that reached the network.
    pub comits_submitted: u64,
    /// Comit submissions that were confirmed on-chain.
    pub comits_confirmed: u64,
    /// Comit submissions that failed or were rejected.
    pub comits_failed: u64,
    /// Etched EVM execution attempts observed during processing.
    pub evm_executions: u64,
    /// Etched SVM execution attempts observed during processing.
    pub svm_executions: u64,
    /// Total number of dual-VM execution attempts recorded by the node.
    pub dual_vm_executions: u64,
}

impl MetricsSnapshot {
    /// Calculate Comit success rate
    pub fn comit_success_rate(&self) -> f64 {
        let total = self.comits_confirmed + self.comits_failed;
        if total == 0 {
            return 100.0;
        }
        (self.comits_confirmed as f64 / total as f64) * 100.0
    }
}

/// Health check status
#[derive(Clone, Debug)]
pub struct HealthStatus {
    /// Node is operational
    pub operational: bool,
    /// Block finality working
    pub finality_healthy: bool,
    /// Network connectivity is good
    pub network_healthy: bool,
    /// Authority participation is active
    pub authority_healthy: bool,
    /// Overall health percentage (0-100)
    pub health_score: u8,
}

impl HealthStatus {
    /// Create new health status
    pub fn new() -> Self {
        Self {
            operational: true,
            finality_healthy: true,
            network_healthy: true,
            authority_healthy: true,
            health_score: 100,
        }
    }

    /// Calculate overall health score
    pub fn calculate_score(&mut self) {
        let mut score = 100u16;

        if !self.operational {
            score = 0;
        } else {
            if !self.finality_healthy {
                score -= 25;
            }
            if !self.network_healthy {
                score -= 25;
            }
            if !self.authority_healthy {
                score -= 25;
            }
        }

        self.health_score = (score as u8).min(100);
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_calculation() {
        let mut health = HealthStatus::new();
        health.finality_healthy = false;
        health.calculate_score();
        assert_eq!(health.health_score, 75);
    }

    #[test]
    fn test_health_status_all_bad() {
        let mut health = HealthStatus::new();
        health.operational = false;
        health.calculate_score();
        assert_eq!(health.health_score, 0);
    }
}
