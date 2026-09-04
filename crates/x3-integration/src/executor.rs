//! X3 VM Executor for X3 Kernel
//!
//! This module provides the main executor that bridges the X3 VM with
//! the X3 Kernel pallet's execution interface.

#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use sp_core::H256;

use crate::error::{X3IntegrationError, X3Result};
use crate::types::{X3ExecutionReceipt, X3GasConfig, X3Value};

#[cfg(feature = "std")]
use x3_vm::{Verifier, VerifyOptions, VM};

/// Configuration for X3 executor
#[derive(Clone, Debug)]
pub struct X3ExecutorConfig {
    /// Maximum gas allowed for execution
    pub gas_limit: u64,
    /// Maximum call stack depth
    pub max_call_depth: usize,
    /// Maximum operand stack size
    pub max_stack_size: usize,
    /// Enable execution tracing (debug only)
    pub trace: bool,
    /// Gas cost configuration
    pub gas_config: X3GasConfig,
    /// Allow debug opcodes (NEVER for on-chain)
    pub allow_debug_ops: bool,
}

impl Default for X3ExecutorConfig {
    fn default() -> Self {
        Self {
            gas_limit: 1_000_000,
            max_call_depth: 64,
            max_stack_size: 1024,
            trace: false,
            gas_config: X3GasConfig::default(),
            allow_debug_ops: false,
        }
    }
}

impl X3ExecutorConfig {
    /// Create config for on-chain execution (strict)
    pub fn on_chain() -> Self {
        Self {
            gas_limit: 500_000,
            max_call_depth: 32,
            max_stack_size: 512,
            trace: false,
            gas_config: X3GasConfig::default(),
            allow_debug_ops: false,
        }
    }

    /// Create config for off-chain simulation (permissive)
    pub fn simulation() -> Self {
        Self {
            gas_limit: 10_000_000,
            max_call_depth: 128,
            max_stack_size: 4096,
            trace: true,
            gas_config: X3GasConfig::default(),
            allow_debug_ops: true,
        }
    }

    /// Set gas limit
    pub fn with_gas_limit(mut self, limit: u64) -> Self {
        self.gas_limit = limit;
        self
    }
}

/// Main X3 executor
pub struct X3Executor;

impl X3Executor {
    /// Execute X3 bytecode and return execution receipt
    ///
    /// This is the main entry point for on-chain X3 execution:
    /// 1. Verify bytecode (mandatory)
    /// 2. Create VM instance
    /// 3. Execute function
    /// 4. Collect state changes and logs
    /// 5. Return receipt
    #[cfg(feature = "std")]
    pub fn execute(
        bytecode: &[u8],
        args: &[X3Value],
        config: X3ExecutorConfig,
    ) -> X3Result<X3ExecutionReceipt> {
        // Step 1: Verify bytecode
        let verify_opts = if config.allow_debug_ops {
            VerifyOptions::default()
        } else {
            VerifyOptions::on_chain()
        };
        Verifier::verify_module_bytes(bytecode, &verify_opts)
            .map_err(|e| X3IntegrationError::VerificationFailed(format!("{:?}", e)))?;

        // Step 2: Create VM
        let mut vm = VM::from_bytes(bytecode)
            .map_err(|e| X3IntegrationError::InvalidBytecode(format!("{:?}", e)))?;

        // Step 3: Convert arguments to VM values
        let vm_args: Vec<x3_vm::Value> = args
            .iter()
            .map(|arg| match arg {
                X3Value::I64(v) => x3_vm::Value::I64(*v),
                X3Value::F64Bits(bits) => x3_vm::Value::F64(f64::from_bits(*bits)),
                X3Value::Bool(v) => x3_vm::Value::Bool(*v),
                X3Value::Bytes(v) => x3_vm::Value::Bytes(v.clone()),
                X3Value::Address(v) => x3_vm::Value::Bytes(v.clone()),
                X3Value::Unit => x3_vm::Value::Unit,
            })
            .collect();

        // Step 4: Execute (function 0 = main/entry)
        let result = vm.call_function(0, &vm_args);

        // Step 5: Build receipt
        let gas_used = vm.gas_used();

        match result {
            Ok(exec_result) => {
                let return_data = match exec_result.value {
                    Some(x3_vm::Value::I64(v)) => v.to_le_bytes().to_vec(),
                    Some(x3_vm::Value::F64(v)) => v.to_bits().to_le_bytes().to_vec(),
                    Some(x3_vm::Value::Bool(v)) => vec![v as u8],
                    Some(x3_vm::Value::Bytes(v)) => v,
                    Some(x3_vm::Value::String(s)) => s.into_bytes(),
                    Some(x3_vm::Value::Unit) => vec![],
                    Some(x3_vm::Value::Addr(a)) => a.to_le_bytes().to_vec(),
                    None => vec![],
                };

                Ok(X3ExecutionReceipt {
                    success: true,
                    gas_used,
                    return_data,
                    logs: vec![], // Hostcall log collection deferred to runtime integration
                    state_changes: vec![], // Hostcall state change collection deferred to runtime integration
                    function_index: 0,
                    instructions_executed: 0, // Instruction counting requires VM instrumentation
                })
            }
            Err(vm_err) => {
                // Check for gas exhaustion
                if gas_used >= config.gas_limit {
                    return Err(X3IntegrationError::GasExhausted {
                        used: gas_used,
                        limit: config.gas_limit,
                    });
                }

                Ok(X3ExecutionReceipt {
                    success: false,
                    gas_used,
                    return_data: format!("{:?}", vm_err).into_bytes(),
                    logs: vec![],
                    state_changes: vec![],
                    function_index: 0,
                    instructions_executed: 0, // Instruction counting requires VM instrumentation
                })
            }
        }
    }

