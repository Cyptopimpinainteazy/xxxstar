//! # X3 Security Events — Phase 2 Security Swarm Spine
//!
//! Defines the canonical `SecurityEvent` enum and the `SecurityEventHook` trait
//! that on-chain pallets call when security-critical transitions occur.
//!
//! ## Emission Points
//!
//! | Pallet            | Event                  | Trigger                          |
//! |-------------------|------------------------|----------------------------------|
//! | `x3-invariants`   | `InvariantBreach`      | Any invariant check fails        |
//! | `x3-invariants`   | `ChainHaltRaised`      | `Halted` storage flag set        |
//! | `x3-invariants`   | `KillSwitchActivated`  | Emergency kill switch triggered  |
//! | `x3-slash`        | `BondSlashed`          | A validator bond is slashed      |
//!
//! ## Usage
//!
//! ```rust,ignore
//! // In a pallet Config:
//! type SecurityHook: SecurityEventHook;
//!
//! // At an emission site:
//! T::SecurityHook::emit(SecurityEvent::InvariantBreach {
//!     invariant_kind: kind_bytes,
//!     block_number,
//!     violation_count,
//! });
//! ```
//!
//! ## `no_std` Compatibility
//!
//! This crate is `no_std` when the `std` feature is disabled. Safe for WASM runtimes.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

// ─────────────────────────────────────────────────────────────────────────────
// Event kind taxonomy
// ─────────────────────────────────────────────────────────────────────────────

/// Category of the security event.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
pub enum SecurityEventKind {
    /// An on-chain invariant check found a breach.
    InvariantBreach,
    /// The chain halt flag was raised by the invariant pallet.
    ChainHaltRaised,
    /// An emergency kill switch was activated for a module.
    KillSwitchActivated,
    /// A validator bond was slashed.
    BondSlashed,
    /// A suspicious settlement timeout was detected (possible griefing).
    SettlementTimeoutSuspect,
    /// Reserved for future security event categories.
    Other([u8; 32]),
}

// ─────────────────────────────────────────────────────────────────────────────
// Canonical security event type
// ─────────────────────────────────────────────────────────────────────────────

/// A single canonical security event emitted by any X3 pallet at a
/// security-critical transition.
///
/// Intentionally flat (no enum variants) so that off-chain indexers can
/// deserialise every event with the same struct without pattern-matching on
/// pallet-specific variants.
///
/// Fields that are irrelevant for a given event kind should be set to
/// zero / empty defaults — off-chain consumers ignore them per `kind`.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct SecurityEvent<BlockNumber> {
    /// Taxonomy of this event.
    pub kind: SecurityEventKind,
    /// Block at which the event occurred.
    pub block_number: BlockNumber,
    /// Opaque 32-byte source identifier (pallet name hash, module ID, etc.).
    pub source_id: [u8; 32],
    /// Opaque 32-byte subject identifier (account hash, bond ID, module ID, etc.).
    pub subject_id: [u8; 32],
    /// Numeric severity: 0 = informational, 1 = warning, 2 = critical.
    pub severity: u8,
    /// Arbitrary detail payload (e.g. violation count, slash amount as SCALE u128).
    pub detail: [u8; 32],
}

impl<BlockNumber: Default> SecurityEvent<BlockNumber> {
    /// Construct an `InvariantBreach` event.
    pub fn invariant_breach(
        invariant_kind_hash: [u8; 32],
        block_number: BlockNumber,
        violation_count: u64,
    ) -> Self {
        let mut detail = [0u8; 32];
        let count_bytes = violation_count.to_le_bytes();
        detail[..8].copy_from_slice(&count_bytes);
        Self {
            kind: SecurityEventKind::InvariantBreach,
            block_number,
            source_id: *b"x3-invariants\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            subject_id: invariant_kind_hash,
            severity: 2,
            detail,
        }
    }

    /// Construct a `ChainHaltRaised` event.
    pub fn chain_halt_raised(block_number: BlockNumber, violation_count: u64) -> Self {
        let mut detail = [0u8; 32];
        let count_bytes = violation_count.to_le_bytes();
        detail[..8].copy_from_slice(&count_bytes);
        Self {
            kind: SecurityEventKind::ChainHaltRaised,
            block_number,
            source_id: *b"x3-invariants\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            subject_id: [0u8; 32],
            severity: 2,
            detail,
        }
    }

    /// Construct a `BondSlashed` event.
    pub fn bond_slashed(
        bond_id: [u8; 32],
        block_number: BlockNumber,
        slash_amount_u128: [u8; 16],
    ) -> Self {
        let mut detail = [0u8; 32];
        detail[..16].copy_from_slice(&slash_amount_u128);
        Self {
            kind: SecurityEventKind::BondSlashed,
            block_number,
            source_id: *b"x3-slash\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            subject_id: bond_id,
            severity: 1,
            detail,
        }
    }

    /// Construct a `KillSwitchActivated` event.
    pub fn kill_switch_activated(module_id: [u8; 32], block_number: BlockNumber) -> Self {
        Self {
            kind: SecurityEventKind::KillSwitchActivated,
            block_number,
            source_id: *b"x3-invariants\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            subject_id: module_id,
            severity: 2,
            detail: [0u8; 32],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Emission trait
// ─────────────────────────────────────────────────────────────────────────────

/// Pallets call [`SecurityEventHook::emit`] when a security-critical transition
/// occurs.  The runtime wires in a concrete implementation; test mocks use
/// [`NoOpHook`].
///
/// The `BlockNumber` type parameter must match the pallet's `T::BlockNumber` /
/// `frame_system::Config::BlockNumber`.
pub trait SecurityEventHook<BlockNumber> {
    /// Emit a security event.  Must be infallible — implementations log or
    /// forward the event but never return an error that would abort the
    /// enclosing dispatchable.
    fn emit(event: SecurityEvent<BlockNumber>);
}

// ─────────────────────────────────────────────────────────────────────────────
// Zero-cost no-op implementation
// ─────────────────────────────────────────────────────────────────────────────

/// A zero-cost no-op implementation of [`SecurityEventHook`].
///
/// Wire this into test mocks and pallets that have not yet been connected to a
/// live security swarm subscriber.  The compiler will eliminate every call to
/// `NoOpHook::emit` entirely.
///
/// ```rust,ignore
/// impl pallet_x3_invariants::Config for Test {
///     type SecurityHook = x3_security_events::NoOpHook;
///     // ...
/// }
/// ```
pub struct NoOpHook;

impl<B> SecurityEventHook<B> for NoOpHook {
    #[inline(always)]
    fn emit(_event: SecurityEvent<B>) {}
}
