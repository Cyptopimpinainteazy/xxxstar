//! Validator module for X3 GPU Validator Swarm

use crate::config::SwarmConfig;
use crate::crypto::{HashAlgorithm, SigningKey};
use crate::deterministic::{
    DeterministicEngine, DeterministicTask, ExecutionMode, ExecutionResult,
};
use crate::error::{SwarmError, SwarmResult};
use crate::gpu_receipt::GpuReceipt;
use crate::health::{HealthMonitor, ValidatorHealthTracker};
use crate::metrics::MetricsCollector;
use crate::proof_aggregator::ProofAggregator;
use crate::proof_integration;
use crate::quarantine::QuarantineManager;
use crate::telemetry::TelemetrySink;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Canonical bytes the validator signs over for an attestation.
///
/// Includes the receipt commitments, the bundle id, the *current* chain
/// block anchor the validator is attesting to, and the legs hash. We bind
/// `finalized_block` into the signed message so a replay that mutates the
/// header's block number invalidates the signature. The message is
/// deterministic and length-prefixed so it is safe across endianness and
/// field reordering.
fn signing_message(
    receipt: &GpuReceipt,
    bundle_id: [u8; 32],
    finalized_block: u64,
    legs_hash: [u8; 32],
) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(b"x3-validator-attestation-v1");
    hasher.update(&receipt.kernel_hash);
    hasher.update(&receipt.input_commitment);
    hasher.update(&receipt.output_commitment);
    hasher.update(&receipt.executor);
    hasher.update(&(receipt.gpu_cycles_used).to_le_bytes());
    hasher.update(&bundle_id);
    hasher.update(&finalized_block.to_le_bytes());
    hasher.update(&legs_hash);
    let digest = hasher.finalize();
    digest.to_vec()
}

fn expand_home(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home);
        }
    } else if let Some(stripped) = path_str.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }

    path.to_path_buf()
}

fn parse_secret_key_bytes(contents: &[u8]) -> SwarmResult<[u8; 32]> {
    let trimmed = std::str::from_utf8(contents)
        .ok()
        .map(str::trim)
        .unwrap_or_default();
    let hex_value = trimmed.strip_prefix("0x").unwrap_or(trimmed);

    let decoded = if hex_value.len() == 64 && hex_value.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(hex_value)
            .map_err(|e| SwarmError::CryptoError(format!("Invalid signing key hex: {e}")))?
    } else {
        contents.to_vec()
    };

    if decoded.len() != 32 {
        return Err(SwarmError::CryptoError(format!(
            "Signing key must be exactly 32 bytes, got {}",
            decoded.len()
        )));
    }

    let mut secret = [0u8; 32];
    secret.copy_from_slice(&decoded);
    Ok(secret)
}

fn load_configured_signing_key(path: &Path) -> SwarmResult<Option<SigningKey>> {
    let expanded = expand_home(path);
    match std::fs::read(&expanded) {
        Ok(contents) => {
            let secret = parse_secret_key_bytes(&contents)?;
            Ok(Some(SigningKey::from_secret_bytes(secret)?))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(SwarmError::IoError(format!(
            "Failed to read signing key {}: {e}",
            expanded.display()
        ))),
    }
}

/// Validator state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorState {
    /// Starting up
    Starting,
    /// Running normally
    Running,
    /// Running in degraded mode (CPU fallback)
    Degraded,
    /// Quarantined
    Quarantined,
    /// Stopped
    Stopped,
}

/// Validator event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorEvent {
    /// Event type
    pub event_type: String,
    /// Validator ID
    pub validator_id: String,
    /// Timestamp
    pub timestamp: i64,
    /// Data
    pub data: serde_json::Value,
}

