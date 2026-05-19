//! Forwarder Module - Transaction forwarding to leaders

use crate::config::GulfstreamConfig;
use crate::error::GulfstreamResult;
use crate::metrics::GulfstreamMetrics;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info};

/// Forwarder for sending transactions to leaders
pub struct Forwarder {
    config: GulfstreamConfig,
    metrics: Arc<GulfstreamMetrics>,
    running: AtomicBool,
    shutdown_tx: RwLock<Option<mpsc::Sender<()>>>,
}

impl Forwarder {
    /// Create new forwarder
    pub fn new(config: GulfstreamConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(GulfstreamMetrics::new()),
            running: AtomicBool::new(false),
            shutdown_tx: RwLock::new(None),
        }
    }

    /// Start the forwarder
    pub async fn start(&self) -> GulfstreamResult<()> {
        info!("Starting transaction forwarder");
        
        let (tx, _rx) = mpsc::channel(100);
        *self.shutdown_tx.write() = Some(tx);
        
        self.running.store(true, Ordering::SeqCst);
        
        Ok(())
    }

    /// Stop the forwarder
    pub async fn stop(&self) -> GulfstreamResult<()> {
        info!("Stopping transaction forwarder");
        
        if let Some(tx) = self.shutdown_tx.write().take() {
            let _ = tx.send(()).await;
        }
        
        self.running.store(false, Ordering::SeqCst);
        
        Ok(())
    }

    /// Forward transaction to a specific leader
    pub async fn forward_to(&self, leader_id: &str, tx_hash: &str) -> GulfstreamResult<()> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(GulfstreamError::NotStarted("Forwarder not running".into()));
        }

        let start = Instant::now();
        
        debug!("Forwarding transaction {} to leader {}", tx_hash, leader_id);
        
        // In real implementation, would send to actual network
        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        let elapsed = start.elapsed().as_millis() as u64;
        self.metrics.record_forward_time(elapsed);
        
        Ok(())
    }

    /// Forward multiple transactions in batch
    pub async fn forward_batch(&self, leader_id: &str, tx_hashes: &[String]) -> GulfstreamResult<()> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(GulfstreamError::NotStarted("Forwarder not running".into()));
        }

        debug!("Forwarding batch of {} transactions to leader {}", tx_hashes.len(), leader_id);
        
        for tx_hash in tx_hashes {
            self.forward_to(leader_id, tx_hash).await?;
        }
        
        Ok(())
    }

    /// Check if forwarder is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}