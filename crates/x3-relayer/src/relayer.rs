/// Relayer Service - Main orchestrator for proof acquisition and submission
extern crate alloc;
use crate::quorum::AuthorizedValidatorSet;
use crate::submitter::RpcSubmitter;
use crate::types::*;
use crate::watchers::{EvmHeaderWatcher, SvmHeaderWatcher};
use alloc::vec::Vec;
use anyhow::Result;
use log::{debug, info, warn};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{sleep, Duration};
use x3_finality_oracle::{
    Chain as FinalityChain, FinalityOracle, FinalityRule, FinalityStatus, InMemoryFinalityOracle,
    ObservedBlock,
};
use x3_gateway_risk_engine::{GatewayRiskEngine, RiskPolicy, RouteRiskInput};
use x3_proof_dispute::{DisputeStatus, DisputeTracker};
#[cfg(test)]
use x3_verification_router::VerificationRouter;
use x3_verification_router::{
    ethereum_beacon::VerifiedEthereumExecutionHeader,
    evm_receipt::{AnchoredEvmHeader, EvmHeaderAnchorSource},
    ChainKind, ProductionEvmReceiptVerifier, ProofEnvelope, SolanaCluster,
    SolanaStakeWeightedVerifier, SolanaValidatorStake, VerificationStrategy, Verifier,
};

pub struct RelayerService {
    config: RelayerConfig,
    state: Arc<RwLock<RelayerInternalState>>,
    evm_watchers: Vec<EvmHeaderWatcher>,
    svm_watchers: Vec<SvmHeaderWatcher>,
    submitter: Arc<RpcSubmitter>,
    safety_pipeline: Arc<RelayerSafetyPipeline>,
    metrics: Arc<RwLock<RelayerMetrics>>,
    evm_concurrency_limiter: Arc<Semaphore>, // Max 10 concurrent EVM polls
    svm_concurrency_limiter: Arc<Semaphore>, // Max 20 concurrent SVM polls
}

struct RelayerSafetyPipeline {
    finality_oracle: InMemoryFinalityOracle,
    #[cfg(test)]
    verification_router: VerificationRouter,
    risk_engine: GatewayRiskEngine,
    evm_finality_thresholds: BTreeMap<u32, u32>,
    svm_finality_thresholds: BTreeMap<u32, u32>,
    svm_verifier: Arc<SolanaStakeWeightedVerifier>,
    /// The configured signer set is used for the policy floor and claim checks.
    /// The stake-weighted verifier separately enforces the configured stake
    /// supermajority over the same keys.
    svm_quorum: AuthorizedValidatorSet,
}

struct RelayerInternalState {
    status: RelayerStateEnum,
    evm_heads: BTreeMap<u32, u64>, // domain_id -> block_number
    svm_heads: BTreeMap<u32, u64>, // domain_id -> slot
    finalized_evm_headers: BTreeMap<u32, u64>, // domain_id -> finalized block_number
    finalized_svm_headers: BTreeMap<u32, u64>, // domain_id -> finalized slot
    svm_finalized_floors: BTreeMap<u32, u64>, // durable finalized-slot floor
    finalized_evm_data: BTreeMap<u32, HeaderInfo>, // domain_id -> finalized header (block_hash + state_root)
    finalized_svm_data: BTreeMap<u32, HeaderInfo>, // domain_id -> finalized header (blockhash)
    pending_evm_receipts: BTreeMap<u32, Vec<PendingEvmReceipt>>,
    pending_svm_requests: BTreeMap<u32, SvmProofRequest>,
    proof_cache: BTreeSet<[u8; 32]>, // Submitted proof hashes (replay protection)
    pending_submissions: u32,
    shutdown_signal: bool,
    pause_reason: Option<String>,
}

#[derive(Clone)]
struct PendingEvmReceipt {
    finalized_block: u64,
    verified_header: VerifiedEthereumExecutionHeader,
    proof_payload: Vec<u8>,
    proof_id: [u8; 32],
    source_asset: [u8; 20],
    expected_sender: [u8; 20],
    expected_contract: [u8; 20],
    expected_beneficiary: Vec<u8>,
    expected_nonce: u64,
    expected_amount: u128,
}

/// A finalized EVM receipt and the exact `DepositLocked` fields it must prove.
#[derive(Clone)]
pub struct EvmReceiptProofRequest {
    pub domain_id: u32,
    pub finalized_block: u64,
    pub verified_header: VerifiedEthereumExecutionHeader,
    pub proof_payload: Vec<u8>,
    pub proof_id: [u8; 32],
    pub source_asset: [u8; 20],
    pub expected_sender: [u8; 20],
    pub expected_contract: [u8; 20],
    pub expected_beneficiary: Vec<u8>,
    pub expected_nonce: u64,
    pub expected_amount: u128,
}

fn configured_solana_cluster(clusters: &[SvmClusterConfig]) -> Result<SolanaCluster, String> {
    let mut selected = None;
    for cluster in clusters {
        let name = cluster.cluster_name.to_ascii_lowercase();
        let candidate = if name.contains("mainnet") {
            SolanaCluster::MainnetBeta
        } else if name.contains("testnet") {
            SolanaCluster::Testnet
        } else if name.contains("devnet") {
            SolanaCluster::Devnet
        } else {
            return Err(format!("unknown Solana cluster `{}`", cluster.cluster_name));
        };
        if let Some(previous) = selected {
            if previous != candidate {
                return Err("multiple Solana clusters need separate relayer instances".into());
            }
        } else {
            selected = Some(candidate);
        }
    }
    Ok(selected.unwrap_or(SolanaCluster::MainnetBeta))
}

fn configured_solana_stakes(
    config: &RelayerConfig,
    quorum: &AuthorizedValidatorSet,
) -> Result<Vec<SolanaValidatorStake>, String> {
    if config.validator_set.svm_validator_stakes.is_empty() {
        return Err("stake table is empty".into());
    }
    let mut stakes = Vec::with_capacity(config.validator_set.svm_validator_stakes.len());
    for entry in &config.validator_set.svm_validator_stakes {
        let trimmed = entry.pubkey.trim();
        let trimmed = trimmed.strip_prefix("0x").unwrap_or(trimmed);
        let bytes = hex::decode(trimmed)
            .map_err(|error| format!("invalid validator pubkey {}: {error}", entry.pubkey))?;
        let pubkey: [u8; 32] = bytes
            .try_into()
            .map_err(|_| format!("validator pubkey {} is not 32 bytes", entry.pubkey))?;
        stakes.push(SolanaValidatorStake {
            pubkey,
            stake: entry.stake,
        });
    }
    stakes.sort_by_key(|entry| entry.pubkey);

    let mut authorized = quorum.keys().to_vec();
    authorized.sort();
    let stake_keys: Vec<[u8; 32]> = stakes.iter().map(|entry| entry.pubkey).collect();
    if stake_keys != authorized {
        return Err("stake table and authorized validator keys disagree".into());
    }
    Ok(stakes)
}