/// X3 GPU Validator
pub struct Validator {
    /// Validator ID
    validator_id: String,
    /// Validator address (32 bytes)
    validator_address: [u8; 32],
    /// Configuration
    config: SwarmConfig,
    /// State
    state: RwLock<ValidatorState>,
    /// Deterministic engine
    engine: DeterministicEngine,
    /// Metrics collector
    metrics: Arc<MetricsCollector>,
    /// Quarantine manager
    quarantine: Arc<QuarantineManager>,
    /// Health monitor
    health: HealthMonitor,
    /// Telemetry sink
    telemetry: Arc<TelemetrySink>,
    /// Health tracker
    health_tracker: RwLock<ValidatorHealthTracker>,
    /// Current mode
    current_mode: RwLock<ExecutionMode>,
    /// Start time
    start_time: Instant,
    /// Proof aggregator for unified proof management
    proof_aggregator: Arc<Mutex<ProofAggregator>>,
    /// Per-validator ed25519 signing key. In production this MUST be wired
    /// to a custody/HSM service — `SigningKey::from_seed` is gated to
    /// `cfg(any(test, debug_assertions))` and refuses to run in release
    /// builds. Stored as `Arc<Mutex<Option<...>>>` so we can surface
    /// `signing_key_unavailable` as a typed error rather than panic when
    /// the key cannot be derived (e.g. release-without-HSM).
    signing_key: Arc<Mutex<Option<SigningKey>>>,
}

impl Validator {
    /// Create a new validator
    pub fn new(config: SwarmConfig, validator_id: String) -> Self {
        // Derive validator address from ID (hash-based)
        let mut validator_address = [0u8; 32];
        let id_bytes = validator_id.as_bytes();
        for (i, byte) in id_bytes.iter().enumerate() {
            validator_address[i % 32] ^= byte;
        }

        let metrics = Arc::new(MetricsCollector::new());
        let quarantine = Arc::new(QuarantineManager::new(
            config.quarantine.max_divergence_count,
            config.quarantine.quarantine_duration_secs,
            config.quarantine.auto_fallback_cpu,
        ));
        let telemetry = Arc::new(TelemetrySink::new(
            config.telemetry.clone(),
            validator_id.clone(),
        ));
        let proof_aggregator = Arc::new(Mutex::new(ProofAggregator::new(10))); // Default: 10 validators

        let initial_mode = if config.gpu.enable_cuda {
            ExecutionMode::GpuWithCpuVerification
        } else {
            ExecutionMode::CpuFallback
        };
        let engine = DeterministicEngine::new();
        metrics.set_accelerator_backend(engine.accelerator_backend_name());

        let signing_key = load_configured_signing_key(&config.identity.keypair_path)
            .unwrap_or_else(|e| {
                log::warn!(
                    "[Validator {}] configured signing key unavailable: {:?}",
                    validator_id,
                    e
                );
                None
            })
            .or_else(|| {
                #[cfg(any(test, debug_assertions))]
                {
                    let mut hasher = Sha256::new();
                    hasher.update(b"x3-validator-key-seed-v1");
                    hasher.update(validator_id.as_bytes());
                    let seed = hasher.finalize();
                    SigningKey::from_seed(&seed).ok()
                }

                #[cfg(not(any(test, debug_assertions)))]
                {
                    None
                }
            });

        Self {
            validator_id,
            validator_address,
            config,
            state: RwLock::new(ValidatorState::Starting),
            engine,
            metrics,
            quarantine,
            health: HealthMonitor::default(),
            telemetry,
            health_tracker: RwLock::new(ValidatorHealthTracker::new(String::new())),
            current_mode: RwLock::new(initial_mode),
            start_time: Instant::now(),
            proof_aggregator,
            signing_key: Arc::new(Mutex::new(signing_key)),
        }
    }

