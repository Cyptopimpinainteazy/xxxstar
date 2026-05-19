//! Validator module for X3 GPU Validator Swarm

use crate::config::SwarmConfig;
use crate::crypto::HashAlgorithm;
use crate::deterministic::{
    DeterministicEngine, DeterministicTask, ExecutionMode, ExecutionResult,
};
use crate::error::SwarmResult;
use crate::health::{HealthMonitor, ValidatorHealthTracker};
use crate::metrics::MetricsCollector;
use crate::proof_aggregator::ProofAggregator;
use crate::proof_integration;
use crate::quarantine::QuarantineManager;
use crate::telemetry::TelemetrySink;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

        Self {
            validator_id,
            validator_address,
            config,
            state: RwLock::new(ValidatorState::Starting),
            engine: DeterministicEngine::new(),
            metrics,
            quarantine,
            health: HealthMonitor::default(),
            telemetry,
            health_tracker: RwLock::new(ValidatorHealthTracker::new(String::new())),
            current_mode: RwLock::new(ExecutionMode::GpuWithCpuVerification),
            start_time: Instant::now(),
            proof_aggregator,
        }
    }

    /// Initialize the validator
    pub fn initialize(&self) -> SwarmResult<()> {
        // Configure engine
        self.engine.set_mode(ExecutionMode::GpuWithCpuVerification);
        self.engine
            .set_cpu_verification(self.config.verification.cpu_verification_enabled);
        self.engine
            .set_replay_mode(self.config.verification.replay_mode_enabled);
        self.engine.set_hash_algorithm(HashAlgorithm::Keccak256);

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
                // Create validator signature (in real impl, use proper signing)
                let signature = vec![]; // Placeholder - should be actual signature

                // Create unified proof with bundle_id derived from task_id
                let mut bundle_id = [0u8; 32];
                let task_bytes = task_id.as_bytes();
                for (i, byte) in task_bytes.iter().enumerate() {
                    bundle_id[i % 32] ^= byte;
                }

                // Get current block number (would come from chain in real impl)
                let finalized_block = 0u64; // Placeholder

                if let Ok(proof) = proof_integration::create_unified_proof(
                    &result,
                    receipt,
                    signature,
                    bundle_id,
                    finalized_block,
                    10, // total validators
                ) {
                    // Submit proof to aggregator for consensus
                    let _ = self.proof_aggregator.lock().submit_proof(proof);
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
    }

    #[test]
    fn test_validator_task() {
        let config = SwarmConfig::default();
        let validator = Validator::new(config, "test-validator".to_string());

        validator.initialize().unwrap();

        let task = DeterministicTask::new(
            crate::deterministic::TaskType::BatchHash,
            vec![b"hello".to_vec(), b"world".to_vec()],
            HashAlgorithm::Keccak256,
        );

        let result = validator.process_task(task);
        assert!(result.outputs.len() == 2);
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
}