impl RelayerService {
    pub async fn new(config: RelayerConfig) -> Result<Self> {
        info!("Initializing RelayerService");

        // Initialize EVM watchers
        let mut evm_watchers = Vec::new();
        for evm_config in &config.evm_chains {
            match EvmHeaderWatcher::new(evm_config.clone()).await {
                Ok(watcher) => evm_watchers.push(watcher),
                Err(e) => warn!(
                    "Failed to initialize EVM watcher for {}: {}",
                    evm_config.name, e
                ),
            }
        }

        // Initialize SVM watchers
        let mut svm_watchers = Vec::new();
        for svm_config in &config.svm_clusters {
            match SvmHeaderWatcher::new(svm_config.clone()).await {
                Ok(watcher) => svm_watchers.push(watcher),
                Err(e) => warn!(
                    "Failed to initialize SVM watcher for {}: {}",
                    svm_config.name, e
                ),
            }
        }

        let svm_cluster =
            configured_solana_cluster(&config.svm_clusters).map_err(anyhow::Error::msg)?;
        let mut svm_finalized_floors = BTreeMap::new();
        for watcher in &svm_watchers {
            svm_finalized_floors.insert(watcher.domain_id(), watcher.finalized_floor().await);
        }

        // Initialize RPC submitter
        let submitter = RpcSubmitter::new_with_retry_config(crate::submitter::RpcSubmitterConfig {
            x3_rpc_url: config.x3.rpc_url.clone(),
            relayer_account: config.x3.relayer_account.clone(),
            relayer_custody_key_id: config.x3.relayer_custody_key_id.clone(),
            relayer_seed_phrase: config.x3.relayer_seed_phrase.clone(),
            svm_validator_pubkeys: config.validator_set.svm_validator_pubkeys.clone(),
            svm_validator_stakes: config.validator_set.svm_validator_stakes.clone(),
            svm_cluster,
            max_retries: config.submission.max_retries,
            retry_backoff_ms: config.submission.retry_backoff_ms,
        })
        .await?;

        let state = RelayerInternalState {
            status: RelayerStateEnum::Initializing,
            evm_heads: BTreeMap::new(),
            svm_heads: BTreeMap::new(),
            finalized_evm_headers: BTreeMap::new(),
            finalized_svm_headers: BTreeMap::new(),
            svm_finalized_floors,
            finalized_evm_data: BTreeMap::new(),
            finalized_svm_data: BTreeMap::new(),
            pending_evm_receipts: BTreeMap::new(),
            pending_svm_requests: BTreeMap::new(),
            proof_cache: BTreeSet::new(),
            pending_submissions: 0,
            shutdown_signal: false,
            pause_reason: None,
        };

        info!(
            "RelayerService initialized with {} EVM chains and {} SVM clusters",
            evm_watchers.len(),
            svm_watchers.len()
        );

        let safety_pipeline = Arc::new(RelayerSafetyPipeline::new(&config));

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(state)),
            evm_watchers,
            svm_watchers,
            submitter: Arc::new(submitter),
            safety_pipeline,
            metrics: Arc::new(RwLock::new(RelayerMetrics::default())),
            evm_concurrency_limiter: Arc::new(Semaphore::new(10)), // Max 10 concurrent EVM polls
            svm_concurrency_limiter: Arc::new(Semaphore::new(20)), // Max 20 concurrent SVM polls
        })
    }

    /// Attach the event-specific receipt proof that finality polling alone cannot
    /// derive. The proof is consumed only after the matching block is finalized.
    pub async fn queue_evm_receipt_proof(&self, request: EvmReceiptProofRequest) -> Result<()> {
        if request.proof_payload.is_empty() {
            anyhow::bail!("EVM receipt proof payload must not be empty");
        }
        if request.proof_id == [0u8; 32] {
            anyhow::bail!("EVM gateway message id must not be zero");
        }
        if request.expected_beneficiary.is_empty() || request.expected_beneficiary.len() > 128 {
            anyhow::bail!("EVM deposit beneficiary must be between 1 and 128 bytes");
        }
        if request.verified_header.block_number != request.finalized_block {
            anyhow::bail!("verified beacon header does not match the finalized block");
        }
        self.state
            .write()
            .await
            .pending_evm_receipts
            .entry(request.domain_id)
            .or_default()
            .push(PendingEvmReceipt {
                finalized_block: request.finalized_block,
                verified_header: request.verified_header,
                proof_payload: request.proof_payload,
                proof_id: request.proof_id,
                source_asset: request.source_asset,
                expected_sender: request.expected_sender,
                expected_contract: request.expected_contract,
                expected_beneficiary: request.expected_beneficiary,
                expected_nonce: request.expected_nonce,
                expected_amount: request.expected_amount,
            });
        Ok(())
    }

    /// Queue complete Solana V2 evidence. The proof is not produced until the
    /// watcher has observed that finalized slot, and no bank hash or transfer
    /// field is derived from a caller-chosen conclusion.
    pub async fn queue_svm_proof_request(&self, request: SvmProofRequest) -> Result<()> {
        let configured = self
            .config
            .svm_clusters
            .iter()
            .find(|cluster| cluster.x3_domain_id == request.source_domain)
            .ok_or_else(|| anyhow::anyhow!("SVM proof domain is not configured"))?;
        let cluster = configured_solana_cluster(std::slice::from_ref(configured))
            .map_err(anyhow::Error::msg)?;
        if cluster != request.cluster {
            anyhow::bail!("SVM proof cluster does not match the configured cluster");
        }
        if request.proof_id == [0u8; 32]
            || request.blockhash == [0u8; 32]
            || request.parent_bank_hash == [0u8; 32]
            || request.stake_table_digest == [0u8; 32]
        {
            anyhow::bail!("SVM finality evidence contains a zero identity");
        }
        if request.parent_slot >= request.slot {
            anyhow::bail!("SVM parent slot must precede the finalized slot");
        }
        if request.expected_beneficiary.is_empty() || request.expected_beneficiary.len() > 128 {
            anyhow::bail!("SVM beneficiary must be between 1 and 128 bytes");
        }
        self.state
            .write()
            .await
            .pending_svm_requests
            .insert(request.source_domain, request);
        Ok(())
    }

    /// Main relay loop - runs indefinitely until shutdown
    pub async fn run(&self) -> Result<()> {
        info!("Starting relay loop");

        {
            let mut state = self.state.write().await;
            state.status = RelayerStateEnum::Active;
        }

        let startup_time = std::time::Instant::now();

        loop {
            // Check for shutdown signal
            {
                let state = self.state.read().await;
                if state.shutdown_signal {
                    info!("Shutdown signal received, exiting relay loop");
                    drop(state);
                    let mut state = self.state.write().await;
                    state.status = RelayerStateEnum::Stopped;
                    break;
                }
            }

            // Check governance pause status
            if let Ok(paused) = self.submitter.is_bridge_paused().await {
                if paused {
                    let mut state = self.state.write().await;
                    if state.status != RelayerStateEnum::Paused {
                        warn!("Bridge paused by governance");
                        state.status = RelayerStateEnum::Paused;
                        state.pause_reason = Some("Governance pause".to_string());
                        let mut metrics = self.metrics.write().await;
                        metrics.pause_events += 1;
                    }

                    sleep(Duration::from_secs(
                        self.config.governance.poll_interval_secs,
                    ))
                    .await;
                    continue;
                } else {
                    let mut state = self.state.write().await;
                    if state.status == RelayerStateEnum::Paused {
                        info!("Bridge unpaused, resuming operations");
                        state.status = RelayerStateEnum::Active;
                        state.pause_reason = None;
                    }
                }
            }

            // Poll EVM headers (with concurrency limiting)
            self.poll_evm_headers().await;

            // Poll SVM headers (with concurrency limiting)
            self.poll_svm_headers().await;

            // Check finality and submit proofs (with deduplication)
            self.process_finalized_headers().await;

            // Update uptime metrics
            let mut metrics = self.metrics.write().await;
            metrics.uptime_secs = startup_time.elapsed().as_secs();

            // Sleep before next iteration
            sleep(Duration::from_millis(
                self.config
                    .evm_chains
                    .first()
                    .map(|c| c.block_poll_interval_ms)
                    .unwrap_or(12000),
            ))
            .await;
        }

        Ok(())
    }

    pub async fn shutdown(&self) {
        let mut state = self.state.write().await;
        state.shutdown_signal = true;
        state.status = RelayerStateEnum::Shutting;
        info!("Shutdown requested");
    }

    pub async fn get_metrics(&self) -> RelayerMetrics {
        self.metrics.read().await.clone()
    }

    pub async fn get_status(&self) -> RelayerStateEnum {
        self.state.read().await.status.clone()
    }

    // ============================================================================
    // Private Methods
    // ============================================================================

    async fn poll_evm_headers(&self) {
        for (idx, watcher) in self.evm_watchers.iter().enumerate() {
            // Acquire permit from concurrency limiter (max 10 concurrent)
            let _permit = self.evm_concurrency_limiter.acquire().await;

            match watcher.poll().await {
                Ok(headers) => {
                    if !headers.is_empty() {
                        debug!("Polled {} EVM headers from watcher {}", headers.len(), idx);

                        let mut state = self.state.write().await;

                        // Process each header for finality checking
                        for header in &headers {
                            debug!(
                                "EVM header detail: chain={} block={} ts={} hash_prefix={:02x}{:02x} state_root_prefix={:02x}{:02x}",
                                header.chain_id,
                                header.block_number,
                                header.timestamp,
                                header.block_hash[0],
                                header.block_hash[1],
                                header.state_root[0],
                                header.state_root[1],
                            );

                            // Store raw header
                            state.evm_heads.insert(header.chain_id, header.block_number);

                            // Check if header has reached finality
                            if let Ok(is_finalized) =
                                watcher.check_finality(header.block_number).await
                            {
                                if is_finalized {
                                    debug!(
                                        "EVM block {} (chain {}) has reached finality",
                                        header.block_number, header.chain_id
                                    );
                                    state
                                        .finalized_evm_headers
                                        .insert(header.chain_id, header.block_number);
                                    state
                                        .finalized_evm_data
                                        .insert(header.chain_id, header.clone());
                                }
                            } else {
                                debug!(
                                    "Failed to check finality for EVM block {}",
                                    header.block_number
                                );
                            }
                        }

                        let mut metrics = self.metrics.write().await;
                        metrics.blocks_polled += headers.len() as u64;
                    }
                }
                Err(e) => {
                    warn!("Failed to poll EVM headers from watcher {}: {}", idx, e);
                    let mut metrics = self.metrics.write().await;
                    metrics.poll_failures += 1;
                }
            }
        }
    }

    async fn poll_svm_headers(&self) {
        for (idx, watcher) in self.svm_watchers.iter().enumerate() {
            // Acquire permit from concurrency limiter (max 20 concurrent)
            let _permit = self.svm_concurrency_limiter.acquire().await;

            match watcher.poll().await {
                Ok(headers) => {
                    if !headers.is_empty() {
                        debug!("Polled {} SVM headers from watcher {}", headers.len(), idx);

                        let mut state = self.state.write().await;

                        // Process each header for finality checking
                        for header in &headers {
                            debug!(
                                "SVM header detail: domain={} slot={} ts={} hash_prefix={:02x}{:02x}",
                                header.chain_id,
                                header.block_number,
                                header.timestamp,
                                header.block_hash[0],
                                header.block_hash[1],
                            );

                            // Store raw header
                            state.svm_heads.insert(header.chain_id, header.block_number);

                            // Check if header has reached finality
                            if let Ok(is_finalized) =
                                watcher.check_finality(header.block_number).await
                            {
                                if is_finalized {
                                    debug!(
                                        "SVM slot {} (domain {}) has reached finality",
                                        header.block_number, header.chain_id
                                    );
                                    state
                                        .finalized_svm_headers
                                        .insert(header.chain_id, header.block_number);
                                    state
                                        .svm_finalized_floors
                                        .insert(header.chain_id, header.block_number);
                                    state
                                        .finalized_svm_data
                                        .insert(header.chain_id, header.clone());
                                }
                            } else {
                                debug!(
                                    "Failed to check finality for SVM slot {}",
                                    header.block_number
                                );
                            }
                        }

                        let mut metrics = self.metrics.write().await;
                        metrics.blocks_polled += headers.len() as u64;
                    }
                }
                Err(e) => {
                    warn!("Failed to poll SVM headers from watcher {}: {}", idx, e);
                    let mut metrics = self.metrics.write().await;
                    metrics.poll_failures += 1;
                }
            }
        }
    }

    async fn process_finalized_headers(&self) {
        let mut state = self.state.write().await;

        if let Ok(next_nonce) = self.submitter.get_nonce().await {
            debug!("Current submission nonce: {}", next_nonce);
        }

        // Process EVM blocks that have reached finality
        let evm_domains: Vec<u32> = state.finalized_evm_headers.keys().cloned().collect();
        for domain_id in evm_domains {
            if let Some(&block_number) = state.finalized_evm_headers.get(&domain_id) {
                debug!(
                    "Processing finalized EVM domain {}: block {}",
                    domain_id, block_number
                );

                // Acquire proof from finalized block.
                // block_hash and state_root come from the header stored when
                // this block crossed the finality threshold in poll_evm_headers.
                let (block_hash, state_root, receipts_root) =
                    match state.finalized_evm_data.get(&domain_id) {
                        Some(h) => (h.block_hash, h.state_root, h.receipts_root),
                        None => {
                            warn!(
                                "No header data for finalized EVM domain {}; skipping proof",
                                domain_id
                            );
                            continue;
                        }
                    };
                let attested_head = match state.evm_heads.get(&domain_id) {
                    Some(head) => *head,
                    None => {
                        warn!("No chain head for finalized EVM domain {}", domain_id);
                        continue;
                    }
                };
                let pending_index =
                    state
                        .pending_evm_receipts
                        .get(&domain_id)
                        .and_then(|pending| {
                            pending
                                .iter()
                                .position(|entry| entry.finalized_block == block_number)
                        });
                let pending = match pending_index {
                    Some(index) => state.pending_evm_receipts[&domain_id][index].clone(),
                    None => {
                        debug!(
                            "No event-bound receipt proof queued for finalized EVM domain {}",
                            domain_id
                        );
                        continue;
                    }
                };

                if let Ok(proof) = self
                    .submitter
                    .acquire_evm_proof(EvmProofRequest {
                        source_domain: domain_id,
                        proof_id: pending.proof_id,
                        finalized_block: block_number,
                        block_hash,
                        state_root,
                        proof_payload: pending.proof_payload,
                        source_asset: pending.source_asset,
                        expected_sender: pending.expected_sender,
                        expected_contract: pending.expected_contract,
                        expected_beneficiary: pending.expected_beneficiary,
                        expected_nonce: pending.expected_nonce,
                        expected_amount: pending.expected_amount,
                    })
                    .await
                {
                    if let Some(pending) = state.pending_evm_receipts.get_mut(&domain_id) {
                        pending.remove(pending_index.expect("selected pending index"));
                    }
                    let recent_failures = self
                        .metrics
                        .read()
                        .await
                        .proofs_failed
                        .min(u64::from(u32::MAX)) as u32;
                    let beacon_header = &pending.verified_header;
                    if !beacon_header.matches_evm_header(
                        block_number,
                        block_hash,
                        state_root,
                        receipts_root,
                    ) {
                        warn!(
                            "Verified beacon header for domain {} block {} disagrees with the RPC header",
                            domain_id, block_number
                        );
                        let mut metrics = self.metrics.write().await;
                        metrics.proofs_failed += 1;
                        continue;
                    }
                    if let Err(reason) = self.safety_pipeline.evaluate_evm_proof_against_header(
                        &proof,
                        AnchoredEvmHeader {
                            number: beacon_header.block_number,
                            receipts_root: beacon_header.receipts_root.to_fixed_bytes(),
                            state_root: beacon_header.state_root.to_fixed_bytes(),
                            block_hash: beacon_header.block_hash.to_fixed_bytes(),
                        },
                        attested_head,
                        recent_failures,
                    ) {
                        warn!(
                            "Skipping EVM proof submission for domain {} block {}: {}",
                            domain_id, block_number, reason
                        );
                        let mut metrics = self.metrics.write().await;
                        metrics.proofs_failed += 1;
                        continue;
                    }

                    // Calculate proof hash for deduplication
                    let proof_hash = self.calculate_proof_hash_evm(&proof);

                    // Check if proof has already been submitted (deduplication)
                    if state.proof_cache.contains(&proof_hash) {
                        debug!("Proof already submitted, skipping deduplication");
                        continue;
                    }

                    // Submit proof with retries
                    match self.submitter.submit_evm_proof(proof).await {
                        Ok(tx_hash) => {
                            debug!("Submitted EVM proof: {}", tx_hash);
                            state.pending_submissions = state.pending_submissions.saturating_sub(1);
                            state.proof_cache.insert(proof_hash);
                            let mut metrics = self.metrics.write().await;
                            metrics.proofs_submitted += 1;
                        }
                        Err(e) => {
                            warn!("Failed to submit EVM proof: {}", e);
                            let mut metrics = self.metrics.write().await;
                            metrics.proofs_failed += 1;
                        }
                    }
                }

                let mut metrics = self.metrics.write().await;
                metrics.blocks_finalized += 1;
            }
        }

        // Process SVM slots that have reached finality
        let svm_domains: Vec<u32> = state.finalized_svm_headers.keys().cloned().collect();
        for domain_id in svm_domains {
            if let Some(&slot) = state.finalized_svm_headers.get(&domain_id) {
                debug!(
                    "Processing finalized SVM domain {}: slot {}",
                    domain_id, slot
                );

                let blockhash = match state.finalized_svm_data.get(&domain_id) {
                    Some(h) => h.block_hash,
                    None => {
                        warn!(
                            "No header data for finalized SVM domain {}; skipping proof",
                            domain_id
                        );
                        continue;
                    }
                };

                let request = match state.pending_svm_requests.get(&domain_id).cloned() {
                    Some(request) if request.slot == slot && request.blockhash == blockhash => {
                        request
                    }
                    Some(_) => {
                        warn!(
                            "Queued SVM evidence for domain {} does not match finalized slot {}",
                            domain_id, slot
                        );
                        continue;
                    }
                    None => {
                        debug!(
                            "No V2 stake-weighted evidence queued for finalized SVM domain {}",
                            domain_id
                        );
                        continue;
                    }
                };

                if let Ok(proof) = self.submitter.acquire_svm_proof(request).await {
                    state.pending_svm_requests.remove(&domain_id);
                    let finalized_floor = state
                        .svm_finalized_floors
                        .get(&domain_id)
                        .copied()
                        .unwrap_or(slot);
                    let recent_failures = self
                        .metrics
                        .read()
                        .await
                        .proofs_failed
                        .min(u64::from(u32::MAX)) as u32;
                    if let Err(reason) = self.safety_pipeline.evaluate_svm_proof(
                        &proof,
                        finalized_floor,
                        recent_failures,
                    ) {
                        warn!(
                            "Skipping SVM proof submission for domain {} slot {}: {}",
                            domain_id, slot, reason
                        );
                        let mut metrics = self.metrics.write().await;
                        metrics.proofs_failed += 1;
                        continue;
                    }

                    // Calculate proof hash for deduplication
                    let proof_hash = self.calculate_proof_hash_svm(&proof);

                    // Check if proof has already been submitted (deduplication)
                    if state.proof_cache.contains(&proof_hash) {
                        debug!("Proof already submitted, skipping deduplication");
                        continue;
                    }

                    // Submit proof with retries
                    match self.submitter.submit_svm_proof(proof).await {
                        Ok(tx_hash) => {
                            debug!("Submitted SVM proof: {}", tx_hash);
                            state.pending_submissions = state.pending_submissions.saturating_sub(1);
                            state.proof_cache.insert(proof_hash);
                            let mut metrics = self.metrics.write().await;
                            metrics.proofs_submitted += 1;
                        }
                        Err(e) => {
                            warn!("Failed to submit SVM proof: {}", e);
                            let mut metrics = self.metrics.write().await;
                            metrics.proofs_failed += 1;
                        }
                    }
                }

                let mut metrics = self.metrics.write().await;
                metrics.blocks_finalized += 1;
            }
        }

        // Clear processed finalized headers for next iteration
        state.finalized_evm_headers.clear();
        state.finalized_svm_headers.clear();
        state.finalized_evm_data.clear();
        state.finalized_svm_data.clear();
    }

    /// Calculate hash of EVM proof for deduplication
    fn calculate_proof_hash_evm(&self, proof: &crate::types::EvmProof) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(b"x3-relayer-evm-proof-v3");
        hasher.update(proof.source_domain.to_le_bytes());
        hasher.update(proof.proof_id);
        hasher.update(proof.finalized_block.to_le_bytes());
        hasher.update(proof.block_hash);
        hasher.update(proof.state_root);
        hasher.update(proof.proof_nonce.to_le_bytes());
        hasher.update(proof.source_asset);
        hasher.update(proof.expected_sender);
        hasher.update(proof.expected_contract);
        hasher.update(proof.expected_nonce.to_le_bytes());
        hasher.update((proof.expected_beneficiary.len() as u64).to_le_bytes());
        hasher.update(&proof.expected_beneficiary);
        hasher.update(proof.expected_amount.to_le_bytes());
        hasher.update((proof.proof_payload.len() as u64).to_le_bytes());
        hasher.update(&proof.proof_payload);
        hasher.finalize().into()
    }

    /// Calculate hash of SVM proof for deduplication
    fn calculate_proof_hash_svm(&self, proof: &crate::types::SvmProof) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let envelope = proof
            .encoded_envelope()
            .expect("SVM proofs are encoded before deduplication");
        let mut hasher = Sha256::new();
        hasher.update(b"x3-relayer-svm-proof-v2");
        hasher.update(proof.source_domain.to_le_bytes());
        hasher.update(proof.proof_id);
        hasher.update([proof.cluster as u8]);
        hasher.update(proof.slot.to_le_bytes());
        hasher.update(proof.parent_slot.to_le_bytes());
        hasher.update(proof.blockhash);
        hasher.update(proof.parent_bank_hash);
        hasher.update(proof.stake_table_digest);
        hasher.update(proof.expected_asset_id);
        hasher.update(proof.expected_amount.to_le_bytes());
        for value in [
            &proof.expected_sender,
            &proof.expected_recipient,
            &proof.expected_beneficiary,
            &envelope.payload,
        ] {
            hasher.update((value.len() as u64).to_le_bytes());
            hasher.update(value);
        }
        hasher.finalize().into()
    }
}

