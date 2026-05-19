//! X3VM Bridge for X3 Chain Integration
//!
//! This module provides the integration layer between the X3VM bytecode executor
//! and the X3 Chain dual-VM runtime. It enables X3 programs to:
//!
//! 1. Execute on Solana via the x3vm-executor Anchor program
//! 2. Run off-chain in the native X3VM for simulation/testing
//! 3. Bridge to EVM contracts via cross-VM hostcalls
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        X3 Chain Runtime                          │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
//! │  │ X3VM Native  │  │ SVM Executor │  │ EVM Executor              │  │
//! │  │ (off-chain)  │  │ (Solana)     │  │ (Frontier)               │  │
//! │  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────────┘  │
//! │         │                 │                     │                   │
//! │         └─────────────────┼─────────────────────┘                   │
//! │                           │                                         │
//! │                    ┌──────▼──────┐                                  │
//! │                    │ X3VM Bridge │                                  │
//! │                    │ (hostcalls) │                                  │
//! │                    └─────────────┘                                  │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use crate::gpu_hostcalls::GpuHostcalls;
use crate::{ExecutionResult, VMError, VMErrorKind, Value, VM};

// Re-export x3-backend types for bytecode helpers (used by tests and callers)
pub use x3_backend::bc_format_helpers;

#[cfg(feature = "x3-evm-integration")]
use x3_evm_integration::{EvmConfig, EvmExecutor};
#[cfg(feature = "x3-svm-integration")]
use x3_svm_integration::{SvmConfig, SvmExecutor};

#[cfg(any(feature = "x3-evm-integration", feature = "x3-svm-integration"))]
use std::sync::Arc;

/// Configuration for the X3VM bridge
#[derive(Clone, Debug)]
pub struct BridgeConfig {
    /// Enable SVM (Solana) hostcalls
    pub enable_svm: bool,
    /// Enable EVM hostcalls
    pub enable_evm: bool,
    /// Enable GPU compute hostcalls (CUDA dispatch)
    pub enable_gpu: bool,
    /// Gas limit for bridge operations
    pub gas_limit: u64,
    /// Maximum CPI depth for Solana calls
    pub max_cpi_depth: u8,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            // Safe-by-default: bridge hostcalls are disabled unless explicitly enabled
            // by the caller. Even when enabled, mocked behavior is only available
            // behind the `bridge-mocks` feature.
            enable_svm: false,
            enable_evm: false,
            enable_gpu: true, // GPU hostcalls are safe (read-only compute, no state mutation)
            gas_limit: 1_000_000,
            max_cpi_depth: 4,
        }
    }
}

#[allow(dead_code)]
fn bridge_disabled_error(name: &str) -> VMError {
    VMError::without_ip(VMErrorKind::HostcallError(format!(
        "bridge hostcall '{}': no executor configured (call X3VMBridge::with_executors())",
        name
    )))
}

// ── Cross-VM trait abstractions ───────────────────────────────────────────────

/// Provides native balance queries and transfers for X3VM hostcalls (0x10/0x12/0x22).
///
/// Implementors connect the bridge to the canonical Substrate ledger — typically
/// `pallet-balances` (SVM pubkeys) and `pallet-evm` (EVM H160 addresses).
/// When not wired the relevant hostcalls remain fail-closed.
pub trait BalanceProvider: Send + Sync {
    /// Get the free balance of `address`. `address` is a raw SVM pubkey (32 B)
    /// or EVM address (20 B) depending on which hostcall is being dispatched.
    fn get_balance(&self, address: &[u8]) -> u128;

    /// Transfer `amount` from `from` to `to`. Returns `Err(&'static str)` on
    /// insufficient balance or if accounts are unknown.
    fn transfer(&self, from: &[u8], to: &[u8], amount: u128) -> Result<(), &'static str>;
}