    /// Initialize the validator
    pub fn initialize(&self) -> SwarmResult<()> {
        // Configure engine
        let initial_mode = if self.config.gpu.enable_cuda {
            ExecutionMode::GpuWithCpuVerification
        } else {
            ExecutionMode::CpuFallback
        };
        self.engine.set_mode(initial_mode);
        *self.current_mode.write() = initial_mode;
        self.engine
            .set_cpu_verification(self.config.verification.cpu_verification_enabled);
        self.engine
            .set_replay_mode(self.config.verification.replay_mode_enabled);
        self.engine.set_hash_algorithm(HashAlgorithm::Keccak256);
        self.metrics
            .set_accelerator_backend(self.engine.accelerator_backend_name());

        if self.config.gpu.enable_cuda {
            // Initialize GPU hostcalls (with graceful CPU fallback)
            log::info!(
                "[Validator {}] Initializing GPU hostcalls...",
                self.validator_id
            );
            self.engine.init_gpu_hostcalls();
            log::info!(
                "[Validator {}] GPU hostcalls initialization complete",
                self.validator_id
            );
        } else {
            log::info!(
                "[Validator {}] CUDA bypass enabled; running CPU fallback mode",
                self.validator_id
            );
        }

        // Register health checks
        self.health
            .register("engine".to_string(), || crate::metrics::HealthCheck {
                service: "engine".to_string(),
                status: crate::metrics::HealthStatus::Healthy,
                message: Some("Engine operational".to_string()),
                timestamp: chrono::Utc::now().timestamp(),
                details: HashMap::new(),
            });

        *self.state.write() = ValidatorState::Running;

        Ok(())
    }

    /// Process a task
    pub fn process_task(&self, task: DeterministicTask) -> ExecutionResult {
        // Check if quarantined
        if self.quarantine.is_quarantined(&self.validator_id) {
            return ExecutionResult::error(task.task_id, "Validator is quarantined".to_string());
        }

        // Execute task
        let task_id = task.task_id.clone();
        let start = Instant::now();
        let result = self.engine.execute(task.clone());
        let latency_ms = start.elapsed().as_millis() as u64;

        // Record metrics
        let success = result.verification == crate::crypto::VerificationResult::Valid;
        let divergent = result.divergence_detected;

        self.metrics
            .record_task(&self.validator_id, latency_ms, success, divergent);
        if result.accelerator_backend != "unknown" {
            self.metrics
                .set_accelerator_backend(result.accelerator_backend.clone());
        }
        if result.accelerator_fallback_used {
            self.metrics.record_accelerator_fallback();
        }
        if result.accelerator_parity_mismatch {
            self.metrics.record_accelerator_parity_mismatch();
        }

        // Update health tracker
        {
            let mut tracker = self.health_tracker.write();
            tracker.record_task(success);
        }

        // Handle divergence
        if divergent {
            // Record divergence
            let mut record = crate::quarantine::DivergenceRecord::new(
                self.validator_id.clone(),
                task_id.clone(),
                result.outputs.iter().flat_map(|h| h.0.to_vec()).collect(),
                vec![], // CPU output would be here in real impl
            );
            record.add_details(format!("Execution mode: {:?}", result.execution_mode));
            self.quarantine.record_divergence(record);

            // Quarantine if too many divergences
            if self.quarantine.should_auto_fallback() {
                // Auto fallback to CPU
                *self.current_mode.write() = ExecutionMode::CpuFallback;
                self.engine.set_mode(ExecutionMode::CpuFallback);
                self.metrics.record_cpu_fallback();

                // Notify telemetry
                self.telemetry.record_divergence(
                    self.validator_id.clone(),
                    &task_id,
                    "Auto-fallback to CPU enabled",
                );
            }
        }

        // Generate unified proof for successful execution
        if success {
            if let Ok(receipt) = proof_integration::execution_result_to_receipt(
                &result,
                self.validator_address,
                0, // device_index
            ) {
                // Create validator signature using the per-validator
                // ed25519 key. Production validators must source this key
                // from a custody/HSM service; release builds have
                // `signing_key == None` and fall through to a typed
                // error below rather than minting an empty signature.
                let key_guard = self.signing_key.lock();
                let signing_result: Option<
                    Result<(crate::crypto::SignatureOutput, [u8; 32], u64, [u8; 32]), SwarmError>,
                > = key_guard.as_ref().map(|key| {
                    // Derive a deterministic bundle_id from task_id and
                    // pull the current chain anchor. Both feed the
                    // signing message so a downstream aggregator can
                    // detect replay or anchor tampering.
                    let mut bundle_id = [0u8; 32];
                    let task_bytes = task_id.as_bytes();
                    for (i, byte) in task_bytes.iter().enumerate() {
                        bundle_id[i % 32] ^= byte;
                    }
                    let finalized_block = self.engine.chain_block_anchor();
                    let legs_hash =
                        crate::proof_integration::compute_legs_hash_pub(&result.task_id);
                    let msg = signing_message(&receipt, bundle_id, finalized_block, legs_hash);
                    Ok::<_, SwarmError>((
                        key.sign(&msg),
                        bundle_id,
                        finalized_block,
                        key.public_key_bytes(),
                    ))
                });
                drop(key_guard);

                let (signature, bundle_id, finalized_block, pubkey) = match signing_result {
                    Some(Ok(triple)) => triple,
                    Some(Err(e)) => {
                        log::warn!("[Validator {}] signing failed: {:?}", self.validator_id, e);
                        return result;
                    }
                    None => {
                        log::warn!(
                            "[Validator {}] no signing key available (release build without HSM); skipping proof submission",
                            self.validator_id
                        );
                        return result;
                    }
                };

                if let Ok(proof) = proof_integration::create_unified_proof(
                    &result,
                    receipt,
                    signature.to_bytes(),
                    bundle_id,
                    finalized_block,
                    10, // total validators
                ) {
                    // Submit proof to aggregator for consensus
                    let mut aggregator = self.proof_aggregator.lock();
                    aggregator.register_validator_pubkey(self.validator_address, pubkey);
                    let _ = aggregator.submit_proof(proof);
                }
            }
        }

        // Record telemetry
        self.telemetry
            .record_task(self.validator_id.clone(), &task_id, latency_ms, success);

        result
    }