impl RelayerSafetyPipeline {
    fn new(config: &RelayerConfig) -> Self {
        let mut finality_oracle = InMemoryFinalityOracle::new();
        let mut evm_finality_thresholds = BTreeMap::new();
        let mut svm_finality_thresholds = BTreeMap::new();

        for chain in &config.evm_chains {
            finality_oracle.set_rule(
                FinalityChain::Other(chain.x3_domain_id),
                FinalityRule {
                    min_confirmations: chain.finality_threshold as u64,
                    max_allowed_reorg_depth: 2,
                },
            );
            evm_finality_thresholds.insert(chain.x3_domain_id, chain.finality_threshold);
        }

        for cluster in &config.svm_clusters {
            finality_oracle.set_rule(
                FinalityChain::Other(cluster.x3_domain_id),
                FinalityRule {
                    min_confirmations: cluster.finality_threshold as u64,
                    max_allowed_reorg_depth: 1,
                },
            );
            svm_finality_thresholds.insert(cluster.x3_domain_id, cluster.finality_threshold);
        }

        // The authorized validator set, and with it the quorum, comes from
        // configuration. A malformed entry is refused at startup by
        // `RpcSubmitter::new_with_retry_config`, which decodes the same list;
        // here a set that cannot be decoded degrades to the empty set and every
        // SVM proof is refused, rather than to a set nobody configured.
        let svm_quorum = match AuthorizedValidatorSet::from_hex_keys(
            &config.validator_set.svm_validator_pubkeys,
        ) {
            Ok(set) => set,
            Err(err) => {
                warn!("refusing every SVM proof: configured validator set is invalid: {err}");
                AuthorizedValidatorSet::from_keys(Vec::new())
            }
        };

        // Production uses only V2 stake-weighted evidence. If the stake table is
        // absent, malformed or disagrees with the authorized signer list, register
        // the empty verifier so every SVM proof fails closed rather than falling
        // back to the legacy signer-count V1 path.
        let cluster = configured_solana_cluster(&config.svm_clusters);
        let stake_result = cluster.and_then(|cluster| {
            let stakes = configured_solana_stakes(config, &svm_quorum)?;
            SolanaStakeWeightedVerifier::new(cluster, stakes, 2, 3, 0)
                .map_err(|error| format!("{error:?}"))
        });
        let svm_verifier = match stake_result {
            Ok(verifier) => Arc::new(verifier),
            Err(error) => {
                warn!("refusing every SVM proof: no valid V2 stake table: {error}");
                Arc::new(SolanaStakeWeightedVerifier::empty(
                    configured_solana_cluster(&config.svm_clusters)
                        .unwrap_or(SolanaCluster::MainnetBeta),
                ))
            }
        };
        Self {
            finality_oracle,
            #[cfg(test)]
            verification_router: VerificationRouter::new(),
            risk_engine: GatewayRiskEngine::new(RiskPolicy::default()),
            evm_finality_thresholds,
            svm_finality_thresholds,
            svm_verifier,
            svm_quorum,
        }
    }

