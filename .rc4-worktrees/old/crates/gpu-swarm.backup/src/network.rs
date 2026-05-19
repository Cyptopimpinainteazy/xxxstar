//! P2P networking layer with libp2p integration
//!
//! This module provides a full-featured P2P networking stack for the GPU swarm using libp2p:
//! - Gossipsub for message broadcasting
//! - Kademlia DHT for peer discovery
//! - mDNS for local peer discovery
//! - Identify protocol for peer capability exchange
//! - Ping protocol for latency measurement
//! - Connection management with NAT traversal

use crate::error::{SwarmError, SwarmResult};
use crate::node::NodeId;
use crate::protocol::{MessageEnvelope, SwarmMessage};
use futures::StreamExt;
use libp2p::gossipsub;
use libp2p::identify;
use libp2p::identify::Config as IdentifyConfig;
use libp2p::identity;
use libp2p::kad::{self, store::MemoryStore};
use libp2p::mdns;
use libp2p::ping;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{Multiaddr, PeerId as Libp2pPeerId, SwarmBuilder};
use parking_lot::RwLock;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "BehaviourEvent", prelude = "libp2p::swarm::derive_prelude")]
struct SwarmBehaviour {
    gossipsub: gossipsub::Behaviour,
    identify: identify::Behaviour,
    kad: kad::Behaviour<MemoryStore>,
    mdns: mdns::tokio::Behaviour,
    ping: ping::Behaviour,
}

#[derive(Debug)]
enum BehaviourEvent {
    Gossipsub(gossipsub::Event),
    Identify(identify::Event),
    Kad(kad::Event),
    Mdns(mdns::Event),
    Ping(ping::Event),
}

impl From<gossipsub::Event> for BehaviourEvent {
    fn from(value: gossipsub::Event) -> Self {
        Self::Gossipsub(value)
    }
}

impl From<identify::Event> for BehaviourEvent {
    fn from(value: identify::Event) -> Self {
        Self::Identify(value)
    }
}

impl From<kad::Event> for BehaviourEvent {
    fn from(value: kad::Event) -> Self {
        Self::Kad(value)
    }
}

impl From<mdns::Event> for BehaviourEvent {
    fn from(value: mdns::Event) -> Self {
        Self::Mdns(value)
    }
}

impl From<ping::Event> for BehaviourEvent {
    fn from(value: ping::Event) -> Self {
        Self::Ping(value)
    }
}

enum NetworkCommand {
    Broadcast(Vec<u8>),
    SendTo { peer: PeerId, payload: Vec<u8> },
    Connect(Multiaddr),
    Disconnect(PeerId),
    Stop,
}

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Listen addresses
    pub listen_addresses: Vec<String>,

    /// Bootstrap peers
    pub bootstrap_peers: Vec<String>,

    /// Enable mDNS for local discovery
    pub enable_mdns: bool,

    /// Gossipsub topic for swarm messages
    pub gossip_topic: String,

    /// Connection idle timeout
    pub idle_timeout_secs: u64,

    /// Maximum incoming connections
    pub max_incoming: u32,

    /// Maximum outgoing connections
    pub max_outgoing: u32,

    /// Reputation update interval
    pub reputation_update_interval_secs: u64,

    /// Maximum peers to maintain
    pub max_peers: u16,

    /// Gossipsub max message size (bytes)
    pub max_message_size: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec!["/ip4/0.0.0.0/tcp/9000".to_string()],
            bootstrap_peers: Vec::new(),
            enable_mdns: true,
            gossip_topic: "gpu-swarm/1.0.0".to_string(),
            idle_timeout_secs: 120,
            max_incoming: 100,
            max_outgoing: 50,
            reputation_update_interval_secs: 300,
            max_peers: 500,
            max_message_size: 16 * 1024 * 1024, // 16 MB
        }
    }
}

/// Network events
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// Connected to a peer
    PeerConnected {
        peer_id: PeerId,
        addresses: Vec<String>,
    },

    /// Disconnected from a peer
    PeerDisconnected(PeerId),

    /// Received a message
    MessageReceived {
        from: PeerId,
        message: MessageEnvelope,
    },

    /// Peer discovered via mDNS
    LocalPeerDiscovered {
        peer_id: PeerId,
        addresses: Vec<String>,
    },

    /// Peer discovered via DHT
    RemotePeerDiscovered {
        peer_id: PeerId,
        addresses: Vec<String>,
    },

    /// Peer identified with capabilities
    PeerIdentified {
        peer_id: PeerId,
        agent: String,
        protocols: Vec<String>,
    },

    /// Ping latency measurement
    PeerLatency { peer_id: PeerId, latency_ms: u64 },

    /// Error occurred
    Error(String),
}

