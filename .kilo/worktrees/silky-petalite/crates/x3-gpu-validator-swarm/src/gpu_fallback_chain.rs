//! GPU Fallback Chain: X3 Kernel degradation strategy
//!
//! When X3 kernel execution fails on GPU, automatically degrade to
//! CPU execution with transparent state changes. Avoids validator crash.

use std::collections::VecDeque;

/// Execution target
#[derive(Clone, Debug)]
pub enum ExecutionTarget {
    GPU,  // Primary: X3 kernel on GPU
    CPU,  // Fallback: CPU-only
    Mock, // Test/development
}

/// Degradation strategy
#[derive(Clone, Debug)]
pub enum DegradationStrategy {
    /// Try GPU → CPU if GPU fails
    Cascading,
    /// Try GPU only; fail if unavailable
    Strict,
    /// Always use CPU
    CPUOnly,
}

/// Fallback event (for analytics)
#[derive(Clone, Debug)]
pub struct FallbackEvent {
    pub block_height: u32,
    pub from_target: ExecutionTarget,
    pub to_target: ExecutionTarget,
    pub reason: String,
    pub recovery_time_ms: u64,
}

/// GPU Kernel instance (X3)
#[derive(Clone)]
pub struct X3KernelInstance {
    pub kernel_id: u32,
    pub name: String,
    pub version: String,
    pub is_operational: bool,
    pub last_error: Option<String>,
}

/// CPU Fallback Engine
#[derive(Clone)]
pub struct CPUFallbackEngine {
    pub name: String,
    pub supports_double_precision: bool,
}

impl CPUFallbackEngine {
    pub fn new() -> Self {
        Self {
            name: "scalar-cpu-executor".to_string(),
            supports_double_precision: true,
        }
    }

    /// Execute scalar operation on CPU
    pub fn execute_scalar(&self, op: &str, args: &[u64]) -> Result<Vec<u64>, String> {
        match op {
            "add" => {
                if args.len() != 2 {
                    return Err("ADD requires 2 arguments".to_string());
                }
                Ok(vec![args[0].wrapping_add(args[1])])
            }
            "mul" => {
                if args.len() != 2 {
                    return Err("MUL requires 2 arguments".to_string());
                }
                Ok(vec![args[0].wrapping_mul(args[1])])
            }
            "hash" => Ok(vec![0xDEADBEEFu64]), // Mock hash
            "verify" => Ok(vec![1u64]),        // Mock verify success
            _ => Err(format!("Unknown operation: {}", op)),
        }
    }
}

/// Fallback Chain Manager
pub struct FallbackChain {
    pub strategy: DegradationStrategy,
    pub primary: Option<X3KernelInstance>,
    pub cpu_engine: CPUFallbackEngine,
    pub current_target: ExecutionTarget,
    pub fallback_history: VecDeque<FallbackEvent>,
    pub max_history: usize,
}

impl FallbackChain {
    pub fn new(strategy: DegradationStrategy) -> Self {
        Self {
            strategy,
            primary: None,
            cpu_engine: CPUFallbackEngine::new(),
            current_target: ExecutionTarget::GPU,
            fallback_history: VecDeque::new(),
            max_history: 100,
        }
    }

    /// Attach X3 GPU kernel
    pub fn attach_gpu_kernel(&mut self, kernel: X3KernelInstance) {
        self.primary = Some(kernel);
    }

    /// Execute with automatic fallback
    pub fn execute(
        &mut self,
        op: &str,
        args: &[u64],
        block_height: u32,
    ) -> Result<Vec<u64>, String> {
        match self.strategy {
            DegradationStrategy::Strict => {
                // No fallback: must succeed on GPU
                self.execute_on_gpu(op, args, block_height)
            }

            DegradationStrategy::CPUOnly => {
                // Always use CPU
                self.execute_on_cpu(op, args)
            }

            DegradationStrategy::Cascading => {
                // Try GPU first, fallback to CPU if it fails
                let start = std::time::Instant::now();

                match self.execute_on_gpu(op, args, block_height) {
                    Ok(result) => {
                        self.current_target = ExecutionTarget::GPU;
                        Ok(result)
                    }
                    Err(gpu_error) => {
                        // GPU failed, try CPU
                        eprintln!(
                            "[FallbackChain] GPU execution failed ({}), degrading to CPU",
                            gpu_error
                        );

                        match self.execute_on_cpu(op, args) {
                            Ok(result) => {
                                let recovery_time_ms = start.elapsed().as_millis() as u64;

                                // Log fallback event
                                self.record_fallback(
                                    ExecutionTarget::GPU,
                                    ExecutionTarget::CPU,
                                    gpu_error,
                                    recovery_time_ms,
                                    block_height,
                                );

                                self.current_target = ExecutionTarget::CPU;
                                Ok(result)
                            }
                            Err(cpu_error) => {
                                // Both failed
                                Err(format!("GPU: {}, CPU: {}", gpu_error, cpu_error))
                            }
                        }
                    }
                }
            }
        }
    }