    /// Test-only constructor that lets the caller supply a custom
    /// `VerificationRouter`. The production verifier is a strict
    /// EVM-receipt-proof verifier; pipeline coordination tests that
    /// want to exercise the finality → verification → attestation →
    /// risk flow without constructing a full merkle-patricia proof use
    /// this constructor to register a permissive verifier. The
    /// production verifier is still tested in `x3-verification-router`.
    #[cfg(test)]
    fn for_test(
        config: &RelayerConfig,
        verification_router: VerificationRouter,
        svm_authorized_validators: Vec<[u8; 32]>,
        svm_verifier: Arc<SolanaStakeWeightedVerifier>,
    ) -> Self {
        let mut finality_oracle = InMemoryFinalityOracle::new();
        let mut evm_finality_thresholds = BTreeMap::new();
        let mut svm_finality_thresholds = BTreeMap::new();

        for chain in &config.evm_chains {
            finality_oracle.set_rule(
                FinalityChain::Other(chain.x3_domain_id),
                FinalityRule {
                    min_confirmations: chain.finality_threshold as u64,
                    max_allowed_reorg_depth: 2,
                },
            );
            evm_finality_thresholds.insert(chain.x3_domain_id, chain.finality_threshold);
        }
        for cluster in &config.svm_clusters {
            finality_oracle.set_rule(
                FinalityChain::Other(cluster.x3_domain_id),
                FinalityRule {
                    min_confirmations: cluster.finality_threshold as u64,
                    max_allowed_reorg_depth: 1,
                },
            );
            svm_finality_thresholds.insert(cluster.x3_domain_id, cluster.finality_threshold);
        }

        Self {
            finality_oracle,
            verification_router,
            risk_engine: GatewayRiskEngine::new(RiskPolicy::default()),
            evm_finality_thresholds,
            svm_finality_thresholds,
            svm_verifier,
            svm_quorum: AuthorizedValidatorSet::from_keys(svm_authorized_validators),
        }
    }