/// Provides cross-VM escrow operations for bridge ops (0x30/0x31).
///
/// Implementors handle lock-and-release between SVM and EVM.  The expected
/// flow for `bridge_svm_to_evm` (0x30) is:
///   1. `lock_svm(from, amount)` → ticket
///   2. `release_evm(to_h160, ticket, amount)`
///
/// For `bridge_evm_to_svm` (0x31):
///   1. `lock_evm(from_h160, amount)` → ticket
///   2. `release_svm(to_pubkey, ticket, amount)`
pub trait CrossVmEscrow: Send + Sync {
    /// Lock `amount` of native tokens on the SVM side.  Returns a 32-byte
    /// escrow ticket that authorises the corresponding EVM release.
    fn lock_svm(&self, from: &[u8], amount: u128) -> Result<[u8; 32], &'static str>;

    /// Release `amount` of wrapped tokens on the EVM side given a valid ticket.
    fn release_evm(
        &self,
        to: &[u8; 20],
        ticket: &[u8; 32],
        amount: u128,
    ) -> Result<(), &'static str>;

    /// Lock `amount` of EVM tokens and return an escrow ticket for SVM release.
    fn lock_evm(&self, from: &[u8; 20], amount: u128) -> Result<[u8; 32], &'static str>;

    /// Release `amount` of native tokens on the SVM side given a valid ticket.
    fn release_svm(&self, to: &[u8], ticket: &[u8; 32], amount: u128) -> Result<(), &'static str>;
}

/// X3VM Bridge for cross-VM execution
pub struct X3VMBridge {
    config: BridgeConfig,
    gpu: Option<GpuHostcalls>,
    #[cfg(feature = "x3-evm-integration")]
    evm: Option<Arc<dyn EvmExecutor + Send + Sync>>,
    #[cfg(feature = "x3-svm-integration")]
    svm: Option<Arc<dyn SvmExecutor + Send + Sync>>,
    /// Optional canonical balance provider for 0x10 / 0x12 / 0x22 hostcalls.
    balance_provider: Option<std::sync::Arc<dyn BalanceProvider>>,
    /// Optional cross-VM escrow for 0x30 / 0x31 hostcalls.
    escrow: Option<std::sync::Arc<dyn CrossVmEscrow>>,
}

impl X3VMBridge {
    /// Create a new X3VM bridge with default configuration (no EVM/SVM executors).
    pub fn new() -> Self {
        Self::with_config(BridgeConfig::default())
    }

    /// Create a new X3VM bridge with custom configuration.
    pub fn with_config(config: BridgeConfig) -> Self {
        let gpu = if config.enable_gpu {
            Some(GpuHostcalls::new())
        } else {
            None
        };
        Self {
            config,
            gpu,
            #[cfg(feature = "x3-evm-integration")]
            evm: None,
            #[cfg(feature = "x3-svm-integration")]
            svm: None,
            balance_provider: None,
            escrow: None,
        }
    }

    /// Attach a canonical balance provider for 0x10/0x12/0x22 hostcalls.
    ///
    /// Without this, `svm_transfer`, `svm_get_balance`, and `evm_balance` remain
    /// fail-closed and return an error directing callers to wire Substrate storage.
    pub fn with_balances(mut self, provider: std::sync::Arc<dyn BalanceProvider>) -> Self {
        self.balance_provider = Some(provider);
        self
    }

    /// Attach a cross-VM escrow provider for 0x30/0x31 bridge hostcalls.
    ///
    /// Without this, `bridge_svm_to_evm` and `bridge_evm_to_svm` remain fail-closed.
    pub fn with_escrow(mut self, escrow: std::sync::Arc<dyn CrossVmEscrow>) -> Self {
        self.escrow = Some(escrow);
        self
    }

    /// Attach real EVM and SVM executors to the bridge.
    ///
    /// When executors are provided the bridge hostcalls (0x10-0x22) route to
    /// them instead of returning an error. The bridge ops (0x30-0x31) that
    /// require canonical ledger context remain fail-closed until the canonical
    /// ledger is wired at the runtime level.
    #[cfg(all(feature = "x3-evm-integration", feature = "x3-svm-integration"))]
    pub fn with_executors(
        mut self,
        evm: Arc<dyn EvmExecutor + Send + Sync>,
        svm: Arc<dyn SvmExecutor + Send + Sync>,
    ) -> Self {
        self.evm = Some(evm);
        self.svm = Some(svm);
        self
    }

