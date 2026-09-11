#![allow(unused, dead_code, deprecated)]

//! # X3 Cross-VM Atomic Trade Coordinator
//!
//! State machine that orchestrates HTLC-based cross-chain atomic swaps
//! across EVM, SVM, and X3VM with integrated flashloan support.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                CROSS-VM COORDINATOR STATE MACHINE                │
//! │                                                                  │
//! │    Phase 1        Phase 2         Phase 3          Phase 4       │
//! │  ┌──────────┐  ┌───────────┐  ┌─────────────┐  ┌────────────┐  │
//! │  │  SETUP   │→ │LOCK HTLCs │→ │  FLASH LEGS │→ │   SETTLE   │  │
//! │  │ H=sha(S) │  │ Both VMs  │  │ Borrow+Swap │  │ Reveal S   │  │
//! │  └──────────┘  └───────────┘  └─────────────┘  └────────────┘  │
//! │       ↓              ↓              ↓                ↓          │
//! │   [ABORT] ←──── [TIMEOUT] ←─── [REVERT] ←──── [REFUND]        │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Design Decisions
//!
//! 1. **Timelocks**: `T_fast` for fast chain (SVM/X3), `T_slow = T_fast + Δ`
//!    for slow chain (EVM). This ensures the claimer on the slow chain always
//!    has time to act after seeing the secret on the fast chain.
//!
//! 2. **Flashloan atomicity**: Each leg borrows, swaps, and repays within a
//!    single atomic transaction on its chain. Failure = revert = no secret reveal.
//!
//! 3. **Secret management**: Secret `S` is generated off-chain, hash `H` is
//!    committed on-chain in both HTLCs. `S` is only revealed when ALL legs succeed.

pub mod abi;
pub mod config;
pub mod concurrent;
pub mod flashloan_adapter;
pub mod htlc;
pub mod lease;
pub mod merkle_settlement; // Gap #3: Merkle-backed settlement for atomic swaps
pub mod persistence;
pub mod relayer;
pub mod rpc_client;
pub mod state_machine;
pub mod types;
#[cfg(feature = "valkey")]
pub mod valkey;

#[cfg(test)]
mod bridge_integration_tests; // Phase 13b: Bridge integration test suite

pub use config::*;
pub use lease::*;
pub use concurrent::*;
pub use persistence::*;
pub use state_machine::*;
pub use types::*;
#[cfg(feature = "valkey")]
pub use valkey::*;

#[cfg(test)]
mod tests;
