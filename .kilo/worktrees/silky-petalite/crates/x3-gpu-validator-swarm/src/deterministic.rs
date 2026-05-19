//! Deterministic Engine for X3 GPU Validator Swarm
//!
//! Provides deterministic GPU execution with CPU verification and replay mode.

use crate::crypto::{HashAlgorithm, HashOutput, VerificationResult};
use crate::error::{SwarmError, SwarmResult};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
use crate::gpu_bytecode;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use x3_vm::{GpuHostcalls, VM};

/// Helper function to convert a 32-byte slice to HashOutput
fn bytes_to_hash_output(chunk: &[u8]) -> HashOutput {
    let mut arr = [0u8; 32];
    arr.copy_from_slice(chunk);
    HashOutput::new(arr)
}

/// Execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutionMode {
    /// GPU-only execution
    GpuOnly,
    /// GPU with CPU verification
    #[default]
    GpuWithCpuVerification,
    /// CPU fallback (when GPU fails or diverges)
    CpuFallback,
    /// CPU-only (for comparison/testing)
    CpuOnly,
}

/// Verification level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VerificationLevel {
    /// Only verify first and last result in batch
    #[default]
    Basic,
    /// Verify all results
    Standard,
    /// Verify all results with multiple algorithms
    Strict,
}

/// Task for deterministic execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterministicTask {
    /// Unique task ID
    pub task_id: String,
    /// Task type (hash, sign, verify, etc.)
    pub task_type: TaskType,
    /// Input data
    pub inputs: Vec<Vec<u8>>,
    /// Hash algorithm to use
    pub hash_algorithm: HashAlgorithm,
    /// Expected output (for verification)
    pub expected_output: Option<HashOutput>,
}

impl DeterministicTask {
    /// Create a new deterministic task
    pub fn new(task_type: TaskType, inputs: Vec<Vec<u8>>, hash_algorithm: HashAlgorithm) -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            task_type,
            inputs,
            hash_algorithm,
            expected_output: None,
        }
    }

    /// Create with expected output for verification
    pub fn with_expected_output(
        task_type: TaskType,
        inputs: Vec<Vec<u8>>,
        hash_algorithm: HashAlgorithm,
        expected: HashOutput,
    ) -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            task_type,
            inputs,
            hash_algorithm,
            expected_output: Some(expected),
        }
    }
}

/// Task type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    /// Hash computation
    Hash,
    /// Batch hash computation
    BatchHash,
    /// Signature verification
    VerifySignature,
    /// Custom computation
    Custom,
}

/// Result of deterministic execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Task ID
    pub task_id: String,
    /// Output hashes
    pub outputs: Vec<HashOutput>,
    /// Verification result
    pub verification: VerificationResult,
    /// Execution mode used
    pub execution_mode: ExecutionMode,
    /// Execution time (microseconds)
    pub execution_time_us: u64,
    /// Whether divergence was detected
    pub divergence_detected: bool,
    /// CPU fallback was used
    pub cpu_fallback_used: bool,
    /// Error message if any
    pub error: Option<String>,
}

impl ExecutionResult {
    /// Create a successful result
    pub fn success(
        task_id: String,
        outputs: Vec<HashOutput>,
        execution_mode: ExecutionMode,
        execution_time_us: u64,
    ) -> Self {
        Self {
            task_id,
            outputs,
            verification: VerificationResult::Valid,
            execution_mode,
            execution_time_us,
            divergence_detected: false,
            cpu_fallback_used: execution_mode == ExecutionMode::CpuFallback,
            error: None,
        }
    }

    /// Create a divergent result
    pub fn divergent(task_id: String, outputs: Vec<HashOutput>, execution_time_us: u64) -> Self {
        Self {
            task_id,
            outputs,
            verification: VerificationResult::Divergent,
            execution_mode: ExecutionMode::GpuWithCpuVerification,
            execution_time_us,
            divergence_detected: true,
            cpu_fallback_used: false,
            error: Some("GPU output diverged from CPU verification".to_string()),
        }
    }

