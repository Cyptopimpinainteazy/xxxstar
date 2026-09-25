//! X3-VM Integration for Bytecode Execution on GPU
//!
//! This module integrates the X3 virtual machine with GPU backends,
//! enabling efficient execution of X3 MIR bytecode on distributed GPUS.

use crate::error::{SwarmError, SwarmResult};
use crate::gpu_backends::{GpuBackendType, GpuExecutor, GpuExecutorManager};
use crate::protocol::TaskResult;
use crate::task::Task;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

#[cfg(feature = "x3-runtime")]
use ::x3_vm::{gpu_hostcalls::GpuHostcalls, VMConfig, Value, Verifier, VerifyOptions, VM};

#[cfg(feature = "x3-runtime")]
use x3_gpu_validator_swarm::gpu_bytecode as bytecode_gen;

/// X3 bytecode execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Interpret bytecode (slower but always works)
    Interpreted,

    /// Just-in-time compile to GPU kernels
    JitCompiled,

    /// Use pre-compiled kernels
    PreCompiled,
}

/// X3 execution profile with optimization hints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X3ExecutionProfile {
    /// Bytecode size
    pub bytecode_size: usize,

    /// Estimated memory requirement
    pub estimated_memory: u64,

    /// Array dimensions (for vectorized operations)
    pub array_dimensions: Vec<usize>,

    /// Parallelization hints
    pub parallelization_hint: String,

    /// Expected output size
    pub expected_output_size: usize,
}

/// X3-VM executor
pub struct X3VmExecutor {
    /// GPU executor manager
    gpu_manager: Arc<GpuExecutorManager>,

    /// Compilation cache
    kernel_cache: Arc<Mutex<std::collections::HashMap<String, Vec<u8>>>>,

    /// Execution mode
    execution_mode: ExecutionMode,

    /// GPU hostcalls for real GPU execution
    #[cfg(feature = "x3-runtime")]
    gpu_hostcalls: Arc<Mutex<Option<Arc<GpuHostcalls>>>>,
}

impl X3VmExecutor {
    /// Create a new X3-VM executor
    pub async fn new(gpu_manager: Arc<GpuExecutorManager>) -> SwarmResult<Self> {
        info!("Initializing X3-VM executor");

        Ok(Self {
            gpu_manager,
            kernel_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            execution_mode: ExecutionMode::JitCompiled,
            #[cfg(feature = "x3-runtime")]
            gpu_hostcalls: Arc::new(Mutex::new(None)),
        })
    }

    /// Execute X3 bytecode task
    pub async fn execute_x3_task(&self, task: &Task, timeout: Duration) -> SwarmResult<TaskResult> {
        debug!("Executing X3 task: {}", task.id);

        // Initialize GPU hostcalls early
        #[cfg(feature = "x3-runtime")]
        let _ = self.init_gpu_hostcalls().await;

        // Parse X3 bytecode from task payload
        let bytecode = match &task.task_type {
            crate::task::TaskType::X3Bytecode { bytecode, .. } => bytecode,
            crate::task::TaskType::Custom { payload, .. } => payload,
            _ => {
                return Err(crate::error::SwarmError::InvalidPayload(
                    "Task type has no bytecode payload".into(),
                ))
            }
        };

        if bytecode.is_empty() {
            return Err(SwarmError::ExecutionError("Empty bytecode".to_string()));
        }

        // Analyze bytecode for optimization
        let profile = self.analyze_bytecode(bytecode)?;
        debug!("X3 bytecode analysis: {:?}", profile);

        // Get best available executor
        let executor = self.gpu_manager.get_best_executor().await.map_err(|e| {
            warn!("No GPU executor available: {}", e);
            e
        })?;

        debug!("Using GPU backend: {}", executor.name());

        // Compile or retrieve cached kernel
        let kernel_id = format!("x3-kernel-{}", blake3::hash(bytecode).to_hex());
        let kernel_binary = self
            .get_or_compile_kernel(executor, &kernel_id, bytecode)
            .await?;

        // Execute on GPU
        match self.execution_mode {
            ExecutionMode::Interpreted => {
                self.interpret_on_gpu(executor, task, bytecode, timeout)
                    .await
            }
            ExecutionMode::JitCompiled => {
                self.jit_execute_on_gpu(executor, task, &kernel_binary, &profile, timeout)
                    .await
            }
            ExecutionMode::PreCompiled => {
                self.execute_precompiled(executor, task, &kernel_binary, timeout)
                    .await
            }
        }
    }

