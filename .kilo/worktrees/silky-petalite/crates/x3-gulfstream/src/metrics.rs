//! Gulfstream Metrics Module

use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Gulfstream metrics collector
pub struct GulfstreamMetrics {
    transactions_submitted: AtomicU64,
    transactions_forwarded: AtomicU64,
    transactions_expired: AtomicU64,
    transactions_confirmed: AtomicU64,
    transactions_failed: AtomicU64,
    forward_times: RwLock<Vec<u64>>,
}

impl GulfstreamMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            transactions_submitted: AtomicU64::new(0),
            transactions_forwarded: AtomicU64::new(0),
            transactions_expired: AtomicU64::new(0),
            transactions_confirmed: AtomicU64::new(0),
            transactions_failed: AtomicU64::new(0),
            forward_times: RwLock::new(Vec::new()),
        }
    }

    /// Record transaction submitted
    pub fn record_transaction_submitted(&self) {
        self.transactions_submitted.fetch_add(1, Ordering::Relaxed);
    }

    /// Record transaction forwarded
    pub fn record_transaction_forwarded(&self) {
        self.transactions_forwarded.fetch_add(1, Ordering::Relaxed);
    }

    /// Record transaction expired
    pub fn record_transaction_expired(&self) {
        self.transactions_expired.fetch_add(1, Ordering::Relaxed);
    }

    /// Record transaction confirmed
    pub fn record_transaction_confirmed(&self) {
        self.transactions_confirmed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record transaction failed
    pub fn record_transaction_failed(&self) {
        self.transactions_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record forward time
    pub fn record_forward_time(&self, time_ms: u64) {
        let mut times = self.forward_times.write();
        times.push(time_ms);
        if times.len() > 1000 {
            times.drain(0..500);
        }
    }

    // Getters
    pub fn transactions_submitted(&self) -> u64 {
        self.transactions_submitted.load(Ordering::Relaxed)
    }

    pub fn transactions_forwarded(&self) -> u64 {
        self.transactions_forwarded.load(Ordering::Relaxed)
    }

    pub fn transactions_expired(&self) -> u64 {
        self.transactions_expired.load(Ordering::Relaxed)
    }

    pub fn transactions_confirmed(&self) -> u64 {
        self.transactions_confirmed.load(Ordering::Relaxed)
    }

    pub fn transactions_failed(&self) -> u64 {
        self.transactions_failed.load(Ordering::Relaxed)
    }

    pub fn avg_forward_time_ms(&self) -> u64 {
        let times = self.forward_times.read();
        if times.is_empty() {
            return 0;
        }
        let sum: u64 = times.iter().sum();
        sum / times.len() as u64
    }
}

impl Default for GulfstreamMetrics {
    fn default() -> Self {
        Self::new()
    }
}