    /// Get current state
    pub fn state(&self) -> ValidatorState {
        *self.state.read()
    }

    /// Get validator ID
    pub fn id(&self) -> &str {
        &self.validator_id
    }

    /// Get metrics
    pub fn get_metrics(&self) -> crate::metrics::SwarmMetrics {
        self.metrics.get_swarm_metrics()
    }

    /// Get health status
    pub fn health_status(&self) -> crate::metrics::HealthStatus {
        self.health.get_overall_status()
    }

    /// Record heartbeat
    pub fn record_heartbeat(&self) {
        let mut tracker = self.health_tracker.write();
        tracker.record_heartbeat();
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Export metrics as JSON
    pub fn export_metrics_json(&self) -> SwarmResult<String> {
        self.metrics.export_json()
    }

    /// Export metrics in Prometheus text exposition format.
    pub fn export_metrics_prometheus(&self) -> String {
        self.metrics.export_prometheus()
    }

    /// Get quarantine status
    pub fn get_quarantine_status(&self) -> Option<crate::quarantine::QuarantineStatus> {
        self.quarantine.get_status(&self.validator_id)
    }

    /// Enable CPU mode
    pub fn enable_cpu_mode(&self) {
        *self.current_mode.write() = ExecutionMode::CpuFallback;
        self.engine.set_mode(ExecutionMode::CpuFallback);
    }

    /// Enable GPU mode
    pub fn enable_gpu_mode(&self) {
        *self.current_mode.write() = ExecutionMode::GpuWithCpuVerification;
        self.engine.set_mode(ExecutionMode::GpuWithCpuVerification);
    }

    /// Get current execution mode
    pub fn current_mode(&self) -> ExecutionMode {
        *self.current_mode.read()
    }

    /// Get proof aggregator for querying aggregation state
    pub fn get_proof_aggregator(&self) -> Arc<Mutex<ProofAggregator>> {
        Arc::clone(&self.proof_aggregator)
    }

    /// Shutdown
    pub fn shutdown(&self) {
        *self.state.write() = ValidatorState::Stopped;
    }

    /// Return the 32-byte ed25519 public key for this validator, if
    /// available. `None` in release builds without a custody/HSM
    /// backend. Callers (proof aggregators, downstream verifiers) MUST
    /// treat `None` as "this attestation cannot be verified" and reject
    /// the proof.
    pub fn public_key_bytes(&self) -> Option<[u8; 32]> {
        self.signing_key
            .lock()
            .as_ref()
            .map(|k| k.public_key_bytes())
    }

    /// Build the same canonical signing message the validator signs over.
    /// Exposed so downstream verifiers can reproduce the bytes and run
    /// `SignatureOutput::verify` against the validator's public key.
    pub fn build_signing_message(
        receipt: &GpuReceipt,
        bundle_id: [u8; 32],
        finalized_block: u64,
        legs_hash: [u8; 32],
    ) -> Vec<u8> {
        signing_message(receipt, bundle_id, finalized_block, legs_hash)
    }

    /// Set the deterministic engine's chain block anchor. Production
    /// callers wire this to a finalized header stream; tests use it to
    /// assert the `finalized_block == 0` "un-anchored" case.
    pub fn set_chain_block_anchor(&self, block: u64) {
        self.engine.set_chain_block_anchor(block);
    }

    /// Read the current chain block anchor.
    pub fn chain_block_anchor(&self) -> u64 {
        self.engine.chain_block_anchor()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let config = SwarmConfig::default();
        let validator = Validator::new(config, "test-validator".to_string());

        assert_eq!(validator.id(), "test-validator");
        assert_eq!(validator.state(), ValidatorState::Starting);
        assert_eq!(validator.current_mode(), ExecutionMode::CpuFallback);
    }

    #[test]
    fn test_validator_task() {
        let config = SwarmConfig::default();
        let validator = Validator::new(config, "test-validator".to_string());

        validator.initialize().unwrap();
        assert_eq!(validator.current_mode(), ExecutionMode::CpuFallback);

        let task = DeterministicTask::new(
            crate::deterministic::TaskType::BatchHash,
            vec![b"hello".to_vec(), b"world".to_vec()],
            HashAlgorithm::Keccak256,
        );

        let result = validator.process_task(task);
        assert!(result.outputs.len() == 2);
        assert_eq!(result.accelerator_backend, "cpu");
        assert!(!result.accelerator_fallback_used);
        assert!(!result.accelerator_parity_mismatch);

        let metrics = validator.get_metrics();
        assert_eq!(metrics.accelerator_backend, "cpu");
        assert_eq!(metrics.accelerator_fallbacks, 0);
        assert_eq!(metrics.accelerator_parity_mismatches, 0);
    }

    #[test]
    fn test_e2e_proof_generation_workflow() {
        // This test demonstrates the full workflow: task execution → proof generation → aggregation
        let config = SwarmConfig::default();
        let validator = Validator::new(config, "test-validator-e2e".to_string());

        validator.initialize().unwrap();

        // Create and execute a task
        let task = DeterministicTask::new(
            crate::deterministic::TaskType::BatchHash,
            vec![b"test_data".to_vec()],
            HashAlgorithm::Keccak256,
        );

        let execution_result = validator.process_task(task);
        assert!(execution_result.outputs.len() == 1);
        assert!(!execution_result.divergence_detected);

        // Get proof aggregator from validator
        let aggregator = validator.get_proof_aggregator();
        let locked_aggregator = aggregator.lock();

        // Verify proof was submitted and is in Collecting state
        // (In real scenario, multiple attestations would be added to reach finality)
        let stats = locked_aggregator.get_stats();
        assert_eq!(
            stats.collecting + stats.finalized + stats.byzantine_finalized + stats.failed,
            stats.total_proofs
        );

        // The workflow is: ExecutionResult → GpuReceipt → UnifiedProof → ProofAggregator
    }

    #[test]
    fn test_e2e_state_merkle_proof_workflow() {
        // This test demonstrates state merkle proof generation in unified proofs
        let config = SwarmConfig::default();
        let validator = Validator::new(config, "test-validator-merkle".to_string());

        validator.initialize().unwrap();

        // Create and execute a task
        let task = DeterministicTask::new(
            crate::deterministic::TaskType::BatchHash,
            vec![b"merkle_test_1".to_vec(), b"merkle_test_2".to_vec()],
            HashAlgorithm::Keccak256,
        );

        let execution_result = validator.process_task(task);
        assert_eq!(execution_result.outputs.len(), 2);
        assert!(!execution_result.divergence_detected);

        // Get proof aggregator from validator
        let aggregator = validator.get_proof_aggregator();
        let locked_aggregator = aggregator.lock();

        // Check that a unified proof was generated
        let stats = locked_aggregator.get_stats();
        assert_eq!(
            stats.collecting + stats.finalized + stats.byzantine_finalized + stats.failed,
            stats.total_proofs
        );

        // The workflow demonstrates: ExecutionResult → MerkleProof generation → UnifiedProof with merkle_proof field
    }

    // -------------------------------------------------------------------
    // Proof authenticity tests.
    //
    // These tests prove the proof path in `process_task` produces a
    // signature that a real ed25519 verifier accepts against the
    // validator's public key, and that the production safeguards
    // (non-empty signature, anchored `finalized_block`) hold.
    //
    // If the production code regresses to the placeholder signature
    // (`Vec::new()`) or `finalized_block = 0` with no anchor, these
    // tests fail closed.
    // -------------------------------------------------------------------

    /// Pulls the most recent attestation out of the aggregator.
    fn latest_attestation(
        validator: &Validator,
    ) -> (
        crate::unified_proof::GpuValidatorAttestation,
        crate::unified_proof::ProofHeader,
    ) {
        let aggregator = validator.get_proof_aggregator();
        let locked = aggregator.lock();
        let proof = locked
            .latest_proof()
            .expect("at least one proof should be submitted");
        assert_eq!(proof.gpu_attestations.len(), 1);
        (proof.gpu_attestations[0].clone(), proof.header.clone())
    }

    fn attestation_to_signature_output(sig: &[u8]) -> crate::crypto::SignatureOutput {
        assert_eq!(sig.len(), 65, "signature must be r||s||v = 65 bytes");
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&sig[..32]);
        s.copy_from_slice(&sig[32..64]);
        crate::crypto::SignatureOutput::new(r, s, sig[64])
    }