    /// Analyze X3 bytecode for optimization
    pub fn analyze_bytecode(&self, bytecode: &[u8]) -> SwarmResult<X3ExecutionProfile> {
        if bytecode.is_empty() {
            return Err(SwarmError::ExecutionError("Empty bytecode".to_string()));
        }

        #[cfg(feature = "x3-runtime")]
        {
            let options = VerifyOptions::default();
            if let Ok(module) = Verifier::verify_module_bytes(bytecode, &options) {
                if let Ok(instructions) = Verifier::decode_all_instructions(&module.code) {
                    let instruction_count = instructions.len().max(1);
                    let opcode_gas_total: u64 = instructions.iter().map(|i| i.gas_cost).sum();
                    let estimated_memory = ((module.code.len()
                        + module.const_pool.entries.len() * 16
                        + module.globals.len() * 8)
                        as u64)
                        .max(1024);
                    let parallel_hint = if instruction_count >= 2048 {
                        "high-throughput"
                    } else if instruction_count >= 256 {
                        "balanced"
                    } else {
                        "low-latency"
                    };

                    return Ok(X3ExecutionProfile {
                        bytecode_size: bytecode.len(),
                        estimated_memory,
                        array_dimensions: vec![instruction_count.next_power_of_two().min(4096), 1],
                        parallelization_hint: parallel_hint.to_string(),
                        expected_output_size: ((opcode_gas_total / 4) as usize).max(32),
                    });
                }
            }

            warn!("X3 verifier rejected bytecode during profiling; using heuristic profile");
        }

        Ok(X3ExecutionProfile {
            bytecode_size: bytecode.len(),
            estimated_memory: (bytecode.len() as u64) * 100,
            array_dimensions: vec![256, 256],
            parallelization_hint: "maps".to_string(),
            expected_output_size: bytecode.len() * 2,
        })
    }

    /// Get or compile kernel
    async fn get_or_compile_kernel(
        &self,
        executor: &(dyn GpuExecutor),
        kernel_id: &str,
        bytecode: &[u8],
    ) -> SwarmResult<Vec<u8>> {
        {
            let cache = self.kernel_cache.lock().await;
            if let Some(kernel) = cache.get(kernel_id) {
                debug!("Using cached kernel: {}", kernel_id);
                return Ok(kernel.clone());
            }
        }

        debug!("Compiling X3 bytecode to kernel: {}", kernel_id);
        let compiled = self.compile_x3_to_gpu_kernel(executor, bytecode).await?;

        let mut cache = self.kernel_cache.lock().await;
        cache.insert(kernel_id.to_string(), compiled.clone());
        Ok(compiled)
    }

    /// Compile X3 bytecode to GPU kernel
    async fn compile_x3_to_gpu_kernel(
        &self,
        executor: &(dyn GpuExecutor),
        bytecode: &[u8],
    ) -> SwarmResult<Vec<u8>> {
        match executor.compile_kernel(bytecode, "x3_main").await {
            Ok(compiled) if !compiled.is_empty() => Ok(compiled),
            Ok(_) => Err(SwarmError::ExecutionError(
                "compiler returned empty kernel".to_string(),
            )),
            Err(e) => {
                warn!(
                    "Backend kernel compile failed, using portable fallback: {}",
                    e
                );
                let mut kernel = vec![0xc0, 0xd3];
                kernel.extend_from_slice(&(bytecode.len() as u32).to_le_bytes());
                kernel.extend_from_slice(&bytecode[..std::cmp::min(128, bytecode.len())]);
                Ok(kernel)
            }
        }
    }

    /// Interpret bytecode on GPU
    async fn interpret_on_gpu(
        &self,
        executor: &(dyn GpuExecutor),
        task: &Task,
        bytecode: &[u8],
        timeout: Duration,
    ) -> SwarmResult<TaskResult> {
        debug!("Interpreting X3 bytecode on {}", executor.name());

        #[cfg(feature = "x3-runtime")]
        {
            if let Ok(native) = self.execute_on_vm(task, bytecode) {
                return Ok(native);
            }
        }

        // Execute via GPU
        executor.execute(task, 0, timeout).await
    }