    /// Execute on GPU (X3 kernel)
    fn execute_on_gpu(
        &mut self,
        op: &str,
        _args: &[u64],
        _block_height: u32,
    ) -> Result<Vec<u64>, String> {
        let kernel = self
            .primary
            .as_ref()
            .ok_or("GPU kernel not attached")?
            .clone();

        if !kernel.is_operational {
            return Err(format!("GPU kernel '{}' not operational", kernel.name));
        }

        // Mock GPU execution
        match op {
            "matmul" => Ok(vec![42u64]), // Mock result
            "conv2d" => Ok(vec![100u64]),
            "hash" => Ok(vec![0xCAFEBABEu64]),
            _ => Err(format!("GPU kernel doesn't support operation: {}", op)),
        }
    }

    /// Execute on CPU
    fn execute_on_cpu(&self, op: &str, args: &[u64]) -> Result<Vec<u64>, String> {
        self.cpu_engine.execute_scalar(op, args)
    }

    /// Mark GPU as operational state change
    pub fn set_gpu_operational(&mut self, operational: bool, reason: Option<String>) {
        if let Some(kernel) = &mut self.primary {
            kernel.is_operational = operational;
            if !operational {
                kernel.last_error = reason;
            }
        }
    }

    /// Record fallback event for analytics
    fn record_fallback(
        &mut self,
        from: ExecutionTarget,
        to: ExecutionTarget,
        reason: String,
        recovery_time_ms: u64,
        block_height: u32,
    ) {
        let event = FallbackEvent {
            block_height,
            from_target: from,
            to_target: to,
            reason,
            recovery_time_ms,
        };

        self.fallback_history.push_back(event);

        // Trim history
        while self.fallback_history.len() > self.max_history {
            self.fallback_history.pop_front();
        }
    }

    /// Get fallback statistics
    pub fn get_stats(&self) -> FallbackStats {
        let mut gpu_failures = 0u32;
        let mut max_recovery_time = 0u64;
        let mut avg_recovery_time = 0u64;

        for event in &self.fallback_history {
            gpu_failures += 1;
            max_recovery_time = max_recovery_time.max(event.recovery_time_ms);
            avg_recovery_time += event.recovery_time_ms;
        }

        if gpu_failures > 0 {
            avg_recovery_time /= gpu_failures as u64;
        }

        FallbackStats {
            total_fallbacks: gpu_failures as u32,
            max_recovery_time_ms: max_recovery_time,
            avg_recovery_time_ms: avg_recovery_time,
            current_target: self.current_target.clone(),
        }
    }

    /// Health check: can GPU execute operations?
    pub fn health_check(&self) -> HealthStatus {
        if let Some(kernel) = &self.primary {
            if kernel.is_operational {
                HealthStatus::Healthy
            } else {
                HealthStatus::Degraded(format!(
                    "GPU kernel '{}' offline: {:?}",
                    kernel.name, kernel.last_error
                ))
            }
        } else {
            HealthStatus::CPUOnly
        }
    }

    /// Clear fallback history
    pub fn clear_history(&mut self) {
        self.fallback_history.clear();
    }
}

#[derive(Clone, Debug)]
pub struct FallbackStats {
    pub total_fallbacks: u32,
    pub max_recovery_time_ms: u64,
    pub avg_recovery_time_ms: u64,
    pub current_target: ExecutionTarget,
}