    /// Execute X3 bytecode with bridge hostcalls enabled
    pub fn execute(
        &self,
        bytecode: &[u8],
        function_index: usize,
        args: &[Value],
    ) -> Result<ExecutionResult, BridgeError> {
        let mut vm =
            VM::from_bytes(bytecode).map_err(|e| BridgeError::VMError(format!("{:?}", e)))?;

        // Configure VM
        vm.config.gas_limit = self.config.gas_limit;

        // Register bridge hostcalls
        self.register_hostcalls(&mut vm);

        // Execute
        vm.call_function(function_index, args)
            .map_err(|e| BridgeError::ExecutionError(format!("{:?}", e)))
    }

    /// Register cross-VM hostcalls on a VM instance.
    ///
    /// This is exposed publicly so callers (e.g. `AtomicSwapOrchestrator`) can
    /// wire bridge hostcalls onto an externally-created VM without going through
    /// `execute()`.
    pub fn register_bridge_hostcalls(&self, vm: &mut VM) {
        self.register_hostcalls(vm);
    }

    /// Register cross-VM hostcalls
    fn register_hostcalls(&self, vm: &mut VM) {
        // GPU compute hostcalls (0xD0 - 0xDF)
        if let Some(ref gpu) = self.gpu {
            gpu.register_on_vm(vm);
        }

        // Clone shared state before the hostcall closures capture it
        let balance_provider = self.balance_provider.clone();
        let escrow = self.escrow.clone();

        // ── SVM hostcalls ────────────────────────────────────────────────
        if self.config.enable_svm {
            #[cfg(feature = "x3-svm-integration")]
            {
                let svm_exec = self.svm.clone();

                vm.register_hostcall(0x10, "svm_transfer", 3, {
                    let bp = balance_provider.clone();
                    let svm = svm_exec.clone();
                    move |args| {
                        if let Some(ref provider) = bp {
                            let from = match args.first() {
                                Some(Value::Bytes(b)) => b.clone(),
                                _ => return Err(VMError::without_ip(VMErrorKind::HostcallError(
                                    "svm_transfer: arg[0] (from) must be Bytes".into(),
                                ))),
                            };
                            let to = match args.get(1) {
                                Some(Value::Bytes(b)) => b.clone(),
                                _ => return Err(VMError::without_ip(VMErrorKind::HostcallError(
                                    "svm_transfer: arg[1] (to) must be Bytes".into(),
                                ))),
                            };
                            let amount = match args.get(2) {
                                Some(Value::I64(v)) => (*v).max(0) as u128,
                                _ => return Err(VMError::without_ip(VMErrorKind::HostcallError(
                                    "svm_transfer: arg[2] (amount) must be I64".into(),
                                ))),
                            };
                            provider.transfer(&from, &to, amount).map_err(|e| {
                                VMError::without_ip(VMErrorKind::HostcallError(format!(
                                    "svm_transfer: {}", e
                                )))
                            })?;
                            Ok(Some(Value::Bool(true)))
                        } else {
                            let _ = (&svm, args);
                            Err(VMError::without_ip(VMErrorKind::HostcallError(
                                "svm_transfer requires canonical ledger (wire via Substrate runtime)"
                                    .into(),
                            )))
                        }
                    }
                });

                vm.register_hostcall(0x11, "svm_invoke", 3, {
                    let svm = svm_exec.clone();
                    move |args| {
                        let executor = svm.as_ref().ok_or_else(|| {
                            VMError::without_ip(VMErrorKind::HostcallError(
                                "svm_invoke: no SVM executor configured".into(),
                            ))
                        })?;
                        let program = match args.first() {
                            Some(Value::Bytes(b)) => b.clone(),
                            _ => {
                                return Err(VMError::without_ip(VMErrorKind::HostcallError(
                                    "svm_invoke: arg[0] (program) must be Bytes".into(),
                                )))
                            }
                        };
                        let input = match args.get(1) {
                            Some(Value::Bytes(b)) => b.clone(),
                            _ => vec![],
                        };
                        let compute_units = match args.get(2) {
                            Some(Value::I64(v)) => (*v).max(0) as u64,
                            _ => 200_000,
                        };
                        let cfg = SvmConfig {
                            compute_unit_limit: compute_units,
                            ..SvmConfig::default()
                        };
                        let result = executor.execute_bpf(&program, &input, &cfg).map_err(|e| {
                            VMError::without_ip(VMErrorKind::HostcallError(format!(
                                "svm_invoke failed: {:?}",
                                e
                            )))
                        })?;
                        Ok(Some(Value::Bytes(result.output)))
                    }
                });

                vm.register_hostcall(0x12, "svm_get_balance", 1, {
                    let bp = balance_provider.clone();
                    let svm = svm_exec;
                    move |args| {
                        if let Some(ref provider) = bp {
                            let addr = match args.first() {
                                Some(Value::Bytes(b)) => b.clone(),
                                _ => return Err(VMError::without_ip(VMErrorKind::HostcallError(
                                    "svm_get_balance: arg[0] (address) must be Bytes".into(),
                                ))),
                            };
                            let bal = provider.get_balance(&addr);
                            Ok(Some(Value::I64(bal.min(i64::MAX as u128) as i64)))
                        } else {
                            let _ = (&svm, args);
                            Err(VMError::without_ip(VMErrorKind::HostcallError(
                                "svm_get_balance requires canonical ledger (wire via Substrate runtime)"
                                    .into(),
                            )))
                        }
                    }
                });
            }

            #[cfg(not(feature = "x3-svm-integration"))]
            {
                vm.register_hostcall(0x10, "svm_transfer", 3, |_args| {
                    Err(bridge_disabled_error("svm_transfer"))
                });
                vm.register_hostcall(0x11, "svm_invoke", 3, |_args| {
                    Err(bridge_disabled_error("svm_invoke"))
                });
                vm.register_hostcall(0x12, "svm_get_balance", 1, |_args| {
                    Err(bridge_disabled_error("svm_get_balance"))
                });
            }
        }

        // ── EVM hostcalls ────────────────────────────────────────────────
        if self.config.enable_evm {
            #[cfg(feature = "x3-evm-integration")]
            {
                let evm_exec = self.evm.clone();

                vm.register_hostcall(0x20, "evm_call", 4, {
                    let evm = evm_exec.clone();
                    move |args| {
                        let executor = evm.as_ref().ok_or_else(|| {
                            VMError::without_ip(VMErrorKind::HostcallError(
                                "evm_call: no EVM executor configured".into(),
                            ))
                        })?;
                        let gas = match args.first() {
                            Some(Value::I64(v)) => (*v).max(0) as u64,
                            _ => 21_000,
                        };
                        let addr_bytes = match args.get(1) {
                            Some(Value::Bytes(b)) if b.len() == 20 => sp_core::H160::from_slice(b),
                            _ => sp_core::H160::zero(),
                        };
                        let value_u256 = match args.get(2) {
                            Some(Value::I64(v)) => sp_core::U256::from((*v).max(0) as u64),
                            _ => sp_core::U256::zero(),
                        };
                        let data = match args.get(3) {
                            Some(Value::Bytes(b)) => b.clone(),
                            _ => vec![],
                        };
                        let cfg = EvmConfig {
                            gas_limit: gas,
                            ..EvmConfig::default()
                        };
                        let result = executor
                            .call(&data, sp_core::H160::zero(), addr_bytes, value_u256, &cfg)
                            .map_err(|e| {
                                VMError::without_ip(VMErrorKind::HostcallError(format!(
                                    "evm_call failed: {:?}",
                                    e
                                )))
                            })?;
                        Ok(Some(Value::Bytes(result.output)))
                    }
                });

                vm.register_hostcall(0x21, "evm_staticcall", 3, {
                    let evm = evm_exec.clone();
                    move |args| {
                        let executor = evm.as_ref().ok_or_else(|| {
                            VMError::without_ip(VMErrorKind::HostcallError(
                                "evm_staticcall: no EVM executor configured".into(),
                            ))
                        })?;
                        let addr_bytes = match args.first() {
                            Some(Value::Bytes(b)) if b.len() == 20 => sp_core::H160::from_slice(b),
                            _ => sp_core::H160::zero(),
                        };
                        let gas = match args.get(1) {
                            Some(Value::I64(v)) => (*v).max(0) as u64,
                            _ => 21_000,
                        };
                        let data = match args.get(2) {
                            Some(Value::Bytes(b)) => b.clone(),
                            _ => vec![],
                        };
                        let cfg = EvmConfig {
                            gas_limit: gas,
                            ..EvmConfig::default()
                        };
                        let result = executor
                            .call(
                                &data,
                                sp_core::H160::zero(),
                                addr_bytes,
                                sp_core::U256::zero(),
                                &cfg,
                            )
                            .map_err(|e| {
                                VMError::without_ip(VMErrorKind::HostcallError(format!(
                                    "evm_staticcall failed: {:?}",
                                    e
                                )))
                            })?;
                        Ok(Some(Value::Bytes(result.output)))
                    }
                });

                vm.register_hostcall(0x22, "evm_balance", 1, {
                    let bp = balance_provider.clone();
                    let evm = evm_exec;
                    move |args| {
                        if let Some(ref provider) = bp {
                            let addr = match args.first() {
                                Some(Value::Bytes(b)) => b.clone(),
                                _ => {
                                    return Err(VMError::without_ip(VMErrorKind::HostcallError(
                                        "evm_balance: arg[0] (address) must be Bytes".into(),
                                    )))
                                }
                            };
                            let bal = provider.get_balance(&addr);
                            Ok(Some(Value::I64(bal.min(i64::MAX as u128) as i64)))
                        } else {
                            let _ = (&evm, args);
                            Err(VMError::without_ip(VMErrorKind::HostcallError(
                                "evm_balance requires canonical ledger (wire via Substrate runtime)"
                                    .into(),
                            )))
                        }
                    }
                });
            }

            #[cfg(not(feature = "x3-evm-integration"))]
            {
                vm.register_hostcall(0x20, "evm_call", 4, |_args| {
                    Err(bridge_disabled_error("evm_call"))
                });
                vm.register_hostcall(0x21, "evm_staticcall", 3, |_args| {
                    Err(bridge_disabled_error("evm_staticcall"))
                });
                vm.register_hostcall(0x22, "evm_balance", 1, |_args| {
                    Err(bridge_disabled_error("evm_balance"))
                });
            }
        }

        // ── Cross-VM bridge ops ───────────────────────────────────────────────
        vm.register_hostcall(0x30, "bridge_svm_to_evm", 3, {
            let esc = escrow.clone();
            move |args| {
                let Some(ref provider) = esc else {
                    return Err(VMError::without_ip(VMErrorKind::HostcallError(
                        "bridge_svm_to_evm: wire canonical escrow (X3VMBridge::with_escrow())"
                            .into(),
                    )));
                };
                let from = match args.first() {
                    Some(Value::Bytes(b)) => b.clone(),
                    _ => {
                        return Err(VMError::without_ip(VMErrorKind::HostcallError(
                            "bridge_svm_to_evm: arg[0] (from_svm_pubkey) must be Bytes".into(),
                        )))
                    }
                };
                let to_bytes = match args.get(1) {
                    Some(Value::Bytes(b)) if b.len() == 20 => {
                        let mut arr = [0u8; 20];
                        arr.copy_from_slice(b);
                        arr
                    }
                    _ => {
                        return Err(VMError::without_ip(VMErrorKind::HostcallError(
                            "bridge_svm_to_evm: arg[1] (to_evm_address) must be 20-byte Bytes"
                                .into(),
                        )))
                    }
                };
                let amount = match args.get(2) {
                    Some(Value::I64(v)) => (*v).max(0) as u128,
                    Some(Value::Bytes(b)) if b.len() == 16 => {
                        u128::from_le_bytes(b.as_slice().try_into().expect("checked len"))
                    }
                    _ => return Err(VMError::without_ip(VMErrorKind::HostcallError(
                        "bridge_svm_to_evm: arg[2] (amount) must be I64 or 16-byte le-encoded u128"
                            .into(),
                    ))),
                };
                let ticket = provider.lock_svm(&from, amount).map_err(|e| {
                    VMError::without_ip(VMErrorKind::HostcallError(format!(
                        "bridge_svm_to_evm lock_svm: {}",
                        e
                    )))
                })?;
                provider
                    .release_evm(&to_bytes, &ticket, amount)
                    .map_err(|e| {
                        VMError::without_ip(VMErrorKind::HostcallError(format!(
                            "bridge_svm_to_evm release_evm: {}",
                            e
                        )))
                    })?;
                Ok(Some(Value::Bytes(ticket.to_vec())))
            }
        });

        vm.register_hostcall(0x31, "bridge_evm_to_svm", 3, {
            let esc = escrow;
            move |args| {
                let Some(ref provider) = esc else {
                    return Err(VMError::without_ip(VMErrorKind::HostcallError(
                        "bridge_evm_to_svm: wire canonical escrow (X3VMBridge::with_escrow())"
                            .into(),
                    )));
                };
                let from_bytes =
                    match args.first() {
                        Some(Value::Bytes(b)) if b.len() == 20 => {
                            let mut arr = [0u8; 20];
                            arr.copy_from_slice(b);
                            arr
                        }
                        _ => return Err(VMError::without_ip(VMErrorKind::HostcallError(
                            "bridge_evm_to_svm: arg[0] (from_evm_address) must be 20-byte Bytes"
                                .into(),
                        ))),
                    };
                let to = match args.get(1) {
                    Some(Value::Bytes(b)) => b.clone(),
                    _ => {
                        return Err(VMError::without_ip(VMErrorKind::HostcallError(
                            "bridge_evm_to_svm: arg[1] (to_svm_pubkey) must be Bytes".into(),
                        )))
                    }
                };
                let amount = match args.get(2) {
                    Some(Value::I64(v)) => (*v).max(0) as u128,
                    Some(Value::Bytes(b)) if b.len() == 16 => {
                        u128::from_le_bytes(b.as_slice().try_into().expect("checked len"))
                    }
                    _ => return Err(VMError::without_ip(VMErrorKind::HostcallError(
                        "bridge_evm_to_svm: arg[2] (amount) must be I64 or 16-byte le-encoded u128"
                            .into(),
                    ))),
                };
                let ticket = provider.lock_evm(&from_bytes, amount).map_err(|e| {
                    VMError::without_ip(VMErrorKind::HostcallError(format!(
                        "bridge_evm_to_svm lock_evm: {}",
                        e
                    )))
                })?;
                provider.release_svm(&to, &ticket, amount).map_err(|e| {
                    VMError::without_ip(VMErrorKind::HostcallError(format!(
                        "bridge_evm_to_svm release_svm: {}",
                        e
                    )))
                })?;
                Ok(Some(Value::Bytes(ticket.to_vec())))
            }
        });
    }
}

