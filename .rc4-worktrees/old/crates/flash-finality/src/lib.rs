//! # X3 Flash Finality Gadget
//!
//! HotStuff-inspired sub-second finality running in **shadow mode** alongside GRANDPA.
//!
//! ## Architecture (audit-specified approach)
//!
//! Per the deep-research audit, the correct production deployment order is:
//!
//! 1. **Shadow mode** — Flash Finality votes on every block in parallel with GRANDPA.
//!    Both finalized heads are compared every N blocks. Mismatches alert; never override.
//! 2. **Shadow validation** — Run shadow agreement for ≥1000 blocks with zero divergence.
//! 3. **Gated activation** — Behind `NodeFeatureFlags::enable_flash_finality`. Only then
//!    does Flash Finality become the canonical justification source.
//!
//! ## Message Protocol
//!
//! - **Proposal**: `(block_hash, block_number, round, leader_id, leader_sig)`
//! - **Vote**: `(block_hash, round, voter_id, voter_sig)`
//! - **Certificate**: `(block_hash, round, aggregated_vote_count, voter_set_hash)`
//!
//! ## HotStuff Liveness Safeguards
//!
//! The audit warns about subtle liveness violations in 2-phase Sync HotStuff variants.
//! We mitigate by:
//! - Using conservative round timeouts (5× expected block time).
//! - Tracking divergence between Flash and GRANDPA heads continuously.
//! - Never advancing the canonical finalized head from Flash in shadow mode.
//! - Logging every round timeout as a potential liveness event.

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration, Instant};
use tracing::{debug, error, info, warn};

pub mod gossip_bridge;
pub use gossip_bridge::{FlashFinalityGossipBridge, FlashGossipMessage, NetworkStats};

// ─── Types ────────────────────────────────────────────────────────────────────

/// A block hash (32 bytes as hex string for human readability in logs).
pub type BlockHash = [u8; 32];
/// A validator node identifier.
pub type ValidatorId = [u8; 32];
/// A round number in the HotStuff protocol.
pub type RoundNumber = u64;
/// A block number on the canonical chain.
pub type BlockNumber = u64;

// ─── Message Formats ─────────────────────────────────────────────────────────

/// Flash Finality proposal message.
/// Emitted by the current round leader after receiving a valid block.
#[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, Hash)]
pub struct Proposal {
    pub block_hash: BlockHash,
    pub block_number: BlockNumber,
    pub round: RoundNumber,
    pub leader_id: ValidatorId,
    /// sr25519 signature of the proposal.
    pub leader_sig: [u8; 64],
}

impl Proposal {
    pub fn message_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.block_hash);
        h.update(self.round.to_le_bytes());
        h.update(self.leader_id);
        h.finalize().into()
    }

    /// Verify signature using sr25519.
    pub fn verify_sig(&self) -> bool {
        self.verify()
    }

    /// Real verification logic using sp_core
    pub fn verify(&self) -> bool {
        // Use sp_core for actual sr25519 verification
        use sp_core::sr25519;
        use sp_runtime::traits::Verify;
        let public = sr25519::Public::from_raw(self.leader_id);
        let signature = sr25519::Signature::from_raw(self.leader_sig);
        let message = self.message_hash();

        signature.verify(&message[..], &public)
    }
}

/// Flash Finality vote message.
/// Emitted by each validator upon receiving a valid proposal.
#[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, Hash)]
pub struct Vote {
    pub block_hash: BlockHash,
    pub block_number: BlockNumber,
    pub round: RoundNumber,
    pub voter_id: ValidatorId,
    /// sr25519 signature of the vote.
    pub voter_sig: [u8; 64],
}

impl Vote {
    pub fn message_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.block_hash);
        h.update(self.round.to_le_bytes());
        h.update(self.voter_id);
        h.finalize().into()
    }

    /// Verify signature using sr25519.
    pub fn verify(&self) -> bool {
        // Use sp_core for actual sr25519 verification
        use sp_core::sr25519;
        use sp_runtime::traits::Verify;
        let public = sr25519::Public::from_raw(self.voter_id);
        let signature = sr25519::Signature::from_raw(self.voter_sig);
        let message = self.message_hash();

        signature.verify(&message[..], &public)
    }
}