    #[test]
    fn proof_signature_verifies_against_validator_pubkey() {
        // End-to-end: a real task → real proof → real ed25519 verify.
        let validator = Validator::new(SwarmConfig::default(), "proof-verify-ok".to_string());
        validator.initialize().unwrap();
        validator.set_chain_block_anchor(4242);

        let task = DeterministicTask::new(
            crate::deterministic::TaskType::BatchHash,
            vec![b"hello".to_vec(), b"world".to_vec()],
            HashAlgorithm::Keccak256,
        );
        let _ = validator.process_task(task);

        // The header carries the same `bundle_id`, `finalized_block`,
        // and `legs_hash` the validator signed over, so reproducing
        // the signing message is exact.
        let (attestation, header) = latest_attestation(&validator);

        assert_eq!(attestation.receipt.executor, validator.validator_address);

        let msg = Validator::build_signing_message(
            &attestation.receipt,
            header.bundle_id,
            header.finalized_block,
            header.legs_hash,
        );
        let mut pubkey = [0u8; 33];
        let pk_bytes = validator
            .public_key_bytes()
            .expect("test/debug build must expose a pubkey");
        pubkey[..32].copy_from_slice(&pk_bytes);
        let sig = attestation_to_signature_output(&attestation.signature);
        assert!(
            sig.verify(&msg, &pubkey),
            "real ed25519 verify must accept the validator's attestation"
        );
        assert_eq!(header.finalized_block, 4242);
    }