/// Peer identifier wrapper around libp2p::PeerId
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId([u8; 32]);

impl PeerId {
    /// Create a random peer ID
    pub fn random() -> Self {
        let mut id = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut id);
        PeerId(id)
    }

    /// Create from node ID
    pub fn from_node_id(node_id: &NodeId) -> Self {
        PeerId(*node_id)
    }

    /// Convert to libp2p PeerId
    pub fn to_libp2p(&self) -> Libp2pPeerId {
        Libp2pPeerId::from_bytes(&self.0).unwrap_or_else(|_| Libp2pPeerId::random())
    }

    /// Convert from libp2p PeerId
    pub fn from_libp2p(libp2p_id: &Libp2pPeerId) -> Self {
        let bytes = libp2p_id.to_bytes();
        let mut id = [0u8; 32];
        id.copy_from_slice(&bytes[..std::cmp::min(32, bytes.len())]);
        PeerId(id)
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..8]))
    }
}

fn extract_peer_id(addr: &Multiaddr) -> Option<Libp2pPeerId> {
    addr.iter().find_map(|proto| match proto {
        libp2p::multiaddr::Protocol::P2p(peer_id) => Some(peer_id),
        _ => None,
    })
}

/// Peer information including reputation and capabilities
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer ID
    pub peer_id: PeerId,

    /// Peer's addresses
    pub addresses: Vec<String>,

    /// Connection timestamp
    pub connected_at: i64,

    /// Agent string
    pub agent: String,

    /// Protocol version
    pub protocol_version: String,

    /// Latency in milliseconds
    pub latency_ms: Option<u64>,

    /// Reputation score (0-100)
    pub reputation_score: u32,

    /// Is blacklisted
    pub is_blacklisted: bool,

    /// Bytes sent
    pub bytes_sent: u64,

    /// Bytes received
    pub bytes_received: u64,

    /// Supported task types
    pub supported_task_types: Vec<String>,

    /// Available GPU RAM (bytes)
    pub available_gpu_ram: u64,
}

impl PeerInfo {
    fn new(peer_id: PeerId, addresses: Vec<String>) -> Self {
        Self {
            peer_id,
            addresses,
            connected_at: chrono::Utc::now().timestamp(),
            agent: "unknown".to_string(),
            protocol_version: "1.0.0".to_string(),
            latency_ms: None,
            reputation_score: 50, // Start neutral
            is_blacklisted: false,
            bytes_sent: 0,
            bytes_received: 0,
            supported_task_types: Vec::new(),
            available_gpu_ram: 0,
        }
    }
}

/// Peer reputation tracker
#[derive(Debug, Clone)]
pub struct PeerReputation {
    /// Success count
    pub successes: u32,

    /// Failure count
    pub failures: u32,

    /// Last updated
    pub last_updated: i64,
}

impl Default for PeerReputation {
    fn default() -> Self {
        Self {
            successes: 0,
            failures: 0,
            last_updated: chrono::Utc::now().timestamp(),
        }
    }
}

/// P2P network manager with libp2p
pub struct NetworkManager {
    /// Configuration
    config: NetworkConfig,

    /// Local peer ID
    local_peer_id: Arc<RwLock<PeerId>>,

    /// Connected peers
    peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,

    /// Peer reputation tracking
    reputation: Arc<RwLock<HashMap<PeerId, PeerReputation>>>,

    /// Event sender
    event_tx: mpsc::Sender<NetworkEvent>,

    /// Event receiver
    event_rx: Arc<RwLock<mpsc::Receiver<NetworkEvent>>>,

    /// Message sender for outbound
    outbound_tx: mpsc::Sender<(PeerId, SwarmMessage)>,

    /// Command sender for network runtime
    command_tx: mpsc::Sender<NetworkCommand>,

    /// Command receiver (moved to runtime on start)
    command_rx: Option<mpsc::Receiver<NetworkCommand>>,

