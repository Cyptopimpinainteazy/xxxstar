//! # X3 Canonical Truth — Phase 9
//!
//! Defines the single authoritative data models for identity, asset, and treasury
//! state shared across the X3 runtime, sidecar services, and client-facing SDKs.
//!
//! ## Design Principles
//!
//! - All types are `no_std`-compatible via `sp-std`.
//! - All types implement SCALE `Encode`/`Decode`/`DecodeWithMemTracking` and
//!   `scale_info::TypeInfo` for runtime-ABI compatibility.
//! - All types implement `serde::Serialize`/`Deserialize` (with the `alloc` feature)
//!   for off-chain JSON transport in sidecar services and client SDKs.
//! - Fixed-layout types (no `Vec` fields) additionally implement `MaxEncodedLen`
//!   so they can be stored in Substrate `StorageMap` / `StorageValue` items.
//!
//! ## Modules
//!
//! - [`identity`] — cross-chain identity, KYC tier, governance registration.
//! - [`asset`] — canonical asset descriptor, cross-chain supply entries, lock proofs.
//! - [`treasury`] — treasury snapshots, actions, and reconciliation reports.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
// Exported type names deliberately repeat the module name for unambiguous glob imports.
#![allow(clippy::module_name_repetitions)]

pub mod asset;
pub mod identity;
pub mod treasury;
/// Drift detection between on-chain canonical truth and a surface-observed snapshot.
///
/// Only compiled when the `std` feature is enabled (service-side use).
#[cfg(feature = "std")]
pub mod sync;

pub use asset::*;
pub use identity::*;
pub use treasury::*;
#[cfg(feature = "std")]
pub use sync::*;

#[cfg(test)]
mod tests;
