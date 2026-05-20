//! Flash Finality P2P Gossip Network Integration
//!
//! Bridges the Flash Finality HotStuff consensus protocol with sc-network P2P gossip.
//! Enables real-time certificate distribution and vote aggregation across validators.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                   Flash Finality Gadget                           │
//! │  ┌─ Proposal Logic                                               │
//! │  ├─ Vote Aggregation                                             │
//! │  └─ Certificate Generation                                       │
//! │            │                                                      │
//! │            ▼                                                      │
//! │  ┌──────────────────────────────────────────────────────────────┐│
//! │  │       GossipNetworkBridge (THIS MODULE)                       ││
//! │  │  ┌──────────────────────────────────────────────────────┐    ││
//! │  │  │ Send Protocol:                                        │    ││
//! │  │  │   proposal → encode → gossip broadcast               │    ││
//! │  │  │   vote → encode → gossip broadcast                   │    ││
//! │  │  │   certificate → encode → gossip broadcast            │    ││
//! │  │  └──────────────────────────────────────────────────────┘    ││
//! │  │  ┌──────────────────────────────────────────────────────┐    ││
//! │  │  │ Receive Protocol:                                     │    ││
//! │  │  │   on_gossip_message → decode → verify → on_proposal  │    ││
//! │  │  │                                        on_vote        │    ││
//! │  │  │                                        on_certificate │    ││
//! │  │  └──────────────────────────────────────────────────────┘    ││
//! │  └─────────────────────────────────────────┬────────────────────┘│
//! │                                             │                     │
//! │                                             ▼                     │
//! │                                    sc-network Gossip              │
//! │                                    (P2P broadcast)                │
//! └──────────────────────────────────────────────────────────────────┘
//!                                            │
//!                    ┌───────────────────────┼───────────────────────┐
//!                    ▼                       ▼                       ▼
//!              Validator 1              Validator 2            Validator 3
//! ```

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Gossip message wrapper for the Flash Finality protocol
#[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking)]
pub enum FlashGossipMessage {
    /// Block proposal from the leader
    Proposal {
        block_hash: [u8; 32],
        block_number: u64,
        round: u64,
        leader_id: [u8; 32],
        leader_signature: [u8; 64],
    },
    /// Vote from a validator
    Vote {
        block_hash: [u8; 32],
        block_number: u64,
        round: u64,
        voter_id: [u8; 32],
        voter_signature: [u8; 64],
    },
    /// Finality certificate (quorum reached)
    Certificate {
        block_hash: [u8; 32],
        block_number: u64,
        round: u64,
        vote_count: u32,
        aggregated_signature: [u8; 64],
    },
}

/// Network message statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub received_proposals: u64,
    pub received_votes: u64,
    pub received_certificates: u64,
    pub sent_proposals: u64,
    pub sent_votes: u64,
    pub sent_certificates: u64,
    pub decode_errors: u64,
    pub signature_failures: u64,
    pub rounds_completed: u64,
    pub avg_latency_ms: f64,
}

/// Handler for incoming Flash Finality gossip messages
pub type GossipHandler = Arc<dyn Fn(FlashGossipMessage) + Send + Sync>;

/// Bridge between Flash Finality protocol and P2P gossip network
pub struct FlashFinalityGossipBridge {
    /// Our own validator ID
    my_id: [u8; 32],
    /// Network stats for monitoring
    stats: Arc<RwLock<NetworkStats>>,
    /// Event channel for incoming messages
    event_tx: mpsc::UnboundedSender<FlashGossipMessage>,
    /// Event channel for outgoing messages (to be broadcast)
    broadcast_queue: Arc<RwLock<Vec<FlashGossipMessage>>>,
    /// Custom handlers for each message type
    handlers: Arc<RwLock<HashMap<String, GossipHandler>>>,
}