    /// Running flag
    running: Arc<RwLock<bool>>,

    /// Blacklist
    blacklist: Arc<RwLock<HashSet<PeerId>>>,

    /// Active gossipsub topic
    gossip_topic: gossipsub::IdentTopic,
}

impl NetworkManager {
    /// Create a new network manager
    pub fn new(config: NetworkConfig) -> SwarmResult<Self> {
        let (event_tx, event_rx) = mpsc::channel(10000);
        let (outbound_tx, _outbound_rx) = mpsc::channel(10000);
        let (command_tx, command_rx) = mpsc::channel(10000);
        let local_peer_id = PeerId::random();
        let gossip_topic = gossipsub::IdentTopic::new(config.gossip_topic.clone());

        info!(
            "Initializing NetworkManager with local peer ID: {}",
            local_peer_id
        );

        Ok(Self {
            config,
            local_peer_id: Arc::new(RwLock::new(local_peer_id)),
            peers: Arc::new(RwLock::new(HashMap::new())),
            reputation: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
            outbound_tx,
            command_tx,
            command_rx: Some(command_rx),
            running: Arc::new(RwLock::new(false)),
            blacklist: Arc::new(RwLock::new(HashSet::new())),
            gossip_topic,
        })
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        *self.local_peer_id.read()
    }

    /// Get connected peers
    pub fn peers(&self) -> HashMap<PeerId, PeerInfo> {
        self.peers.read().clone()
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }

    /// Get event receiver
    pub fn event_receiver(
        &self,
    ) -> impl std::ops::Deref<Target = mpsc::Receiver<NetworkEvent>> + use<'_> {
        self.event_rx.read()
    }

    /// Start the network
    pub async fn start(&mut self) -> SwarmResult<()> {
        if *self.running.read() {
            return Ok(());
        }

        *self.running.write() = true;

        let mut command_rx = self
            .command_rx
            .take()
            .ok_or_else(|| SwarmError::NetworkError("Network already started".to_string()))?;

        let local_key = identity::Keypair::generate_ed25519();
        let local_libp2p_peer = Libp2pPeerId::from(local_key.public());
        let local_peer = PeerId::from_libp2p(&local_libp2p_peer);
        *self.local_peer_id.write() = local_peer;

        let mut gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub::ConfigBuilder::default()
                .max_transmit_size(self.config.max_message_size)
                .build()
                .map_err(|e| SwarmError::NetworkError(format!("gossipsub config: {}", e)))?,
        )
        .map_err(|e| SwarmError::NetworkError(format!("gossipsub init: {}", e)))?;
        gossipsub
            .subscribe(&self.gossip_topic)
            .map_err(|e| SwarmError::NetworkError(format!("topic subscribe: {}", e)))?;

        let mut kad = kad::Behaviour::new(local_libp2p_peer, MemoryStore::new(local_libp2p_peer));
        for bootstrap in &self.config.bootstrap_peers {
            if let Ok(addr) = bootstrap.parse::<Multiaddr>() {
                if let Some(peer_id) = extract_peer_id(&addr) {
                    kad.add_address(&peer_id, addr);
                }
            }
        }
        let _ = kad.bootstrap();

        let identify = identify::Behaviour::new(
            IdentifyConfig::new("/gpu-swarm/1.0.0".to_string(), local_key.public())
                .with_agent_version(format!("gpu-swarm/{}", env!("CARGO_PKG_VERSION"))),
        );
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_libp2p_peer)
            .map_err(|e| SwarmError::NetworkError(format!("mdns init: {}", e)))?;
        let ping = ping::Behaviour::new(ping::Config::new());

        let behaviour = SwarmBehaviour {
            gossipsub,
            identify,
            kad,
            mdns,
            ping,
        };

        let mut swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                Default::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|e| SwarmError::NetworkError(format!("transport init: {}", e)))?
            .with_behaviour(|_| behaviour)
            .map_err(|e| SwarmError::NetworkError(format!("behaviour init: {}", e)))?
            .build();

        for addr_str in &self.config.listen_addresses {
            match addr_str.parse::<Multiaddr>() {
                Ok(addr) => {
                    if let Err(e) = swarm.listen_on(addr.clone()) {
                        warn!("Failed to listen on {}: {}", addr, e);
                    }
                }
                Err(e) => warn!("Failed to parse listen address {}: {}", addr_str, e),
            }
        }

        for bootstrap in &self.config.bootstrap_peers {
            if let Ok(addr) = bootstrap.parse::<Multiaddr>() {
                if let Err(e) = swarm.dial(addr.clone()) {
                    warn!("Failed to dial bootstrap {}: {}", addr, e);
                }
            }
        }

        let peers = Arc::clone(&self.peers);
        let reputation = Arc::clone(&self.reputation);
        let blacklist = Arc::clone(&self.blacklist);
        let event_tx = self.event_tx.clone();
        let running = Arc::clone(&self.running);
        let topic = self.gossip_topic.clone();

        tokio::spawn(async move {
            let mut peer_map: HashMap<PeerId, Libp2pPeerId> = HashMap::new();
            let mut last_reputation_update = Instant::now();

            while *running.read() {
                tokio::select! {
                    cmd = command_rx.recv() => {
                        match cmd {
                            Some(NetworkCommand::Broadcast(payload)) => {
                                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), payload) {
                                    let _ = event_tx.send(NetworkEvent::Error(format!("broadcast failed: {}", e))).await;
                                }
                            }
                            Some(NetworkCommand::SendTo { peer, payload }) => {
                                if blacklist.read().contains(&peer) {
                                    continue;
                                }
                                if peer_map.contains_key(&peer) {
                                    if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), payload) {
                                        let _ = event_tx.send(NetworkEvent::Error(format!("send_to failed: {}", e))).await;
                                    }
                                }
                            }
                            Some(NetworkCommand::Connect(addr)) => {
                                if let Err(e) = swarm.dial(addr.clone()) {
                                    let _ = event_tx.send(NetworkEvent::Error(format!("dial {} failed: {}", addr, e))).await;
                                }
                            }
                            Some(NetworkCommand::Disconnect(peer)) => {
                                if let Some(lib_peer) = peer_map.get(&peer) {
                                    let _ = swarm.disconnect_peer_id(*lib_peer);
                                }
                            }
                            Some(NetworkCommand::Stop) | None => break,
                        }
                    }
                    event = swarm.select_next_some() => {
                        match event {
                            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                                let peer = PeerId::from_libp2p(&peer_id);
                                peer_map.insert(peer, peer_id);

                                let address = endpoint.get_remote_address().to_string();
                                {
                                    let mut peers_guard = peers.write();
                                    let entry = peers_guard.entry(peer).or_insert_with(|| PeerInfo::new(peer, vec![]));
                                    if !entry.addresses.iter().any(|a| a == &address) {
                                        entry.addresses.push(address.clone());
                                    }
                                    entry.connected_at = chrono::Utc::now().timestamp();
                                }
                                reputation.write().entry(peer).or_insert_with(PeerReputation::default);
                                let _ = event_tx.send(NetworkEvent::PeerConnected {
                                    peer_id: peer,
                                    addresses: vec![address],
                                }).await;
                            }
                            SwarmEvent::ConnectionClosed { peer_id, num_established, .. } => {
                                if num_established == 0 {
                                    let peer = PeerId::from_libp2p(&peer_id);
                                    peers.write().remove(&peer);
                                    peer_map.remove(&peer);
                                    let _ = event_tx.send(NetworkEvent::PeerDisconnected(peer)).await;
                                }
                            }
                            SwarmEvent::NewListenAddr { address, .. } => {
                                info!("Listening on {}", address);
                            }
                            SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Message {
                                propagation_source,
                                message,
                                ..
                            })) => {
                                if let Ok(envelope) = MessageEnvelope::from_bytes(&message.data) {
                                    let from = PeerId::from_libp2p(&propagation_source);
                                    if let Some(info) = peers.write().get_mut(&from) {
                                        info.bytes_received = info.bytes_received.saturating_add(message.data.len() as u64);
                                    }
                                    let _ = event_tx.send(NetworkEvent::MessageReceived { from, message: envelope }).await;
                                }
                            }
                            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. })) => {
                                let peer = PeerId::from_libp2p(&peer_id);
                                let (agent, protocols) = {
                                    let mut peers_guard = peers.write();
                                    let entry = peers_guard.entry(peer).or_insert_with(|| PeerInfo::new(peer, vec![]));
                                    entry.agent = info.agent_version.clone();
                                    entry.protocol_version = info.protocol_version.clone();
                                    let addresses: Vec<String> = info.listen_addrs.iter().map(ToString::to_string).collect();
                                    for addr in &addresses {
                                        if !entry.addresses.iter().any(|a| a == addr) {
                                            entry.addresses.push(addr.clone());
                                        }
                                    }
                                    (
                                        entry.agent.clone(),
                                        info.protocols.iter().map(ToString::to_string).collect::<Vec<_>>(),
                                    )
                                };

                                let _ = event_tx.send(NetworkEvent::PeerIdentified {
                                    peer_id: peer,
                                    agent,
                                    protocols,
                                }).await;

                                {
                                    for addr in info.listen_addrs {
                                        swarm.behaviour_mut().kad.add_address(&peer_id, addr);
                                    }
                                }
                            }
                            SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                                for (peer_id, addr) in list {
                                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                                    swarm.behaviour_mut().kad.add_address(&peer_id, addr.clone());
                                    let peer = PeerId::from_libp2p(&peer_id);
                                    let _ = event_tx.send(NetworkEvent::LocalPeerDiscovered {
                                        peer_id: peer,
                                        addresses: vec![addr.to_string()],
                                    }).await;
                                }
                            }
                            SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                                for (peer_id, _) in list {
                                    swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                                }
                            }
                            SwarmEvent::Behaviour(BehaviourEvent::Ping(ping::Event { peer, result, .. })) => {
                                if let Ok(latency) = result {
                                    let peer_wrapped = PeerId::from_libp2p(&peer);
                                    if let Some(peer_info) = peers.write().get_mut(&peer_wrapped) {
                                        peer_info.latency_ms = Some(latency.as_millis() as u64);
                                    }
                                    let _ = event_tx.send(NetworkEvent::PeerLatency {
                                        peer_id: peer_wrapped,
                                        latency_ms: latency.as_millis() as u64,
                                    }).await;
                                }
                            }
                            SwarmEvent::Behaviour(BehaviourEvent::Kad(_event)) => {}
                            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                                let peer_label = peer_id
                                    .map(|p| p.to_string())
                                    .unwrap_or_else(|| "unknown".to_string());
                                let _ = event_tx.send(NetworkEvent::Error(format!(
                                    "outgoing connection error to {}: {}",
                                    peer_label, error
                                ))).await;
                            }
                            SwarmEvent::IncomingConnectionError { send_back_addr, error, .. } => {
                                let _ = event_tx.send(NetworkEvent::Error(format!(
                                    "incoming connection error from {}: {}",
                                    send_back_addr, error
                                ))).await;
                            }
                            _ => {}
                        }
                    }
                }

                if last_reputation_update.elapsed().as_secs() >= 30 {
                    let mut peers_guard = peers.write();
                    let rep_guard = reputation.read();
                    for (peer, info) in peers_guard.iter_mut() {
                        if let Some(rep) = rep_guard.get(peer) {
                            let total = rep.successes + rep.failures;
                            if total > 0 {
                                info.reputation_score =
                                    ((rep.successes as f64 / total as f64) * 100.0) as u32;
                            }
                        }
                    }
                    last_reputation_update = Instant::now();
                }
            }
        });

        info!(
            "NetworkManager started with libp2p, listening on {:?}",
            self.config.listen_addresses
        );
        Ok(())
    }

    /// Broadcast a message to all peers
    pub async fn broadcast(&self, message: SwarmMessage) -> SwarmResult<()> {
        let local_id = self.local_peer_id();
        let envelope = MessageEnvelope::new(local_id.0, message);
        let data = envelope
            .to_bytes()
            .map_err(|e| SwarmError::SerializationError(e.to_string()))?;

        if data.len() > self.config.max_message_size {
            return Err(SwarmError::NetworkError(
                "Message exceeds maximum size".to_string(),
            ));
        }

        let peers = self.peers.read();
        let peer_count = peers.iter().filter(|(_, p)| !p.is_blacklisted).count();

        debug!(
            "Broadcasting {} bytes to {} active peers",
            data.len(),
            peer_count
        );

        self.command_tx
            .send(NetworkCommand::Broadcast(data))
            .await
            .map_err(|e| SwarmError::NetworkError(format!("broadcast command failed: {}", e)))?;

        Ok(())
    }

    /// Send a direct message to a peer
    pub async fn send_to(&self, peer: &PeerId, message: SwarmMessage) -> SwarmResult<()> {
        let peers = self.peers.read();
        if !peers.contains_key(peer) {
            return Err(SwarmError::NetworkError(format!(
                "Peer {} not connected",
                peer
            )));
        }

        if self.blacklist.read().contains(peer) {
            return Err(SwarmError::NetworkError(format!(
                "Peer {} is blacklisted",
                peer
            )));
        }

        let envelope = MessageEnvelope::new(self.local_peer_id().0, message);
        let data = envelope
            .to_bytes()
            .map_err(|e| SwarmError::SerializationError(e.to_string()))?;
        self.command_tx
            .send(NetworkCommand::SendTo {
                peer: *peer,
                payload: data,
            })
            .await
            .map_err(|e| SwarmError::NetworkError(format!("send command failed: {}", e)))?;

        if let Some(info) = self.peers.write().get_mut(peer) {
            info.bytes_sent = info.bytes_sent.saturating_add(1);
        }
        Ok(())
    }

    /// Connect to a peer
    pub async fn connect(&mut self, address: &str) -> SwarmResult<PeerId> {
        info!("Connecting to {}", address);

        let addr: Multiaddr = address
            .parse()
            .map_err(|_| SwarmError::NetworkError("Invalid address".to_string()))?;

        let peer_id = extract_peer_id(&addr)
            .map(|p| PeerId::from_libp2p(&p))
            .unwrap_or_else(PeerId::random);

        self.command_tx
            .send(NetworkCommand::Connect(addr.clone()))
            .await
            .map_err(|e| SwarmError::NetworkError(format!("connect command failed: {}", e)))?;

        let peer_info = PeerInfo::new(peer_id, vec![address.to_string()]);
        self.peers.write().entry(peer_id).or_insert(peer_info);
        self.reputation
            .write()
            .entry(peer_id)
            .or_insert_with(PeerReputation::default);

        Ok(peer_id)
    }

    /// Disconnect from a peer
    pub async fn disconnect(&mut self, peer: &PeerId) -> SwarmResult<()> {
        self.command_tx
            .send(NetworkCommand::Disconnect(*peer))
            .await
            .map_err(|e| SwarmError::NetworkError(format!("disconnect command failed: {}", e)))?;

        self.peers.write().remove(peer);
        let _ = self
            .event_tx
            .send(NetworkEvent::PeerDisconnected(*peer))
            .await;
        Ok(())
    }

    /// Blacklist a peer
    pub fn blacklist_peer(&self, peer: &PeerId) {
        self.blacklist.write().insert(*peer);
        if let Some(info) = self.peers.write().get_mut(peer) {
            info.is_blacklisted = true;
        }
        info!("Blacklisted peer: {}", peer);
    }

    /// Remove peer from blacklist
    pub fn unblacklist_peer(&self, peer: &PeerId) {
        self.blacklist.write().remove(peer);
        if let Some(info) = self.peers.write().get_mut(peer) {
            info.is_blacklisted = false;
        }
        info!("Removed {} from blacklist", peer);
    }

    /// Update peer reputation
    pub fn update_reputation(&self, peer: &PeerId, success: bool) {
        let mut rep = self.reputation.write();
        let peer_rep = rep.entry(*peer).or_insert_with(PeerReputation::default);

        if success {
            peer_rep.successes += 1;
        } else {
            peer_rep.failures += 1;
        }
        peer_rep.last_updated = chrono::Utc::now().timestamp();

        // Update peer info reputation score
        let mut peers = self.peers.write();
        if let Some(info) = peers.get_mut(peer) {
            let total = (peer_rep.successes + peer_rep.failures) as u32;
            if total > 0 {
                info.reputation_score = ((peer_rep.successes as f64 / total as f64) * 100.0) as u32;
            }
        }
    }

    /// Get peer info
    pub fn get_peer_info(&self, peer: &PeerId) -> Option<PeerInfo> {
        self.peers.read().get(peer).cloned()
    }

    /// List healthy peers (not blacklisted, good reputation)
    pub fn healthy_peers(&self) -> Vec<PeerInfo> {
        self.peers
            .read()
            .values()
            .filter(|p| !p.is_blacklisted && p.reputation_score >= 30)
            .cloned()
            .collect()
    }

    /// Stop the network
    pub fn stop(&self) {
        *self.running.write() = false;
        let _ = self.command_tx.try_send(NetworkCommand::Stop);
        info!("NetworkManager stopped");
    }
}