    /// JIT compile and execute on GPU
    async fn jit_execute_on_gpu(
        &self,
        executor: &(dyn GpuExecutor),
        task: &Task,
        kernel_binary: &[u8],
        profile: &X3ExecutionProfile,
        timeout: Duration,
    ) -> SwarmResult<TaskResult> {
        debug!("JIT executing X3 bytecode on {}", executor.name());

        // Create execution profile from X3 analysis
        let exec_profile = crate::gpu_backends::ExecutionProfile {
            kernel_name: "x3_main".to_string(),
            grid_size: (
                profile.array_dimensions.get(0).copied().unwrap_or(256) as u32,
                1,
                1,
            ),
            block_size: (256, 1, 1),
            shared_memory: (profile.estimated_memory / 1024) as u32,
            registers_per_thread: 128,
            estimated_time_ms: (profile.bytecode_size as u64) / 100,
        };

        let (result, _metrics) = executor
            .execute_with_profile(task, 0, &exec_profile, timeout)
            .await?;

        Ok(result)
    }

    /// Execute precompiled kernel on GPU
    async fn execute_precompiled(
        &self,
        executor: &(dyn GpuExecutor),
        task: &Task,
        _kernel_binary: &[u8],
        timeout: Duration,
    ) -> SwarmResult<TaskResult> {
        debug!("Executing precompiled X3 kernel on {}", executor.name());

        // Execute precompiled kernel
        executor.execute(task, 0, timeout).await
    }

    #[cfg(feature = "x3-runtime")]
    async fn init_gpu_hostcalls(&self) -> SwarmResult<()> {
        let mut hostcalls_opt = self.gpu_hostcalls.lock().await;
        if hostcalls_opt.is_some() {
            return Ok(());
        }

        let gpu_hostcalls = GpuHostcalls::new();
        if gpu_hostcalls.is_available() {
            info!("GPU hostcalls initialized successfully");
            *hostcalls_opt = Some(Arc::new(gpu_hostcalls));
        } else {
            warn!("GPU hostcalls not available, CPU fallback will be used");
        }
        Ok(())
    }

    #[cfg(feature = "x3-runtime")]
    async fn get_gpu_hostcalls(&self) -> Option<Arc<GpuHostcalls>> {
        let _ = self.init_gpu_hostcalls().await;
        self.gpu_hostcalls.lock().await.clone()
    }

    #[cfg(feature = "x3-runtime")]
    fn bytes_to_hash_output(chunk: &[u8]) -> Option<[u8; 32]> {
        if chunk.len() < 32 {
            return None;
        }
        let mut result = [0u8; 32];
        result.copy_from_slice(&chunk[..32]);
        Some(result)
    }

    #[cfg(feature = "x3-runtime")]
    fn execute_on_vm(&self, task: &Task, bytecode: &[u8]) -> SwarmResult<TaskResult> {
        let gas_limit = match &task.task_type {
            crate::task::TaskType::X3Bytecode { gas_budget, .. } => *gas_budget,
            _ => 1_000_000,
        };

        let module = Verifier::verify_module_bytes(bytecode, &VerifyOptions::default())
            .map_err(|e| SwarmError::ExecutionError(format!("x3 verifier failed: {}", e)))?;
        let mut vm = VM::with_config(
            module,
            VMConfig {
                gas_limit,
                ..VMConfig::default()
            },
        );

        // Try to register GPU hostcalls if available (non-blocking check)
        if let Some(hostcalls_arc) = self
            .gpu_hostcalls
            .try_lock()
            .ok()
            .and_then(|guard| guard.clone())
        {
            debug!("[X3VmExecutor] Registering GPU hostcalls");
            hostcalls_arc.register_on_vm(&mut vm);
            info!("[X3VmExecutor] GPU hostcalls registered, using GPU execution path");
        } else {
            debug!("[X3VmExecutor] GPU hostcalls not available, CPU fallback");
        }

        let exec = vm
            .call_function(0, &[])
            .map_err(|e| SwarmError::ExecutionError(format!("x3 vm execute failed: {:?}", e)))?;

        let result_data = match exec.value {
            Some(Value::Bytes(b)) => b,
            Some(Value::String(s)) => s.into_bytes(),
            Some(Value::I64(v)) => v.to_le_bytes().to_vec(),
            Some(Value::F64(v)) => v.to_le_bytes().to_vec(),
            Some(Value::Bool(v)) => vec![u8::from(v)],
            Some(Value::Addr(v)) => v.to_le_bytes().to_vec(),
            Some(Value::Unit) | None => Vec::new(),
        };

        let mut result_hash = [0u8; 32];
        result_hash.copy_from_slice(blake3::hash(&result_data).as_bytes());

        let mut input_hash = [0u8; 32];
        input_hash.copy_from_slice(blake3::hash(bytecode).as_bytes());
        let mut proof = crate::protocol::ExecutionProof::new(input_hash);
        proof.add_checkpoint(result_hash, exec.instruction_count);
        proof.finalize(result_hash);

        Ok(TaskResult {
            task_id: task.id,
            executor: [0u8; 32],
            success: true,
            result_data,
            result_hash,
            compute_units: exec.gas_used,
            execution_time_ms: exec.instruction_count / 10,
            execution_proof: proof,
            error: None,
            signature: crate::protocol::Signature::default(),
        })
    }