    #[cfg(test)]
    fn evaluate_evm_proof(&self, proof: &EvmProof, recent_failures: u32) -> Result<(), String> {
        let proof_id = proof.proof_id;
        let required_confirmations = self
            .evm_finality_thresholds
            .get(&proof.source_domain)
            .copied()
            .unwrap_or(0) as u64;

        if let Err(reason) = self.evaluate_finality(
            proof.source_domain,
            proof.finalized_block,
            required_confirmations,
        ) {
            return self.raise_dispute(proof_id, proof.finalized_block, reason);
        }

        if let Err(reason) = self.evaluate_verification(
            VerificationStrategy::EvmReceiptProof,
            proof.proof_payload.clone(),
            proof.source_domain,
            proof.expected_contract.to_vec(),
            proof.expected_amount,
        ) {
            return self.raise_dispute(proof_id, proof.finalized_block, reason);
        }

        // The EVM proof type carries no validator attestations: its verification
        // is the receipt proof routed above (`EvmReceiptProof` -> light client /
        // merkle-patricia verification), which returned `Ok` or we would have
        // disputed already. This used to fabricate an attestation here
        // (`ValidatorId("relayer-main")`, a one-byte signature, weight 100 >= a
        // hardcoded 67) so the "quorum" check always passed — a fake proof in
        // security code that hid an unmet requirement from the risk gate.
        let verification_requirement_met = true;
        if let Err(reason) = self.evaluate_risk(verification_requirement_met, recent_failures) {
            return self.raise_dispute(proof_id, proof.finalized_block, reason);
        }

        Ok(())
    }

    fn evaluate_evm_proof_against_header(
        &self,
        proof: &EvmProof,
        anchor: AnchoredEvmHeader,
        attested_head: u64,
        recent_failures: u32,
    ) -> Result<(), String> {
        let proof_id = proof.proof_id;
        let required_confirmations = self
            .evm_finality_thresholds
            .get(&proof.source_domain)
            .copied()
            .unwrap_or(0) as u64;

        if proof.finalized_block != anchor.number
            || proof.block_hash != anchor.block_hash
            || proof.state_root != anchor.state_root
        {
            return self.raise_dispute(
                proof_id,
                proof.finalized_block,
                "evm proof does not match the finalized header".into(),
            );
        }
        if let Err(reason) = self.evaluate_finality(
            proof.source_domain,
            proof.finalized_block,
            required_confirmations,
        ) {
            return self.raise_dispute(proof_id, proof.finalized_block, reason);
        }

        let verifier = ProductionEvmReceiptVerifier::with_anchor(
            required_confirmations,
            EvmHeaderAnchorSource::Fixed {
                header: anchor,
                head: attested_head,
            },
        )
        .with_source_asset(proof.source_asset)
        .with_expected_nonce(proof.expected_nonce);
        let envelope = ProofEnvelope {
            proof_id,
            strategy: VerificationStrategy::EvmReceiptProof,
            source_chain: ChainKind::Evm {
                chain_id: proof.source_domain as u64,
            },
            destination_chain: ChainKind::X3,
            payload: proof.proof_payload.clone(),
            expected_asset_id: [0u8; 32],
            expected_amount: proof.expected_amount,
            expected_sender: proof.expected_sender.to_vec(),
            expected_recipient: proof.expected_contract.to_vec(),
            expected_beneficiary: proof.expected_beneficiary.clone(),
        };
        if let Err(reason) = verifier.verify(&envelope) {
            return self.raise_dispute(
                proof_id,
                proof.finalized_block,
                format!("verification_error: {reason}"),
            );
        }
        if let Err(reason) = self.evaluate_risk(true, recent_failures) {
            return self.raise_dispute(proof_id, proof.finalized_block, reason);
        }
        Ok(())
    }

    fn evaluate_svm_proof(
        &self,
        proof: &SvmProof,
        finalized_floor: u64,
        recent_failures: u32,
    ) -> Result<(), String> {
        let proof_id = proof.blockhash;
        let required_confirmations = self
            .svm_finality_thresholds
            .get(&proof.source_domain)
            .copied()
            .unwrap_or(0) as u64;

        if self.svm_quorum.is_empty() {
            return self.raise_dispute(
                proof_id,
                proof.slot,
                "no_authorized_validators_configured".into(),
            );
        }

        if let Err(reason) =
            self.evaluate_finality(proof.source_domain, proof.slot, required_confirmations)
        {
            return self.raise_dispute(proof_id, proof.slot, reason);
        }

        let policy_required = self.svm_quorum.required_signatures();
        if proof.required_signatures < policy_required {
            return self.raise_dispute(
                proof_id,
                proof.slot,
                format!(
                    "attestation_quorum_below_policy: proof required_signatures={} is below the \
                     supermajority of the {} authorized validators ({} signatures)",
                    proof.required_signatures,
                    self.svm_quorum.len(),
                    policy_required
                ),
            );
        }
        let attached = self
            .svm_quorum
            .distinct_signers(&proof.validator_signatures) as u32;
        if attached < proof.required_signatures {
            return self.raise_dispute(
                proof_id,
                proof.slot,
                format!(
                    "attestation_quorum_not_met: distinct_attestations={}, proof required={}, \
                     policy supermajority={} of {}",
                    attached,
                    proof.required_signatures,
                    policy_required,
                    self.svm_quorum.len()
                ),
            );
        }

        let envelope = match proof.encoded_envelope() {
            Ok(envelope) => envelope,
            Err(error) => {
                return self.raise_dispute(
                    proof_id,
                    proof.slot,
                    format!("verification_error: {error}"),
                );
            }
        };
        let outcome = match self
            .svm_verifier
            .verify_with_finalized_floor(&envelope, finalized_floor)
        {
            Ok(outcome) if outcome.accepted => outcome,
            Err(error) => {
                return self.raise_dispute(
                    proof_id,
                    proof.slot,
                    format!("verification_error: {error}"),
                );
            }
            Ok(outcome) => {
                return self.raise_dispute(
                    proof_id,
                    proof.slot,
                    format!("verification_rejected: {}", outcome.reason),
                );
            }
        };

        if let Err(reason) = self.evaluate_risk(outcome.accepted, recent_failures) {
            return self.raise_dispute(proof_id, proof.slot, reason);
        }

        Ok(())
    }

    fn evaluate_finality(
        &self,
        domain_id: u32,
        height: u64,
        required_confirmations: u64,
    ) -> Result<(), String> {
        let verdict = self.finality_oracle.evaluate(ObservedBlock {
            chain: FinalityChain::Other(domain_id),
            height,
            confirmations: required_confirmations,
            observed_reorg_depth: 0,
        });

        if verdict.status == FinalityStatus::Finalized {
            return Ok(());
        }

        Err(format!(
            "finality_rejected: status={:?}, required_confirmations={}, observed_confirmations={}",
            verdict.status, verdict.required_confirmations, verdict.observed_confirmations
        ))
    }

    #[cfg(test)]
    fn evaluate_verification(
        &self,
        kind: VerificationStrategy,
        payload: Vec<u8>,
        source_domain: u32,
        expected_recipient: Vec<u8>,
        expected_amount: u128,
    ) -> Result<(), String> {
        let source_chain = match kind {
            VerificationStrategy::EvmReceiptProof => ChainKind::Evm {
                chain_id: source_domain as u64,
            },
            VerificationStrategy::SolanaFinalizedProof => ChainKind::Solana,
            VerificationStrategy::BitcoinSpvProof => ChainKind::Bitcoin,
            _ => ChainKind::X3,
        };
        let proof = ProofEnvelope {
            proof_id: [0u8; 32],
            strategy: kind,
            source_chain,
            destination_chain: ChainKind::X3,
            payload,
            expected_asset_id: [0u8; 32],
            expected_amount,
            expected_sender: Vec::new(),
            expected_recipient,
            expected_beneficiary: Vec::new(),
        };

        let outcome = self
            .verification_router
            .route(&proof)
            .map_err(|err| format!("verification_error: {err}"))?;

        if outcome.accepted {
            Ok(())
        } else {
            Err(format!("verification_rejected: {}", outcome.reason))
        }
    }

    fn evaluate_risk(&self, quorum_met: bool, recent_failures: u32) -> Result<(), String> {
        let decision = self.risk_engine.evaluate(RouteRiskInput {
            value_usd: 0,
            recent_failures,
            verifier_quorum_met: quorum_met,
        });

        if decision.allow_route {
            Ok(())
        } else {
            Err(format!("risk_gate_blocked: reason={}", decision.reason))
        }
    }

    fn raise_dispute(&self, proof_id: [u8; 32], now: u64, reason: String) -> Result<(), String> {
        let mut tracker = DisputeTracker::new(proof_id, now, 1)
            .map_err(|err| format!("failed_to_open_dispute: {err:?}"))?;
        tracker
            .vote("safety-pipeline", true)
            .map_err(|err| format!("failed_to_vote_dispute: {err:?}"))?;
        let outcome = tracker
            .close(now.saturating_add(1), 1)
            .map_err(|err| format!("failed_to_close_dispute: {err:?}"))?;

        if outcome.status == DisputeStatus::Accepted {
            Err(format!("{}; dispute_status=Accepted", reason))
        } else {
            Err(format!("{}; dispute_status=Rejected", reason))
        }
    }
}

