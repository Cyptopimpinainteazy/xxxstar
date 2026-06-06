//! CPU/GPU parity: ensures X3VM produces identical results on CPU and GPU paths.
//!
//! X3 validators may execute bytecode on either CPU or GPU (for acceleration).
//! This module provides the determinism contract: same inputs → same outputs
//! regardless of execution path. Includes a harness for parity testing.

use std::sync::Arc;

use x3_backend::bc_format::BytecodeModule;

use crate::{GpuHostcalls, VMConfig, VMResult, Value, VM};

/// The canonical output of a single VM execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionOutput {
    /// Final stack top value (or 0 if stack is empty).
    pub return_value: u64,
    /// Gas consumed during execution.
    pub gas_consumed: u64,
    /// Whether the execution succeeded.
    pub success: bool,
    /// Final state root hash (32 bytes).
    pub state_root: [u8; 32],
}

/// A set of input parameters for a determinism test.
#[derive(Clone, Debug)]
pub struct DeterminismTestCase {
    pub name: &'static str,
    /// Bytecode to execute.
    pub bytecode: Vec<u8>,
    /// Input parameters.
    pub inputs: Vec<u64>,
    /// Gas limit.
    pub gas_limit: u64,
}

/// Result of a CPU/GPU parity check.
#[derive(Debug)]
pub struct ParityResult {
    pub test_name: &'static str,
    pub cpu_output: ExecutionOutput,
    pub gpu_output: ExecutionOutput,
    pub passed: bool,
}

impl ParityResult {
    pub fn check(
        test_name: &'static str,
        cpu_output: ExecutionOutput,
        gpu_output: ExecutionOutput,
    ) -> Self {
        let passed = cpu_output == gpu_output;
        Self {
            test_name,
            cpu_output,
            gpu_output,
            passed,
        }
    }
}

/// Trait implemented by CPU and GPU execution backends.
pub trait ExecutionBackend {
    fn execute(&self, test_case: &DeterminismTestCase) -> ExecutionOutput;
    fn name(&self) -> &'static str;
}

/// Run parity checks across CPU and GPU backends.
pub fn run_parity_suite(
    cases: &[DeterminismTestCase],
    cpu: &dyn ExecutionBackend,
    gpu: &dyn ExecutionBackend,
) -> Vec<ParityResult> {
    cases
        .iter()
        .map(|case| {
            let cpu_out = cpu.execute(case);
            let gpu_out = gpu.execute(case);
            ParityResult::check(case.name, cpu_out, gpu_out)
        })
        .collect()
}

/// CPU backend backed by the real X3VM interpreter.
pub struct RealVmCpuBackend;

impl ExecutionBackend for RealVmCpuBackend {
    fn execute(&self, case: &DeterminismTestCase) -> ExecutionOutput {
        execute_vm_bytes(&case.bytecode, case.gas_limit, &case.inputs, None)
    }

    fn name(&self) -> &'static str {
        "cpu-vm"
    }
}

/// GPU backend backed by the X3 kernel CUDA hostcall runtime.
///
/// This backend executes normal X3BC through the canonical VM interpreter, but
/// registers the real GPU hostcalls before dispatch. Bytecode containing GPU
/// opcodes (0xD0-0xD9) reaches the CUDA FFI libraries loaded by
/// [`GpuHostcalls`]. If `require_gpu` is true and no CUDA libraries are loaded,
/// execution fails instead of silently pretending to be GPU work.
pub struct X3KernelGpuBackend {
    hostcalls: Arc<GpuHostcalls>,
    require_gpu: bool,
}

impl X3KernelGpuBackend {
    /// Load available GPU hostcall libraries and require at least one of them.
    pub fn new() -> Self {
        Self::with_hostcalls(Arc::new(GpuHostcalls::new()), true)
    }

    /// Create a backend with caller-supplied hostcalls.
    pub fn with_hostcalls(hostcalls: Arc<GpuHostcalls>, require_gpu: bool) -> Self {
        Self {
            hostcalls,
            require_gpu,
        }
    }

    /// Return true when at least one CUDA hostcall library was loaded.
    pub fn is_available(&self) -> bool {
        self.hostcalls.is_available()
    }

    /// Execute an already decoded module with GPU hostcalls registered.
    pub fn execute_module(
        &self,
        module: BytecodeModule,
        gas_limit: u64,
    ) -> VMResult<crate::ExecutionResult> {
        if self.require_gpu && !self.is_available() {
            return Err(crate::VMError::without_ip(
                crate::VMErrorKind::HostcallError("X3 kernel GPU runtime unavailable".to_string()),
            ));
        }

        let mut vm = VM::new(module);
        vm.config = VMConfig {
            gas_limit,
            ..vm.config.clone()
        };
        self.hostcalls.register_on_vm(&mut vm);
        vm.call_function(0, &[])
    }
}

impl Default for X3KernelGpuBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionBackend for X3KernelGpuBackend {
    fn execute(&self, case: &DeterminismTestCase) -> ExecutionOutput {
        if self.require_gpu && !self.is_available() {
            return ExecutionOutput {
                return_value: 0,
                gas_consumed: 0,
                success: false,
                state_root: state_root(0, 0, false),
            };
        }
        execute_vm_bytes(
            &case.bytecode,
            case.gas_limit,
            &case.inputs,
            Some(&self.hostcalls),
        )
    }

