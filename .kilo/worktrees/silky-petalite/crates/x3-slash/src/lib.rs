//! # X3 Slashing Engine
//!
//! Automatic, deterministic punishment for protocol violations.
//! No humans. No voting. No appeals to authority.
//!
//! Slashing is triggered by:
//! - Failed execution within a slashable scope
//! - State divergence detected during replay
//! - Bond expiry without settlement
//! - Proof invalidity
//!
//! All slashing records are permanent and public.

pub mod bond;
pub mod engine;
pub mod error;
pub mod record;
pub mod types;

pub use bond::BondManager;
pub use engine::SlashingEngine;
pub use error::SlashError;
pub use record::SlashRecord;
pub use types::*;
