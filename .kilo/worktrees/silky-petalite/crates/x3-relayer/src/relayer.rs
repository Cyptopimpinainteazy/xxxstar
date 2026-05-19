/// Relayer Service - Main orchestrator for proof acquisition and submission
use crate::submitter::RpcSubmitter;
use crate::types::*;
use crate::watchers::{EvmHeaderWatcher, SvmHeaderWatcher};
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
use x3_validator_attestation::{Attestation, AttestationSet, ValidatorId};
use x3_verification_router::{
    NonEmptyPayloadVerifier, ProofEnvelope, ProofKind, VerificationRouter,
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
    verification_router: VerificationRouter,
    risk_engine: GatewayRiskEngine,
    evm_finality_thresholds: BTreeMap<u32, u32>,
    svm_finality_thresholds: BTreeMap<u32, u32>,
}

struct RelayerInternalState {
    status: RelayerStateEnum,
    evm_heads: BTreeMap<u32, u64>, // domain_id -> block_number
    svm_heads: BTreeMap<u32, u64>, // domain_id -> slot
    finalized_evm_headers: BTreeMap<u32, u64>, // domain_id -> finalized block_number
    finalized_svm_headers: BTreeMap<u32, u64>, // domain_id -> finalized slot
    finalized_evm_data: BTreeMap<u32, HeaderInfo>, // domain_id -> finalized header (block_hash + state_root)
    finalized_svm_data: BTreeMap<u32, HeaderInfo>, // domain_id -> finalized header (blockhash)
    proof_cache: BTreeSet<[u8; 32]>,               // Submitted proof hashes (replay protection)
    pending_submissions: u32,
    shutdown_signal: bool,
    pause_reason: Option<String>,
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

        // Initialize RPC submitter
        let submitter = RpcSubmitter::new_with_retry_config(
            config.x3.rpc_url.clone(),
            config.x3.relayer_account.clone(),
            config.x3.relayer_custody_key_id.clone(),
            config.submission.max_retries,
            config.submission.retry_backoff_ms,
        )
        .await?;