#[derive(Clone, Debug)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    CPUOnly,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_chain_creation() {
        let chain = FallbackChain::new(DegradationStrategy::Cascading);
        assert!(matches!(chain.current_target, ExecutionTarget::GPU));
    }

    #[test]
    fn test_cpu_engine_basic_ops() {
        let engine = CPUFallbackEngine::new();

        let add_result = engine.execute_scalar("add", &[10, 20]);
        assert_eq!(add_result.unwrap(), vec![30]);

        let mul_result = engine.execute_scalar("mul", &[5, 6]);
        assert_eq!(mul_result.unwrap(), vec![30]);
    }

    #[test]
    fn test_cpu_engine_invalid_args() {
        let engine = CPUFallbackEngine::new();

        let result = engine.execute_scalar("add", &[10]); // Missing arg
        assert!(result.is_err());
    }

    #[test]
    fn test_strict_strategy() {
        let mut chain = FallbackChain::new(DegradationStrategy::Strict);

        // Without GPU kernel, should fail
        let result = chain.execute("add", &[1, 2], 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_cpu_only_strategy() {
        let mut chain = FallbackChain::new(DegradationStrategy::CPUOnly);

        // Should succeed even without GPU kernel
        let result = chain.execute("add", &[5, 7], 0);
        assert_eq!(result.unwrap(), vec![12]);
    }

    #[test]
    fn test_cascading_strategy_gpu_available() {
        let mut chain = FallbackChain::new(DegradationStrategy::Cascading);

        let kernel = X3KernelInstance {
            kernel_id: 1,
            name: "test_kernel".to_string(),
            version: "1.0.0".to_string(),
            is_operational: true,
            last_error: None,
        };

        chain.attach_gpu_kernel(kernel);

        let result = chain.execute("matmul", &[], 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cascading_fallback_to_cpu() {
        let mut chain = FallbackChain::new(DegradationStrategy::Cascading);

        let kernel = X3KernelInstance {
            kernel_id: 1,
            name: "faulty_kernel".to_string(),
            version: "1.0.0".to_string(),
            is_operational: false,
            last_error: Some("CUDA timeout".to_string()),
        };

        chain.attach_gpu_kernel(kernel);

        // GPU is down, should cascade to CPU
        let result = chain.execute("add", &[3, 7], 0);
        assert_eq!(result.unwrap(), vec![10]);

        // Should now be on CPU target
        assert!(matches!(chain.current_target, ExecutionTarget::CPU));
    }

    #[test]
    fn test_fallback_history() {
        let mut chain = FallbackChain::new(DegradationStrategy::Cascading);

        let kernel = X3KernelInstance {
            kernel_id: 1,
            name: "kernel".to_string(),
            version: "1.0.0".to_string(),
            is_operational: false,
            last_error: Some("CUDA error".to_string()),
        };

        chain.attach_gpu_kernel(kernel);

        // Trigger fallback
        let _ = chain.execute("add", &[1, 2], 100);

        let stats = chain.get_stats();
        assert!(stats.total_fallbacks > 0);
    }

    #[test]
    fn test_health_check_healthy() {
        let mut chain = FallbackChain::new(DegradationStrategy::Cascading);

        let kernel = X3KernelInstance {
            kernel_id: 1,
            name: "kernel".to_string(),
            version: "1.0.0".to_string(),
            is_operational: true,
            last_error: None,
        };

        chain.attach_gpu_kernel(kernel);

        match chain.health_check() {
            HealthStatus::Healthy => {}
            _ => panic!("Expected healthy status"),
        }
    }

    #[test]
    fn test_health_check_degraded() {
        let mut chain = FallbackChain::new(DegradationStrategy::Cascading);

        let kernel = X3KernelInstance {
            kernel_id: 1,
            name: "kernel".to_string(),
            version: "1.0.0".to_string(),
            is_operational: false,
            last_error: Some("CUDA OOM".to_string()),
        };

        chain.attach_gpu_kernel(kernel);

        match chain.health_check() {
            HealthStatus::Degraded(_) => {}
            _ => panic!("Expected degraded status"),
        }
    }

    #[test]
    fn test_set_gpu_operational() {
        let mut chain = FallbackChain::new(DegradationStrategy::Cascading);

        let kernel = X3KernelInstance {
            kernel_id: 1,
            name: "kernel".to_string(),
            version: "1.0.0".to_string(),
            is_operational: true,
            last_error: None,
        };

        chain.attach_gpu_kernel(kernel);
        chain.set_gpu_operational(false, Some("Manual disable".to_string()));

        assert!(!chain.primary.as_ref().unwrap().is_operational);
    }

    #[test]
    fn test_clear_history() {
        let mut chain = FallbackChain::new(DegradationStrategy::CPUOnly);

        let _ = chain.execute("add", &[1, 2], 0);
        let _ = chain.execute("mul", &[3, 4], 1);

        chain.clear_history();
        assert_eq!(chain.fallback_history.len(), 0);
    }
}