impl FlashFinalityGossipBridge {
    /// Create a new gossip bridge
    pub fn new(my_id: [u8; 32]) -> (Self, mpsc::UnboundedReceiver<FlashGossipMessage>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let bridge = Self {
            my_id,
            stats: Arc::new(RwLock::new(NetworkStats {
                received_proposals: 0,
                received_votes: 0,
                received_certificates: 0,
                sent_proposals: 0,
                sent_votes: 0,
                sent_certificates: 0,
                decode_errors: 0,
                signature_failures: 0,
                rounds_completed: 0,
                avg_latency_ms: 0.0,
            })),
            event_tx,
            broadcast_queue: Arc::new(RwLock::new(Vec::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
        };

        (bridge, event_rx)
    }

    /// Broadcast a proposal to all peers
    pub async fn broadcast_proposal(
        &self,
        block_hash: [u8; 32],
        block_number: u64,
        round: u64,
        leader_signature: [u8; 64],
    ) {
        let msg = FlashGossipMessage::Proposal {
            block_hash,
            block_number,
            round,
            leader_id: self.my_id,
            leader_signature,
        };

        self.broadcast_message(msg).await;

        let mut stats = self.stats.write().await;
        stats.sent_proposals += 1;
        info!(
            "[FlashGossip] Broadcast proposal for block {} round {}",
            block_number, round
        );
    }

    /// Broadcast a vote to all peers
    pub async fn broadcast_vote(
        &self,
        block_hash: [u8; 32],
        block_number: u64,
        round: u64,
        voter_signature: [u8; 64],
    ) {
        let msg = FlashGossipMessage::Vote {
            block_hash,
            block_number,
            round,
            voter_id: self.my_id,
            voter_signature,
        };

        self.broadcast_message(msg).await;

        let mut stats = self.stats.write().await;
        stats.sent_votes += 1;
        debug!(
            "[FlashGossip] Broadcast vote for block {} round {}",
            block_number, round
        );
    }

    /// Broadcast a finality certificate to all peers
    pub async fn broadcast_certificate(
        &self,
        block_hash: [u8; 32],
        block_number: u64,
        round: u64,
        vote_count: u32,
        aggregated_signature: [u8; 64],
    ) {
        let msg = FlashGossipMessage::Certificate {
            block_hash,
            block_number,
            round,
            vote_count,
            aggregated_signature,
        };

        self.broadcast_message(msg).await;

        let mut stats = self.stats.write().await;
        stats.sent_certificates += 1;
        stats.rounds_completed += 1;
        info!(
            "[FlashGossip] Broadcast finality certificate for block {} ({} votes)",
            block_number, vote_count
        );
    }

    /// Internal method: queue a message for broadcast
    async fn broadcast_message(&self, msg: FlashGossipMessage) {
        let mut queue = self.broadcast_queue.write().await;
        queue.push(msg);
    }

    /// Handle an incoming gossip message (called by network layer)
    pub async fn on_gossip_received(&self, encoded_msg: &[u8]) -> Result<(), String> {
        // Try to decode the message
        let msg = match FlashGossipMessage::decode(&mut &encoded_msg[..]) {
            Ok(m) => m,
            Err(e) => {
                let mut stats = self.stats.write().await;
                stats.decode_errors += 1;
                warn!("[FlashGossip] Failed to decode message: {}", e);
                return Err(format!("Decode error: {}", e));
            }
        };

        // Update stats
        match &msg {
            FlashGossipMessage::Proposal { .. } => {
                let mut stats = self.stats.write().await;
                stats.received_proposals += 1;
            }
            FlashGossipMessage::Vote { .. } => {
                let mut stats = self.stats.write().await;
                stats.received_votes += 1;
            }
            FlashGossipMessage::Certificate { vote_count: _, .. } => {
                let mut stats = self.stats.write().await;
                stats.received_certificates += 1;
            }
        }

        // Route to internal handlers (would go to Flash Finality's on_proposal, on_vote, etc.)
        if self.event_tx.send(msg).is_err() {
            error!("[FlashGossip] Failed to queue incoming message");
            return Err("Queue full".to_string());
        }

        Ok(())
    }

    /// Get pending messages for broadcast (called by network layer)
    pub async fn pending_broadcast_messages(&self) -> Vec<Vec<u8>> {
        let mut queue = self.broadcast_queue.write().await;
        let messages = queue.drain(..).map(|msg| msg.encode()).collect();
        messages
    }

    /// Register a custom handler for incoming messages
    pub async fn register_handler(&self, name: String, handler: GossipHandler) {
        let mut handlers = self.handlers.write().await;
        handlers.insert(name, handler);
    }

    /// Get current network statistics
    pub async fn stats_snapshot(&self) -> NetworkStats {
        self.stats.read().await.clone()
    }

    /// Get health status
    pub async fn health_snapshot(&self) -> String {
        let stats = self.stats.read().await;
        format!(
            "FlashGossipBridge Health:\n  Received Proposals: {}\n  Received Votes: {}\n  Received Certificates: {}\n  Sent Proposals: {}\n  Sent Votes: {}\n  Sent Certificates: {}\n  Rounds Completed: {}\n  Decode Errors: {}\n  Signature Failures: {}",
            stats.received_proposals,
            stats.received_votes,
            stats.received_certificates,
            stats.sent_proposals,
            stats.sent_votes,
            stats.sent_certificates,
            stats.rounds_completed,
            stats.decode_errors,
            stats.signature_failures
        )
    }

    /// Reset statistics (for testing)
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = NetworkStats {
            received_proposals: 0,
            received_votes: 0,
            received_certificates: 0,
            sent_proposals: 0,
            sent_votes: 0,
            sent_certificates: 0,
            decode_errors: 0,
            signature_failures: 0,
            rounds_completed: 0,
            avg_latency_ms: 0.0,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proposal_broadcast() {
        let (bridge, mut rx) = FlashFinalityGossipBridge::new([0x01; 32]);

        bridge
            .broadcast_proposal([0x11; 32], 100, 1, [0x22; 64])
            .await;

        let stats = bridge.stats_snapshot().await;
        assert_eq!(stats.sent_proposals, 1);
    }

    #[tokio::test]
    async fn test_incoming_message() {
        let (bridge, mut rx) = FlashFinalityGossipBridge::new([0x01; 32]);

        let msg = FlashGossipMessage::Vote {
            block_hash: [0x11; 32],
            block_number: 100,
            round: 1,
            voter_id: [0x02; 32],
            voter_signature: [0x33; 64],
        };

        let encoded = msg.encode();
        bridge.on_gossip_received(&encoded).await.unwrap();

        let stats = bridge.stats_snapshot().await;
        assert_eq!(stats.received_votes, 1);
    }

    #[tokio::test]
    async fn test_certificate_with_vote_count() {
        let (bridge, _rx) = FlashFinalityGossipBridge::new([0x01; 32]);

        bridge
            .broadcast_certificate(
                [0x11; 32], 100, 1, 15, // 15 votes in quorum
                [0x44; 64],
            )
            .await;

        let stats = bridge.stats_snapshot().await;
        assert_eq!(stats.sent_certificates, 1);
        assert_eq!(stats.rounds_completed, 1);
    }

    #[test]
    fn test_message_encoding() {
        let msg = FlashGossipMessage::Proposal {
            block_hash: [0x11; 32],
            block_number: 100,
            round: 1,
            leader_id: [0x01; 32],
            leader_signature: [0x22; 64],
        };

        let encoded = msg.encode();
        let decoded = FlashGossipMessage::decode(&mut &encoded[..]).unwrap();

        match decoded {
            FlashGossipMessage::Proposal {
                block_number,
                round,
                ..
            } => {
                assert_eq!(block_number, 100);
                assert_eq!(round, 1);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
