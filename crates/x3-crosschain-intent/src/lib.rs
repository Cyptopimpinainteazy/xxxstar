//! # X3 Cross-Chain Intent System
//!
//! A cross-chain intent is a high-level user declaration of *desired outcome*
//! across one or more chains, with explicit safety constraints and recovery paths.
//!
//! ## Design Contract
//!
//! Every intent answers four questions:
//!
//! 1. **What** does the user want?
//! 2. **What safety limits** must never be crossed?
//! 3. **What proof** must exist before each step executes?
//! 4. **What happens** if execution fails?
//!
//! The user signs the high-level request. X3 handles DEX routing, bridge
//! selection, relayer coordination, proof verification, and rollback — entirely
//! behind the curtain.
//!
//! ## Modules
//!
//! - [`types`] — primitive types: chains, assets, finality, slippage, proofs
//! - [`intent`] — [`CrossChainIntent`] — the structured intent struct
//! - [`lifecycle`] — 16-state machine tracking every intent from Draft → Finalized
//! - [`instructions`] — [`X3Instruction`] — typed execution plan produced by the compiler
//! - [`compiler`] — [`IntentCompiler`] — intent → instructions with 13 safety checks
//! - [`simulation`] — pre-execution simulation: fees, slippage, liquidity, risk score
//! - [`error`] — all error types with diagnostic codes (X3-INTENT-NNN)

pub mod compiler;
pub mod error;
pub mod instructions;
pub mod intent;
pub mod lifecycle;
pub mod simulation;
pub mod types;

pub use compiler::IntentCompiler;
pub use error::{IntentCompileError, IntentValidationError};
pub use instructions::X3Instruction;
pub use intent::CrossChainIntent;
pub use lifecycle::{CrossChainIntentState, IntentStateMachine};
pub use types::{
    AssetRef, ChainKind, FailureAction, FinalityLevel, ProofRequirement, ReceiptSpec, RouteSpec,
    TimeoutSpec,
};
