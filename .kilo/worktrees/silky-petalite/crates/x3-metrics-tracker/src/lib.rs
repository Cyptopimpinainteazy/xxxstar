//! # X3 Metrics Tracker — Phase 12 A-Tier Hard Metrics
//!
//! This crate defines the **canonical hard-metric types** used by the Phase 12
//! A-tier progress tracking system. All monetary values are stored in **USD
//! cents** (`u64`) to avoid floating-point arithmetic inside WASM runtimes or
//! low-level sidecar services.
//!
//! ## A-Tier Threshold Summary
//!
//! An [`ATierSnapshot`] qualifies as A-tier when **all** of the following hold:
//!
//! | Metric                        | Minimum        |
//! |-------------------------------|----------------|
//! | Average TPS                   | 100            |
//! | TVL                           | $10 M USD      |
//! | Route volume (daily)          | $1 M USD       |
//! | Daily active users            | 1 000          |
//! | P1 incidents (monthly)        | 0              |
//!
//! ## `no_std` Compatibility
//!
//! This crate is `no_std` when the `std` feature is disabled and is safe to
//! import from both WASM runtimes and native sidecar binaries.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[cfg(test)]
mod tests;

// ─── Throughput metrics ──────────────────────────────────────────────────────

/// On-chain throughput measurements for a snapshot window.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ThroughputMetrics {
    /// Peak transactions per second observed during the window.
    pub tps_peak: u32,
    /// Average transactions per second over the window.
    pub tps_avg: u32,
    /// Average block production time in milliseconds.
    pub block_time_ms_avg: u32,
    /// Average time from inclusion to finality in milliseconds.
    pub finality_time_ms_avg: u32,
}

// ─── Treasury metrics ────────────────────────────────────────────────────────

/// Treasury and TVL health metrics for a snapshot window.
///
/// All monetary values are expressed in **USD cents** to avoid floating-point
/// arithmetic. A value of `100` represents $1.00 USD.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct TreasuryMetrics {
    /// Total value locked across all X3 vaults (USD cents).
    pub tvl_usd_cents: u64,
    /// Current on-chain treasury balance (USD cents).
    pub treasury_balance_usd_cents: u64,
    /// Liquidity currently deployed in AMM/lending positions (USD cents).
    pub deployed_float_usd_cents: u64,
    /// Funds held in the protocol insurance reserve (USD cents).
    pub insurance_reserve_usd_cents: u64,
    /// Week-over-week growth rate in basis points. Negative values indicate
    /// contraction (e.g. `-500` = −5.00 %).
    pub growth_rate_bps_weekly: i32,
}

// ─── Route metrics ───────────────────────────────────────────────────────────

/// Cross-chain and intra-chain route execution metrics.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteMetrics {
    /// Gross route volume settled during the day (USD cents).
    pub route_volume_usd_cents_daily: u64,
    /// Number of routes that settled successfully.
    pub successful_routes_daily: u32,
    /// Number of routes that timed out or reverted.
    pub failed_routes_daily: u32,
    /// Median settlement time across all routes in milliseconds.
    pub avg_settlement_time_ms: u32,
    /// 99th-percentile settlement time in milliseconds.
    pub p99_settlement_time_ms: u32,
}

// ─── User metrics ────────────────────────────────────────────────────────────

/// User engagement metrics for a snapshot window.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct UserMetrics {
    /// Distinct on-chain addresses active during the day.
    pub dau: u32,
    /// Distinct on-chain addresses active during the trailing 30 days.
    pub mau: u32,
    /// New wallet addresses created during the day.
    pub new_wallets_daily: u32,
    /// Returning-user rate expressed in hundredths of a percent.
    /// A value of `7500` represents 75.00 %.
    pub returning_users_pct: u32,
}

// ─── Incident metrics ────────────────────────────────────────────────────────

/// Safety and operational incident counters.
///
/// All `_daily` fields are reset at UTC midnight. `p1_incidents_monthly` is
/// the rolling 30-day count of P1-severity outages.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct IncidentMetrics {
    /// Number of routes/lanes currently suspended by the circuit breaker.
    pub frozen_lanes: u32,
    /// Number of times the solvency gate rejected a transaction today.
    pub solvency_gate_failures_daily: u32,
    /// Routes that exceeded their settlement deadline today.
    pub settlement_timeout_count_daily: u32,
    /// Inventory positions that breached their lower threshold today.
    pub under_threshold_incidents_daily: u32,
    /// Rolling 30-day count of P1 (service-impacting) incidents.
    pub p1_incidents_monthly: u32,
}

// ─── A-Tier snapshot ─────────────────────────────────────────────────────────

/// A complete point-in-time snapshot of all A-tier metrics.
///
/// The proving harness writes one [`ATierSnapshot`] per finalized block (or per
/// epoch, depending on configuration). Sidecar services consume these snapshots
/// to produce dashboards and compliance reports without querying chain state
/// directly.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ATierSnapshot {
    /// The finalized block number at which this snapshot was taken.
    pub snapshot_block: u32,
    /// Throughput measurements.
    pub throughput: ThroughputMetrics,
    /// Treasury and TVL health.
    pub treasury: TreasuryMetrics,
    /// Route execution statistics.
    pub routes: RouteMetrics,
    /// User engagement counters.
    pub users: UserMetrics,
    /// Safety and operational incident counters.
    pub incidents: IncidentMetrics,
}

impl ATierSnapshot {
    /// Returns `true` when every hard A-tier threshold is satisfied.
    ///
    /// | Metric                        | Minimum threshold           |
    /// |-------------------------------|-----------------------------|
    /// | `throughput.tps_avg`          | 100 TPS                     |
    /// | `treasury.tvl_usd_cents`      | 1 000 000 000 ($10 M USD)   |
    /// | `routes.route_volume_…_daily` | 100 000 000 ($1 M USD/day)  |
    /// | `users.dau`                   | 1 000 addresses/day         |
    /// | `incidents.p1_incidents…`     | 0 (zero tolerance)          |
    #[must_use]
    pub fn meets_a_tier_threshold(&self) -> bool {
        self.throughput.tps_avg >= 100
            && self.treasury.tvl_usd_cents >= 10_000_000_00 // $10 M in cents
            && self.routes.route_volume_usd_cents_daily >= 1_000_000_00 // $1 M/day in cents
            && self.users.dau >= 1_000
            && self.incidents.p1_incidents_monthly == 0
    }
}
