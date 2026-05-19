//! Peer Module - P2P peer management

use crate::config::TurbineConfig;
use crate::error::TurbineResult;
use lru::LruCache;
use parking_lot::RwLock;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Peer role in the network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerRole {
    /// Validator node
    Validator,
    /// RPC node
    Rpc,
    /// Archive node
    Archive,
    /// Regular peer
    Peer,
}

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub id: String,
    pub address: String,
    pub role: PeerRole,
    pub stake: u64,
    pub latency_ms: u64,
    pub last_seen: Instant,
    pub is_active: bool,
}

impl PeerInfo {
    /// Create new peer info
    pub fn new(id: String, address: String, role: PeerRole) -> Self {
        Self {
            id,
            address,
            role,
            stake: 0,
            latency_ms: 0,
            last_seen: Instant::now(),
            is_active: true,
        }
    }
}

/// Peer connection state
#[derive(Debug)]
struct PeerState {
    peer: PeerInfo,
    _shreds_received: u64,
    _shreds_sent: u64,
    _last_slot: Option<u64>,
}

/// Peer manager for handling peer connections
pub struct PeerManager {
    _config: TurbineConfig,
    peers: RwLock<HashMap<String, PeerState>>,
    recent_slots: RwLock<VecDeque<u64>>,
    peer_cache: RwLock<LruCache<String, PeerInfo>>,
}

impl PeerManager {
    /// Create new peer manager
    pub fn new(config: TurbineConfig) -> Self {
        let peer_cache = LruCache::new(
            NonZeroUsize::new(config.peer_cache_size.max(1))
                .expect("peer cache size must be non-zero"),
        );

        Self {
            _config: config,
            peers: RwLock::new(HashMap::new()),
            recent_slots: RwLock::new(VecDeque::new()),
            peer_cache: RwLock::new(peer_cache),
        }
    }

    /// Start the peer manager
    pub async fn start(&self) -> TurbineResult<()> {
        info!("Starting peer manager");
        // In real implementation, would connect to bootstrap nodes
        Ok(())
    }

    /// Stop the peer manager
    pub async fn stop(&self) -> TurbineResult<()> {
        info!("Stopping peer manager");
        Ok(())
    }

    /// Add a peer
    pub fn add_peer(&self, peer: PeerInfo) {
        let mut peers = self.peers.write();
        peers.insert(
            peer.id.clone(),
            PeerState {
                peer: peer.clone(),
                _shreds_received: 0,
                _shreds_sent: 0,
                _last_slot: None,
            },
        );

        // Update cache
        self.peer_cache.write().put(peer.id.clone(), peer);
    }

    /// Remove a peer
    pub fn remove_peer(&self, peer_id: &str) {
        self.peers.write().remove(peer_id);
        self.peer_cache.write().pop(peer_id);
    }

    /// Get peer by ID
    pub fn get_peer(&self, peer_id: &str) -> Option<PeerInfo> {
        self.peers.read().get(peer_id).map(|s| s.peer.clone())
    }

    /// Get all active peers
    pub fn get_active_peers(&self) -> Vec<PeerInfo> {
        self.peers
            .read()
            .values()
            .filter(|s| s.peer.is_active)
            .map(|s| s.peer.clone())
            .collect()
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }

    /// Get peers for a specific slot
    pub fn get_peers_for_slot(&self, slot: u64, max_peers: usize) -> Vec<PeerInfo> {
        // Update recent slots
        {
            let mut slots = self.recent_slots.write();
            if slots.len() > 100 {
                slots.pop_front();
            }
            slots.push_back(slot);
        }

        // Get peers sorted by stake (higher stake = more likely to have data)
        let mut peers: Vec<_> = self
            .peers
            .read()
            .values()
            .filter(|s| s.peer.is_active)
            .map(|s| &s.peer)
            .cloned()
            .collect();

        // Sort by stake (descending)
        peers.sort_by(|a, b| b.stake.cmp(&a.stake));

        peers.into_iter().take(max_peers).collect()
    }

    /// Request shreds from peers
    pub async fn request_shreds(&self, slot: u64, indices: &[u32]) -> TurbineResult<()> {
        debug!("Requesting shreds for slot {}: {:?}", slot, indices);

        let peers = self.get_peers_for_slot(slot, 5);

        for peer in peers {
            debug!("Requesting from peer: {}", peer.id);
            // In real implementation, would send actual request
        }

        Ok(())
    }

    /// Update peer latency
    pub fn update_latency(&self, peer_id: &str, latency_ms: u64) {
        if let Some(state) = self.peers.write().get_mut(peer_id) {
            state.peer.latency_ms = latency_ms;
            state.peer.last_seen = Instant::now();
        }
    }

    /// Update peer stake
    pub fn update_stake(&self, peer_id: &str, stake: u64) {
        if let Some(state) = self.peers.write().get_mut(peer_id) {
            state.peer.stake = stake;
        }
    }

    /// Check for stale peers
    pub fn cleanup_stale_peers(&self, max_age: Duration) {
        let mut peers = self.peers.write();
        peers.retain(|_, state| state.peer.last_seen.elapsed() < max_age);
    }