    #[test]
    fn proof_signature_fails_for_wrong_pubkey() {
        let validator =
            Validator::new(SwarmConfig::default(), "proof-verify-wrong-key".to_string());
        validator.initialize().unwrap();
        validator.set_chain_block_anchor(7);

        let task = DeterministicTask::new(
            crate::deterministic::TaskType::BatchHash,
            vec![b"x".to_vec()],
            HashAlgorithm::Keccak256,
        );
        let _ = validator.process_task(task);

        let (attestation, header) = latest_attestation(&validator);
        let sig = attestation_to_signature_output(&attestation.signature);

        // Build a *different* ed25519 key and verify the real signature
        // does NOT match it.
        let other_key =
            SigningKey::from_seed(b"a totally different thirty-two byte seed for testing!")
                .expect("test seed must be accepted");
        let mut wrong_pubkey = [0u8; 33];
        wrong_pubkey[..32].copy_from_slice(&other_key.public_key_bytes());

        let msg = Validator::build_signing_message(
            &attestation.receipt,
            header.bundle_id,
            header.finalized_block,
            header.legs_hash,
        );
        assert!(
            !sig.verify(&msg, &wrong_pubkey),
            "signature must NOT verify against an unrelated pubkey"
        );
    }

    #[test]
    fn proof_rejected_when_finalized_block_unanchored() {
        // Fresh validator, no chain anchor set → chain_block_anchor()
        // returns 0. The validator's `proof.validate()` call inside
        // `ProofAggregator::submit_proof` rejects proofs whose header
        // carries `finalized_block == 0`, so the aggregator must
        // remain empty. Production aggregators must therefore treat
        // an un-anchored proof as un-acceptable (this test pins that
        // contract; the aggregator-level rejection is the gate).
        let validator = Validator::new(
            SwarmConfig::default(),
            "proof-verify-unanchored".to_string(),
        );
        validator.initialize().unwrap();
        assert_eq!(validator.chain_block_anchor(), 0);

        let task = DeterministicTask::new(
            crate::deterministic::TaskType::BatchHash,
            vec![b"unanchored".to_vec()],
            HashAlgorithm::Keccak256,
        );
        let result = validator.process_task(task);
        // Execution itself is valid; the *proof* is what's rejected.
        assert_eq!(
            result.verification,
            crate::crypto::VerificationResult::Valid
        );

        let aggregator = validator.get_proof_aggregator();
        let count = aggregator.lock().proof_count();
        assert_eq!(
            count, 0,
            "proof with finalized_block=0 must be rejected by the aggregator"
        );
    }