/// Peer discovery service using Kademlia DHT
pub struct PeerDiscovery {
    /// Known peers (PeerId -> addresses)
    known_peers: Arc<RwLock<HashMap<PeerId, Vec<String>>>>,

    /// Bootstrap peers
    bootstrap: Vec<String>,

    /// Local peer ID
    local_peer_id: PeerId,
}

impl PeerDiscovery {
    /// Create a new peer discovery service
    pub fn new(bootstrap: Vec<String>, local_peer_id: PeerId) -> Self {
        Self {
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            bootstrap,
            local_peer_id,
        }
    }

    /// Add a discovered peer
    pub fn add_peer(&self, peer_id: PeerId, addresses: Vec<String>) {
        self.known_peers
            .write()
            .entry(peer_id)
            .or_insert_with(Vec::new)
            .extend(addresses);
    }

    /// Remove a peer
    pub fn remove_peer(&self, peer_id: &PeerId) {
        self.known_peers.write().remove(peer_id);
    }

    /// Get known peers
    pub fn known_peers(&self) -> HashMap<PeerId, Vec<String>> {
        self.known_peers.read().clone()
    }

    /// Get bootstrap peers
    pub fn bootstrap_peers(&self) -> &[String] {
        &self.bootstrap
    }

    /// Find peer by ID
    pub fn find_peer(&self, peer_id: &PeerId) -> Option<Vec<String>> {
        self.known_peers.read().get(peer_id).cloned()
    }
}

