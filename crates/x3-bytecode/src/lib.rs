#![cfg_attr(not(feature = "std"), no_std)]
#![allow(
    clippy::manual_range_contains,
    clippy::unnecessary_map_or,
    clippy::vec_init_then_push
)]

//! X3BC: the X3 instruction set and binary module format.
//!
//! One definition, used everywhere X3 bytecode is written or read:
//!
//! - the compiler (`x3-backend`) emits modules with it, and re-exports these
//!   modules so existing `x3_backend::bc_format` / `x3_backend::opcode` paths
//!   keep working;
//! - the VM (`x3-vm`) decodes and executes them, off chain and, because this
//!   crate builds without `std`, inside the wasm runtime as well.
//!
//! Keeping the opcode table and the reader in one `no_std` crate is what lets
//! the chain and every off-chain tool run the same engine instead of each
//! keeping its own copy of the format.

extern crate alloc;

pub mod bc_format;
pub mod bc_format_helpers;
pub mod error;
pub mod opcode;

pub use bc_format::{
    BytecodeModule, ConstPool, ConstValue, DebugInfo, FeatureFlags, FunctionEntry, GlobalEntry,
    ModuleMetadata, VersionInfo, MAGIC, VERSION,
};
pub use error::{BytecodeError, BytecodeResult};
pub use opcode::{Opcode, Register};