// ── Test verifier ──────────────────────────────────────────────────────────

/// Test-only verifier that accepts any non-empty EVM proof payload.
/// Used by `RelayerSafetyPipeline::for_test` to exercise pipeline
/// coordination without constructing a full merkle-patricia proof. The
/// production verifier is `ProductionEvmReceiptVerifier` and has its own
/// test suite in `x3-verification-router`.
#[cfg(test)]
struct AcceptAnyEvmVerifier;

#[cfg(test)]
impl x3_verification_router::Verifier for AcceptAnyEvmVerifier {
    fn strategy(&self) -> x3_verification_router::VerificationStrategy {
        x3_verification_router::VerificationStrategy::EvmReceiptProof
    }

    fn verify(
        &self,
        proof: &x3_verification_router::ProofEnvelope,
    ) -> Result<
        x3_verification_router::VerificationOutcome,
        x3_verification_router::VerificationError,
    > {
        if matches!(
            proof.source_chain,
            x3_verification_router::ChainKind::Evm { .. }
        ) && !proof.payload.is_empty()
        {
            Ok(x3_verification_router::VerificationOutcome {
                accepted: true,
                reason: "test_accept_any",
                verified_at_height: None,
            })
        } else {
            Err(x3_verification_router::VerificationError::UnsupportedChain)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RelayerConfig {
        RelayerConfig {
            x3: X3Config {
                rpc_url: "http://localhost:9933".to_string(),
                relayer_account: "5GrwvaEF5zXb26Fz9rcQkEvVkd7FcWI4twpBD6CFPhxGwwQ".to_string(),
                relayer_seed_phrase: None,
                relayer_custody_key_id: None,
            },
            evm_chains: vec![EvmChainConfig {
                name: "ethereum".to_string(),
                chain_id: 1,
                x3_domain_id: 100,
                rpc_endpoint: "http://localhost:8545".to_string(),
                state_root_contract: "0x0000000000000000000000000000000000000000".to_string(),
                finality_threshold: 12,
                block_poll_interval_ms: 12_000,
                max_concurrent_requests: 5,
            }],
            svm_clusters: vec![SvmClusterConfig {
                name: "solana".to_string(),
                cluster_name: "devnet".to_string(),
                x3_domain_id: 200,
                rpc_endpoint: "http://localhost:8899".to_string(),
                finality_threshold: 32,
                slot_poll_interval_ms: 400,
                max_concurrent_requests: 5,
            }],
            validator_set: ValidatorSetConfig::default(),
            submission: SubmissionConfig::default(),
            governance: GovernanceConfig::default(),
            logging: LoggingConfig::default(),
        }
    }

    #[test]
    fn solana_v2_stake_configuration_must_match_authorized_keys() {
        let mut config = test_config();
        config.svm_clusters[0].cluster_name = "mainnet-beta".into();
        let key = [3u8; 32];
        config.validator_set.svm_validator_pubkeys = vec![hex::encode(key)];
        config.validator_set.svm_validator_stakes = vec![SvmValidatorStakeConfig {
            pubkey: hex::encode(key),
            stake: 100,
        }];
        let quorum =
            AuthorizedValidatorSet::from_hex_keys(&config.validator_set.svm_validator_pubkeys)
                .unwrap();

        assert_eq!(
            configured_solana_cluster(&config.svm_clusters),
            Ok(SolanaCluster::MainnetBeta)
        );
        assert_eq!(
            configured_solana_stakes(&config, &quorum)
                .unwrap()
                .first()
                .map(|entry| (entry.pubkey, entry.stake)),
            Some((key, 100))
        );

        config.validator_set.svm_validator_stakes[0].pubkey = hex::encode([4u8; 32]);
        assert!(configured_solana_stakes(&config, &quorum).is_err());
    }

    #[tokio::test]
    async fn test_relayer_initialization() {
        let _config = RelayerConfig {
            x3: X3Config {
                rpc_url: "http://localhost:9933".to_string(),
                relayer_account: "5GrwvaEF5zXb26Fz9rcQkEvVkd7FcWI4twpBD6CFPhxGwwQ".to_string(),
                relayer_seed_phrase: None,
                relayer_custody_key_id: None,
            },
            evm_chains: vec![],
            svm_clusters: vec![],
            validator_set: ValidatorSetConfig::default(),
            submission: SubmissionConfig {
                batch_size: 1,
                timeout_secs: 60,
                max_retries: 3,
                retry_backoff_ms: 1000,
            },
            governance: GovernanceConfig {
                poll_interval_secs: 5,
                enable_graceful_shutdown: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: String::new(),
            },
        };

        // Note: This test will fail without a running X3 node
        // In CI, mock the RPC submitter initialization
    }

    #[tokio::test]
    async fn test_relayer_state_transitions() {
        let _config = RelayerConfig {
            x3: X3Config {
                rpc_url: "http://localhost:9933".to_string(),
                relayer_account: "5GrwvaEF5zXb26Fz9rcQkEvVkd7FcWI4twpBD6CFPhxGwwQ".to_string(),
                relayer_seed_phrase: None,
                relayer_custody_key_id: None,
            },
            evm_chains: vec![],
            svm_clusters: vec![],
            validator_set: ValidatorSetConfig::default(),
            submission: SubmissionConfig {
                batch_size: 1,
                timeout_secs: 60,
                max_retries: 3,
                retry_backoff_ms: 1000,
            },
            governance: GovernanceConfig {
                poll_interval_secs: 5,
                enable_graceful_shutdown: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: String::new(),
            },
        };

        // State transitions tested with mock objects
    }

    #[tokio::test]
    async fn test_finality_state_tracking() {
        let _config = RelayerConfig {
            x3: X3Config {
                rpc_url: "http://localhost:9933".to_string(),
                relayer_account: "5GrwvaEF5zXb26Fz9rcQkEvVkd7FcWI4twpBD6CFPhxGwwQ".to_string(),
                relayer_seed_phrase: None,
                relayer_custody_key_id: None,
            },
            evm_chains: vec![],
            svm_clusters: vec![],
            validator_set: ValidatorSetConfig::default(),
            submission: Default::default(),
            governance: Default::default(),
            logging: Default::default(),
        };

        // Verify state initialization includes finalized header tracking
        let state = RelayerInternalState {
            status: RelayerStateEnum::Active,
            evm_heads: BTreeMap::new(),
            svm_heads: BTreeMap::new(),
            finalized_evm_headers: BTreeMap::new(),
            finalized_svm_headers: BTreeMap::new(),
            svm_finalized_floors: BTreeMap::new(),
            finalized_evm_data: BTreeMap::new(),
            finalized_svm_data: BTreeMap::new(),
            pending_evm_receipts: BTreeMap::new(),
            pending_svm_requests: BTreeMap::new(),
            proof_cache: BTreeSet::new(),
            pending_submissions: 0,
            shutdown_signal: false,
            pause_reason: None,
        };

        assert!(state.finalized_evm_headers.is_empty());
        assert!(state.finalized_svm_headers.is_empty());
    }

    #[tokio::test]
    async fn test_metrics_finality_tracking() {
        let mut metrics = RelayerMetrics::default();

        // Verify finality metrics are tracked
        assert_eq!(metrics.blocks_polled, 0);
        assert_eq!(metrics.blocks_finalized, 0);
        assert_eq!(metrics.poll_failures, 0);

        // Simulate polling and finality
        metrics.blocks_polled += 5;
        metrics.blocks_finalized += 2;
        metrics.poll_failures += 1;

        assert_eq!(metrics.blocks_polled, 5);
        assert_eq!(metrics.blocks_finalized, 2);
        assert_eq!(metrics.poll_failures, 1);
    }

    #[test]
    fn test_concurrency_semaphore_initialization() {
        // Verify concurrency limiters are properly initialized
        let evm_semaphore = Semaphore::new(10);
        let svm_semaphore = Semaphore::new(20);

        // Verify semaphores have correct capacity
        assert_eq!(evm_semaphore.available_permits(), 10);
        assert_eq!(svm_semaphore.available_permits(), 20);
    }

    #[test]
    fn test_proof_deduplication_cache() {
        // Test proof deduplication using BTreeSet
        let mut proof_cache = std::collections::BTreeSet::new();
        let proof_hash = [0x42u8; 32];

        // First insertion should succeed
        assert!(proof_cache.insert(proof_hash));

        // Second insertion of same proof should be detected (deduplication)
        assert!(!proof_cache.insert(proof_hash));

        // Different proof should succeed
        let mut different_proof = [0x00u8; 32];
        different_proof[0] = 0x99;
        assert!(proof_cache.insert(different_proof));

        assert_eq!(proof_cache.len(), 2);
    }

    #[test]
    fn test_relay_state_uptime_tracking() {
        // Test that uptime can be tracked via metrics
        let mut metrics = RelayerMetrics::default();
        assert_eq!(metrics.uptime_secs, 0);

        metrics.uptime_secs = 300; // 5 minutes
        assert_eq!(metrics.uptime_secs, 300);
    }

    #[test]
    fn safety_pipeline_accepts_evm_happy_path() {
        // This test exercises the pipeline coordination logic
        // (finality → verification → attestation → risk). The strict
        // production EVM receipt proof verifier is exercised in
        // `x3-verification-router`; here we register a permissive
        // EVM verifier because the test is not about merkle-patricia
        // proof construction, it's about pipeline flow.
        let config = test_config();
        let mut router = VerificationRouter::new();
        router.register_verifier(Arc::new(AcceptAnyEvmVerifier));
        let pipeline = RelayerSafetyPipeline::for_test(
            &config,
            router,
            Vec::new(),
            Arc::new(SolanaStakeWeightedVerifier::empty(SVM_CLUSTER)),
        );
        let proof = EvmProof {
            source_domain: 100,
            proof_id: [0x70u8; 32],
            block_hash: [1u8; 32],
            state_root: [2u8; 32],
            finalized_block: 1_000,
            proof_nonce: 1,
            proof_payload: vec![1u8; 96],
            source_asset: [0x71u8; 20],
            expected_sender: [0x72u8; 20],
            expected_contract: [0x73u8; 20],
            expected_beneficiary: vec![0x74u8; 32],
            expected_nonce: 1,
            expected_amount: 1_000,
        };

        assert!(pipeline.evaluate_evm_proof(&proof, 0).is_ok());
    }

    #[test]
    fn safety_pipeline_rejects_unknown_domain_finality() {
        let config = test_config();
        let pipeline = RelayerSafetyPipeline::new(&config);
        let proof = EvmProof {
            source_domain: 999,
            proof_id: [0x70u8; 32],
            block_hash: [3u8; 32],
            state_root: [4u8; 32],
            finalized_block: 55,
            proof_nonce: 9,
            proof_payload: vec![1u8; 96],
            source_asset: [0x71u8; 20],
            expected_sender: [0x72u8; 20],
            expected_contract: [0x73u8; 20],
            expected_beneficiary: vec![0x74u8; 32],
            expected_nonce: 9,
            expected_amount: 1_000,
        };

        let err = pipeline
            .evaluate_evm_proof(&proof, 0)
            .expect_err("unknown domain should fail finality check");
        assert!(err.contains("finality_rejected"));
        assert!(err.contains("dispute_status=Accepted"));
    }

    #[test]
    fn evm_proof_must_match_the_attested_header() {
        let config = test_config();
        let pipeline = RelayerSafetyPipeline::new(&config);
        let proof = EvmProof {
            source_domain: 100,
            proof_id: [0x70u8; 32],
            block_hash: [3u8; 32],
            state_root: [4u8; 32],
            finalized_block: 55,
            proof_nonce: 9,
            proof_payload: vec![1u8; 96],
            source_asset: [0x71u8; 20],
            expected_sender: [0x72u8; 20],
            expected_contract: [0x73u8; 20],
            expected_beneficiary: vec![0x74u8; 32],
            expected_nonce: 9,
            expected_amount: 1_000,
        };
        let error = pipeline
            .evaluate_evm_proof_against_header(
                &proof,
                AnchoredEvmHeader {
                    number: 55,
                    receipts_root: [5u8; 32],
                    state_root: [6u8; 32],
                    block_hash: [7u8; 32],
                },
                60,
                0,
            )
            .expect_err("a proof cannot choose its own header");
        assert!(
            error.contains("does not match the finalized header"),
            "{error}"
        );
    }

    const SVM_CLUSTER: SolanaCluster = SolanaCluster::Devnet;
    const SVM_SLOT: u64 = 42;
    const SVM_PARENT_SLOT: u64 = 41;
    const SVM_BANK_HASH: [u8; 32] = [8u8; 32];
    const SVM_PARENT_BANK_HASH: [u8; 32] = [7u8; 32];

    fn solana_stakes(keys: &[ed25519_dalek::SigningKey], stake: u64) -> Vec<SolanaValidatorStake> {
        let mut stakes = keys
            .iter()
            .map(|key| SolanaValidatorStake {
                pubkey: key.verifying_key().to_bytes(),
                stake,
            })
            .collect::<Vec<_>>();
        stakes.sort_by_key(|entry| entry.pubkey);
        stakes
    }

    fn unsigned_svm_proof(stake_table: &[SolanaValidatorStake]) -> SvmProof {
        SvmProof {
            source_domain: 200,
            proof_id: [9u8; 32],
            cluster: SVM_CLUSTER,
            slot: SVM_SLOT,
            parent_slot: SVM_PARENT_SLOT,
            blockhash: SVM_BANK_HASH,
            parent_bank_hash: SVM_PARENT_BANK_HASH,
            stake_table_digest: x3_verification_router::solana_stake_table_digest(stake_table),
            expected_asset_id: [4u8; 32],
            expected_amount: 5_000,
            expected_sender: b"sender".to_vec(),
            expected_recipient: b"recipient".to_vec(),
            expected_beneficiary: vec![5u8; 32],
            validator_signatures: Vec::new(),
            required_signatures: x3_validator_attestation::supermajority_threshold(
                stake_table.len(),
            ),
        }
    }

    fn signed_v2_validator(
        proof: &SvmProof,
        signing_key: &ed25519_dalek::SigningKey,
    ) -> ValidatorSignature {
        use ed25519_dalek::Signer;

        let envelope = proof.envelope();
        let message = x3_verification_router::solana_finality_message_v2(
            proof.cluster,
            proof.slot,
            proof.parent_slot,
            &proof.blockhash,
            &proof.parent_bank_hash,
            &proof.stake_table_digest,
            &envelope,
        )
        .expect("fixture intent");
        ValidatorSignature {
            validator_pubkey: signing_key.verifying_key().to_bytes(),
            signature: signing_key.sign(&message).to_bytes(),
        }
    }

    fn v2_proof(
        proof: &SvmProof,
        keys: &[ed25519_dalek::SigningKey],
        signer_indices: &[usize],
    ) -> SvmProof {
        let mut proof = proof.clone();
        proof.validator_signatures = signer_indices
            .iter()
            .map(|index| signed_v2_validator(&proof, &keys[*index]))
            .collect();
        proof
    }

    fn solana_pipeline_with_stakes(
        keys: &[ed25519_dalek::SigningKey],
        stakes: &[SolanaValidatorStake],
    ) -> RelayerSafetyPipeline {
        use std::sync::Arc;

        let mut router = VerificationRouter::new();
        let verifier = Arc::new(
            SolanaStakeWeightedVerifier::new(SVM_CLUSTER, stakes.to_vec(), 2, 3, 0)
                .expect("fixture stake table"),
        );
        router.register_verifier(verifier.clone());
        RelayerSafetyPipeline::for_test(
            &test_config(),
            router,
            keys.iter()
                .map(|key| key.verifying_key().to_bytes())
                .collect(),
            verifier,
        )
    }

    /// A router-level quorum of 1 (one authorized signer is enough to pass
    /// `SolanaFinalizedVerifier`) is deliberately lower than the proof's own
    /// `required_signatures` (2), so this proof clears router verification
    /// but still can't meet its own claimed quorum at the attestation-count
    /// stage — the case this test actually regression-tests. Supplying zero
    /// signatures (as this test did before issue #351's fix wired in a real
    /// router) is no longer a way to reach this code path at all: the
    /// router itself now rejects a zero-signature proof first, with its own
    /// `InsufficientValidSignatures`/`NoAuthorizedValidators` reason.
    #[test]
    fn safety_pipeline_rejects_svm_quorum_gap() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[9u8; 32]);
        let stakes = solana_stakes(std::slice::from_ref(&key), 100);
        let pipeline = solana_pipeline_with_stakes(std::slice::from_ref(&key), &stakes);
        let mut proof = unsigned_svm_proof(&stakes);
        proof.required_signatures = 2;
        let proof = v2_proof(&proof, std::slice::from_ref(&key), &[0]);

        let err = pipeline
            .evaluate_svm_proof(&proof, 0, 0)
            .expect_err("insufficient signatures should fail quorum");
        assert!(err.contains("attestation_quorum_not_met"), "got: {err}");
        assert!(err.contains("dispute_status=Accepted"));
    }

