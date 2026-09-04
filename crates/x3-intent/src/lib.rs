//! # X3 Arb Intent System
//!
//! Intent-based execution for the X3 jurisdiction.
//!
//! ## Lifecycle
//!
//! 1. **submit_intent** — Agent submits an ArbIntent with bond
//! 2. **bind_route** — Route is sealed (commit to execution path)
//! 3. **execute** — Deterministic execution in the X3 VM
//! 4. **finalize** — Settlement or slashing
//!
//! ## Design Principles
//!
//! - Intent-based execution > bot racing
//! - All intents carry bonds (skin in the game)
//! - Routes are sealed — no front-running after bind
//! - Fees are pre-calculated and locked
//! - Failure within slashable scope triggers automatic punishment

pub mod error;
pub mod intent;
pub mod lifecycle;
pub mod registry;
pub mod types;

pub use error::IntentError;
pub use intent::ArbIntent;
pub use lifecycle::IntentLifecycle;
pub use registry::IntentRegistry;
pub use types::*;