    /// Get peers by role
    pub fn get_peers_by_role(&self, role: PeerRole) -> Vec<PeerInfo> {
        self.peers
            .read()
            .values()
            .filter(|s| s.peer.role == role && s.peer.is_active)
            .map(|s| s.peer.clone())
            .collect()
    }

    /// Build a Turbine propagation tree for a specific slot and shred index.
    ///
    /// This method replicates the core architectural insight of Solana's Turbine:
    /// 1. Takes all active peers.
    /// 2. Sorts them deterministically by stake (descending).
    /// 3. Computes a pseudo-random permutation derived from `slot` and `shred_index`.
    /// 4. Organizes them into tree layers based on the `fanout` parameter.
    ///
    /// Returns a list of `PeerInfo` representing the children nodes this node
    /// should forward the received shred to.
    pub fn get_broadcast_children(
        &self,
        slot: u64,
        shred_index: u32,
        my_id: &str,
        fanout: usize,
    ) -> Vec<PeerInfo> {
        if fanout == 0 {
            return vec![];
        }

        // 1. Gather & Sort (deterministic base list)
        let mut peers = self.get_active_peers();
        peers.sort_by(|a, b| b.stake.cmp(&a.stake).then_with(|| a.id.cmp(&b.id)));

        // The generator node must be removed (it's the implicit root)
        // For simplicity in this implementation, we assume `peers` is the full set
        // of receivers (meaning the leader is excluded from this list, or handled separately).

        if peers.is_empty() {
            return vec![];
        }

        // 2. Deterministic shuffle based on slot/shred metadata
        // This ensures every node computes the exact same tree structure locally.
        let mut seed = [0u8; 32];
        let mut bytes = Vec::with_capacity(12);
        bytes.extend_from_slice(&slot.to_le_bytes());
        bytes.extend_from_slice(&shred_index.to_le_bytes());

        let hash = blake3::hash(&bytes);
        seed[..32].copy_from_slice(hash.as_bytes());

        let mut rng = rand::rngs::StdRng::from_seed(seed);
        peers.shuffle(&mut rng);

        // 3. Find my position in the randomly permuted tree
        let my_pos = peers.iter().position(|p| p.id == my_id);
        let my_pos = match my_pos {
            Some(pos) => pos,
            None => return vec![], // I'm not in the tree or I am the leader at root -1
        };

        // 4. Calculate children indices using k-ary heap property
        // For 0-indexed complete k-ary tree:
        // Children of node i are at `fanout * i + 1` to `fanout * i + fanout`
        let start_idx = fanout * my_pos + 1;

        if start_idx >= peers.len() {
            return vec![]; // Leaf node, no children
        }

        let end_idx = (start_idx + fanout).min(peers.len());

        peers[start_idx..end_idx].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turbine_propagation_tree() {
        let config = TurbineConfig::default();
        let manager = PeerManager::new(config);

        // Add 13 peers (1 root + 3 children + 9 grandchildren) -> exactly fits fanout=3
        for i in 0..13 {
            manager.add_peer(PeerInfo::new(
                format!("peer-{}", i),
                format!("/ip4/127.0.0.1/tcp/{}", 8000 + i),
                PeerRole::Validator,
            ));
        }

        // Test one deterministic permutation
        let children = manager.get_broadcast_children(100, 5, "peer-0", 3);

        assert!(children.len() <= 3, "Fanout limit respected");

        // Let's verify no duplicate children overall by manually reconstructing the tree
        let mut all_assigned_children = std::collections::HashSet::new();
        let mut total_edges = 0;

        for i in 0..13 {
            let peer_id = format!("peer-{}", i);
            let my_children = manager.get_broadcast_children(100, 5, &peer_id, 3);
            for c in my_children {
                assert!(
                    all_assigned_children.insert(c.id.clone()),
                    "Duplicate child assigned!"
                );
                total_edges += 1;
            }
        }

        // 13 nodes mapped to a k-ary tree means 1 root and 12 edges
        assert_eq!(total_edges, 12, "Tree should have N-1 edges");

        // Verify different shred indices yield different trees
        let children_shred5 = manager.get_broadcast_children(100, 5, "peer-0", 3);
        let children_shred6 = manager.get_broadcast_children(100, 6, "peer-0", 3);

        // While theoretically they could be identical randomly, the probability is 1/13!
        // so we can safely assert they differ for at least one node in practice.
        let mut diff_found = false;
        for i in 0..13 {
            let pid = format!("peer-{}", i);
            let c5 = manager.get_broadcast_children(100, 5, &pid, 3);
            let c6 = manager.get_broadcast_children(100, 6, &pid, 3);
            if c5.iter().map(|c| &c.id).collect::<Vec<_>>()
                != c6.iter().map(|c| &c.id).collect::<Vec<_>>()
            {
                diff_found = true;
                break;
            }
        }
        assert!(
            diff_found,
            "Tree shape should jump unpredictably based on shred index"
        );
    }
}
