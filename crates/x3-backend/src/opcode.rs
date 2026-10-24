//! The X3 instruction set.
//!
//! The canonical definition lives in `x3bc-core` (`x3bc_core::opcode`, with every opcode's operand
//! widths, gas, on-chain admission and traps in `x3bc_core::spec`). This module re-exports it so
//! the back end, the engine and every tool name one table.

pub use x3bc_core::opcode::*;