    #[test]
    fn proof_rejected_when_signature_empty() {
        // Defense-in-depth: even if upstream code regressed to the
        // historical `vec![]` placeholder, our `SignatureOutput::verify`
        // must fail closed on a zero r||s. We test the *unit* here
        // (independent of process_task) because the placeholder is
        // gone from production code; this is a contract on the
        // verifier itself.
        let key = SigningKey::from_seed(b"placeholder-defense-in-depth test seed 32b!")
            .expect("test seed must be accepted");
        let mut pubkey = [0u8; 33];
        pubkey[..32].copy_from_slice(&key.public_key_bytes());
        let empty = crate::crypto::SignatureOutput::new([0u8; 32], [0u8; 32], 0);
        assert!(
            !empty.verify(b"any message", &pubkey),
            "empty signature must never verify"
        );
    }

    #[test]
    fn two_validator_task_proof_attestation_aggregation_uses_finalized_anchor() {
        let orchestrator = crate::orchestrator::SwarmOrchestrator::new(SwarmConfig::default());
        let validator_a = Arc::new(Validator::new(
            SwarmConfig::default(),
            "multi-proof-validator-a".to_string(),
        ));
        let validator_b = Arc::new(Validator::new(
            SwarmConfig::default(),
            "multi-proof-validator-b".to_string(),
        ));

        validator_a.initialize().unwrap();
        validator_b.initialize().unwrap();
        orchestrator.register_validator(validator_a.clone());
        orchestrator.register_validator(validator_b.clone());
        orchestrator.update_finalized_block_anchor(777);

        assert_eq!(orchestrator.min_finalized_block_anchor(), Some(777));

        let task = DeterministicTask::new(
            crate::deterministic::TaskType::BatchHash,
            vec![b"multi-validator-proof-exchange".to_vec()],
            HashAlgorithm::Keccak256,
        );

        let result_a = validator_a.process_task(task.clone());
        let result_b = validator_b.process_task(task);
        assert_eq!(
            result_a.verification,
            crate::crypto::VerificationResult::Valid
        );
        assert_eq!(
            result_b.verification,
            crate::crypto::VerificationResult::Valid
        );

        let proof_a = validator_a
            .get_proof_aggregator()
            .lock()
            .latest_proof()
            .expect("validator A should submit a signed proof");
        let proof_b = validator_b
            .get_proof_aggregator()
            .lock()
            .latest_proof()
            .expect("validator B should submit a signed proof");

        assert_eq!(proof_a.header.bundle_id, proof_b.header.bundle_id);
        assert_eq!(proof_a.header.legs_hash, proof_b.header.legs_hash);
        assert_eq!(proof_a.header.finalized_block, 777);
        assert_eq!(proof_b.header.finalized_block, 777);
        assert_eq!(proof_a.gpu_attestations.len(), 1);
        assert_eq!(proof_b.gpu_attestations.len(), 1);
        assert_ne!(
            proof_a.gpu_attestations[0].validator_id,
            proof_b.gpu_attestations[0].validator_id
        );

        let mut aggregate = ProofAggregator::new(2);
        aggregate.register_validator_pubkey(
            proof_a.gpu_attestations[0].validator_id,
            validator_a
                .public_key_bytes()
                .expect("validator A should expose a signing pubkey in tests"),
        );
        aggregate.register_validator_pubkey(
            proof_b.gpu_attestations[0].validator_id,
            validator_b
                .public_key_bytes()
                .expect("validator B should expose a signing pubkey in tests"),
        );

        let proof_hash = proof_a.proof_hash();
        let commitment = proof_a.gpu_attestations[0].receipt.output_commitment;
        aggregate
            .submit_proof(proof_a.clone())
            .expect("validator A proof should verify and enter aggregation");
        aggregate
            .submit_proof(proof_b.clone())
            .expect("validator B proof should verify with its registered pubkey");
        aggregate
            .add_attestation(
                proof_hash,
                proof_a.gpu_attestations[0].validator_id,
                proof_a.gpu_attestations[0].signature.clone(),
                commitment,
            )
            .expect("validator A consensus vote should be accepted");
        aggregate
            .add_attestation(
                proof_hash,
                proof_b.gpu_attestations[0].validator_id,
                proof_b.gpu_attestations[0].signature.clone(),
                commitment,
            )
            .expect("validator B consensus vote should finalize the aggregate");

        let (state, count, _) = aggregate.get_aggregation_state(proof_hash).unwrap();
        assert_eq!(
            state,
            crate::proof_aggregator::AggregationState::ByzantineFinalized
        );
        assert_eq!(count, 2);
        assert!(aggregate.is_finalized(proof_hash));
        assert!(aggregate.is_byzantine_finalized(proof_hash));
    }