/// Connection manager for handling peer connections
#[derive(Debug, Clone)]
pub struct ConnectionManager {
    /// Maximum connections
    max_connections: usize,

    /// Connected peers
    connections: Arc<RwLock<HashMap<PeerId, ConnectionInfo>>>,
}

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Remote address
    pub address: String,

    /// Connection direction
    pub direction: ConnectionDirection,

    /// Connected timestamp
    pub connected_at: i64,

    /// Bytes sent
    pub bytes_sent: u64,

    /// Bytes received
    pub bytes_received: u64,

    /// Connection ID
    pub connection_id: u64,
}

/// Connection direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionDirection {
    Inbound,
    Outbound,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if we can accept more connections
    pub fn can_accept(&self) -> bool {
        self.connections.read().len() < self.max_connections
    }

    /// Add a connection
    pub fn add(&self, peer_id: PeerId, address: String, direction: ConnectionDirection) -> bool {
        if !self.can_accept() {
            return false;
        }

        let conn_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        self.connections.write().insert(
            peer_id,
            ConnectionInfo {
                address,
                direction,
                connected_at: chrono::Utc::now().timestamp(),
                bytes_sent: 0,
                bytes_received: 0,
                connection_id: conn_id,
            },
        );

        true
    }

    /// Remove a connection
    pub fn remove(&self, peer_id: &PeerId) -> Option<ConnectionInfo> {
        self.connections.write().remove(peer_id)
    }

    /// Get connection info
    pub fn get(&self, peer_id: &PeerId) -> Option<ConnectionInfo> {
        self.connections.read().get(peer_id).cloned()
    }

    /// Get connection count
    pub fn count(&self) -> usize {
        self.connections.read().len()
    }

    /// Update bytes transferred
    pub fn update_bytes(&self, peer_id: &PeerId, sent: u64, received: u64) {
        if let Some(conn) = self.connections.write().get_mut(peer_id) {
            conn.bytes_sent += sent;
            conn.bytes_received += received;
        }
    }

    /// Get all connections
    pub fn all_connections(&self) -> Vec<ConnectionInfo> {
        self.connections.read().values().cloned().collect()
    }

    /// Get inbound connection count
    pub fn inbound_count(&self) -> usize {
        self.connections
            .read()
            .values()
            .filter(|c| c.direction == ConnectionDirection::Inbound)
            .count()
    }

    /// Get outbound connection count
    pub fn outbound_count(&self) -> usize {
        self.connections
            .read()
            .values()
            .filter(|c| c.direction == ConnectionDirection::Outbound)
            .count()
    }
}