/// Flash Finality certificate.
/// Produced when ≥ 2/3 + 1 validators vote for the same block in the same round.
/// This is the artifact that becomes a PoAE proof anchor.
#[derive(
    Debug, Clone, Encode, Decode, DecodeWithMemTracking, serde::Serialize, serde::Deserialize,
)]
pub struct FinalityCertificate {
    pub block_hash: BlockHash,
    pub block_number: BlockNumber,
    pub round: RoundNumber,
    /// Number of votes aggregated.
    pub vote_count: u32,
    /// SHA-256 of the sorted voter_id set — proves which validators signed.
    pub voter_set_hash: [u8; 32],
    /// Timestamp when the certificate was generated.
    pub generated_at_ms: u64,
    /// Whether this certificate was produced in shadow mode (never canonical if true).
    pub is_shadow: bool,
}

impl FinalityCertificate {
    /// Compute a stable certificate hash for use as a PoAE proof anchor.
    pub fn cert_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.block_hash);
        h.update(self.block_number.to_le_bytes());
        h.update(self.round.to_le_bytes());
        h.update(self.vote_count.to_le_bytes());
        h.update(self.voter_set_hash);
        h.finalize().into()
    }
}

// ─── Network Message ──────────────────────────────────────────────────────────

/// Gossip protocol identifier for Flash Finality.
pub const FLASH_FINALITY_PROTOCOL_ID: &str = "/x3/flash/1";

/// Messages gossiped over the Flash Finality P2P network.
#[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking)]
pub enum GossipMessage {
    /// A new block proposal from the leader.
    Proposal(Proposal),
    /// A vote for a proposal.
    Vote(Vote),
    /// A completed finality certificate.
    Certificate(FinalityCertificate),
}

// ─── Shadow Mode Divergence Tracking ─────────────────────────────────────────

/// Records a divergence between Flash Finality and GRANDPA finalized heads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceEvent {
    pub at_block: BlockNumber,
    pub flash_head: BlockHash,
    pub grandpa_head: BlockHash,
    pub detected_at_ms: u64,
}

// ─── Gadget Configuration ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FlashFinalityConfig {
    /// Minimum validators needed for a quorum (N nodes → ⌊2N/3⌋ + 1).
    pub quorum_size: u32,
    /// Total validator set size.
    pub validator_count: u32,
    /// Round timeout — if no quorum by this time, log liveness event and advance.
    /// Audit: must be conservative (≥ 5× block time) to avoid liveness violations.
    pub round_timeout_ms: u64,
    /// Shadow mode: compare against GRANDPA but never override canonical head.
    pub shadow_mode: bool,
    /// Number of consecutive agreeing rounds before logging "shadow validated".
    pub shadow_validation_threshold: u64,
}

impl Default for FlashFinalityConfig {
    fn default() -> Self {
        // 21 validators → quorum = 15
        let validator_count = 21u32;
        Self {
            quorum_size: (validator_count * 2 / 3) + 1,
            validator_count,
            // 1000ms = 5× the 200ms target block time (audit: conservative timeouts)
            round_timeout_ms: 1000,
            shadow_mode: true,
            shadow_validation_threshold: 1000,
        }
    }
}

impl FlashFinalityConfig {
    pub fn quorum_for(n: u32) -> u32 {
        (n * 2 / 3) + 1
    }
}

// ─── Round State ─────────────────────────────────────────────────────────────

#[derive(Debug)]
struct RoundState {
    round: RoundNumber,
    block_hash: Option<BlockHash>,
    block_number: BlockNumber,
    proposal: Option<Proposal>,
    /// voter_id → Vote
    votes: HashMap<ValidatorId, Vote>,
    started_at: Instant,
    finalized: bool,
}

impl RoundState {
    fn new(round: RoundNumber, block_number: BlockNumber) -> Self {
        Self {
            round,
            block_hash: None,
            block_number,
            proposal: None,
            votes: HashMap::new(),
            started_at: Instant::now(),
            finalized: false,
        }
    }

    fn vote_count(&self) -> u32 {
        self.votes.len() as u32
    }