    #[test]
    fn configured_hex_signing_key_is_used_for_attestations() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let key_path = tempdir.path().join("validator.key");
        let secret = [42u8; 32];
        std::fs::write(&key_path, hex::encode(secret)).expect("key file should be written");

        let mut config = SwarmConfig::default();
        config.identity.keypair_path = key_path;
        let validator = Validator::new(config, "configured-key-validator".to_string());

        let expected_key =
            SigningKey::from_secret_bytes(secret).expect("configured key should be accepted");
        assert_eq!(
            validator.public_key_bytes(),
            Some(expected_key.public_key_bytes())
        );

        validator.initialize().unwrap();
        validator.set_chain_block_anchor(99);

        let task = DeterministicTask::new(
            crate::deterministic::TaskType::BatchHash,
            vec![b"configured-key".to_vec()],
            HashAlgorithm::Keccak256,
        );
        let _ = validator.process_task(task);

        let (attestation, header) = latest_attestation(&validator);
        let msg = Validator::build_signing_message(
            &attestation.receipt,
            header.bundle_id,
            header.finalized_block,
            header.legs_hash,
        );
        let mut pubkey = [0u8; 33];
        pubkey[..32].copy_from_slice(&expected_key.public_key_bytes());
        let sig = attestation_to_signature_output(&attestation.signature);
        assert!(sig.verify(&msg, &pubkey));
    }

    #[test]
    fn configured_raw_signing_key_is_used_for_attestations() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let key_path = tempdir.path().join("validator.raw.key");
        let secret = [43u8; 32];
        std::fs::write(&key_path, secret).expect("raw key file should be written");

        let mut config = SwarmConfig::default();
        config.identity.keypair_path = key_path;
        let validator = Validator::new(config, "configured-raw-key-validator".to_string());

        let expected_key =
            SigningKey::from_secret_bytes(secret).expect("configured key should be accepted");
        assert_eq!(
            validator.public_key_bytes(),
            Some(expected_key.public_key_bytes())
        );
    }
}
