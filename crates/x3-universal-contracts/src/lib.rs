//! X3 Universal Contracts SDK
//!
//! This crate is the developer-facing facade over the X3 execution stack.
//! It connects three underlying layers into one fluent API:
//!
//! - **`x3-intent`** — lifecycle state machine for ArbIntents
//! - **`x3-ixl`** — IXL instruction set / planner / interpreter
//! - **`x3-packet-standard`** — IBC-style packet lifecycle
//!
//! # Quick-start
//!
//! ```rust
//! use x3_universal_contracts::{
//!     sdk::UniversalContract,
//!     actions::{Action, Domain},
//! };
//!
//! let result = UniversalContract::new([1u8; 32])
//!     .fee_cap(1_000_000)
//!     .submitted_at(10)
//!     .action(Action::Lock { asset_id: 1, amount: 100_000, domain: Domain::X3Native })
//!     .action(Action::Mint { asset_id: 2, amount: 100_000, domain: Domain::X3Evm })
//!     .compile();
//!
//! assert!(result.is_ok());
//! ```

pub mod actions;
pub mod compiler;
pub mod error;
pub mod intents;
pub mod sdk;

#[cfg(test)]
mod tests;

pub use error::UcError;
pub use sdk::{CompiledContract, UniversalContract};