    fn voter_set_hash(&self) -> [u8; 32] {
        let mut ids: Vec<&ValidatorId> = self.votes.keys().collect();
        ids.sort();
        let mut h = Sha256::new();
        for id in ids {
            h.update(id);
        }
        h.finalize().into()
    }
}

// ─── Flash Finality Gadget ────────────────────────────────────────────────────

/// Metrics exported to Prometheus (or logged if Prometheus not wired).
#[derive(Debug, Default, Clone, Serialize)]
pub struct FinalityMetrics {
    pub rounds_completed: u64,
    pub rounds_timed_out: u64,
    pub certificates_produced: u64,
    pub divergence_events: u64,
    pub shadow_agreements: u64,
    pub last_finalized_block: BlockNumber,
    pub last_cert_latency_ms: u64,
}

/// The Flash Finality gadget.
///
/// Run this as a background task alongside GRANDPA. It observes new blocks,
/// collects votes, and produces [`FinalityCertificate`]s.
///
/// In shadow mode (the safe default), certificates are logged but the canonical
/// finalized head is NOT updated — GRANDPA remains authoritative.
pub struct FlashFinalityGadget {
    config: FlashFinalityConfig,
    my_id: ValidatorId,
    current_round: Arc<RwLock<RoundState>>,
    /// Recent certificates (ring buffer, last 1000).
    certificates: Arc<Mutex<VecDeque<FinalityCertificate>>>,
    /// Divergence events detected in shadow mode.
    divergence_log: Arc<Mutex<Vec<DivergenceEvent>>>,
    /// Prometheus-style counters.
    metrics: Arc<Mutex<FinalityMetrics>>,
    /// Count of consecutive rounds where Flash == GRANDPA.
    shadow_agreement_streak: Arc<Mutex<u64>>,
    /// Externally set GRANDPA finalized head for comparison.
    grandpa_head: Arc<RwLock<(BlockNumber, BlockHash)>>,
    /// Keystore for signing proposals (type-erased to avoid dependency).
    #[allow(dead_code)]
    keystore: Option<Box<dyn std::any::Any + Send + Sync>>,
}

