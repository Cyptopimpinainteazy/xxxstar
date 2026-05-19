#![warn(dead_code, unused_imports, unused_variables)]

//! X3 Virtual Machine
//!
//! This crate provides a bytecode verifier and deterministic interpreter for X3 bytecode.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        X3 Bytecode Module                           │
//! │                     (from x3-backend::bc_format)                    │
//! └─────────────────────────────────────────────────────────────────────┘
//!                                  │
//!                                  ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         Bytecode Verifier                           │
//! │  • Structural validation (magic, sections, bounds)                  │
//! │  • CFG validation (jump targets, reachability)                      │
//! │  • Const pool validation (index bounds, types)                      │
//! │  • Atomic block validation (balanced begin/end)                     │
//! │  • Gas estimation (conservative upper bounds)                       │
//! │  • On-chain restrictions (no debug opcodes)                         │
//! └─────────────────────────────────────────────────────────────────────┘
//!                                  │
//!                                  ▼ (if valid)
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         VM Interpreter                              │
//! │  • Deterministic execution (no randomness, no timing)               │
//! │  • Register-based execution model                                   │
//! │  • Hostcall interface for external functions                        │
//! │  • Atomic window tracking (snapshot/rollback)                       │
//! │  • Gas metering and limits                                          │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use x3_vm::{VM, Verifier, VMConfig};
//! use x3_backend::bc_format::BytecodeModule;
//!
//! // Load bytecode
//! let bytes = std::fs::read("module.x3bc")?;
//!
//! // Verify before execution (mandatory)
//! Verifier::verify_module_bytes(&bytes, false)?;
//!
//! // Create VM and execute
//! let mut vm = VM::new_from_bytes(&bytes)?;
//! let result = vm.call_function(0, &[])?;
//! ```
//!
//! # Safety Guarantees
//!
//! 1. **Memory safety**: All array/register accesses are bounds-checked
//! 2. **Determinism**: Same inputs always produce same outputs
//! 3. **Gas limits**: Execution cannot exceed configured gas budget
//! 4. **Atomic isolation**: Atomic blocks can be rolled back on failure
//! 5. **No undefined behavior**: All opcodes have well-defined semantics

pub mod bridge;
pub mod contract_upgrade_pattern;
pub mod dap_debugging;
pub mod error;
pub mod execution_guards;
pub mod gas_metering_audit;
pub mod gpu_hostcalls;
pub mod hostcall;
pub mod jit_compiler;
pub mod verifier;
pub mod vm;

// Re-exports
pub use bridge::{BridgeConfig, BridgeError, X3VMBridge};
pub use contract_upgrade_pattern::{
    ProxyContract, StorageLayout, UpgradeSafetyChecker, UpgradeableConfig,
};
pub use dap_debugging::{DAPMessage, DAPServer, DAPSession};
pub use error::{VMError, VMErrorKind, VMResult, VerifierError, VerifierErrorKind};
pub use execution_guards::*;
pub use gas_metering_audit::{GasMeteringTable, OpcodeGasAudit};
pub use gpu_hostcalls::{GpuConfig, GpuHostcalls};
pub use hostcall::{Hostcall, HostcallRegistry};
pub use jit_compiler::{CompiledFunction, HotPathTracker, JitCompiler, JitConfig, JitStats};
pub use verifier::{opcode_gas_cost, DecodedInstr, Verifier, VerifyOptions};
pub use vm::{ExecutionResult, Frame, VMConfig, Value, VM};
pub use x3_backend::bc_format::BytecodeModule;

// X3VM sub-modules
pub mod bytecode;
pub mod events;
pub mod gas;
pub mod gpu;
pub mod isolation;
pub mod revert;
pub mod state;
pub mod storage;