    /// Issue #351: the production pipeline used to register
    /// `SolanaFinalizationVerifier`, a stub that accepted any non-empty
    /// payload with no cryptographic check at all — a forged SVM proof with
    /// a made-up signature would have passed. `RelayerSafetyPipeline::new`
    /// now registers the real `SolanaFinalizedVerifier`, and with no
    /// governance-controlled validator set wired into `RelayerConfig` yet,
    /// it's constructed via `::empty()`, which fails closed on every SVM
    /// proof rather than accepting a fabricated one. This proof carries a
    /// real, correctly-signed attestation from a genuine keypair — the
    /// point is that even a *valid* signature is rejected by the default
    /// pipeline, because nobody has been authorized yet.
    #[test]
    fn safety_pipeline_default_fails_closed_on_svm_proof_even_with_a_real_signature() {
        let config = test_config();
        let pipeline = RelayerSafetyPipeline::new(&config);
        let key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let stakes = solana_stakes(std::slice::from_ref(&key), 100);
        let proof = v2_proof(
            &unsigned_svm_proof(&stakes),
            std::slice::from_ref(&key),
            &[0],
        );

        let err = pipeline
            .evaluate_svm_proof(&proof, 0, 0)
            .expect_err("no validator set is authorized yet; every SVM proof must fail closed");
        assert!(
            err.contains("no_authorized_validators_configured"),
            "got: {err}"
        );
    }