    /// Create an error result
    pub fn error(task_id: String, error: String) -> Self {
        Self {
            task_id,
            outputs: vec![],
            verification: VerificationResult::Invalid,
            execution_mode: ExecutionMode::CpuFallback,
            execution_time_us: 0,
            divergence_detected: false,
            cpu_fallback_used: true,
            error: Some(error),
        }
    }
}

/// Record for replay verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRecord {
    /// Original task
    pub task: DeterministicTask,
    /// Original GPU output
    pub original_output: Vec<HashOutput>,
    /// Replay GPU output
    pub replay_output: Vec<HashOutput>,
    /// CPU verification output
    pub cpu_output: Vec<HashOutput>,
    /// Whether replay matched original
    pub replay_matches_original: bool,
    /// Whether CPU matches GPU
    pub cpu_matches_gpu: bool,
    /// Timestamp
    pub timestamp: i64,
}

/// Deterministic execution engine
pub struct DeterministicEngine {
    /// Execution mode
    mode: RwLock<ExecutionMode>,
    /// Verification level
    verification_level: RwLock<VerificationLevel>,
    /// Hash algorithm
    hash_algorithm: RwLock<HashAlgorithm>,
    /// Enable CPU verification
    cpu_verification_enabled: AtomicBool,
    /// Enable replay mode
    replay_mode_enabled: AtomicBool,
    /// Replay records (for analysis)
    replay_records: RwLock<Vec<ReplayRecord>>,
    /// Statistics
    stats: EngineStats,
    /// GPU hostcalls (initialized on first use)
    gpu_hostcalls: RwLock<Option<Arc<GpuHostcalls>>>,
}

#[derive(Debug, Default)]
pub struct EngineStats {
    pub total_tasks: AtomicU32,
    pub successful_tasks: AtomicU32,
    pub divergent_tasks: AtomicU32,
    pub cpu_fallbacks: AtomicU32,
    pub replay_verifications: AtomicU32,
}

impl DeterministicEngine {
    /// Create a new deterministic engine
    pub fn new() -> Self {
        Self {
            mode: RwLock::new(ExecutionMode::GpuWithCpuVerification),
            verification_level: RwLock::new(VerificationLevel::Standard),
            hash_algorithm: RwLock::new(HashAlgorithm::Keccak256),
            cpu_verification_enabled: AtomicBool::new(true),
            replay_mode_enabled: AtomicBool::new(true),
            replay_records: RwLock::new(Vec::new()),
            stats: EngineStats::default(),
            gpu_hostcalls: RwLock::new(None),
        }
    }

    /// Initialize GPU hostcalls (lazy initialization on first use)
    pub fn init_gpu_hostcalls(&self) -> bool {
        let mut hostcalls_guard = self.gpu_hostcalls.write();
        if hostcalls_guard.is_none() {
            let gpu_hostcalls = GpuHostcalls::new();
            if gpu_hostcalls.is_available() {
                info!("[Deterministic Engine] GPU hostcalls initialized and available");
                *hostcalls_guard = Some(Arc::new(gpu_hostcalls));
                return true;
            } else {
                warn!("[Deterministic Engine] GPU hostcalls unavailable, will use CPU fallback");
                return false;
            }
        }
        hostcalls_guard.is_some()
    }

    /// Get GPU hostcalls if available
    fn get_gpu_hostcalls(&self) -> Option<Arc<GpuHostcalls>> {
        // Try to initialize if not yet done
        self.init_gpu_hostcalls();

        self.gpu_hostcalls.read().clone()
    }

    /// Set execution mode
    pub fn set_mode(&self, mode: ExecutionMode) {
        *self.mode.write() = mode;
    }

    /// Get current execution mode
    pub fn get_mode(&self) -> ExecutionMode {
        *self.mode.read()
    }

    /// Set verification level
    pub fn set_verification_level(&self, level: VerificationLevel) {
        *self.verification_level.write() = level;
    }

    /// Set hash algorithm
    pub fn set_hash_algorithm(&self, algorithm: HashAlgorithm) {
        *self.hash_algorithm.write() = algorithm;
    }