    fn name(&self) -> &'static str {
        "x3-kernel-gpu"
    }
}

fn execute_vm_bytes(
    bytecode: &[u8],
    gas_limit: u64,
    inputs: &[u64],
    hostcalls: Option<&GpuHostcalls>,
) -> ExecutionOutput {
    let mut vm = match VM::from_bytes(bytecode) {
        Ok(vm) => vm,
        Err(_) => {
            return ExecutionOutput {
                return_value: 0,
                gas_consumed: 0,
                success: false,
                state_root: state_root(0, 0, false),
            };
        }
    };

    if let Some(hostcalls) = hostcalls {
        hostcalls.register_on_vm(&mut vm);
    }

    vm.config = VMConfig {
        gas_limit,
        ..vm.config.clone()
    };
    let args: Vec<Value> = inputs
        .iter()
        .map(|value| Value::I64(*value as i64))
        .collect();
    match vm.call_function(0, &args) {
        Ok(result) => execution_output_from_result(result, true, gas_limit),
        Err(_) => ExecutionOutput {
            return_value: 0,
            gas_consumed: gas_limit,
            success: false,
            state_root: state_root(0, gas_limit, false),
        },
    }
}

fn execution_output_from_result(
    result: crate::ExecutionResult,
    success: bool,
    _gas_limit: u64,
) -> ExecutionOutput {
    let return_value = match result.value {
        Some(Value::I64(value)) => value as u64,
        Some(Value::Bool(value)) => u64::from(value),
        Some(Value::Bytes(bytes)) => bytes
            .iter()
            .take(8)
            .enumerate()
            .fold(0u64, |acc, (idx, byte)| acc | ((*byte as u64) << (idx * 8))),
        _ => 0,
    };
    ExecutionOutput {
        return_value,
        gas_consumed: result.gas_used,
        success,
        state_root: state_root(return_value, result.gas_used, success),
    }
}

fn state_root(return_value: u64, gas_consumed: u64, success: bool) -> [u8; 32] {
    let mut root = [0u8; 32];
    root[..8].copy_from_slice(&return_value.to_le_bytes());
    root[8..16].copy_from_slice(&gas_consumed.to_le_bytes());
    root[16] = u8::from(success);
    root
}

/// Divergent GPU backend — used to verify parity failures are detected.
#[cfg(test)]
pub struct DivergentGpuBackend;

#[cfg(test)]
impl ExecutionBackend for DivergentGpuBackend {
    fn execute(&self, _case: &DeterminismTestCase) -> ExecutionOutput {
        ExecutionOutput {
            return_value: 0xDEADBEEF,
            gas_consumed: 999,
            success: false,
            state_root: [0xFF; 32],
        }
    }
    fn name(&self) -> &'static str {
        "gpu-divergent"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(name: &'static str, inputs: Vec<u64>) -> DeterminismTestCase {
        DeterminismTestCase {
            name,
            bytecode: x3_backend::bc_format_helpers::assemble_simple_module(),
            inputs,
            gas_limit: 10_000,
        }
    }

    #[test]
    fn test_cpu_gpu_parity_passes() {
        let gpu = X3KernelGpuBackend::new();
        if !gpu.is_available() {
            eprintln!("skipping real GPU parity test: X3 kernel GPU runtime unavailable");
            return;
        }
        let cases = vec![
            case("add_two", vec![1, 2]),
            case("add_three", vec![10, 20, 30]),
        ];
        let results = run_parity_suite(&cases, &RealVmCpuBackend, &gpu);
        assert!(
            results.iter().all(|r| r.passed),
            "parity failed: {results:?}"
        );
    }

    #[test]
    fn test_gpu_backend_fails_closed_when_runtime_missing() {
        let gpu = X3KernelGpuBackend::with_hostcalls(Arc::new(GpuHostcalls::disabled()), true);
        if gpu.is_available() {
            eprintln!("skipping unavailable-runtime assertion: GPU runtime is present");
            return;
        }

        let result = gpu.execute(&case("requires_gpu", vec![1, 2]));
        assert!(!result.success);
    }

    #[test]
    fn test_gpu_backend_bypasses_cuda_when_env_enabled() {
        std::env::set_var("X3_BYPASS_CUDA", "1");
        let gpu = X3KernelGpuBackend::new();
        std::env::remove_var("X3_BYPASS_CUDA");

        assert!(!gpu.is_available());
        let result = gpu.execute(&case("bypassed_cuda", vec![1, 2]));
        assert!(!result.success);
    }

    #[test]
    fn test_cpu_gpu_parity_detects_divergence() {
        let cases = vec![case("test_div", vec![5, 6])];
        let results = run_parity_suite(&cases, &RealVmCpuBackend, &DivergentGpuBackend);
        assert!(results.iter().any(|r| !r.passed));
    }

    #[test]
    fn test_parity_result_fields() {
        let cpu = ExecutionOutput {
            return_value: 42,
            gas_consumed: 10,
            success: true,
            state_root: [0u8; 32],
        };
        let gpu = cpu.clone();
        let result = ParityResult::check("test", cpu, gpu);
        assert!(result.passed);
    }

    #[test]
    fn test_identical_outputs_are_equal() {
        let out = ExecutionOutput {
            return_value: 100,
            gas_consumed: 50,
            success: true,
            state_root: [1u8; 32],
        };
        assert_eq!(out.clone(), out);
    }
}
