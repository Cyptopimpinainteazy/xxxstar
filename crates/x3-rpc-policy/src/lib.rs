// Provider health scoring types for X3 RPC operational policy.
//
// This crate is `no_std`-compatible and encodes only pure data types with no I/O,
// no heap allocation, and no async runtime dependency.  It is intended to be
// consumed by the node binary, the solvency sidecar, and any off-chain service
// that needs to reason about RPC provider health without pulling in the full
// node dependency graph.
//
// The authoritative threshold constants (FAILOVER_THRESHOLD, FREEZE_THRESHOLD,
// MAX_BLOCK_DRIFT, MAX_ERROR_RATE_BPS, DEGRADED_BLOCK_DRIFT) are the single
// source of truth for values referenced in docs/RPC_OPERATIONAL_POLICY.md.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::all, clippy::pedantic)]
#![deny(unsafe_code)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

// ---------------------------------------------------------------------------
// Provider tier
// ---------------------------------------------------------------------------

/// Operational tier that determines selection priority for a provider.
///
/// Lower discriminant values are preferred: Tier 0 (X3Owned) is always tried
/// before Tier 1 (ManagedProvider), which is always tried before Tier 2
/// (PublicFallback).
///
/// See §3 of `docs/RPC_OPERATIONAL_POLICY.md` for the full tier definitions.
#[derive(
    Encode, Decode, TypeInfo, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub enum ProviderTier {
    /// Tier 0: X3-owned infrastructure. No external rate limits; full custom
    /// method support; must maintain sync within 2 blocks of network tip.
    X3Owned = 0,
    /// Tier 1: Third-party managed providers (Alchemy, dRPC/Nodecore, Ankr,
    /// QuickNode). Operate under a commercial SLA with X3-managed API keys.
    ManagedProvider = 1,
    /// Tier 2: Unauthenticated or lightly-authenticated public fallback
    /// endpoints. Read-only, severely capacity-constrained. Used only when all
    /// Tier 0 and Tier 1 endpoints are unavailable.
    PublicFallback = 2,
}

// ---------------------------------------------------------------------------
// Chain family
// ---------------------------------------------------------------------------

/// VM / protocol family that an RPC endpoint serves.
///
/// The failover router filters the candidate provider pool by chain family
/// before scoring (see §5 of `docs/RPC_OPERATIONAL_POLICY.md`).
#[derive(
    Encode, Decode, TypeInfo, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub enum ChainFamily {
    /// Substrate / Polkadot-compatible JSON-RPC endpoint (`chain_*`, `state_*`,
    /// `author_*`, and custom `x3_*` namespaces).
    Substrate,
    /// Ethereum-compatible JSON-RPC endpoint (`eth_*`, `net_*`, `web3_*`).
    Evm,
    /// Solana-compatible RPC endpoint.
    Svm,
}

// ---------------------------------------------------------------------------
// Provider health score
// ---------------------------------------------------------------------------

/// Point-in-time health measurement for a single RPC endpoint.
///
/// `score` is the composite 0–100 value computed by the health monitor in
/// `node/src/metrics.rs` every 30 seconds (see §4 of the operational policy).
/// The other fields are the raw signals that feed into the score formula.
///
/// # Score penalty schedule (summary)
///
/// | Signal | Max deduction |
/// |--------|---------------|
/// | Finality lag | 30 points |
/// | Error rate | 30 points |
/// | Response latency (p95) | 25 points |
/// | Block drift | 15 points |
///
/// See §4.2 of `docs/RPC_OPERATIONAL_POLICY.md` for the full penalty tables.
#[derive(
    Encode, Decode, TypeInfo, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub struct ProviderHealthScore {
    /// Composite health score in [0, 100].  100 is perfect; 0 means offline.
    pub score: u8,
    /// Blocks since the last observed finalized block on this endpoint.
    pub finality_lag_blocks: u32,
    /// Observed error rate in basis points (0 = 0 %, 10 000 = 100 %).
    pub error_rate_bps: u32,
    /// p95 response latency in milliseconds for the last 30-second window.
    pub latency_ms: u32,
    /// Absolute difference between this endpoint's best block and the
    /// canonical chain tip observed from Tier 0 nodes.
    pub block_drift: u32,
}

// ---------------------------------------------------------------------------
// Provider status
// ---------------------------------------------------------------------------

/// Operational status derived from [`ProviderHealthScore::status`].
///
/// The status classification maps directly to the score thresholds defined in
/// §4.3 of `docs/RPC_OPERATIONAL_POLICY.md`.
#[derive(
    Encode, Decode, TypeInfo, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub enum ProviderStatus {
    /// Score ≥ 60 and all signal thresholds within bounds.  Normal routing.
    Healthy,
    /// Score 30–59 or block drift 5–10.  Read-only; writes routed elsewhere.
    Degraded,
    /// Score 1–29.  No new traffic.  Connections drained and closed.
    Frozen,
    /// Score = 0 or endpoint unreachable.  Removed from routing pool.
    Offline,
}

// ---------------------------------------------------------------------------
// Provider configuration
// ---------------------------------------------------------------------------

