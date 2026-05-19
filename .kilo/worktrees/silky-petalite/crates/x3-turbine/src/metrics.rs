//! Turbine Metrics Module

use crate::shred::ShredType;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Turbine metrics collector
pub struct TurbineMetrics {
    blocks_received: AtomicU64,
    blocks_shredded: AtomicU64,
    blocks_reconstructed: AtomicU64,
    shreds_received: AtomicU64,
    shreds_sent: AtomicU64,
    data_shreds_received: AtomicU64,
    coding_shreds_received: AtomicU64,
    shred_errors: AtomicU64,
    recovery_success: AtomicU64,
    recovery_failed: AtomicU64,
    broadcast_times: RwLock<Vec<u64>>,
}

impl TurbineMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            blocks_received: AtomicU64::new(0),
            blocks_shredded: AtomicU64::new(0),
            blocks_reconstructed: AtomicU64::new(0),
            shreds_received: AtomicU64::new(0),
            shreds_sent: AtomicU64::new(0),
            data_shreds_received: AtomicU64::new(0),
            coding_shreds_received: AtomicU64::new(0),
            shred_errors: AtomicU64::new(0),
            recovery_success: AtomicU64::new(0),
            recovery_failed: AtomicU64::new(0),
            broadcast_times: RwLock::new(Vec::new()),
        }
    }

    /// Record block received
    pub fn record_block_received(&self) {
        self.blocks_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Record block shredded
    pub fn record_block_shredded(&self, shred_count: u64) {
        self.blocks_shredded.fetch_add(1, Ordering::Relaxed);
        self.shreds_sent.fetch_add(shred_count, Ordering::Relaxed);
    }

    /// Record block reconstructed
    pub fn record_block_reconstructed(&self) {
        self.blocks_reconstructed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record shred received
    pub fn record_shred_received(&self, shred_type: ShredType) {
        self.shreds_received.fetch_add(1, Ordering::Relaxed);
        match shred_type {
            ShredType::Data => self.data_shreds_received.fetch_add(1, Ordering::Relaxed),
            ShredType::Coding => self.coding_shreds_received.fetch_add(1, Ordering::Relaxed),
        };
    }

    /// Record shred error
    pub fn record_shred_error(&self) {
        self.shred_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record recovery success
    pub fn record_recovery_success(&self) {
        self.recovery_success.fetch_add(1, Ordering::Relaxed);
    }

    /// Record recovery failed
    pub fn record_recovery_failed(&self) {
        self.recovery_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record broadcast time
    pub fn record_broadcast_time(&self, time_ms: u64) {
        let mut times = self.broadcast_times.write();
        times.push(time_ms);
        // Keep only last 1000 measurements
        if times.len() > 1000 {
            times.drain(0..500);
        }
    }

    // Getters
    pub fn blocks_received(&self) -> u64 {
        self.blocks_received.load(Ordering::Relaxed)
    }

    pub fn blocks_shredded(&self) -> u64 {
        self.blocks_shredded.load(Ordering::Relaxed)
    }

    pub fn blocks_reconstructed(&self) -> u64 {
        self.blocks_reconstructed.load(Ordering::Relaxed)
    }

    pub fn shreds_received(&self) -> u64 {
        self.shreds_received.load(Ordering::Relaxed)
    }

    pub fn shreds_sent(&self) -> u64 {
        self.shreds_sent.load(Ordering::Relaxed)
    }

    pub fn shred_errors(&self) -> u64 {
        self.shred_errors.load(Ordering::Relaxed)
    }

    pub fn recovery_success(&self) -> u64 {
        self.recovery_success.load(Ordering::Relaxed)
    }

    pub fn recovery_failed(&self) -> u64 {
        self.recovery_failed.load(Ordering::Relaxed)
    }

    pub fn avg_broadcast_time_ms(&self) -> u64 {
        let times = self.broadcast_times.read();
        if times.is_empty() {
            return 0;
        }
        let sum: u64 = times.iter().sum();
        sum / times.len() as u64
    }
}

impl Default for TurbineMetrics {
    fn default() -> Self {
        Self::new()
    }
}
