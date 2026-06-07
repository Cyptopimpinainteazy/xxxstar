//! Network module for X3 GPU Validator Swarm
//! Simplified implementation for cross-platform compatibility

use crate::error::{SwarmError, SwarmResult};
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

/// Network trait for P2P communication
#[async_trait]
pub trait Network: Send + Sync {
    /// Connect to a peer
    async fn connect(&mut self, peer_id: &str, address: &str) -> Result<(), SwarmError>;

    /// Disconnect from a peer
    async fn disconnect(&mut self, peer_id: &str) -> Result<(), SwarmError>;

    /// Send a message to a peer
    async fn send(&mut self, peer_id: &str, message: Vec<u8>) -> Result<(), SwarmError>;

    /// Broadcast a message to all peers
    async fn broadcast(&mut self, message: Vec<u8>) -> Result<(), SwarmError>;

    /// Get local peer ID
    fn local_peer_id(&self) -> &str;

    /// Get list of connected peers
    fn get_peers(&self) -> Vec<String>;
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen addresses
    pub listen_addresses: Vec<String>,
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<String>,
    /// Max peers
    pub max_peers: usize,
    /// Enable mDNS
    pub enable_mdns: bool,
    /// Enable DHT
    pub enable_dht: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec!["/ip4/0.0.0.0/tcp/30334".to_string()],
            bootstrap_nodes: vec![],
            max_peers: 100,
            enable_mdns: false,
            enable_dht: false,
        }
    }
}

/// Network event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NetworkEvent {
    /// Peer connected
    PeerConnected { peer_id: String },
    /// Peer disconnected
    PeerDisconnected { peer_id: String },
    /// Message received
    MessageReceived { peer_id: String, message: Vec<u8> },
    /// New external address
    NewExternalAddress { address: String },
}

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NetworkMessage {
    /// Task request
    TaskRequest { task_id: String, data: Vec<u8> },
    /// Task result
    TaskResult { task_id: String, result: Vec<u8> },
    /// Ping
    Ping { nonce: u64 },
    /// Pong
    Pong { nonce: u64 },
}

/// Simple network peer
#[derive(Debug, Clone)]
pub struct NetworkPeer {
    /// Peer ID
    pub peer_id: String,
    /// Address
    pub address: Option<String>,
    /// Last seen
    pub last_seen: u64,
    /// Is connected
    pub connected: bool,
}

/// Network manager for P2P communication
pub struct NetworkManager {
    /// Configuration
    _config: NetworkConfig,
    /// Local peer ID
    local_peer_id: String,
    /// Connected peers
    peers: Arc<RwLock<HashMap<String, NetworkPeer>>>,
    /// Event sender
    event_tx: mpsc::UnboundedSender<NetworkEvent>,
    /// Running flag
    running: Arc<RwLock<bool>>,
}

impl NetworkManager {
    /// Create a new network manager
    pub fn new(config: NetworkConfig) -> Self {
        let (event_tx, _) = mpsc::unbounded_channel();
        let local_peer_id = Self::generate_peer_id();

        Self {
            _config: config,
            local_peer_id,
            peers: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Generate a simple peer ID
    fn generate_peer_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("x3_{:x}", timestamp)
    }

    /// Start the network manager
    pub async fn start(&self) -> SwarmResult<()> {
        *self.running.write() = true;
        info!(
            "Network manager started with peer ID: {}",
            self.local_peer_id
        );
        Ok(())
    }

    /// Stop the network manager
    pub async fn stop(&self) -> SwarmResult<()> {
        *self.running.write() = false;
        self.peers.write().clear();
        info!("Network manager stopped");
        Ok(())
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> &str {
        &self.local_peer_id
    }

    /// Connect to a peer
    pub async fn connect(&self, peer_id: &str, address: &str) -> SwarmResult<()> {
        let peer = NetworkPeer {
            peer_id: peer_id.to_string(),
            address: Some(address.to_string()),
            last_seen: 0,
            connected: true,
        };
        self.peers.write().insert(peer_id.to_string(), peer);

        let _ = self.event_tx.send(NetworkEvent::PeerConnected {
            peer_id: peer_id.to_string(),
        });

        info!("Connected to peer: {} at {}", peer_id, address);
        Ok(())
    }

    /// Disconnect from a peer
    pub async fn disconnect(&self, peer_id: &str) -> SwarmResult<()> {
        self.peers.write().remove(peer_id);

        let _ = self.event_tx.send(NetworkEvent::PeerDisconnected {
            peer_id: peer_id.to_string(),
        });

        info!("Disconnected from peer: {}", peer_id);
        Ok(())
    }

    /// Send a message to a peer
    pub async fn send(&self, peer_id: &str, message: NetworkMessage) -> SwarmResult<()> {
        let peers = self.peers.read();
        if !peers.contains_key(peer_id) {
            return Err(SwarmError::PeerNotFound(peer_id.to_string()));
        }

        // In a real implementation, this would send over the network
        info!("Sending message to {}: {:?}", peer_id, message);
        Ok(())
    }

    /// Broadcast a message to all peers
    pub async fn broadcast(&self, message: NetworkMessage) -> SwarmResult<()> {
        let peers: Vec<_> = self.peers.read().keys().cloned().collect();
        for peer_id in peers {
            self.send(&peer_id, message.clone()).await?;
        }
        Ok(())
    }

    /// Get list of connected peers
    pub fn get_peers(&self) -> Vec<String> {
        self.peers.read().keys().cloned().collect()
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }

    /// Check if connected to a peer
    pub fn is_connected(&self, peer_id: &str) -> bool {
        self.peers.read().contains_key(peer_id)
    }

    /// Get event receiver
    pub fn event_receiver(&self) -> mpsc::UnboundedReceiver<NetworkEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        // Keep tx alive - in production would store it
        let _ = tx;
        rx
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }
}

#[async_trait]
impl crate::network::Network for NetworkManager {
    async fn connect(&mut self, peer_id: &str, address: &str) -> Result<(), SwarmError> {
        NetworkManager::connect(self, peer_id, address).await
    }

    async fn disconnect(&mut self, peer_id: &str) -> Result<(), SwarmError> {
        NetworkManager::disconnect(self, peer_id).await
    }

    async fn send(&mut self, peer_id: &str, message: Vec<u8>) -> Result<(), SwarmError> {
        let msg = NetworkMessage::TaskRequest {
            task_id: "unknown".to_string(),
            data: message,
        };
        NetworkManager::send(self, peer_id, msg).await
    }

    async fn broadcast(&mut self, message: Vec<u8>) -> Result<(), SwarmError> {
        let msg = NetworkMessage::TaskRequest {
            task_id: "unknown".to_string(),
            data: message,
        };
        NetworkManager::broadcast(self, msg).await
    }

    fn local_peer_id(&self) -> &str {
        &self.local_peer_id
    }

    fn get_peers(&self) -> Vec<String> {
        NetworkManager::get_peers(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_manager() {
        let config = NetworkConfig::default();
        let network = NetworkManager::new(config);

        network.start().await.unwrap();
        assert!(network.is_running());

        network.stop().await.unwrap();
        assert!(!network.is_running());
    }

    #[tokio::test]
    async fn test_peer_connection() {
        let config = NetworkConfig::default();
        let network = NetworkManager::new(config);

        network.start().await.unwrap();

        network
            .connect("peer1", "/ip4/1.2.3.4/tcp/30334")
            .await
            .unwrap();
        assert!(network.is_connected("peer1"));
        assert_eq!(network.peer_count(), 1);

        network.disconnect("peer1").await.unwrap();
        assert!(!network.is_connected("peer1"));
    }
}
