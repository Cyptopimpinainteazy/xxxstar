//! X3 Chain Gulf-stream - Transaction Forwarding and Propagation
//!
//! Gulf-stream is X3 Chain's transaction forwarding protocol inspired by Solana.
//! It provides fast transaction propagation across the network by forwarding
//! transactions to the next scheduled leader before they are included in a block.
//!
//! ## Key Features
//!
//! - **Transaction Forwarding**: Forward transactions to upcoming leaders
//! - **Stale Transaction Detection**: Detect and remove old transactions
//! - **Leader Scheduling**: Track leader schedule for optimal forwarding
//! - **Transaction Deduplication**: Prevent duplicate transactions
//! - **Priority Queuing**: Handle priority transactions first

pub mod config;
pub mod error;
pub mod forwarder;
pub mod leader;
pub mod mempool;
pub mod metrics;
pub mod transaction;

pub use config::GulfstreamConfig;
pub use error::{GulfstreamError, GulfstreamResult};
pub use forwarder::Forwarder;
pub use leader::LeaderSchedule;
pub use mempool::TransactionMempool;
pub use metrics::GulfstreamMetrics;
pub use transaction::{Transaction, TransactionMeta, TransactionStatus};

use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{debug, info};

/// Main Gulfstream service
pub struct Gulfstream {
    config: GulfstreamConfig,
    mempool: Arc<TransactionMempool>,
    forwarder: Arc<Forwarder>,
    leader_schedule: Arc<RwLock<LeaderSchedule>>,
    metrics: Arc<GulfstreamMetrics>,
}

impl Gulfstream {
    /// Create new Gulfstream instance
    pub fn new(config: GulfstreamConfig) -> Self {
        let mempool = Arc::new(TransactionMempool::new(config.clone()));
        let forwarder = Arc::new(Forwarder::new(config.clone()));
        let leader_schedule = Arc::new(RwLock::new(LeaderSchedule::new()));
        let metrics = Arc::new(GulfstreamMetrics::new());

        Self {
            config,
            mempool,
            forwarder,
            leader_schedule,
            metrics,
        }
    }

    /// The configuration this service was built with. Exposed for introspection
    /// (the mempool is the component that consults it today).
    pub fn config(&self) -> &GulfstreamConfig {
        &self.config
    }

    /// Start Gulfstream service
    pub async fn start(&self) -> GulfstreamResult<()> {
        info!("Starting Gulfstream transaction forwarding service");

        // Start mempool cleanup task
        self.mempool.start_cleanup_task().await;

        // Start forwarder
        self.forwarder.start().await?;

        info!("Gulfstream service started successfully");
        Ok(())
    }

    /// Stop Gulfstream service
    pub async fn stop(&self) -> GulfstreamResult<()> {
        info!("Stopping Gulfstream service");

        self.forwarder.stop().await?;

        Ok(())
    }

    /// Submit a transaction to the network
    pub async fn submit_transaction(&self, transaction: Transaction) -> GulfstreamResult<String> {
        // Validate transaction
        transaction.validate()?;

        // Add to mempool
        let tx_hash = self.mempool.add_transaction(transaction).await?;

        // Forward to next leaders
        let leaders = self.get_leaders(5);
        for leader in leaders {
            self.forwarder.forward_to(&leader, &tx_hash).await?;
        }

        self.metrics.record_transaction_submitted();

        debug!("Transaction submitted: {}", tx_hash);
        Ok(tx_hash)
    }

    /// Get transaction status
    pub fn get_transaction_status(&self, tx_hash: &str) -> Option<TransactionStatus> {
        self.mempool.get_status(tx_hash)
    }

    /// Get leaders for the upcoming slots
    pub fn get_leaders(&self, count: usize) -> Vec<String> {
        self.leader_schedule.read().get_upcoming_leaders(count)
    }

    /// Update leader schedule
    pub fn update_leader_schedule(&self, schedule: LeaderSchedule) {
        *self.leader_schedule.write() = schedule;
    }

    /// Get current mempool size
    pub fn mempool_size(&self) -> usize {
        self.mempool.size()
    }

    /// Get Gulfstream statistics
    pub fn get_stats(&self) -> GulfstreamStats {
        GulfstreamStats {
            transactions_submitted: self.metrics.transactions_submitted(),
            transactions_forwarded: self.metrics.transactions_forwarded(),
            transactions_expired: self.metrics.transactions_expired(),
            mempool_size: self.mempool_size(),
            avg_forward_time_ms: self.metrics.avg_forward_time_ms(),
        }
    }
}

/// Gulfstream statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GulfstreamStats {
    pub transactions_submitted: u64,
    pub transactions_forwarded: u64,
    pub transactions_expired: u64,
    pub mempool_size: usize,
    pub avg_forward_time_ms: u64,
}

// A second `impl Default for GulfstreamConfig` used to sit here, filling every
// field except one via `..Default::default()` inside the impl itself — a
// duplicate that both failed to compile (E0119) and would have recursed. The
// canonical impl lives next to the type in `config.rs`.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gulfstream_creation() {
        let config = GulfstreamConfig::default();
        let gs = Gulfstream::new(config);

        // Just verify it was created
        assert_eq!(gs.mempool_size(), 0);
    }
}