    /// Enable/disable CPU verification
    pub fn set_cpu_verification(&self, enabled: bool) {
        self.cpu_verification_enabled
            .store(enabled, Ordering::SeqCst);
    }

    /// Enable/disable replay mode
    pub fn set_replay_mode(&self, enabled: bool) {
        self.replay_mode_enabled.store(enabled, Ordering::SeqCst);
    }

    /// Execute a deterministic task
    pub fn execute(&self, task: DeterministicTask) -> ExecutionResult {
        let start = std::time::Instant::now();
        let mode = self.get_mode();
        let algorithm = task.hash_algorithm;

        self.stats.total_tasks.fetch_add(1, Ordering::SeqCst);

        // Execute based on mode
        let result = match mode {
            ExecutionMode::GpuOnly => self.execute_gpu(&task, algorithm),
            ExecutionMode::GpuWithCpuVerification => {
                self.execute_with_verification(&task, algorithm)
            }
            ExecutionMode::CpuFallback | ExecutionMode::CpuOnly => {
                self.execute_cpu(&task, algorithm)
            }
        };

        // Handle result
        let execution_time_us = start.elapsed().as_micros() as u64;
        match result {
            Ok(mut exec_result) => {
                exec_result.execution_time_us = execution_time_us;
                self.stats.successful_tasks.fetch_add(1, Ordering::SeqCst);
                exec_result
            }
            Err(e) => {
                // Fallback to CPU on error
                if mode != ExecutionMode::CpuFallback && mode != ExecutionMode::CpuOnly {
                    self.stats.cpu_fallbacks.fetch_add(1, Ordering::SeqCst);
                    let cpu_result = self.execute_cpu(&task, algorithm);
                    match cpu_result {
                        Ok(mut cpu_exec_result) => {
                            cpu_exec_result.execution_time_us = execution_time_us;
                            cpu_exec_result
                        }
                        Err(cpu_e) => ExecutionResult::error(task.task_id, cpu_e.to_string()),
                    }
                } else {
                    ExecutionResult::error(task.task_id, e.to_string())
                }
            }
        }
    }

