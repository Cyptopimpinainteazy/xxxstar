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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub mod bond;
#[cfg(feature = "std")]
pub mod engine;
pub mod error;
#[cfg(feature = "std")]
pub mod record;
pub mod types;

#[cfg(feature = "std")]
pub use bond::BondManager;
#[cfg(feature = "std")]
pub use engine::SlashingEngine;
pub use error::SlashError;
#[cfg(feature = "std")]
pub use record::SlashRecord;
pub use types::*;
