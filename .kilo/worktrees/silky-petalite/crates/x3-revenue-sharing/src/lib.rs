//! # X3 Revenue Sharing — Phase 8
//!
//! Canonical types for dApp hub revenue-split policies and earnings tracking.
//! All types are `no_std` compatible and `MaxEncodedLen`-safe for on-chain storage.
//!
//! ## Key Types
//!
//! - [`RevenueSplitPolicy`] — fixed-size policy backed by up to 8 [`RevenueSplitEntry`] slots.
//! - [`validate_split`] — compile-time–free check that a policy sums to exactly 10 000 bps.
//! - [`EarningsSummary`] — snapshot of accumulated earnings for a developer.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[cfg(test)]
mod tests;

// ── Constants ──────────────────────────────────────────────────────────────────

/// Maximum number of active entries in a [`RevenueSplitPolicy`].
pub const MAX_SPLIT_ENTRIES: u32 = 8;

// ── Revenue destination ────────────────────────────────────────────────────────

/// Where a revenue share is routed after collection.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen)]
pub enum RevenueDestination {
    Treasury         = 0,
    DeveloperAccount = 1,
    ProtocolBurn     = 2,
    LiquidityPool    = 3,
    Stakers          = 4,
}

// ── Split entry ────────────────────────────────────────────────────────────────

/// One line item in a revenue split: a destination and its share in basis points.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen)]
pub struct RevenueSplitEntry {
    pub destination: RevenueDestination,
    /// Share expressed in basis points (100 bps = 1 %).
    pub share_bps: u32,
}

// ── Split policy ───────────────────────────────────────────────────────────────

/// A complete revenue-split policy backed by a fixed 8-slot array.
///
/// Only `entries[..entries_len]` are considered active; the remainder may hold
/// any value.  A valid policy must have `validate_split(policy) == true`.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen)]
pub struct RevenueSplitPolicy {
    pub policy_id: u32,
    pub entries_len: u8,
    pub entries: [RevenueSplitEntry; 8],
}

// ── Placement tier ─────────────────────────────────────────────────────────────

/// Marketplace placement tier for a registered dApp.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen)]
pub enum PlacementTier {
    Standard = 0,
    Featured = 1,
    Premium  = 2,
}

// ── Approval status ────────────────────────────────────────────────────────────

/// Governance approval lifecycle for a registered dApp.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen)]
pub enum ApprovalStatus {
    Pending   = 0,
    Approved  = 1,
    Rejected  = 2,
    Suspended = 3,
}

// ── Earnings summary ───────────────────────────────────────────────────────────

/// Cumulative earnings snapshot for a developer account.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen)]
pub struct EarningsSummary {
    pub total_revenue:    u128,
    pub developer_share:  u128,
    pub protocol_share:   u128,
    pub burn_amount:      u128,
}

// ── Validation ─────────────────────────────────────────────────────────────────

/// Returns `true` if and only if the active entries of `policy` sum to exactly
/// 10 000 basis points (i.e. 100 %).
pub fn validate_split(policy: &RevenueSplitPolicy) -> bool {
    let sum: u32 = policy.entries[..policy.entries_len as usize]
        .iter()
        .map(|e| e.share_bps)
        .sum();
    sum == 10_000
}