impl Default for X3VMBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during bridge operations
#[derive(Debug, Clone)]
pub enum BridgeError {
    /// Error loading or validating bytecode
    VMError(String),
    /// Error during execution
    ExecutionError(String),
    /// SVM operation failed
    SVMError(String),
    /// EVM operation failed
    EVMError(String),
    /// Bridge operation failed
    BridgeOperationError(String),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::VMError(msg) => write!(f, "VM Error: {}", msg),
            BridgeError::ExecutionError(msg) => write!(f, "Execution Error: {}", msg),
            BridgeError::SVMError(msg) => write!(f, "SVM Error: {}", msg),
            BridgeError::EVMError(msg) => write!(f, "EVM Error: {}", msg),
            BridgeError::BridgeOperationError(msg) => write!(f, "Bridge Error: {}", msg),
        }
    }
}

impl std::error::Error for BridgeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_execute_simple() {
        let bridge = X3VMBridge::new();
        let bytecode = bc_format_helpers::assemble_simple_module();

        let result = bridge.execute(&bytecode, 0, &[]);
        assert!(result.is_ok());

        let exec_result = result.unwrap();
        assert_eq!(exec_result.value, Some(Value::I64(49)));
    }

    #[test]
    fn test_bridge_with_args() {
        let bridge = X3VMBridge::new();
        let bytecode = bc_format_helpers::assemble_param_module();

        let result = bridge.execute(&bytecode, 0, &[Value::I64(100), Value::I64(200)]);
        assert!(result.is_ok());

        let exec_result = result.unwrap();
        assert_eq!(exec_result.value, Some(Value::I64(300)));
    }

    #[test]
    fn test_bridge_config() {
        let config = BridgeConfig {
            enable_svm: true,
            enable_evm: false,
            enable_gpu: true,
            gas_limit: 500_000,
            max_cpi_depth: 2,
        };

        let bridge = X3VMBridge::with_config(config);
        assert!(!bridge.config.enable_evm);
        assert_eq!(bridge.config.gas_limit, 500_000);
    }

    // ── BalanceProvider mock + tests ─────────────────────────────────────────

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex as StdMutex};

    struct MockBalanceProvider {
        ledger: StdMutex<HashMap<Vec<u8>, u128>>,
    }

    impl MockBalanceProvider {
        fn new(initial: &[(&[u8], u128)]) -> Self {
            let mut m = HashMap::new();
            for (addr, bal) in initial {
                m.insert(addr.to_vec(), *bal);
            }
            Self {
                ledger: StdMutex::new(m),
            }
        }
    }

    impl BalanceProvider for MockBalanceProvider {
        fn get_balance(&self, address: &[u8]) -> u128 {
            *self.ledger.lock().unwrap().get(address).unwrap_or(&0)
        }

        fn transfer(&self, from: &[u8], to: &[u8], amount: u128) -> Result<(), &'static str> {
            let mut ledger = self.ledger.lock().unwrap();
            let from_bal = *ledger.get(from).unwrap_or(&0);
            if from_bal < amount {
                return Err("insufficient balance");
            }
            *ledger.entry(from.to_vec()).or_insert(0) -= amount;
            *ledger.entry(to.to_vec()).or_insert(0) += amount;
            Ok(())
        }
    }

    #[test]
    fn test_balance_provider_get_balance() {
        let alice = b"alice_svm_pubkey_32bytes_pad00000" as &[u8];
        let provider = MockBalanceProvider::new(&[(alice, 1_000_000)]);
        assert_eq!(provider.get_balance(alice), 1_000_000);
        assert_eq!(provider.get_balance(b"unknown"), 0);
    }

    #[test]
    fn test_balance_provider_transfer_success() {
        let alice = b"alice00000000000000000000000000000" as &[u8];
        let bob = b"bob0000000000000000000000000000000" as &[u8];
        let provider = MockBalanceProvider::new(&[(alice, 500), (bob, 100)]);
        assert!(provider.transfer(alice, bob, 200).is_ok());
        assert_eq!(provider.get_balance(alice), 300);
        assert_eq!(provider.get_balance(bob), 300);
    }

    #[test]
    fn test_balance_provider_transfer_insufficient() {
        let alice = b"alice00000000000000000000000000000" as &[u8];
        let bob = b"bob0000000000000000000000000000000" as &[u8];
        let provider = MockBalanceProvider::new(&[(alice, 50)]);
        let result = provider.transfer(alice, bob, 100);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "insufficient balance");
        // Balance unchanged
        assert_eq!(provider.get_balance(alice), 50);
    }

    #[test]
    fn test_with_balances_builder_sets_provider() {
        let provider: Arc<dyn BalanceProvider> = Arc::new(MockBalanceProvider::new(&[(b"x", 999)]));
        let bridge = X3VMBridge::new().with_balances(provider);
        assert!(bridge.balance_provider.is_some());
    }

    // ── CrossVmEscrow mock + tests ───────────────────────────────────────────

    struct MockCrossVmEscrow {
        /// Tracks locked SVM amounts: ticket → amount
        svm_locks: StdMutex<HashMap<[u8; 32], u128>>,
        /// Tracks locked EVM amounts: ticket → amount
        evm_locks: StdMutex<HashMap<[u8; 32], u128>>,
    }

    impl MockCrossVmEscrow {
        fn new() -> Self {
            Self {
                svm_locks: StdMutex::new(HashMap::new()),
                evm_locks: StdMutex::new(HashMap::new()),
            }
        }
    }

    impl CrossVmEscrow for MockCrossVmEscrow {
        fn lock_svm(&self, from: &[u8], amount: u128) -> Result<[u8; 32], &'static str> {
            let mut ticket = [0u8; 32];
            ticket[..from.len().min(16)].copy_from_slice(&from[..from.len().min(16)]);
            ticket[16..24].copy_from_slice(&amount.to_le_bytes()[..8]);
            self.svm_locks.lock().unwrap().insert(ticket, amount);
            Ok(ticket)
        }

        fn release_evm(
            &self,
            _to: &[u8; 20],
            ticket: &[u8; 32],
            amount: u128,
        ) -> Result<(), &'static str> {
            let mut locks = self.svm_locks.lock().unwrap();
            match locks.get(ticket) {
                Some(&locked) if locked == amount => {
                    locks.remove(ticket);
                    Ok(())
                }
                Some(_) => Err("amount mismatch"),
                None => Err("ticket not found"),
            }
        }

        fn lock_evm(&self, from: &[u8; 20], amount: u128) -> Result<[u8; 32], &'static str> {
            let mut ticket = [0u8; 32];
            ticket[..20].copy_from_slice(from);
            ticket[20..28].copy_from_slice(&amount.to_le_bytes()[..8]);
            self.evm_locks.lock().unwrap().insert(ticket, amount);
            Ok(ticket)
        }

        fn release_svm(
            &self,
            _to: &[u8],
            ticket: &[u8; 32],
            amount: u128,
        ) -> Result<(), &'static str> {
            let mut locks = self.evm_locks.lock().unwrap();
            match locks.get(ticket) {
                Some(&locked) if locked == amount => {
                    locks.remove(ticket);
                    Ok(())
                }
                Some(_) => Err("amount mismatch"),
                None => Err("ticket not found"),
            }
        }
    }

    #[test]
    fn test_escrow_lock_svm_produces_ticket() {
        let esc = MockCrossVmEscrow::new();
        let ticket = esc.lock_svm(b"alice_svm_pubkey", 1_000).unwrap();
        assert_ne!(ticket, [0u8; 32]);
    }

    #[test]
    fn test_escrow_svm_to_evm_round_trip() {
        let esc = MockCrossVmEscrow::new();
        let to_evm = [0xABu8; 20];
        let ticket = esc.lock_svm(b"alice_svm_pubkey", 500).unwrap();
        assert!(esc.release_evm(&to_evm, &ticket, 500).is_ok());
        // Double-release should fail (ticket consumed)
        assert!(esc.release_evm(&to_evm, &ticket, 500).is_err());
    }

    #[test]
    fn test_escrow_evm_to_svm_round_trip() {
        let esc = MockCrossVmEscrow::new();
        let from_evm = [0x01u8; 20];
        let ticket = esc.lock_evm(&from_evm, 250).unwrap();
        assert!(esc.release_svm(b"bob_svm_pubkey", &ticket, 250).is_ok());
        assert!(esc.release_svm(b"bob_svm_pubkey", &ticket, 250).is_err());
    }

    #[test]
    fn test_with_escrow_builder_sets_provider() {
        let esc: Arc<dyn CrossVmEscrow> = Arc::new(MockCrossVmEscrow::new());
        let bridge = X3VMBridge::new().with_escrow(esc);
        assert!(bridge.escrow.is_some());
    }
}
