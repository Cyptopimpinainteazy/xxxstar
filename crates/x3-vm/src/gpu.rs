//! CPU/GPU parity: ensures X3VM produces identical results on CPU and GPU paths.
//!
//! X3 validators may execute bytecode on either CPU or GPU (for acceleration).
//! This module provides the determinism contract: same inputs → same outputs
//! regardless of execution path. Includes a harness for parity testing.

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

/// Stub CPU backend for testing.
#[cfg(test)]
pub struct StubCpuBackend;

#[cfg(test)]
impl ExecutionBackend for StubCpuBackend {
    fn execute(&self, case: &DeterminismTestCase) -> ExecutionOutput {
        // Deterministic stub: return_value = sum of inputs
        let return_value = case.inputs.iter().copied().fold(0u64, u64::wrapping_add);
        ExecutionOutput {
            return_value,
            gas_consumed: case.inputs.len() as u64 * 10,
            success: true,
            state_root: [0u8; 32],
        }
    }
    fn name(&self) -> &'static str {
        "cpu-stub"
    }
}

/// Stub GPU backend for testing (identical output to CPU stub → parity passes).
#[cfg(test)]
pub struct StubGpuBackend;

#[cfg(test)]
impl ExecutionBackend for StubGpuBackend {
    fn execute(&self, case: &DeterminismTestCase) -> ExecutionOutput {
        let return_value = case.inputs.iter().copied().fold(0u64, u64::wrapping_add);
        ExecutionOutput {
            return_value,
            gas_consumed: case.inputs.len() as u64 * 10,
            success: true,
            state_root: [0u8; 32],
        }
    }
    fn name(&self) -> &'static str {
        "gpu-stub"
    }
}

/// Divergent GPU stub — used to verify parity failures are detected.
#[cfg(test)]
pub struct DivergentGpuBackend;

#[cfg(test)]
impl ExecutionBackend for DivergentGpuBackend {
    fn execute(&self, case: &DeterminismTestCase) -> ExecutionOutput {
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
            bytecode: vec![],
            inputs,
            gas_limit: 10_000,
        }
    }

    #[test]
    fn test_cpu_gpu_parity_passes() {
        let cases = vec![
            case("add_two", vec![1, 2]),
            case("add_three", vec![10, 20, 30]),
        ];
        let results = run_parity_suite(&cases, &StubCpuBackend, &StubGpuBackend);
        assert!(
            results.iter().all(|r| r.passed),
            "parity failed: {results:?}"
        );
    }

    #[test]
    fn test_cpu_gpu_parity_detects_divergence() {
        let cases = vec![case("test_div", vec![5, 6])];
        let results = run_parity_suite(&cases, &StubCpuBackend, &DivergentGpuBackend);
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
