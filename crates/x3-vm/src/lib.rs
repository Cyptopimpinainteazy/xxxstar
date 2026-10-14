#![cfg_attr(not(feature = "std"), no_std)]
#![warn(dead_code, unused_imports, unused_variables)]
#![allow(clippy::single_match)]
#![allow(clippy::extend_with_drain)]
#![allow(clippy::len_without_is_empty)]

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
//! use x3_bytecode::bc_format::BytecodeModule;
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

extern crate alloc;

/// The `alloc` types the engine uses, imported by every module that builds without `std`.
///
/// Under `std` these are the same types the standard prelude already names, so a module reads the
/// same in both builds.
pub(crate) mod prelude {
    pub use alloc::boxed::Box;
    pub use alloc::collections::{BTreeMap, BTreeSet};
    pub use alloc::format;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec;
    pub use alloc::vec::Vec;
}

// ---------------------------------------------------------------------------
// The engine: builds with and without `std`.
//
// This is the code a block executes. Everything here is deterministic and depends only on `core`
// and `alloc`, so the wasm runtime and every off-chain tool run the same interpreter, verifier,
// storage journal and gas rules.
// ---------------------------------------------------------------------------
pub mod bytecode;
pub mod error;
pub mod events;
pub mod execution_guards;
pub mod gas;
pub mod hostcall;
pub mod isolation;
pub mod revert;
pub mod state;
pub mod storage;
pub mod verifier;
pub mod vm;

pub use error::{VMError, VMErrorKind, VMResult, VerifierError, VerifierErrorKind};
pub use execution_guards::*;
pub use hostcall::{Hostcall, HostcallRegistry};
pub use verifier::{opcode_gas_cost, DecodedInstr, Verifier, VerifyOptions};
pub use vm::{ExecutionResult, Frame, VMConfig, Value, VM};
pub use x3_bytecode::bc_format::BytecodeModule;

// ---------------------------------------------------------------------------
// Off-chain services: `std` only.
//
// GPU acceleration, the cross-VM bridge client, JIT hot-path tracking, the debugger, escrow
// providers, upgrade tooling and the gas audit table use threads, dynamic loading, clocks or the
// environment. None of them changes what a program computes, so the chain does without them and
// the tools keep them.
// ---------------------------------------------------------------------------
#[cfg(feature = "std")]
pub mod bridge;
#[cfg(feature = "std")]
pub mod contract_upgrade_pattern;
#[cfg(feature = "std")]
pub mod dap_debugging;
#[cfg(feature = "std")]
pub mod gas_metering_audit;
#[cfg(feature = "std")]
pub mod gpu;
#[cfg(feature = "std")]
pub mod gpu_hostcalls;
#[cfg(feature = "std")]
pub mod jit_compiler;
#[cfg(feature = "std")]
pub mod universal_escrow;

#[cfg(feature = "std")]
pub use bridge::{BridgeConfig, BridgeError, X3VMBridge};
#[cfg(feature = "std")]
pub use contract_upgrade_pattern::{
    ProxyContract, StorageLayout, UpgradeSafetyChecker, UpgradeableConfig,
};
#[cfg(feature = "std")]
pub use dap_debugging::{DAPMessage, DAPServer, DAPSession};
#[cfg(feature = "std")]
pub use gas_metering_audit::{GasMeteringTable, OpcodeGasAudit};
#[cfg(feature = "std")]
pub use gpu_hostcalls::{GpuConfig, GpuHostcalls};
#[cfg(feature = "std")]
pub use jit_compiler::{CompiledFunction, HotPathTracker, JitCompiler, JitConfig, JitStats};