    /// Execute X3BC bytecode (no_std — uses mini_x3 interpreter)
    #[cfg(not(feature = "std"))]
    pub fn execute(
        bytecode: &[u8],
        _args: &[X3Value],
        config: X3ExecutorConfig,
    ) -> X3Result<X3ExecutionReceipt> {
        use crate::mini_x3;
        match mini_x3::execute_x3bc(bytecode, config.gas_limit) {
            Ok(res) => Ok(X3ExecutionReceipt {
                success: true,
                gas_used: res.gas_used,
                return_data: res.return_val.to_bytes(),
                logs: vec![],
                state_changes: vec![],
                function_index: 0,
                instructions_executed: res.gas_used,
            }),
            Err(mini_x3::X3Error::GasExhausted) => Ok(X3ExecutionReceipt {
                success: false,
                gas_used: config.gas_limit,
                return_data: b"gas exhausted".to_vec(),
                logs: vec![],
                state_changes: vec![],
                function_index: 0,
                instructions_executed: config.gas_limit,
            }),
            Err(e) => Err(X3IntegrationError::ExecutionFailed(format!("{:?}", e))),
        }
    }

    /// Verify bytecode without execution
    #[cfg(feature = "std")]
    pub fn verify(bytecode: &[u8], allow_debug_ops: bool) -> X3Result<()> {
        let opts = if allow_debug_ops {
            VerifyOptions::default()
        } else {
            VerifyOptions::on_chain()
        };
        Verifier::verify_module_bytes(bytecode, &opts)
            .map_err(|e| X3IntegrationError::VerificationFailed(format!("{:?}", e)))?;
        Ok(())
    }

    /// Verify bytecode (no_std — uses mini_x3 validator)
    #[cfg(not(feature = "std"))]
    pub fn verify(bytecode: &[u8], _allow_debug_ops: bool) -> X3Result<()> {
        use crate::mini_x3;
        mini_x3::validate_x3bc(bytecode)
            .map_err(|e| X3IntegrationError::VerificationFailed(format!("{:?}", e)))
    }

    /// Estimate gas for bytecode execution
    #[cfg(feature = "std")]
    pub fn estimate_gas(bytecode: &[u8]) -> X3Result<u64> {
        // Use verifier's gas estimation
        let report = x3_verifier::Verifier::new(x3_verifier::SafetyRules::default())
            .verify_bytecode(bytecode)
            .map_err(|e| X3IntegrationError::VerificationFailed(format!("{:?}", e)))?;

        // Get gas estimate from report, or use conservative fallback
        let gas_estimate = report
            .gas_report
            .map(|gr| gr.total_gas)
            .unwrap_or_else(|| (bytecode.len() as u64) * 10);

        Ok(gas_estimate)
    }

    /// Estimate gas (no_std — uses mini_x3 estimator)
    #[cfg(not(feature = "std"))]
    pub fn estimate_gas(bytecode: &[u8]) -> X3Result<u64> {
        use crate::mini_x3;
        Ok(mini_x3::estimate_gas_x3bc(bytecode))
    }

    /// Compute code hash for bytecode
    pub fn code_hash(bytecode: &[u8]) -> H256 {
        H256::from(sp_io::hashing::blake2_256(bytecode))
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_executor_config_defaults() {
        let config = X3ExecutorConfig::default();
        assert_eq!(config.gas_limit, 1_000_000);
        assert!(!config.allow_debug_ops);
    }

    #[test]
    fn test_on_chain_config() {
        let config = X3ExecutorConfig::on_chain();
        assert_eq!(config.gas_limit, 500_000);
        assert!(!config.allow_debug_ops);
        assert!(!config.trace);
    }

    #[test]
    fn test_simulation_config() {
        let config = X3ExecutorConfig::simulation();
        assert_eq!(config.gas_limit, 10_000_000);
        assert!(config.allow_debug_ops);
        assert!(config.trace);
    }
}
