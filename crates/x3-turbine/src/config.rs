//! Turbine Configuration Module
//!
//! Defines all configuration parameters for the Turbine propagation system

use serde::{Deserialize, Serialize};

/// Main Turbine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurbineConfig {
    /// Size of each shred in bytes
    pub shred_size: usize,

    /// Number of data shreds per block
    pub num_data_shreds: usize,

    /// Number of coding (parity) shreds per block
    pub num_coding_shreds: usize,

    /// Maximum number of pending blocks to track
    pub max_pending_blocks: usize,

    /// Shred recovery timeout in milliseconds
    pub shred_recovery_timeout_ms: u64,

    /// Enable shred recovery for missing shreds
    pub enable_shred_recovery: bool,

    /// Maximum number of peers to cache
    pub peer_cache_size: usize,

    /// Maximum peers to use for broadcasting per slot
    pub max_peers_per_slot: usize,

    /// Number of shreds to send per broadcast batch
    pub broadcast_batch_size: usize,

    /// Timeout for repair requests in milliseconds
    pub repair_request_timeout_ms: u64,

    /// Network bind address for shred reception
    pub bind_address: String,

    /// Enable UDP for low-latency shred transfer
    pub enable_udp: bool,

    /// Enable TCP for reliable shred transfer
    pub enable_tcp: bool,

    /// Maximum packet size in bytes
    pub max_packet_size: usize,

    /// Connection pool size
    pub connection_pool_size: usize,

    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for TurbineConfig {
    fn default() -> Self {
        Self {
            shred_size: 16384,
            num_data_shreds: 32,
            num_coding_shreds: 16,
            max_pending_blocks: 100,
            shred_recovery_timeout_ms: 5000,
            enable_shred_recovery: true,
            peer_cache_size: 1000,
            max_peers_per_slot: 20,
            broadcast_batch_size: 64,
            repair_request_timeout_ms: 2000,
            bind_address: "0.0.0.0:8001".to_string(),
            enable_udp: true,
            enable_tcp: true,
            max_packet_size: 65536,
            connection_pool_size: 100,
            enable_metrics: true,
        }
    }
}

/// Shred generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShredConfig {
    /// Size of each shred in bytes
    pub shred_size: usize,

    /// Number of coding (parity) shreds
    pub coding_shreds: usize,

    /// Number of data shreds
    pub data_shreds: usize,
}

impl Default for ShredConfig {
    fn default() -> Self {
        Self {
            shred_size: 16384,
            coding_shreds: 16,
            data_shreds: 32,
        }
    }
}

/// Broadcast configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastConfig {
    /// Maximum peers to broadcast to simultaneously
    pub max_parallel_peers: usize,

    /// Batch size for sending shreds
    pub batch_size: usize,

    /// Timeout for peer connection in ms
    pub connect_timeout_ms: u64,

    /// Timeout for sending data in ms
    pub send_timeout_ms: u64,

    /// Enable erasure coding based distribution
    pub use_erasure_distribution: bool,
}

impl Default for BroadcastConfig {
    fn default() -> Self {
        Self {
            max_parallel_peers: 20,
            batch_size: 64,
            connect_timeout_ms: 5000,
            send_timeout_ms: 3000,
            use_erasure_distribution: true,
        }
    }
}

impl TurbineConfig {
    /// Get the total number of shreds per block
    pub fn total_shreds(&self) -> usize {
        self.num_data_shreds + self.num_coding_shreds
    }

    /// Get shred recovery timeout duration
    pub fn shred_recovery_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.shred_recovery_timeout_ms)
    }

    /// Get repair request timeout duration
    pub fn repair_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.repair_request_timeout_ms)
    }
}