    /// Verify X3 execution deterministically
    pub async fn verify_x3_execution(
        &self,
        task: &Task,
        original_result: &TaskResult,
        timeout: Duration,
    ) -> SwarmResult<bool> {
        debug!("Verifying X3 task execution: {}", task.id);

        // Re-execute task
        let verify_result = self.execute_x3_task(task, timeout).await?;

        // Compare output
        let matches = verify_result.result_data == original_result.result_data;
        info!(
            "X3 execution verification: {} (original: {} bytes, verify: {} bytes)",
            if matches { "PASS" } else { "FAIL" },
            original_result.result_data.len(),
            verify_result.result_data.len()
        );

        Ok(matches)
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.kernel_cache.lock().await;
        let count = cache.len();
        let size: usize = cache.values().map(|k| k.len()).sum();
        (count, size)
    }

    /// Clear kernel cache
    pub async fn clear_cache(&self) {
        let mut cache = self.kernel_cache.lock().await;
        cache.clear();
        info!("X3-VM kernel cache cleared");
    }

    /// Set execution mode
    pub fn set_execution_mode(&mut self, mode: ExecutionMode) {
        info!("X3-VM execution mode: {:?}", mode);
        self.execution_mode = mode;
    }
}

/// X3 task types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum X3TaskType {
    /// Simple arithmetic computation
    Arithmetic,

    /// Linear algebra operation (matrix multiplication, decomposition)
    LinearAlgebra,

    /// Signal processing (FFT, filtering)
    SignalProcessing,

    /// Machine learning inference
    MlInference,

    /// Machine learning training
    MlTraining,

    /// Cryptographic operation
    Cryptographic,

    /// Custom user-defined
    Custom(String),
}

/// X3 task specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X3TaskSpec {
    /// Task type
    pub task_type: X3TaskType,

    /// Bytecode payload
    pub bytecode: Vec<u8>,

    /// Input data
    pub input_data: Vec<u8>,

    /// Execution hints
    pub execution_mode: ExecutionMode,

    /// Preferred GPU backend (None = auto-select)
    pub preferred_backend: Option<GpuBackendType>,

    /// Timeout in seconds
    pub timeout_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn test_x3_bytecode_analysis() {
        let executor = block_on(async {
            let gpu_manager = Arc::new(GpuExecutorManager::new());
            X3VmExecutor::new(gpu_manager).await.unwrap()
        });

        #[cfg(feature = "x3-runtime")]
        let bytecode = ::x3_vm::bridge::bc_format_helpers::assemble_simple_module();
        #[cfg(not(feature = "x3-runtime"))]
        let bytecode = vec![0x01, 0x02, 0x03, 0x04];
        let profile = executor.analyze_bytecode(&bytecode).unwrap();

        assert_eq!(profile.bytecode_size, bytecode.len());
        assert!(profile.estimated_memory > 0);
    }

    #[test]
    fn test_cache_management() {
        let executor = block_on(async {
            let gpu_manager = Arc::new(GpuExecutorManager::new());
            X3VmExecutor::new(gpu_manager).await.unwrap()
        });

        block_on(async {
            executor.clear_cache().await;
            let (count, size) = executor.get_cache_stats().await;
            assert_eq!(count, 0);
            assert_eq!(size, 0);
        });
    }
}
