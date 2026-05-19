// crates/gpu-swarm/src/performance/network_tuning.rs
// Network performance tuning

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{debug, span, Level};

pub struct NetworkTuner {
    // TCP tuning
    tcp_window_size: Arc<AtomicU32>,
    tcp_buffer_size: Arc<AtomicU32>,
    tcp_keepalive_interval: Arc<AtomicU32>,

    // UDP/Datagram tuning
    udp_buffer_size: Arc<AtomicU32>,
    udp_batch_size: Arc<AtomicU32>,

    // Gossip protocol tuning
    gossip_fanout: Arc<AtomicU32>,
    gossip_interval_ms: Arc<AtomicU32>,
    gossip_max_peers: Arc<AtomicU32>,

    // Metrics
    packets_sent: Arc<AtomicU32>,
    packets_received: Arc<AtomicU32>,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    packet_loss_count: Arc<AtomicU32>,
}

impl NetworkTuner {
    pub fn new() -> Self {
        NetworkTuner {
            tcp_window_size: Arc::new(AtomicU32::new(65536)),
            tcp_buffer_size: Arc::new(AtomicU32::new(131072)),
            tcp_keepalive_interval: Arc::new(AtomicU32::new(300)),
            udp_buffer_size: Arc::new(AtomicU32::new(65536)),
            udp_batch_size: Arc::new(AtomicU32::new(32)),
            gossip_fanout: Arc::new(AtomicU32::new(4)),
            gossip_interval_ms: Arc::new(AtomicU32::new(100)),
            gossip_max_peers: Arc::new(AtomicU32::new(100)),
            packets_sent: Arc::new(AtomicU32::new(0)),
            packets_received: Arc::new(AtomicU32::new(0)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            packet_loss_count: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Optimize TCP window size based on latency
    pub fn optimize_tcp_window(&self, latency_ms: u32) {
        let span = span!(Level::DEBUG, "optimize_tcp_window", latency = latency_ms);
        let _enter = span.enter();

        // BDP = Bandwidth * Delay
        // Assume 1 Gbps bandwidth
        let bandwidth_mbps = 1000u32;
        let bdp = (bandwidth_mbps / 8) * latency_ms; // in KB
        let recommended_window = (bdp * 2).max(65536).min(2097152);

        self.tcp_window_size.store(recommended_window, Ordering::Relaxed);
        debug!("📊 TCP window optimized to {} bytes", recommended_window);
    }

    /// Optimize UDP batching
    pub fn optimize_udp_batch(&self, throughput_mbps: u32) {
        let span = span!(Level::DEBUG, "optimize_udp_batch", throughput = throughput_mbps);
        let _enter = span.enter();

        // Scale batch size based on throughput
        let batch_size = match throughput_mbps {
            0..=100 => 16,
            101..=500 => 32,
            501..=1000 => 64,
            _ => 128,
        };

        self.udp_batch_size.store(batch_size, Ordering::Relaxed);
        debug!("📦 UDP batch size optimized to {}", batch_size);
    }

    /// Optimize gossip protocol parameters
    pub fn optimize_gossip(&self, peer_count: u32) {
        let span = span!(Level::DEBUG, "optimize_gossip", peers = peer_count);
        let _enter = span.enter();

        // Fanout: log10(peer_count) + 1, clamped to 2-8
        let fanout = ((peer_count as f32).log10() as u32 + 1).max(2).min(8);

        // Interval: increase with peer count to reduce overhead
        let interval_ms = match peer_count {
            0..=10 => 100,
            11..=50 => 150,
            51..=100 => 200,
            _ => 300,
        };

        self.gossip_fanout.store(fanout, Ordering::Relaxed);
        self.gossip_interval_ms.store(interval_ms, Ordering::Relaxed);

        debug!("🔄 Gossip optimized: fanout={}, interval={}ms", fanout, interval_ms);
    }

    /// Record packet sent
    pub fn record_packet_sent(&self, bytes: u32) {
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Record packet received
    pub fn record_packet_received(&self, bytes: u32) {
        self.packets_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Record packet loss
    pub fn record_packet_loss(&self) {
        self.packet_loss_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get network stats
    pub fn get_stats(&self) -> NetworkStats {
        NetworkStats {
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            packets_received: self.packets_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            packet_loss_count: self.packet_loss_count.load(Ordering::Relaxed),
            tcp_window: self.tcp_window_size.load(Ordering::Relaxed),
            gossip_fanout: self.gossip_fanout.load(Ordering::Relaxed),
            gossip_interval_ms: self.gossip_interval_ms.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub packets_sent: u32,
    pub packets_received: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packet_loss_count: u32,
    pub tcp_window: u32,
    pub gossip_fanout: u32,
    pub gossip_interval_ms: u32,
}

impl Default for NetworkTuner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_optimization() {
        let tuner = NetworkTuner::new();
        tuner.optimize_tcp_window(50);
        let stats = tuner.get_stats();
        assert!(stats.tcp_window > 65536);
    }

    #[test]
    fn test_gossip_optimization() {
        let tuner = NetworkTuner::new();
        tuner.optimize_gossip(50);
        let stats = tuner.get_stats();
        assert!(stats.gossip_fanout > 1);
        assert!(stats.gossip_interval_ms > 100);
    }

    #[test]
    fn test_packet_recording() {
        let tuner = NetworkTuner::new();
        tuner.record_packet_sent(1024);
        tuner.record_packet_received(512);
        tuner.record_packet_loss();

        let stats = tuner.get_stats();
        assert_eq!(stats.packets_sent, 1);
        assert_eq!(stats.packets_received, 1);
        assert_eq!(stats.bytes_sent, 1024);
        assert_eq!(stats.bytes_received, 512);
        assert_eq!(stats.packet_loss_count, 1);
    }
}