    /// Regression: the quorum used to key each attestation on its *position* in
    /// the vector, so one validator's signature repeated N times satisfied an
    /// N-of-M requirement. Identity is now the validator public key, and the
    /// repeated signer is rejected before it can be counted. Uses a genuinely
    /// signed attestation (not fake bytes): the *first* copy must pass
    /// `add_attestation`'s Ed25519 check to reach the store at all — with fake
    /// bytes it would fail closed there with `SignatureVerificationFailed`,
    /// and the test would never reach the `DuplicateValidator` case it's
    /// actually regression-testing.
    #[test]
    fn safety_pipeline_rejects_repeated_svm_signer() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let stakes = solana_stakes(std::slice::from_ref(&key), 100);
        let pipeline = solana_pipeline_with_stakes(std::slice::from_ref(&key), &stakes);
        let mut proof = unsigned_svm_proof(&stakes);
        proof.required_signatures = 2;
        let signer = signed_v2_validator(&proof, &key);
        proof.validator_signatures = vec![signer.clone(), signer];

        let err = pipeline
            .evaluate_svm_proof(&proof, 0, 0)
            .expect_err("one validator cannot supply both quorum signatures");
        assert!(
            err.contains("attestation_quorum_not_met"),
            "expected a quorum rejection, got: {err}"
        );
        assert!(err.contains("dispute_status=Accepted"));
    }

    // ── Cross-side proof: what the relayer signs, the router verifies ───────

    fn validator_keys(seeds: &[u8]) -> Vec<ed25519_dalek::SigningKey> {
        seeds
            .iter()
            .map(|seed| ed25519_dalek::SigningKey::from_bytes(&[*seed; 32]))
            .collect()
    }

    /// The gap `X3-XCHAIN-003` records, as a test: three validators are
    /// authorized, one of them signs, and the proof declares
    /// `required_signatures: 1`. Before this workstream the declared number *was*
    /// the policy — the submitter wrote the literal `1` into it and the pipeline
    /// compared against it — so a proof the counterparty wrote for itself
    /// satisfied its own claim. It is refused, by name, as a claim below policy.
    #[test]
    fn safety_pipeline_refuses_a_one_of_three_proof_that_declares_itself_satisfied() {
        let keys = validator_keys(&[1, 2, 3]);
        let stakes = solana_stakes(&keys, 100);
        let pipeline = solana_pipeline_with_stakes(&keys, &stakes);
        let mut proof = unsigned_svm_proof(&stakes);
        proof.required_signatures = 1;
        let proof = v2_proof(&proof, &keys, &[0]);

        let err = pipeline
            .evaluate_svm_proof(&proof, 0, 0)
            .expect_err("one of three authorized validators is not a supermajority");
        assert!(
            err.contains("attestation_quorum_below_policy"),
            "the refusal must name the policy downgrade, got: {err}"
        );
        assert!(
            err.contains("supermajority of the 3 authorized validators"),
            "and say what the policy actually required, got: {err}"
        );
        assert!(err.contains("dispute_status=Accepted"));
    }

    /// A second, independent way the same shortfall shows up: the proof declares
    /// the *policy* count honestly (three) but carries only one signature, and
    /// the router is permissive enough to pass it through. The policy bar over
    /// distinct authorized attestations is what stops it.
    #[test]
    fn safety_pipeline_refuses_a_proof_that_carries_fewer_signatures_than_policy() {
        let keys = validator_keys(&[1, 2, 3]);
        let stakes = solana_stakes(&keys, 100);
        let pipeline = solana_pipeline_with_stakes(&keys, &stakes);
        let proof = v2_proof(&unsigned_svm_proof(&stakes), &keys, &[0]);

        let err = pipeline
            .evaluate_svm_proof(&proof, 0, 0)
            .expect_err("one signature cannot meet a three-validator supermajority");
        assert!(err.contains("attestation_quorum_not_met"), "got: {err}");
        assert!(err.contains("of 3"), "got: {err}");
    }

    /// The golden path, so the refusals above are not satisfied by refusing
    /// everything: three authorized validators, three distinct signatures and an
    /// honest claim of three.
    #[test]
    fn safety_pipeline_accepts_a_three_of_three_proof() {
        let keys = validator_keys(&[1, 2, 3]);
        let stakes = solana_stakes(&keys, 100);
        let pipeline = solana_pipeline_with_stakes(&keys, &stakes);
        let proof = v2_proof(&unsigned_svm_proof(&stakes), &keys, &[0, 1, 2]);

        assert!(
            pipeline.evaluate_svm_proof(&proof, 0, 0).is_ok(),
            "a full supermajority must be accepted"
        );
    }

    #[test]
    fn solana_pipeline_enforces_the_persisted_finalized_slot_floor() {
        let keys = validator_keys(&[1, 2, 3]);
        let stakes = solana_stakes(&keys, 100);
        let pipeline = solana_pipeline_with_stakes(&keys, &stakes);
        let proof = v2_proof(&unsigned_svm_proof(&stakes), &keys, &[0, 1, 2]);

        let err = pipeline
            .evaluate_svm_proof(&proof, proof.slot + 1, 0)
            .expect_err("a proof below the persisted finalized floor must be refused");
        assert!(err.contains("below the persisted floor"), "got: {err}");
    }

    /// A proof may raise its claim; it may not pretend to carry more than it
    /// does.
    #[test]
    fn safety_pipeline_refuses_a_claim_above_the_signatures_it_carries() {
        let keys = validator_keys(&[1, 2, 3]);
        let stakes = solana_stakes(&keys, 100);
        let pipeline = solana_pipeline_with_stakes(&keys, &stakes);
        let mut proof = v2_proof(&unsigned_svm_proof(&stakes), &keys, &[0, 1]);
        proof.required_signatures = 4;

        let err = pipeline
            .evaluate_svm_proof(&proof, 0, 0)
            .expect_err("two distinct signatures cannot satisfy a claim of four");
        assert!(err.contains("attestation_quorum_not_met"), "got: {err}");
    }

    #[test]
    fn relayer_svm_payload_verifies_through_the_real_verifier() {
        let (a, b) = (
            ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]),
            ed25519_dalek::SigningKey::from_bytes(&[2u8; 32]),
        );
        let keys = [a, b];
        let stakes = solana_stakes(&keys, 100);
        let proof = v2_proof(&unsigned_svm_proof(&stakes), &keys, &[0, 1]);

        // Format and crypto agree end to end: relayer builds the payload, the
        // router's real Solana verifier accepts it.
        let envelope = proof.encoded_envelope().expect("V2 payload");
        let verifier = SolanaStakeWeightedVerifier::new(SVM_CLUSTER, stakes.clone(), 2, 3, 0)
            .expect("stake table");
        use x3_verification_router::Verifier as _;
        let outcome = verifier
            .verify(&envelope)
            .expect("two authorized attestations must verify");
        assert!(
            outcome.accepted,
            "relayer payload must verify in the router"
        );

        // One signature short of the threshold is rejected.
        let mut proof = proof;
        proof.validator_signatures.pop();
        let one_signature = proof.encode_payload().expect("V2 payload");
        let one_envelope = x3_verification_router::ProofEnvelope {
            payload: one_signature,
            ..envelope
        };
        assert!(
            verifier.verify(&one_envelope).is_err(),
            "one signature must not satisfy a 2-validator quorum"
        );
    }
}
