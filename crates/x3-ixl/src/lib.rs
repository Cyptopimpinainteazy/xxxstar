//! X3 Internal eXchange Language (IXL).
//!
//! IXL is a small, deterministic instruction set the cross-VM router uses to
//! describe an internal atomic bundle.  A bundle is parsed once, validated by
//! the [`planner`], and then executed by the [`interpreter`] which records
//! each side-effect on a [`Receipt`].  If any step fails, the
//! [`rollback`] module replays the receipt in reverse to restore the
//! pre-bundle state.
//!
//! Scope (intentionally small for v0.4 internal-only mainnet):
//!
//! * `Lock`    — debit a source asset and place it under router custody.
//! * `Mint`    — credit a destination asset to a receiver from custody.
//! * `Burn`    — destroy custody-held asset (used on refund / cleanup).
//! * `Swap`    — execute a single internal AMM/spot swap.
//! * `Settle`  — finalise router escrow into the destination account.
//! * `EmitProof` — write a packet commitment for an inter-VM hop.
//! * `Refund`  — release escrow back to the original payer.
//! * `Abort`   — explicit abort instruction (used by validators on hazard).
//!
//! Anything more advanced (parallel execution, external bridge instructions,
//! AppZone deployment) is **out of scope** for v0.4 and lives in a separate
//! crate when added.
//!
//! Determinism rules:
//!
//! * No floating point.
//! * No iteration order that depends on hash-map iteration; always sort or
//!   index by explicit position.
//! * Every error is structured so the rollback path is data-driven, never
//!   panic-driven.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod instruction;
pub mod interpreter;
pub mod planner;
pub mod receipt;
pub mod rollback;
pub mod verifier;

pub use instruction::{AssetId, AssetKind, Instruction, IxlError};
pub use interpreter::{ExecutionContext, Interpreter, LedgerEffect};
pub use planner::{ExecutionPlan, Planner};
pub use receipt::{Receipt, ReceiptEntry};
pub use rollback::Rollback;
pub use verifier::{Verifier, VerifyError};