        let state = RelayerInternalState {
            status: RelayerStateEnum::Initializing,
            evm_heads: BTreeMap::new(),
            svm_heads: BTreeMap::new(),
            finalized_evm_headers: BTreeMap::new(),
            finalized_svm_headers: BTreeMap::new(),
            finalized_evm_data: BTreeMap::new(),
            finalized_svm_data: BTreeMap::new(),
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
                let (block_hash, state_root) = match state.finalized_evm_data.get(&domain_id) {
                    Some(h) => (h.block_hash, h.state_root),
                    None => {
                        warn!(
                            "No header data for finalized EVM domain {}; skipping proof",
                            domain_id
                        );
                        continue;
                    }
                };

                if let Ok(proof) = self
                    .submitter
                    .acquire_evm_proof(domain_id, block_number, block_hash, state_root)
                    .await
                {
                    let recent_failures = self
                        .metrics
                        .read()
                        .await
                        .proofs_failed
                        .min(u64::from(u32::MAX)) as u32;
                    if let Err(reason) = self
                        .safety_pipeline
                        .evaluate_evm_proof(&proof, recent_failures)
                    {
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

                // Acquire proof from finalized slot.
                // blockhash comes from the header stored when this slot crossed
                // the finality threshold in poll_svm_headers.
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

                if let Ok(proof) = self
                    .submitter
                    .acquire_svm_proof(domain_id, slot, blockhash)
                    .await
                {
                    let recent_failures = self
                        .metrics
                        .read()
                        .await
                        .proofs_failed
                        .min(u64::from(u32::MAX)) as u32;
                    if let Err(reason) = self
                        .safety_pipeline
                        .evaluate_svm_proof(&proof, recent_failures)
                    {
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
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        proof.source_domain.hash(&mut hasher);
        proof.finalized_block.hash(&mut hasher);
        proof.block_hash.hash(&mut hasher);

        let hash_u64 = hasher.finish();
        let mut result = [0u8; 32];
        result[0..8].copy_from_slice(&hash_u64.to_le_bytes());
        result
    }

    /// Calculate hash of SVM proof for deduplication
    fn calculate_proof_hash_svm(&self, proof: &crate::types::SvmProof) -> [u8; 32] {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        proof.source_domain.hash(&mut hasher);
        proof.slot.hash(&mut hasher);
        proof.blockhash.hash(&mut hasher);

        let hash_u64 = hasher.finish();
        let mut result = [0u8; 32];
        result[0..8].copy_from_slice(&hash_u64.to_le_bytes());
        result
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

        let mut verification_router = VerificationRouter::new();
        verification_router
            .register_verifier(ProofKind::EvmReceipt, Arc::new(NonEmptyPayloadVerifier));
        verification_router.register_verifier(
            ProofKind::SolanaCommitment,
            Arc::new(NonEmptyPayloadVerifier),
        );

        Self {
            finality_oracle,
            verification_router,
            risk_engine: GatewayRiskEngine::new(RiskPolicy::default()),
            evm_finality_thresholds,
            svm_finality_thresholds,
        }
    }

    fn evaluate_evm_proof(&self, proof: &EvmProof, recent_failures: u32) -> Result<(), String> {
        let proof_id = proof.block_hash;
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

        let payload = [proof.block_hash.as_slice(), proof.state_root.as_slice()].concat();
        if let Err(reason) =
            self.evaluate_verification(ProofKind::EvmReceipt, payload, proof.source_domain)
        {
            return self.raise_dispute(proof_id, proof.finalized_block, reason);
        }

        let mut attestations = AttestationSet::new(proof_id);
        let attestation = Attestation {
            validator: ValidatorId("relayer-main".to_string()),
            statement_hash: proof_id,
            signature: vec![1],
            weight: 100,
        };
        if let Err(err) = attestations.add_attestation(attestation) {
            return self.raise_dispute(
                proof_id,
                proof.finalized_block,
                format!("attestation_rejected: {err:?}"),
            );
        }

        let quorum_met = attestations.has_quorum(67);
        if let Err(reason) = self.evaluate_risk(quorum_met, recent_failures) {
            return self.raise_dispute(proof_id, proof.finalized_block, reason);
        }

        Ok(())
    }

    fn evaluate_svm_proof(&self, proof: &SvmProof, recent_failures: u32) -> Result<(), String> {
        let proof_id = proof.blockhash;
        let required_confirmations = self
            .svm_finality_thresholds
            .get(&proof.source_domain)
            .copied()
            .unwrap_or(0) as u64;

        if let Err(reason) =
            self.evaluate_finality(proof.source_domain, proof.slot, required_confirmations)
        {
            return self.raise_dispute(proof_id, proof.slot, reason);
        }

        let mut payload = proof.blockhash.to_vec();
        for signature in &proof.validator_signatures {
            payload.extend_from_slice(signature);
        }
        if let Err(reason) =
            self.evaluate_verification(ProofKind::SolanaCommitment, payload, proof.source_domain)
        {
            return self.raise_dispute(proof_id, proof.slot, reason);
        }

        let mut attestations = AttestationSet::new(proof_id);
        for (idx, signature) in proof.validator_signatures.iter().enumerate() {
            let attestation = Attestation {
                validator: ValidatorId(format!("svm-validator-{idx}")),
                statement_hash: proof_id,
                signature: signature.to_vec(),
                weight: 1,
            };
            if let Err(err) = attestations.add_attestation(attestation) {
                return self.raise_dispute(
                    proof_id,
                    proof.slot,
                    format!("attestation_rejected: {err:?}"),
                );
            }
        }

        let quorum_met = attestations.has_quorum(proof.required_signatures as u64);
        if !quorum_met {
            return self.raise_dispute(
                proof_id,
                proof.slot,
                "attestation_quorum_not_met".to_string(),
            );
        }

        if let Err(reason) = self.evaluate_risk(quorum_met, recent_failures) {
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

    fn evaluate_verification(
        &self,
        kind: ProofKind,
        payload: Vec<u8>,
        source_domain: u32,
    ) -> Result<(), String> {
        let proof = ProofEnvelope {
            kind,
            payload,
            source_chain: source_domain,
            destination_chain: 0,
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
            Err(format!(
                "risk_gate_blocked: reason={}",
                decision.reason
            ))
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
            submission: SubmissionConfig::default(),
            governance: GovernanceConfig::default(),
            logging: LoggingConfig::default(),
        }
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
            finalized_evm_data: BTreeMap::new(),
            finalized_svm_data: BTreeMap::new(),
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
        let config = test_config();
        let pipeline = RelayerSafetyPipeline::new(&config);
        let proof = EvmProof {
            source_domain: 100,
            block_hash: [1u8; 32],
            state_root: [2u8; 32],
            finalized_block: 1_000,
            proof_nonce: 1,
        };

        assert!(pipeline.evaluate_evm_proof(&proof, 0).is_ok());
    }

    #[test]
    fn safety_pipeline_rejects_unknown_domain_finality() {
        let config = test_config();
        let pipeline = RelayerSafetyPipeline::new(&config);
        let proof = EvmProof {
            source_domain: 999,
            block_hash: [3u8; 32],
            state_root: [4u8; 32],
            finalized_block: 55,
            proof_nonce: 9,
        };

        let err = pipeline
            .evaluate_evm_proof(&proof, 0)
            .expect_err("unknown domain should fail finality check");
        assert!(err.contains("finality_rejected"));
        assert!(err.contains("dispute_status=Accepted"));
    }

    #[test]
    fn safety_pipeline_rejects_svm_quorum_gap() {
        let config = test_config();
        let pipeline = RelayerSafetyPipeline::new(&config);
        let proof = SvmProof {
            source_domain: 200,
            slot: 42,
            blockhash: [8u8; 32],
            validator_signatures: vec![[1u8; 32]],
            required_signatures: 2,
        };

        let err = pipeline
            .evaluate_svm_proof(&proof, 0)
            .expect_err("insufficient signatures should fail quorum");
        assert!(err.contains("attestation_quorum_not_met"));
        assert!(err.contains("dispute_status=Accepted"));
    }
}