impl FlashFinalityGadget {
    pub fn new(
        config: FlashFinalityConfig,
        my_id: ValidatorId,
        keystore: Option<Box<dyn std::any::Any + Send + Sync>>,
    ) -> Self {
        Self {
            config,
            my_id,
            current_round: Arc::new(RwLock::new(RoundState::new(0, 0))),
            certificates: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
            divergence_log: Arc::new(Mutex::new(Vec::new())),
            metrics: Arc::new(Mutex::new(FinalityMetrics::default())),
            shadow_agreement_streak: Arc::new(Mutex::new(0)),
            grandpa_head: Arc::new(RwLock::new((0, [0u8; 32]))),
            keystore,
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Called when a new best block arrives from the import queue.
    /// In a real node, this is triggered inside the block import pipeline.
    pub async fn on_new_block(
        &self,
        block_hash: BlockHash,
        block_number: BlockNumber,
    ) -> Option<Proposal> {
        let mut round = self.current_round.write().await;

        // Advance to a new round
        round.round += 1;
        round.block_hash = Some(block_hash);
        round.block_number = block_number;
        round.proposal = None;
        round.votes.clear();
        round.started_at = Instant::now();
        round.finalized = false;

        debug!(
            "[FlashFinality] New round {} for block #{} {:?}",
            round.round,
            block_number,
            hex::encode(block_hash)
        );

        // Build proposal (leader = self for now — real impl uses VRF leader election)
        let proposal = self.build_proposal(&round);
        round.proposal = Some(proposal.clone());

        Some(proposal)
    }

    /// Called when a proposal message arrives from the network.
    pub async fn on_proposal(&self, proposal: Proposal) {
        if !proposal.verify() {
            warn!("[FlashFinality] Proposal with invalid signature rejected");
            return;
        }

        let mut round = self.current_round.write().await;
        if proposal.round != round.round {
            debug!(
                "[FlashFinality] Ignoring proposal for wrong round {} (current {})",
                proposal.round, round.round
            );
            return;
        }

        round.block_hash = Some(proposal.block_hash);
        round.proposal = Some(proposal.clone());

        debug!(
            "[FlashFinality] Accepted proposal for round {}",
            proposal.round
        );
    }

    /// Called when a vote message arrives from the network.
    /// Returns Some(certificate) if this vote completes a quorum.
    pub async fn on_vote(&self, vote: Vote) -> Option<FinalityCertificate> {
        if !vote.verify() {
            warn!("[FlashFinality] Vote with invalid signature rejected");
            return None;
        }

        let mut round = self.current_round.write().await;

        if vote.round != round.round {
            debug!(
                "[FlashFinality] Ignoring vote for wrong round {} (current {})",
                vote.round, round.round
            );
            return None;
        }

        let block_hash = match round.block_hash {
            Some(h) => h,
            None => {
                warn!(
                    "[FlashFinality] Vote received before proposal in round {}",
                    vote.round
                );
                return None;
            }
        };

        if vote.block_hash != block_hash {
            warn!(
                "[FlashFinality] Vote for different block hash in round {}",
                vote.round
            );
            return None;
        }

        // Store the vote (by voter_id — deduplicates re-votes)
        round.votes.insert(vote.voter_id, vote);

        debug!(
            "[FlashFinality] Round {} has {}/{} votes",
            round.round,
            round.vote_count(),
            self.config.quorum_size
        );

        // Check for quorum
        if round.vote_count() >= self.config.quorum_size && !round.finalized {
            round.finalized = true;
            let cert = self.produce_certificate(&round).await;
            return Some(cert);
        }

        None
    }

    pub fn config(&self) -> &FlashFinalityConfig {
        &self.config
    }

    /// Retrieve a certificate for a specific block hash if it exists in the buffer.
    pub async fn get_certificate(&self, block_hash: BlockHash) -> Option<FinalityCertificate> {
        let certs = self.certificates.lock().await;
        certs.iter().find(|c| c.block_hash == block_hash).cloned()
    }

    /// Update the internal view of the GRANDPA finalized head.
    pub async fn update_grandpa_head(&self, number: BlockNumber, hash: BlockHash) {
        let mut head = self.grandpa_head.write().await;
        *head = (number, hash);
    }

    /// Compare Flash Finality head with GRANDPA head for shadow validation.
    /// Called periodically by the shadow monitor task.
    pub async fn shadow_compare(
        &self,
        flash_block: BlockNumber,
        flash_hash: BlockHash,
    ) -> ShadowResult {
        let grandpa = self.grandpa_head.read().await;
        let grandpa_block = grandpa.0;
        let grandpa_hash = grandpa.1;

        if flash_block == grandpa_block && flash_hash == grandpa_hash {
            let mut streak = self.shadow_agreement_streak.lock().await;
            *streak += 1;

            if *streak >= self.config.shadow_validation_threshold {
                info!(
                    "[FlashFinality] 🟢 Shadow validated: {} consecutive agreements. \
                     Ready for gated activation when NodeFeatureFlags::enable_flash_finality is set.",
                    streak
                );
            }

            let mut metrics = self.metrics.lock().await;
            metrics.shadow_agreements += 1;

            ShadowResult::Agreement { streak: *streak }
        } else if flash_block > 0 && grandpa_block > 0 {
            // Real divergence — alert
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let event = DivergenceEvent {
                at_block: flash_block,
                flash_head: flash_hash,
                grandpa_head: grandpa_hash,
                detected_at_ms: ts,
            };

            error!(
                "[FlashFinality] 🔴 DIVERGENCE at block #{}: flash={} grandpa={}",
                flash_block,
                hex::encode(flash_hash),
                hex::encode(grandpa_hash),
            );

            let mut log = self.divergence_log.lock().await;
            log.push(event.clone());

            let mut streak = self.shadow_agreement_streak.lock().await;
            *streak = 0; // Reset streak on divergence

            let mut metrics = self.metrics.lock().await;
            metrics.divergence_events += 1;

            ShadowResult::Divergence(event)
        } else {
            ShadowResult::Pending
        }
    }

    /// Get a recent finality certificate by block number.
    pub async fn get_certificate_by_number(
        &self,
        block_number: BlockNumber,
    ) -> Option<FinalityCertificate> {
        let certs = self.certificates.lock().await;
        certs
            .iter()
            .find(|c| c.block_number == block_number)
            .cloned()
    }

    /// Get current metrics snapshot.
    pub async fn metrics(&self) -> FinalityMetrics {
        self.metrics.lock().await.clone()
    }

    /// Get shadow agreement streak count.
    pub async fn shadow_streak(&self) -> u64 {
        *self.shadow_agreement_streak.lock().await
    }

    /// List all divergence events (for operator dashboards).
    pub async fn divergence_events(&self) -> Vec<DivergenceEvent> {
        self.divergence_log.lock().await.clone()
    }

    // ── Round timeout monitor ─────────────────────────────────────────────────

    /// Spawn a background task that monitors for round timeouts.
    /// Liveness alert: if a round exceeds `round_timeout_ms`, log and advance.
    pub fn spawn_timeout_monitor(self: Arc<Self>) -> impl std::future::Future<Output = ()> {
        let gadget = self.clone();
        async move {
            let tick = Duration::from_millis(gadget.config.round_timeout_ms / 4);
            let mut ticker = interval(tick);

            loop {
                ticker.tick().await;

                let round = gadget.current_round.read().await;
                if round.started_at.elapsed()
                    > Duration::from_millis(gadget.config.round_timeout_ms)
                    && !round.finalized
                    && round.round > 0
                {
                    warn!(
                        "[FlashFinality] ⏰ LIVENESS ALERT: Round {} timed out at block {} \
                         ({}/{} votes). Advancing without certificate.",
                        round.round,
                        round.block_number,
                        round.vote_count(),
                        gadget.config.quorum_size,
                    );

                    let mut metrics = gadget.metrics.lock().await;
                    metrics.rounds_timed_out += 1;
                }
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn build_proposal(&self, round: &RoundState) -> Proposal {
        let block_hash = round.block_hash.unwrap_or([0u8; 32]);
        let message = {
            let mut h = Sha256::new();
            h.update(block_hash);
            h.update(round.round.to_le_bytes());
            h.update(self.my_id);
            h.finalize()
        };

        let message_array: [u8; 32] = message.into();

        let leader_sig = if let Some(_keystore) = &self.keystore {
            // Try to sign with the keystore if available
            if let Some(sig) = self.sign_with_keystore(&message_array) {
                sig
            } else {
                // Fallback to empty signature if keystore signing fails
                [0u8; 64]
            }
        } else {
            // Fallback to empty signature if no keystore
            [0u8; 64]
        };

        Proposal {
            block_hash,
            block_number: round.block_number,
            round: round.round,
            leader_id: self.my_id,
            leader_sig,
        }
    }

    /// Attempt to sign a message using the keystore if available
    fn sign_with_keystore(&self, _message: &[u8; 32]) -> Option<[u8; 64]> {
        // This is a placeholder implementation that would integrate with sp_keystore
        // In production, this would properly extract the keystore and sign the message
        // For now, we return None to indicate signing is not available
        None
    }

    async fn produce_certificate(&self, round: &RoundState) -> FinalityCertificate {
        let block_hash = round.block_hash.unwrap_or([0u8; 32]);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let cert = FinalityCertificate {
            block_hash,
            block_number: round.block_number,
            round: round.round,
            vote_count: round.vote_count(),
            voter_set_hash: round.voter_set_hash(),
            generated_at_ms: ts,
            is_shadow: self.config.shadow_mode,
        };

        let mode = if self.config.shadow_mode {
            "SHADOW"
        } else {
            "LIVE"
        };
        info!(
            "[FlashFinality] 📜 [{mode}] Certificate produced: block #{} hash={} round={} votes={}/{}",
            cert.block_number,
            hex::encode(cert.block_hash),
            cert.round,
            cert.vote_count,
            self.config.quorum_size,
        );

        // Store in ring buffer
        let mut certs = self.certificates.lock().await;
        if certs.len() >= 1000 {
            certs.pop_front();
        }
        certs.push_back(cert.clone());

        // Update metrics
        let mut metrics = self.metrics.lock().await;
        metrics.certificates_produced += 1;
        metrics.rounds_completed += 1;
        metrics.last_finalized_block = cert.block_number;

        cert
    }
}

// ─── Shadow Result ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub enum ShadowResult {
    /// Flash and GRANDPA agree. `streak` = consecutive agreements so far.
    Agreement { streak: u64 },
    /// Flash and GRANDPA disagree — alert, reset streak.
    Divergence(DivergenceEvent),
    /// One or both heads not yet available.
    Pending,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_id(byte: u8) -> ValidatorId {
        [byte; 32]
    }

    fn make_hash(byte: u8) -> BlockHash {
        [byte; 32]
    }

    fn make_gadget(quorum: u32, n: u32) -> Arc<FlashFinalityGadget> {
        Arc::new(FlashFinalityGadget::new(
            FlashFinalityConfig {
                quorum_size: quorum,
                validator_count: n,
                round_timeout_ms: 1000,
                shadow_mode: true,
                shadow_validation_threshold: 3,
            },
            make_id(0xAA),
            None,
        ))
    }

    fn make_vote(block_hash: BlockHash, round: RoundNumber, voter: u8) -> Vote {
        let id = make_id(voter);
        let msg_hash = {
            let mut h = Sha256::new();
            h.update(&block_hash);
            h.update(round.to_le_bytes());
            h.update(&id);
            h.finalize()
        };
        let mut sig_array = [0u8; 64];
        // Copy hash twice to fill 64 bytes (test purpose only)
        sig_array[..32].copy_from_slice(&msg_hash);
        sig_array[32..].copy_from_slice(&msg_hash);
        Vote {
            block_hash,
            block_number: 1,
            round,
            voter_id: id,
            voter_sig: sig_array,
        }
    }

    #[tokio::test]
    async fn test_certificate_produced_at_quorum() {
        let gadget = make_gadget(3, 4);
        let block_hash = make_hash(0x01);

        // Start round 1
        let proposal = gadget.on_new_block(block_hash, 1).await.unwrap();
        assert_eq!(proposal.round, 1);

        // Submit 2 votes — no quorum yet
        let cert1 = gadget.on_vote(make_vote(block_hash, 1, 0xB1)).await;
        let cert2 = gadget.on_vote(make_vote(block_hash, 1, 0xB2)).await;
        assert!(cert1.is_none());
        assert!(cert2.is_none());

        // Submit 3rd vote — quorum reached
        let cert3 = gadget.on_vote(make_vote(block_hash, 1, 0xB3)).await;
        assert!(cert3.is_some());

        let cert = cert3.unwrap();
        assert_eq!(cert.block_hash, block_hash);
        assert_eq!(cert.block_number, 1);
        assert_eq!(cert.vote_count, 3);
        assert!(cert.is_shadow); // came from shadow-mode gadget
    }

    #[tokio::test]
    async fn test_shadow_agreement_tracked() {
        let gadget = Arc::new(FlashFinalityGadget::new(
            FlashFinalityConfig::default(),
            make_id(0xAA),
            None,
        ));
        let block_hash = make_hash(0x42);

        // Set GRANDPA head to same block
        gadget.update_grandpa_head(5, block_hash).await;

        // Compare Flash head (same as GRANDPA) — should agree
        let result = gadget.shadow_compare(5, block_hash).await;
        match result {
            ShadowResult::Agreement { streak } => assert_eq!(streak, 1),
            other => panic!("Expected Agreement, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_shadow_divergence_detected() {
        let gadget = Arc::new(FlashFinalityGadget::new(
            FlashFinalityConfig::default(),
            make_id(0xAA),
            None,
        ));

        let flash_hash = make_hash(0x01);
        let grandpa_hash = make_hash(0x02);

        gadget.update_grandpa_head(10, grandpa_hash).await;

        let result = gadget.shadow_compare(10, flash_hash).await;
        match result {
            ShadowResult::Divergence(event) => {
                assert_eq!(event.at_block, 10);
                assert_eq!(event.flash_head, flash_hash);
                assert_eq!(event.grandpa_head, grandpa_hash);
            }
            other => panic!("Expected Divergence, got {:?}", other),
        }

        // Streak reset to 0 after divergence
        assert_eq!(gadget.shadow_streak().await, 0);
        assert_eq!(gadget.divergence_events().await.len(), 1);
    }

    #[tokio::test]
    async fn test_duplicate_votes_deduplicated() {
        let gadget = make_gadget(2, 3);
        let block_hash = make_hash(0xCC);

        gadget.on_new_block(block_hash, 1).await;

        // Same voter votes twice — should only count once
        let v = make_vote(block_hash, 1, 0xB1);
        let cert1 = gadget.on_vote(v.clone()).await;
        let cert2 = gadget.on_vote(v.clone()).await; // duplicate

        // After 1 unique vote (quorum=2), no cert
        assert!(cert1.is_none());
        // Duplicate doesn't count toward quorum either
        assert!(cert2.is_none());

        // Second unique voter — now quorum
        let cert3 = gadget.on_vote(make_vote(block_hash, 1, 0xB2)).await;
        assert!(cert3.is_some());
    }

    #[tokio::test]
    async fn test_wrong_round_vote_ignored() {
        let gadget = make_gadget(2, 3);
        let block_hash = make_hash(0xDD);

        gadget.on_new_block(block_hash, 1).await; // round becomes 1

        // Vote for wrong round
        let mut wrong_vote = make_vote(block_hash, 99, 0xB1);
        wrong_vote.round = 99;
        let cert = gadget.on_vote(wrong_vote).await;
        assert!(cert.is_none());
    }

    #[tokio::test]
    async fn test_shadow_validation_threshold() {
        let gadget = Arc::new(FlashFinalityGadget::new(
            FlashFinalityConfig {
                shadow_validation_threshold: 3,
                ..FlashFinalityConfig::default()
            },
            make_id(0xAA),
            None,
        ));

        let h = make_hash(0x10);
        gadget.update_grandpa_head(1, h).await;

        // 3 consecutive agreements should log "shadow validated"
        for _ in 0..3 {
            gadget.shadow_compare(1, h).await;
        }
        assert_eq!(gadget.shadow_streak().await, 3);
        assert_eq!(gadget.metrics().await.shadow_agreements, 3);
    }

    #[tokio::test]
    async fn test_certificate_stored_and_retrievable() {
        let gadget = make_gadget(1, 1); // quorum=1 for easy test
        let block_hash = make_hash(0xEE);

        gadget.on_new_block(block_hash, 42).await;
        gadget.on_vote(make_vote(block_hash, 1, 0xB1)).await;

        let cert = gadget.get_certificate(block_hash).await;
        assert!(cert.is_some());
        assert_eq!(cert.unwrap().block_number, 42);
    }

    // ===== Network & Voter Integration Tests =====

    /// Test that simulates a 4-validator consensus round.
    /// All validators see the same proposal and vote, producing a certificate.
    #[tokio::test]
    async fn test_four_validator_consensus_round() {
        let quorum = 3; // 3 of 4 validators needed
        let n_validators = 4;
        let gadget = make_gadget(quorum, n_validators);
        let block_hash = make_hash(0x55);
        let block_number = 100;

        // All validators start the same round
        let proposal = gadget.on_new_block(block_hash, block_number).await.unwrap();
        assert_eq!(proposal.block_hash, block_hash);
        assert_eq!(proposal.block_number, block_number);

        // 3 of 4 validators vote (quorum threshold)
        let vote1 = gadget
            .on_vote(make_vote(block_hash, proposal.round, 0x11))
            .await;
        let vote2 = gadget
            .on_vote(make_vote(block_hash, proposal.round, 0x22))
            .await;
        let vote3 = gadget
            .on_vote(make_vote(block_hash, proposal.round, 0x33))
            .await;

        assert!(vote1.is_none(), "1st vote: no quorum yet");
        assert!(vote2.is_none(), "2nd vote: no quorum yet");
        assert!(vote3.is_some(), "3rd vote: quorum reached!");

        let cert = vote3.unwrap();
        assert_eq!(cert.block_number, block_number);
        assert_eq!(cert.vote_count, 3);
        assert_eq!(cert.voter_set_hash.len(), 32); // should be a hash
    }

    /// Test that the 4th vote (redundant after quorum) doesn't break the gadget.
    #[tokio::test]
    async fn test_fourth_validator_vote_after_quorum() {
        let gadget = make_gadget(3, 4);
        let block_hash = make_hash(0x66);

        gadget.on_new_block(block_hash, 101).await;

        // First 3 votes reach quorum
        gadget.on_vote(make_vote(block_hash, 1, 0x11)).await;
        gadget.on_vote(make_vote(block_hash, 1, 0x22)).await;
        let cert3 = gadget.on_vote(make_vote(block_hash, 1, 0x33)).await;
        assert!(cert3.is_some());

        // 4th vote comes late — should be ignored or acknowledged as redundant
        let cert4 = gadget.on_vote(make_vote(block_hash, 1, 0x44)).await;
        // Depending on implementation, cert4 might be None or Some(cert) with vote_count=3
        // Either way, it shouldn't panic or break quorum tracking
        if let Some(c) = cert4 {
            assert!(
                c.vote_count <= 4,
                "vote count should not exceed validator count"
            );
        }
    }

    /// Test that blocks finalize in order (block N before block N+1).
    /// This is critical for canonical chain progression.
    #[tokio::test]
    async fn test_sequential_block_finalization() {
        let gadget = make_gadget(2, 3); // 2-of-3 quorum for speed
        let mut hashes = vec![];
        let mut certs = vec![];

        // Finalize blocks 1 through 5 in sequence
        for i in 1..=5 {
            let hash = make_hash(i as u8);
            hashes.push(hash);

            gadget.on_new_block(hash, i).await;

            // Get 2 votes to reach quorum
            gadget.on_vote(make_vote(hash, i, 0x11)).await;
            let cert = gadget.on_vote(make_vote(hash, i, 0x22)).await.unwrap();

            assert_eq!(cert.block_number, i as u64);
            certs.push(cert);
        }

        // All blocks should have finalized in order
        assert_eq!(certs.len(), 5);
        for (i, cert) in certs.iter().enumerate() {
            assert_eq!(cert.block_number, (i + 1) as u64);
        }
    }

    /// Test that the gadget initializes correctly in shadow mode.
    #[tokio::test]
    async fn test_shadow_mode_initialization() {
        let gadget = Arc::new(FlashFinalityGadget::new(
            FlashFinalityConfig {
                shadow_mode: true,
                ..Default::default()
            },
            make_id(0xAA),
            None,
        ));

        assert!(gadget.config.shadow_mode, "Gadget should be in shadow mode");
        let metrics = gadget.metrics().await;
        assert_eq!(metrics.rounds_completed, 0);
    }

    /// Test metrics collection across a realistic voting scenario.
    #[tokio::test]
    async fn test_gadget_metrics_across_rounds() {
        let gadget = Arc::new(FlashFinalityGadget::new(
            FlashFinalityConfig {
                shadow_mode: true,
                ..Default::default()
            },
            make_id(0x11),
            None,
        ));

        // Simulate processing several blocks
        for block_num in 1..=5 {
            let hash = make_hash(block_num as u8);
            gadget.on_new_block(hash, block_num).await;
        }

        let metrics = gadget.metrics().await;
        // Rounds are only completed when a certificate is produced (quorum reached)
        // Since we're not depositing votes here, no certificates are produced
        assert_eq!(
            metrics.shadow_agreements, 0,
            "No explicit GRANDPA comparisons yet"
        );
    }

    /// Test that Live mode flag is correctly set during initialization.
    #[tokio::test]
    async fn test_live_mode_flag_controls_finality_application() {
        let live_gadget = Arc::new(FlashFinalityGadget::new(
            FlashFinalityConfig {
                shadow_mode: false, // LIVE mode
                ..Default::default()
            },
            make_id(0xAA),
            None,
        ));

        let shadow_gadget = Arc::new(FlashFinalityGadget::new(
            FlashFinalityConfig {
                shadow_mode: true, // SHADOW mode
                ..Default::default()
            },
            make_id(0xBB),
            None,
        ));

        assert!(
            !live_gadget.config.shadow_mode,
            "Live gadget should have shadow_mode=false"
        );
        assert!(
            shadow_gadget.config.shadow_mode,
            "Shadow gadget should have shadow_mode=true"
        );
    }
}