    /// Execute on GPU using real GPU hostcalls via X3 VM
    fn execute_gpu(
        &self,
        task: &DeterministicTask,
        algorithm: HashAlgorithm,
    ) -> SwarmResult<ExecutionResult> {
        // Without GPU features compiled in, route straight to CPU
        #[cfg(not(any(
            feature = "cuda",
            feature = "opencl",
            feature = "metal",
            feature = "vulkan"
        )))]
        return self.execute_cpu(task, algorithm);

        // With GPU features: use GPU hostcalls
        #[cfg(any(
            feature = "cuda",
            feature = "opencl",
            feature = "metal",
            feature = "vulkan"
        ))]
        {
            let gpu_hostcalls = match self.get_gpu_hostcalls() {
                Some(hostcalls) => hostcalls,
                None => {
                    warn!(
                        "[Deterministic Engine] GPU unavailable for task {}, using CPU",
                        task.task_id
                    );
                    return self.execute_cpu(task, algorithm);
                }
            };

            match self.exec_on_gpu_device(task, algorithm, gpu_hostcalls) {
                Ok(outputs) => {
                    debug!(
                        "[Deterministic Engine] GPU execution completed for task {} (count: {})",
                        task.task_id,
                        outputs.len()
                    );
                    Ok(ExecutionResult::success(
                        task.task_id.clone(),
                        outputs,
                        ExecutionMode::GpuOnly,
                        0,
                    ))
                }
                Err(e) => {
                    error!(
                        "[Deterministic Engine] GPU execution failed for task {}: {:?}",
                        task.task_id, e
                    );
                    Err(e)
                }
            }
        }
    }

    /// Low-level GPU execution via X3 VM bytecode dispatch (requires a GPU feature flag)
    #[cfg(any(
        feature = "cuda",
        feature = "opencl",
        feature = "metal",
        feature = "vulkan"
    ))]
    fn exec_on_gpu_device(
        &self,
        task: &DeterministicTask,
        algorithm: HashAlgorithm,
        gpu_hostcalls: Arc<GpuHostcalls>,
    ) -> SwarmResult<Vec<HashOutput>> {
        let mut batch_data = Vec::new();
        for input in &task.inputs {
            batch_data.extend_from_slice(input);
        }

        let module = gpu_bytecode::generate_gpu_bytecode_for_algorithm(
            algorithm,
            batch_data,
            task.inputs.len() as i64,
        );

        let mut vm = VM::new(module);
        gpu_hostcalls.register_on_vm(&mut vm);

        match vm.call_function(0, &[]) {
            Ok(execution_result) => match execution_result.value {
                Some(x3_vm::Value::Bytes(hashes)) => {
                    let output_size = 32;
                    let mut outputs = Vec::new();
                    for chunk in hashes.chunks(output_size) {
                        outputs.push(bytes_to_hash_output(chunk));
                    }
                    Ok(outputs)
                }
                other => {
                    error!(
                        "[Deterministic Engine] GPU returned unexpected value type: {:?}",
                        other
                    );
                    Err(SwarmError::GpuError(format!(
                        "GPU returned unexpected value: {:?}",
                        other
                    )))
                }
            },
            Err(e) => {
                error!(
                    "[Deterministic Engine] GPU execution failed: {:?}",
                    e
                );
                Err(SwarmError::GpuError(format!("GPU execution failed: {}", e)))
            }
        }
    }

    /// Compare outputs with tolerance for floating point precision
    fn compare_with_tolerance(a: &[HashOutput], b: &[HashOutput], tolerance: f64) -> bool {
        if a.len() != b.len() {
            return false;
        }

        for (a_val, b_val) in a.iter().zip(b.iter()) {
            if !Self::within_tolerance(a_val, b_val, tolerance) {
                return false;
            }
        }
        true
    }

    /// Check if two hash outputs are within tolerance
    fn within_tolerance(a: &HashOutput, b: &HashOutput, _tolerance: f64) -> bool {
        // For hash outputs, we do exact comparison
        // For numerical computations, tolerance would be applied to decoded values
        a == b
    }

    /// Check if divergence is acceptable (minor floating point differences)
    fn is_acceptable_divergence(gpu_outputs: &[HashOutput], cpu_outputs: &[HashOutput]) -> bool {
        // For hash operations, any divergence is unacceptable
        // For numerical computations with floating point, small differences are acceptable
        gpu_outputs == cpu_outputs
    }

    /// Execute on GPU with CPU verification
    fn execute_with_verification(
        &self,
        task: &DeterministicTask,
        algorithm: HashAlgorithm,
    ) -> SwarmResult<ExecutionResult> {
        #[cfg(not(any(
            feature = "cuda",
            feature = "opencl",
            feature = "metal",
            feature = "vulkan"
        )))]
        return self.execute_cpu(task, algorithm);

        #[cfg(any(
            feature = "cuda",
            feature = "opencl",
            feature = "metal",
            feature = "vulkan"
        ))]
        self.execute_with_verification_gpu(task, algorithm)
    }

    /// GPU+CPU dual-verification logic (only compiled when a GPU backend is enabled)
    #[cfg(any(
        feature = "cuda",
        feature = "opencl",
        feature = "metal",
        feature = "vulkan"
    ))]
    fn execute_with_verification_gpu(
        &self,
        task: &DeterministicTask,
        algorithm: HashAlgorithm,
    ) -> SwarmResult<ExecutionResult> {
        let cpu_verification_enabled = self.cpu_verification_enabled.load(Ordering::SeqCst);
        let replay_mode_enabled = self.replay_mode_enabled.load(Ordering::SeqCst);
        let verification_level = *self.verification_level.read();

        // Step 1: Execute on GPU
        let gpu_outputs: Vec<HashOutput> = {
            let gpu_hostcalls = match self.get_gpu_hostcalls() {
                Some(hostcalls) => hostcalls,
                None => {
                    warn!(
                        "[Deterministic Engine] GPU unavailable for task {}, using CPU verification only",
                        task.task_id
                    );
                    let cpu_outputs: Vec<HashOutput> = task
                        .inputs
                        .iter()
                        .map(|input| crate::crypto::compute_hash(&algorithm, input))
                        .collect();
                    return Ok(ExecutionResult::success(
                        task.task_id.clone(),
                        cpu_outputs,
                        ExecutionMode::CpuFallback,
                        0,
                    ));
                }
            };

            match self.exec_on_gpu_device(task, algorithm, gpu_hostcalls) {
                Ok(outputs) => {
                    debug!(
                        "[Deterministic Engine] GPU execution completed for task {} (count: {})",
                        task.task_id,
                        outputs.len()
                    );
                    outputs
                }
                Err(e) => {
                    error!(
                        "[Deterministic Engine] GPU execution failed for task {}: {:?}",
                        task.task_id, e
                    );
                    return Err(crate::error::SwarmError::GpuError(format!(
                        "GPU execution failed: {}",
                        e
                    )));
                }
            }
        };

        // Step 2: CPU verification (if enabled)
        if cpu_verification_enabled {
            let cpu_outputs: Vec<HashOutput> = task
                .inputs
                .iter()
                .map(|input| crate::crypto::compute_hash(&algorithm, input))
                .collect();

            let needs_verification = match verification_level {
                VerificationLevel::Basic | VerificationLevel::Standard | VerificationLevel::Strict => true,
            };

            let outputs_match = if task.task_type == TaskType::Custom {
                Self::compare_with_tolerance(&gpu_outputs, &cpu_outputs, 1e-9)
            } else {
                gpu_outputs == cpu_outputs
            };

            if needs_verification && !outputs_match {
                if Self::is_acceptable_divergence(&gpu_outputs, &cpu_outputs) {
                    warn!(
                        "[Deterministic Engine] Minor divergence detected within tolerance for task {}",
                        task.task_id
                    );
                } else {
                    // Step 3: Replay mode - re-run GPU to confirm divergence
                    if replay_mode_enabled {
                        info!(
                            "[Deterministic Engine] Divergence detected for task {}, entering replay mode",
                            task.task_id
                        );

                        let gpu_hostcalls = match self.get_gpu_hostcalls() {
                            Some(hostcalls) => hostcalls,
                            None => {
                                error!(
                                    "[Deterministic Engine] GPU unavailable during replay for task {}",
                                    task.task_id
                                );
                                self.stats.divergent_tasks.fetch_add(1, Ordering::SeqCst);
                                return Ok(ExecutionResult::divergent(
                                    task.task_id.clone(),
                                    gpu_outputs,
                                    0,
                                ));
                            }
                        };

                        let replay_outputs =
                            match self.exec_on_gpu_device(task, algorithm, gpu_hostcalls) {
                                Ok(outputs) => outputs,
                                Err(e) => {
                                    error!(
                                        "[Deterministic Engine] GPU replay failed for task {}: {}",
                                        task.task_id, e
                                    );
                                    gpu_outputs.clone()
                                }
                            };

                        let record = ReplayRecord {
                            task: task.clone(),
                            original_output: gpu_outputs.clone(),
                            replay_output: replay_outputs.clone(),
                            cpu_output: cpu_outputs.clone(),
                            replay_matches_original: gpu_outputs == replay_outputs,
                            cpu_matches_gpu: false,
                            timestamp: chrono::Utc::now().timestamp(),
                        };
                        self.replay_records.write().push(record);
                        self.stats.replay_verifications.fetch_add(1, Ordering::SeqCst);

                        if gpu_outputs == replay_outputs {
                            error!(
                                "[Deterministic Engine] GPU divergence confirmed after replay for task {} (GPU differs from CPU)",
                                task.task_id
                            );
                            self.stats.divergent_tasks.fetch_add(1, Ordering::SeqCst);
                            return Ok(ExecutionResult::divergent(
                                task.task_id.clone(),
                                gpu_outputs,
                                0,
                            ));
                        }
                    }

                    error!(
                        "[Deterministic Engine] GPU/CPU divergence detected for task {}",
                        task.task_id
                    );
                    self.stats.divergent_tasks.fetch_add(1, Ordering::SeqCst);
                    return Ok(ExecutionResult::divergent(
                        task.task_id.clone(),
                        gpu_outputs,
                        0,
                    ));
                }
            }
        }

        debug!(
            "[Deterministic Engine] Task {} verification passed",
            task.task_id
        );
        Ok(ExecutionResult::success(
            task.task_id.clone(),
            gpu_outputs,
            ExecutionMode::GpuWithCpuVerification,
            0,
        ))
    }

    /// Execute on CPU only
    fn execute_cpu(
        &self,
        task: &DeterministicTask,
        algorithm: HashAlgorithm,
    ) -> SwarmResult<ExecutionResult> {
        let outputs: Vec<HashOutput> = task
            .inputs
            .iter()
            .map(|input| crate::crypto::compute_hash(&algorithm, input))
            .collect();

        Ok(ExecutionResult::success(
            task.task_id.clone(),
            outputs,
            ExecutionMode::CpuFallback,
            0,
        ))
    }

    /// Get replay records
    pub fn get_replay_records(&self) -> Vec<ReplayRecord> {
        self.replay_records.read().clone()
    }

    /// Get engine statistics
    pub fn get_stats(&self) -> EngineStats {
        EngineStats {
            total_tasks: AtomicU32::new(self.stats.total_tasks.load(Ordering::SeqCst)),
            successful_tasks: AtomicU32::new(self.stats.successful_tasks.load(Ordering::SeqCst)),
            divergent_tasks: AtomicU32::new(self.stats.divergent_tasks.load(Ordering::SeqCst)),
            cpu_fallbacks: AtomicU32::new(self.stats.cpu_fallbacks.load(Ordering::SeqCst)),
            replay_verifications: AtomicU32::new(
                self.stats.replay_verifications.load(Ordering::SeqCst),
            ),
        }
    }

    /// Clear replay records
    pub fn clear_records(&self) {
        self.replay_records.write().clear();
    }
}

