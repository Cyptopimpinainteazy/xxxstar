//! X3 Chain Turbine - Block Propagation and Data Distribution Network
//!
//! Turbine is X3 Chain's high-performance block propagation mechanism inspired by Solana.
//! It uses erasure coding (Reed-Solomon) to distribute block data across the network
//! with redundancy, ensuring fast and reliable block propagation even with partial network failure.
//!
//! ## Key Components
//!
//! - **Shredder**: Splits blocks into erasure-coded shreds (data + parity)
//! - **Blockstore**: Manages received shreds and reconstructs blocks
//! - **Broadcast**: Efficient multi-peer block distribution
//! - **Recovery**: Handles missing shreds through erasure coding

pub mod blockstore;
pub mod broadcast;
pub mod config;
pub mod error;
pub mod metrics;
pub mod packet;
pub mod peer;
pub mod recovery;
pub mod shred;
#[cfg(test)]
pub mod test_utils;

pub use blockstore::{Blockstore, BlockstoreConfig, ReceivedShred};
pub use broadcast::{BroadcastService, BroadcastStats};
pub use config::{BroadcastConfig, ShredConfig, TurbineConfig};
pub use error::{TurbineError, TurbineResult};
pub use metrics::TurbineMetrics;
pub use packet::{Packet, PacketPool};
pub use peer::{PeerInfo, PeerManager, PeerRole};
pub use recovery::{RecoveryConfig, ShredRecovery};
pub use shred::{Shred, ShredFlag, ShredPayload, ShredType};

use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Main Turbine propagation service
pub struct Turbine {
    config: TurbineConfig,
    shredder: Arc<shred::Shredder>,
    blockstore: Arc<Blockstore>,
    broadcast: Arc<RwLock<Option<Arc<BroadcastService>>>>,
    peer_manager: Arc<PeerManager>,
    metrics: Arc<TurbineMetrics>,
    shutdown_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,
}

impl Turbine {
    /// Create a new Turbine instance
    pub fn new(config: TurbineConfig) -> Self {
        let shred_config = ShredConfig {
            shred_size: config.shred_size,
            coding_shreds: config.num_coding_shreds,
            data_shreds: config.num_data_shreds,
        };

        let blockstore_config = BlockstoreConfig {
            max_pending_blocks: config.max_pending_blocks,
            shred_recovery_timeout: config.shred_recovery_timeout(),
            enable_recovery: config.enable_shred_recovery,
        };

        let metrics = Arc::new(TurbineMetrics::new());
        let shredder = Arc::new(shred::Shredder::new(shred_config));
        let blockstore = Arc::new(Blockstore::new(blockstore_config, metrics.clone()));
        let peer_manager = Arc::new(PeerManager::new(config.clone()));

        Self {
            config,
            shredder,
            blockstore,
            broadcast: Arc::new(RwLock::new(None)),
            peer_manager,
            metrics,
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the Turbine service
    pub async fn start(&self) -> TurbineResult<()> {
        info!("Starting Turbine propagation service");

        // Initialize broadcast service
        let broadcast = BroadcastService::new(
            self.config.clone(),
            self.peer_manager.clone(),
            self.metrics.clone(),
        )
        .await?;

        *self.broadcast.write() = Some(Arc::new(broadcast));

        // Start peer manager
        self.peer_manager.start().await?;

        info!("Turbine propagation service started successfully");
        Ok(())
    }

    /// Stop the Turbine service
    pub async fn stop(&self) -> TurbineResult<()> {
        info!("Stopping Turbine propagation service");

        let shutdown_tx = { self.shutdown_tx.write().take() };
        if let Some(tx) = shutdown_tx {
            let _ = tx.send(()).await;
        }

        let broadcast = { self.broadcast.write().take() };
        if let Some(broadcast) = broadcast {
            broadcast.shutdown().await;
        }

        self.peer_manager.stop().await?;

        info!("Turbine propagation service stopped");
        Ok(())
    }

    /// Process incoming shred
    pub async fn process_shred(&self, shred: Shred) -> TurbineResult<Option<Vec<u8>>> {
        debug!(
            "Processing shred: slot={}, index={}",
            shred.slot(),
            shred.shred_index()
        );

        self.metrics.record_shred_received(shred.shred_type());

        match self.blockstore.insert_shred(shred) {
            Ok(Some(block_data)) => {
                self.metrics.record_block_reconstructed();
                debug!("Block reconstructed from shreds");
                Ok(Some(block_data))
            }
            Ok(None) => {
                // More shreds needed
                Ok(None)
            }
            Err(e) => {
                self.metrics.record_shred_error();
                error!("Error processing shred: {}", e);
                Err(e)
            }
        }
    }

    /// Create and broadcast a new block
    pub async fn broadcast_block(&self, slot: u64, block_data: Vec<u8>) -> TurbineResult<()> {
        debug!(
            "Broadcasting block: slot={}, size={} bytes",
            slot,
            block_data.len()
        );

        let start_time = std::time::Instant::now();

        // Create shreds from block data
        let shreds = self.shredder.create_shreds(slot, block_data)?;
        let shred_count = shreds.len();

        self.metrics.record_block_shredded(shred_count as u64);

        // Get broadcast service
        let broadcast = {
            let guard = self.broadcast.read();
            guard.as_ref().cloned().ok_or_else(|| {
                TurbineError::NotStarted("Broadcast service not initialized".into())
            })?
        };

        // Broadcast to peers
        broadcast.broadcast_shreds(shreds).await?;

        let elapsed = start_time.elapsed();
        self.metrics
            .record_broadcast_time(elapsed.as_millis() as u64);

        info!(
            "Block broadcast completed: slot={}, shreds={}, time={}ms",
            slot,
            shred_count,
            elapsed.as_millis()
        );

        Ok(())
    }

    /// Request missing shreds from peers
    pub async fn request_missing_shreds(&self, slot: u64, indices: &[u32]) -> TurbineResult<()> {
        debug!(
            "Requesting missing shreds: slot={}, indices={:?}",
            slot, indices
        );

        self.peer_manager.request_shreds(slot, indices).await
    }

    /// Get the current peer set
    pub fn get_peers(&self) -> Vec<PeerInfo> {
        self.peer_manager.get_active_peers()
    }

    /// Get turbine statistics
    pub fn get_stats(&self) -> TurbineStats {
        TurbineStats {
            blocks_received: self.metrics.blocks_received(),
            blocks_shredded: self.metrics.blocks_shredded(),
            shreds_received: self.metrics.shreds_received(),
            shreds_sent: self.metrics.shreds_sent(),
            recovery_success: self.metrics.recovery_success(),
            recovery_failed: self.metrics.recovery_failed(),
            avg_broadcast_time_ms: self.metrics.avg_broadcast_time_ms(),
            peer_count: self.peer_manager.peer_count(),
        }
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        self.broadcast.read().is_some()
    }
}

/// Turbine statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TurbineStats {
    pub blocks_received: u64,
    pub blocks_shredded: u64,
    pub shreds_received: u64,
    pub shreds_sent: u64,
    pub recovery_success: u64,
    pub recovery_failed: u64,
    pub avg_broadcast_time_ms: u64,
    pub peer_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_turbine_creation() {
        let config = TurbineConfig::default();
        let turbine = Turbine::new(config);
        assert!(!turbine.is_running());
    }

    #[tokio::test]
    async fn test_shred_creation() {
        let config = TurbineConfig::default();
        let turbine = Turbine::new(config);

        let block_data = vec![0u8; 32000];
        let shreds = turbine.shredder.create_shreds(1, block_data).unwrap();

        assert!(!shreds.is_empty());
    }
}