/// Static configuration for a single RPC endpoint used by the routing layer.
///
/// This type is intentionally separate from [`ProviderHealthScore`] so that
/// configuration can be stored on-chain or in genesis without carrying
/// mutable runtime state.
#[derive(
    Encode, Decode, TypeInfo, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub struct ProviderConfig {
    /// Operational tier (Tier 0 / 1 / 2).
    pub tier: ProviderTier,
    /// Chain family served by this endpoint.
    pub chain_family: ChainFamily,
    /// Lower value = higher priority.  0 is the highest possible priority.
    /// Within the same priority level, selection is weighted by health score.
    pub failover_priority: u8,
    /// Maximum requests per second this endpoint can sustain under normal load.
    /// 0 means "hardware-bound" (used for Tier 0 nodes).
    pub max_requests_per_second: u32,
    /// Default TTL in seconds for cached responses from this endpoint.
    /// Actual TTL per method class may be shorter (see §7 of the policy).
    pub cache_ttl_seconds: u32,
}

// ---------------------------------------------------------------------------
// Authoritative threshold constants
// ---------------------------------------------------------------------------

/// Health score threshold below which failover is triggered.
///
/// A provider with `score < FAILOVER_THRESHOLD` is considered Degraded or worse
/// and will not receive new write traffic.  The routing layer begins routing all
/// requests to the next healthy endpoint immediately.
///
/// Reference: §4.3 and §4.4 of `docs/RPC_OPERATIONAL_POLICY.md`.
pub const FAILOVER_THRESHOLD: u8 = 60;

/// Health score threshold below which an endpoint is Frozen.
///
/// A provider with `score < FREEZE_THRESHOLD` is removed from the routing pool
/// entirely.  Existing connections are drained and no new connections are
/// accepted until the score recovers above `FAILOVER_THRESHOLD` for two
/// consecutive 30-second windows.
///
/// Reference: §4.3 of `docs/RPC_OPERATIONAL_POLICY.md`.
pub const FREEZE_THRESHOLD: u8 = 30;

/// Maximum block drift before mandatory failover, regardless of score.
///
/// If an endpoint's `block_drift` exceeds this value the routing layer treats
/// it as if `score < FAILOVER_THRESHOLD`, even when the score itself is healthy.
/// This prevents serving stale state from a slow or stalled node.
///
/// Reference: §4.4 and §6.5 of `docs/RPC_OPERATIONAL_POLICY.md`.
pub const MAX_BLOCK_DRIFT: u32 = 10;

/// Maximum error rate in basis points before mandatory failover, regardless of score.
///
/// If an endpoint's `error_rate_bps` exceeds this value the routing layer
/// initiates failover.  500 BPS = 5 %.
///
/// Reference: §4.4 of `docs/RPC_OPERATIONAL_POLICY.md`.
pub const MAX_ERROR_RATE_BPS: u32 = 500;

/// Block drift at which the endpoint enters DEGRADED mode (but not yet failover).
///
/// Drifts between `DEGRADED_BLOCK_DRIFT` and `MAX_BLOCK_DRIFT` cause the
/// endpoint to operate in degraded mode: cache TTLs are doubled, writes are
/// queued, but reads are still served.
///
/// Reference: §6.5 of `docs/RPC_OPERATIONAL_POLICY.md`.
pub const DEGRADED_BLOCK_DRIFT: u32 = 5;

// ---------------------------------------------------------------------------
// ProviderHealthScore methods
// ---------------------------------------------------------------------------

impl ProviderHealthScore {
    /// Derive the [`ProviderStatus`] from this health score.
    ///
    /// The mapping is:
    ///
    /// | Score | Status |
    /// |-------|--------|
    /// | ≥ 60 | `Healthy` |
    /// | 30 – 59 | `Degraded` |
    /// | 1 – 29 | `Frozen` |
    /// | 0 | `Offline` |
    ///
    /// This is the canonical implementation of §4.3 of the operational policy.
    /// All routing decisions that depend on status must derive it through this
    /// method rather than comparing scores directly, to ensure a single
    /// source of truth.
    #[must_use]
    pub fn status(&self) -> ProviderStatus {
        if self.score >= FAILOVER_THRESHOLD {
            ProviderStatus::Healthy
        } else if self.score >= FREEZE_THRESHOLD {
            ProviderStatus::Degraded
        } else if self.score > 0 {
            ProviderStatus::Frozen
        } else {
            ProviderStatus::Offline
        }
    }

    /// Return `true` when any failover condition is active for this endpoint.
    ///
    /// Failover is triggered when ANY of the following hold:
    /// - `score < FAILOVER_THRESHOLD` (60),
    /// - `block_drift > MAX_BLOCK_DRIFT` (10 blocks),
    /// - `error_rate_bps > MAX_ERROR_RATE_BPS` (500 BPS / 5 %).
    ///
    /// The routing layer calls this method before each routing decision.  A
    /// `true` result means the endpoint must not receive new requests and the
    /// next-priority endpoint should be tried instead.
    ///
    /// Reference: §4.4 of `docs/RPC_OPERATIONAL_POLICY.md`.
    #[must_use]
    pub fn should_failover(&self) -> bool {
        self.score < FAILOVER_THRESHOLD
            || self.block_drift > MAX_BLOCK_DRIFT
            || self.error_rate_bps > MAX_ERROR_RATE_BPS
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
