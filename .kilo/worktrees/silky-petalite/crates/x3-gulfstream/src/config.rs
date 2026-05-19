//! Gulfstream Configuration

use serde::{Deserialize, Serialize};

/// Gulfstream configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GulfstreamConfig {
    /// Maximum mempool size
    pub max_mempool_size: usize,
    
    /// Maximum transaction age in slots
    pub max_transaction_age_slots: u64,
    
    /// Batch size for forwarding transactions
    pub forward_batch_size: usize,
    
    /// Forward timeout in milliseconds
    pub forward_timeout_ms: u64,
    
    /// Enable transaction prioritization
    pub enable_prioritization: bool,
    
    /// Number of priority levels
    pub priority_levels: usize,
    
    /// Stale transaction check interval in ms
    pub stale_check_interval_ms: u64,
    
    /// Deduplication cache size
    pub dedup_cache_size: usize,
    
    /// Network bind address
    pub bind_address: String,
    
    /// Maximum connections
    pub max_connections: usize,
    
    /// Enable metrics
    pub enable_metrics: bool,
}

impl Default for GulfstreamConfig {
    fn default() -> Self {
        Self {
            max_mempool_size: 50000,
            max_transaction_age_slots: 100,
            forward_batch_size: 100,
            forward_timeout_ms: 5000,
            enable_prioritization: true,
            priority_levels: 5,
            stale_check_interval_ms: 1000,
            dedup_cache_size: 100000,
            bind_address: "0.0.0.0:8002".to_string(),
            max_connections: 1000,
            enable_metrics: true,
        }
    }
}