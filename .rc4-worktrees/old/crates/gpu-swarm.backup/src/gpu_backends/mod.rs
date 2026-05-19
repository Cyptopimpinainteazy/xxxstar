//! GPU backend implementations
//!
//! Provides multiple GPU compute backends:
//! - CUDA for NVIDIA GPUs
//! - Vulkan for cross-platform compute
//! - OpenCL for AMD/Intel GPUs
//! - Metal for Apple Silicon
//! - WebGPU for browser-based nodes

use crate::error::{SwarmError, SwarmResult};
use crate::protocol::TaskResult;
use crate::task::Task;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub mod cuda;
pub mod metal;
pub mod opencl;
pub mod vulkan;
pub mod webgpu;

/// GPU device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDeviceInfo {
    /// Device ID
    pub device_id: u32,

    /// Device name
    pub name: String,

    /// Compute capability (e.g., "8.6" for RTX 3090)
    pub compute_capability: String,

    /// Total memory in bytes
    pub total_memory: u64,

    /// Available memory in bytes
    pub available_memory: u64,

    /// Backend type
    pub backend: GpuBackendType,

    /// Clock speed in MHz
    pub clock_speed_mhz: u32,

    /// Memory bandwidth in GB/s
    pub memory_bandwidth_gbs: f32,

    /// Peak FP32 TFLOPS
    pub peak_fp32_tflops: f32,

    /// Is available
    pub is_available: bool,
}

/// GPU backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GpuBackendType {
    CUDA,
    Vulkan,
    OpenCL,
    Metal,
    WebGPU,
}

impl std::fmt::Display for GpuBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuBackendType::CUDA => write!(f, "CUDA"),
            GpuBackendType::Vulkan => write!(f, "Vulkan"),
            GpuBackendType::OpenCL => write!(f, "OpenCL"),
            GpuBackendType::Metal => write!(f, "Metal"),
            GpuBackendType::WebGPU => write!(f, "WebGPU"),
        }
    }
}

/// Execution profile for GPU kernels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProfile {
    /// Kernel name
    pub kernel_name: String,

    /// Grid size (x, y, z)
    pub grid_size: (u32, u32, u32),

    /// Block/workgroup size (x, y, z)
    pub block_size: (u32, u32, u32),

    /// Shared memory size in bytes
    pub shared_memory: u32,

    /// Registers per thread
    pub registers_per_thread: u32,

    /// Expected execution time in milliseconds
    pub estimated_time_ms: u64,
}

/// Performance metrics for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Task ID
    pub task_id: String,

    /// Backend used
    pub backend: GpuBackendType,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Peak memory used in bytes
    pub peak_memory_bytes: u64,

    /// Average GPU utilization (0-100%)
    pub avg_gpu_utilization: u8,

    /// Average memory bandwidth utilization (0-100%)
    pub avg_memory_utilization: u8,

    /// Power consumption in watts
    pub power_consumption_w: f32,

    /// Achieved throughput in GFLOPS
    pub achieved_gflops: f64,

    /// Framework overhead in milliseconds
    pub framework_overhead_ms: u64,
}

/// GPU executor trait
#[async_trait]
pub trait GpuExecutor: Send + Sync {
    /// Get executor name
    fn name(&self) -> &str;

    /// Get backend type
    fn backend_type(&self) -> GpuBackendType;

    /// Check if executor is available
    async fn is_available(&self) -> bool;

    /// List available GPU devices
    async fn list_devices(&self) -> SwarmResult<Vec<GpuDeviceInfo>>;

    /// Get device info for a specific device
    async fn get_device_info(&self, device_id: u32) -> SwarmResult<GpuDeviceInfo>;

    /// Execute a task on GPU
    async fn execute(
        &self,
        task: &Task,
        device_id: u32,
        timeout: Duration,
    ) -> SwarmResult<TaskResult>;

    /// Execute with execution profile
    async fn execute_with_profile(
        &self,
        task: &Task,
        device_id: u32,
        profile: &ExecutionProfile,
        timeout: Duration,
    ) -> SwarmResult<(TaskResult, PerformanceMetrics)>;

    /// Compile kernel
    async fn compile_kernel(&self, kernel_source: &[u8], kernel_name: &str)
        -> SwarmResult<Vec<u8>>;

    /// Get device memory status
    async fn get_memory_status(&self, device_id: u32) -> SwarmResult<(u64, u64)>;

    /// Set device priority
    async fn set_device_priority(&self, device_id: u32, priority: u32) -> SwarmResult<()>;

    /// Get performance metrics for last execution
    async fn get_last_metrics(&self) -> Option<PerformanceMetrics>;

    /// Reset device
    async fn reset_device(&self, device_id: u32) -> SwarmResult<()>;
}

/// GPU executor manager
pub struct GpuExecutorManager {
    /// Available executors
    executors: Vec<Box<dyn GpuExecutor>>,

    /// Preferred device per backend
    device_preferences: std::collections::HashMap<GpuBackendType, u32>,
}

impl GpuExecutorManager {
    /// Create a new GPU executor manager
    pub fn new() -> Self {
        Self {
            executors: Vec::new(),
            device_preferences: std::collections::HashMap::new(),
        }
    }

    /// Register a GPU executor
    pub fn register(&mut self, executor: Box<dyn GpuExecutor>) {
        tracing::info!("Registered GPU executor: {}", executor.name());
        self.executors.push(executor);
    }

    /// Get executor by backend type
    pub fn get_executor(&self, backend: GpuBackendType) -> SwarmResult<&dyn GpuExecutor> {
        self.executors
            .iter()
            .find(|e| e.backend_type() == backend)
            .map(|e| e.as_ref())
            .ok_or_else(|| {
                SwarmError::ExecutionError(format!("No executor for backend: {}", backend))
            })
    }

    /// Get best available executor (in order of preference)
    pub async fn get_best_executor(&self) -> SwarmResult<&dyn GpuExecutor> {
        for executor in &self.executors {
            if executor.is_available().await {
                return Ok(executor.as_ref());
            }
        }
        Err(SwarmError::ExecutionError(
            "No GPU executors available".to_string(),
        ))
    }

    /// List all available GPU devices
    pub async fn list_all_devices(&self) -> SwarmResult<Vec<GpuDeviceInfo>> {
        let mut devices = Vec::new();
        for executor in &self.executors {
            if executor.is_available().await {
                devices.extend(executor.list_devices().await?);
            }
        }
        Ok(devices)
    }

    /// Execute task with auto-selection
    pub async fn execute_auto(&self, task: &Task, timeout: Duration) -> SwarmResult<TaskResult> {
        let executor = self.get_best_executor().await?;

        // Get first available device
        let devices = executor.list_devices().await?;
        let device_id = devices
            .first()
            .ok_or_else(|| SwarmError::ExecutionError("No GPU devices found".to_string()))?
            .device_id;

        executor.execute(task, device_id, timeout).await
    }

    /// Execute task on specific backend
    pub async fn execute_on_backend(
        &self,
        task: &Task,
        backend: GpuBackendType,
        timeout: Duration,
    ) -> SwarmResult<TaskResult> {
        let executor = self.get_executor(backend)?;
        let device_id = self.device_preferences.get(&backend).copied().unwrap_or(0);

        executor.execute(task, device_id, timeout).await
    }
}

impl Default for GpuExecutorManager {
    fn default() -> Self {
        Self::new()
    }
}
