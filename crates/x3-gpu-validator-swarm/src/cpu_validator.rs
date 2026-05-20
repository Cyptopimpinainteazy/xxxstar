//! CPU Validator - Easy Validation Without GPU
//!
//! This module allows anyone to validate using just their CPU.
//! No GPU required - full participation in the swarm.

use crate::config::SwarmConfig;
use crate::crypto::{HashAlgorithm, HashOutput};
use crate::deterministic::{DeterministicEngine, DeterministicTask, ExecutionMode, TaskType};
use crate::metrics::SwarmMetrics;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// CPU Validator - runs entirely on CPU
pub struct CpuValidator {
    /// Validator ID
    validator_id: String,
    /// Engine (CPU mode)
    engine: DeterministicEngine,
    /// Config
    _config: SwarmConfig,
    /// Start time
    start_time: Instant,
    /// Tasks processed
    tasks_processed: RwLock<u64>,
    /// Successful tasks
    tasks_successful: RwLock<u64>,
}

impl CpuValidator {
    /// Create a new CPU validator
    pub fn new(config: SwarmConfig, validator_id: String) -> Self {
        let engine = DeterministicEngine::new();

        // Force CPU-only mode
        engine.set_mode(ExecutionMode::CpuOnly);

        Self {
            validator_id,
            engine,
            _config: config,
            start_time: Instant::now(),
            tasks_processed: RwLock::new(0),
            tasks_successful: RwLock::new(0),
        }
    }

    /// Process a task on CPU
    pub fn process_task(&self, inputs: Vec<Vec<u8>>, algorithm: HashAlgorithm) -> CpuTaskResult {
        let task = DeterministicTask::new(TaskType::BatchHash, inputs, algorithm);

        let result = self.engine.execute(task);

        // Update counters
        {
            let mut processed = self.tasks_processed.write();
            *processed += 1;
            if result.verification == crate::crypto::VerificationResult::Valid {
                let mut successful = self.tasks_successful.write();
                *successful += 1;
            }
        }

        CpuTaskResult {
            task_id: result.task_id,
            outputs: result.outputs,
            success: result.verification == crate::crypto::VerificationResult::Valid,
            execution_time_ms: result.execution_time_us,
        }
    }

    /// Get validator ID
    pub fn id(&self) -> &str {
        &self.validator_id
    }

    /// Get metrics
    pub fn get_metrics(&self) -> CpuValidatorMetrics {
        let processed = *self.tasks_processed.read();
        let successful = *self.tasks_successful.read();

        CpuValidatorMetrics {
            validator_id: self.validator_id.clone(),
            tasks_processed: processed,
            tasks_successful: successful,
            success_rate: if processed > 0 {
                successful as f64 / processed as f64
            } else {
                0.0
            },
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    /// Get swarm metrics
    pub fn get_swarm_metrics(&self) -> SwarmMetrics {
        let processed = *self.tasks_processed.read();

        SwarmMetrics {
            total_validators: 1,
            active_validators: 1,
            quarantined_validators: 0,
            total_tasks: processed,
            successful_tasks: *self.tasks_successful.read(),
            failed_tasks: processed - *self.tasks_successful.read(),
            divergent_tasks: 0,
            cpu_fallbacks: 0,
            avg_task_latency_ms: 0.0,
            tasks_per_second: 0.0,
        }
    }
}

/// CPU task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuTaskResult {
    pub task_id: String,
    pub outputs: Vec<HashOutput>,
    pub success: bool,
    pub execution_time_ms: u64,
}

/// CPU validator metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuValidatorMetrics {
    pub validator_id: String,
    pub tasks_processed: u64,
    pub tasks_successful: u64,
    pub success_rate: f64,
    pub uptime_secs: u64,
}

/// Easy CPU Validator - Simplified interface for CPU-only validation
pub struct EasyCpuValidator {
    /// Inner CPU validator
    inner: CpuValidator,
}

impl EasyCpuValidator {
    /// Create a new easy CPU validator
    pub fn new(validator_id: String) -> Self {
        let config = SwarmConfig::default();
        Self {
            inner: CpuValidator::new(config, validator_id),
        }
    }

    /// Process a single hash
    pub fn hash(&self, data: &[u8]) -> HashOutput {
        let result = self
            .inner
            .process_task(vec![data.to_vec()], HashAlgorithm::Keccak256);
        result.outputs.into_iter().next().unwrap_or_default()
    }

    /// Process a batch of hashes
    pub fn hash_batch(&self, data: Vec<Vec<u8>>) -> Vec<HashOutput> {
        let result = self.inner.process_task(data, HashAlgorithm::Keccak256);
        result.outputs
    }

    /// Process with any algorithm
    pub fn hash_with(&self, data: &[u8], algorithm: HashAlgorithm) -> HashOutput {
        let result = self.inner.process_task(vec![data.to_vec()], algorithm);
        result.outputs.into_iter().next().unwrap_or_default()
    }

    /// Get metrics
    pub fn metrics(&self) -> CpuValidatorMetrics {
        self.inner.get_metrics()
    }
}

/// Default implementation for easy use
impl Default for EasyCpuValidator {
    fn default() -> Self {
        Self::new(format!("cpu-validator-{}", uuid::Uuid::new_v4()))
    }
}

/// Standalone CPU validation function - no setup required
pub fn validate_cpu(data: &[u8]) -> HashOutput {
    let validator = EasyCpuValidator::default();
    validator.hash(data)
}

/// Standalone CPU batch validation
pub fn validate_cpu_batch(data: Vec<Vec<u8>>) -> Vec<HashOutput> {
    let validator = EasyCpuValidator::default();
    validator.hash_batch(data)
}

/// Standalone validation with any algorithm
pub fn validate_cpu_with(data: &[u8], algorithm: HashAlgorithm) -> HashOutput {
    let validator = EasyCpuValidator::default();
    validator.hash_with(data, algorithm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_validator_creation() {
        let validator = EasyCpuValidator::default();
        let metrics = validator.metrics();

        assert!(!metrics.validator_id.is_empty());
        assert_eq!(metrics.tasks_processed, 0);
    }

    #[test]
    fn test_cpu_hash() {
        let validator = EasyCpuValidator::default();
        let hash = validator.hash(b"hello world");

        assert_ne!(hash.0, [0u8; 32]);
    }

    #[test]
    fn test_cpu_batch() {
        let validator = EasyCpuValidator::default();
        let hashes =
            validator.hash_batch(vec![b"hello".to_vec(), b"world".to_vec(), b"test".to_vec()]);

        assert_eq!(hashes.len(), 3);
    }

    #[test]
    fn test_standalone() {
        let hash = validate_cpu(b"test data");
        assert_ne!(hash.0, [0u8; 32]);
    }

    #[test]
    fn test_standalone_batch() {
        let hashes = validate_cpu_batch(vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);

        assert_eq!(hashes.len(), 3);
    }

    #[test]
    fn test_different_algorithms() {
        let validator = EasyCpuValidator::default();

        let keccak = validator.hash_with(b"test", HashAlgorithm::Keccak256);
        let sha256 = validator.hash_with(b"test", HashAlgorithm::Sha256);

        assert_ne!(keccak.0, sha256.0);
    }
}