impl Default for DeterministicEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a deterministic task for batch hashing
pub fn create_batch_hash_task(inputs: Vec<Vec<u8>>, algorithm: HashAlgorithm) -> DeterministicTask {
    DeterministicTask::new(TaskType::BatchHash, inputs, algorithm)
}

/// Create a deterministic task for verification
pub fn create_verification_task(
    inputs: Vec<Vec<u8>>,
    expected: HashOutput,
    algorithm: HashAlgorithm,
) -> DeterministicTask {
    DeterministicTask::with_expected_output(TaskType::BatchHash, inputs, algorithm, expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_execution() {
        let engine = DeterministicEngine::new();

        let inputs = vec![b"hello".to_vec(), b"world".to_vec(), b"test".to_vec()];

        let task = create_batch_hash_task(inputs, HashAlgorithm::Keccak256);
        let result = engine.execute(task);

        assert_eq!(result.verification, VerificationResult::Valid);
        assert_eq!(result.outputs.len(), 3);
    }

    #[test]
    fn test_cpu_fallback() {
        let engine = DeterministicEngine::new();
        engine.set_mode(ExecutionMode::CpuFallback);

        let inputs = vec![b"test".to_vec()];
        let task = create_batch_hash_task(inputs, HashAlgorithm::Keccak256);
        let result = engine.execute(task);

        assert!(result.cpu_fallback_used);
    }

    #[test]
    fn test_replay_mode() {
        let engine = DeterministicEngine::new();
        engine.set_replay_mode(true);

        let inputs = vec![b"test".to_vec()];
        let task = create_batch_hash_task(inputs, HashAlgorithm::Keccak256);

        // Run multiple times - should be deterministic
        let result1 = engine.execute(task.clone());
        let result2 = engine.execute(task);

        assert_eq!(result1.outputs, result2.outputs);
    }
}
