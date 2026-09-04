//! Network Parameters Module

use serde::{Deserialize, Serialize};

/// Network parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkParams {
    /// Maximum peers
    pub max_peers: usize,
    /// Maximum connections per IP
    pub max_connections_per_ip: usize,
    /// Connection timeout in ms
    pub connection_timeout_ms: u64,
    /// Request timeout in ms
    pub request_timeout_ms: u64,
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    /// Maximum pending transactions
    pub max_pending_transactions: usize,
    /// Maximum incoming buffer size
    pub max_incoming_buffer_size: usize,
    /// Maximum outgoing buffer size
    pub max_outgoing_buffer_size: usize,
    /// Enable UDP for shreds
    pub enable_udp_shreds: bool,
    /// Enable TCP for transactions
    pub enable_tcp_transactions: bool,
}

impl Default for NetworkParams {
    fn default() -> Self {
        Self {
            max_peers: 100,
            max_connections_per_ip: 5,
            connection_timeout_ms: 5000,
            request_timeout_ms: 10000,
            max_concurrent_requests: 1000,
            max_pending_transactions: 50000,
            max_incoming_buffer_size: 10_000_000,
            max_outgoing_buffer_size: 10_000_000,
            enable_udp_shreds: true,
            enable_tcp_transactions: true,
        }
    }
}

impl NetworkParams {
    /// High throughput configuration
    pub fn high_throughput() -> Self {
        Self {
            max_peers: 200,
            max_connections_per_ip: 10,
            connection_timeout_ms: 3000,
            request_timeout_ms: 5000,
            max_concurrent_requests: 5000,
            max_pending_transactions: 100000,
            max_incoming_buffer_size: 50_000_000,
            max_outgoing_buffer_size: 50_000_000,
            enable_udp_shreds: true,
            enable_tcp_transactions: true,
        }
    }

    /// Low latency configuration
    pub fn low_latency() -> Self {
        Self {
            max_peers: 50,
            max_connections_per_ip: 3,
            connection_timeout_ms: 2000,
            request_timeout_ms: 3000,
            max_concurrent_requests: 500,
            max_pending_transactions: 25000,
            max_incoming_buffer_size: 5_000_000,
            max_outgoing_buffer_size: 5_000_000,
            enable_udp_shreds: true,
            enable_tcp_transactions: true,
        }
    }

    /// Archival node configuration
    pub fn archival() -> Self {
        Self {
            max_peers: 300,
            max_connections_per_ip: 3,
            connection_timeout_ms: 10000,
            request_timeout_ms: 30000,
            max_concurrent_requests: 2000,
            max_pending_transactions: 100000,
            max_incoming_buffer_size: 100_000_000,
            max_outgoing_buffer_size: 100_000_000,
            enable_udp_shreds: false,
            enable_tcp_transactions: true,
        }
    }

    /// Validate network parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.max_peers == 0 {
            return Err("max_peers must be > 0".into());
        }
        
        if self.max_pending_transactions == 0 {
            return Err("max_pending_transactions must be > 0".into());
        }
        
        Ok(())
    }
}