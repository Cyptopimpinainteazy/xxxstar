//! Unit tests for `x3-metrics-tracker`.
//!
//! Coverage (10 tests):
//!  1. meets_a_tier_threshold — all metrics at exact minimums returns true.
//!  2. meets_a_tier_threshold — tps_avg one below minimum returns false.
//!  3. meets_a_tier_threshold — tvl one cent below minimum returns false.
//!  4. meets_a_tier_threshold — route volume one cent below minimum returns false.
//!  5. meets_a_tier_threshold — dau one below minimum returns false.
//!  6. meets_a_tier_threshold — any p1 incident returns false.
//!  7. meets_a_tier_threshold — all metrics well above minimums returns true.
//!  8. SCALE codec roundtrip for ATierSnapshot.
//!  9. SCALE codec roundtrip for ThroughputMetrics.
//! 10. SCALE codec roundtrip for TreasuryMetrics with negative growth rate.

use super::*;
use codec::{Decode, Encode};

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Construct an [`ATierSnapshot`] that passes all A-tier thresholds exactly.
fn passing_snapshot() -> ATierSnapshot {
    ATierSnapshot {
        snapshot_block: 1_000,
        throughput: ThroughputMetrics {
            tps_peak: 250,
            tps_avg: 100, // exact minimum
            block_time_ms_avg: 6_000,
            finality_time_ms_avg: 12_000,
        },
        treasury: TreasuryMetrics {
            tvl_usd_cents: 10_000_000_00, // $10 M exactly
            treasury_balance_usd_cents: 5_000_000_00,
            deployed_float_usd_cents: 4_000_000_00,
            insurance_reserve_usd_cents: 1_000_000_00,
            growth_rate_bps_weekly: 200, // +2.00 %
        },
        routes: RouteMetrics {
            route_volume_usd_cents_daily: 1_000_000_00, // $1 M exactly
            successful_routes_daily: 5_000,
            failed_routes_daily: 10,
            avg_settlement_time_ms: 800,
            p99_settlement_time_ms: 3_200,
        },
        users: UserMetrics {
            dau: 1_000, // exact minimum
            mau: 15_000,
            new_wallets_daily: 50,
            returning_users_pct: 7_500, // 75.00 %
        },
        incidents: IncidentMetrics {
            frozen_lanes: 0,
            solvency_gate_failures_daily: 0,
            settlement_timeout_count_daily: 0,
            under_threshold_incidents_daily: 0,
            p1_incidents_monthly: 0, // zero tolerance
        },
    }
}

// ─── 1. Exact minimums pass ───────────────────────────────────────────────────

#[test]
fn meets_a_tier_threshold_exact_minimums_passes() {
    assert!(
        passing_snapshot().meets_a_tier_threshold(),
        "snapshot at exact minimums must return true"
    );
}

// ─── 2. tps_avg one below minimum ────────────────────────────────────────────

#[test]
fn meets_a_tier_threshold_tps_below_fails() {
    let mut snap = passing_snapshot();
    snap.throughput.tps_avg = 99;
    assert!(
        !snap.meets_a_tier_threshold(),
        "tps_avg=99 must fail (threshold is 100)"
    );
}

// ─── 3. tvl one cent below minimum ───────────────────────────────────────────

#[test]
fn meets_a_tier_threshold_tvl_below_fails() {
    let mut snap = passing_snapshot();
    snap.treasury.tvl_usd_cents = 10_000_000_00 - 1;
    assert!(
        !snap.meets_a_tier_threshold(),
        "tvl one cent below $10 M must fail"
    );
}

// ─── 4. route volume one cent below minimum ───────────────────────────────────

#[test]
fn meets_a_tier_threshold_route_volume_below_fails() {
    let mut snap = passing_snapshot();
    snap.routes.route_volume_usd_cents_daily = 1_000_000_00 - 1;
    assert!(
        !snap.meets_a_tier_threshold(),
        "route volume one cent below $1 M must fail"
    );
}

// ─── 5. dau one below minimum ────────────────────────────────────────────────

#[test]
fn meets_a_tier_threshold_dau_below_fails() {
    let mut snap = passing_snapshot();
    snap.users.dau = 999;
    assert!(
        !snap.meets_a_tier_threshold(),
        "dau=999 must fail (threshold is 1 000)"
    );
}

// ─── 6. any p1 incident fails ────────────────────────────────────────────────

#[test]
fn meets_a_tier_threshold_p1_incident_fails() {
    let mut snap = passing_snapshot();
    snap.incidents.p1_incidents_monthly = 1;
    assert!(
        !snap.meets_a_tier_threshold(),
        "p1_incidents_monthly=1 must fail (zero tolerance)"
    );
}

// ─── 7. all well above minimums ──────────────────────────────────────────────

#[test]
fn meets_a_tier_threshold_well_above_all_passes() {
    let mut snap = passing_snapshot();
    snap.throughput.tps_avg = 500;
    snap.treasury.tvl_usd_cents = 50_000_000_00; // $50 M
    snap.routes.route_volume_usd_cents_daily = 10_000_000_00; // $10 M
    snap.users.dau = 50_000;
    snap.incidents.p1_incidents_monthly = 0;
    assert!(
        snap.meets_a_tier_threshold(),
        "snapshot well above all thresholds must return true"
    );
}

// ─── 8. SCALE codec roundtrip — ATierSnapshot ────────────────────────────────

#[test]
fn a_tier_snapshot_scale_roundtrip() {
    let snap = passing_snapshot();

    let encoded = snap.encode();
    let decoded = ATierSnapshot::decode(&mut &encoded[..]).expect("SCALE decode must succeed");
    assert_eq!(snap, decoded, "roundtrip equality failed for ATierSnapshot");
}

// ─── 9. SCALE codec roundtrip — ThroughputMetrics ────────────────────────────

#[test]
fn throughput_metrics_scale_roundtrip() {
    let metrics = ThroughputMetrics {
        tps_peak: 1_024,
        tps_avg: 512,
        block_time_ms_avg: 3_000,
        finality_time_ms_avg: 9_000,
    };

    let encoded = metrics.encode();
    let decoded = ThroughputMetrics::decode(&mut &encoded[..]).expect("SCALE decode must succeed");
    assert_eq!(
        metrics, decoded,
        "roundtrip equality failed for ThroughputMetrics"
    );
}

// ─── 10. SCALE codec roundtrip — TreasuryMetrics with negative growth ─────────

#[test]
fn treasury_metrics_negative_growth_scale_roundtrip() {
    let metrics = TreasuryMetrics {
        tvl_usd_cents: 8_000_000_00,
        treasury_balance_usd_cents: 3_000_000_00,
        deployed_float_usd_cents: 2_500_000_00,
        insurance_reserve_usd_cents: 500_000_00,
        growth_rate_bps_weekly: -350, // −3.50 % contraction
    };

    let encoded = metrics.encode();
    let decoded = TreasuryMetrics::decode(&mut &encoded[..]).expect("SCALE decode must succeed");
    assert_eq!(
        metrics, decoded,
        "roundtrip equality failed for TreasuryMetrics"
    );
    assert_eq!(
        decoded.growth_rate_bps_weekly, -350,
        "negative growth rate must survive SCALE roundtrip"
    );
}

// ─── Serde roundtrips (std only) ─────────────────────────────────────────────

#[cfg(feature = "std")]
mod serde_tests {
    use super::*;

    #[test]
    fn a_tier_snapshot_serde_roundtrip() {
        let snap = passing_snapshot();
        let json = serde_json::to_string(&snap).expect("serde_json serialization must succeed");
        let decoded: ATierSnapshot =
            serde_json::from_str(&json).expect("serde_json deserialization must succeed");
        assert_eq!(
            snap, decoded,
            "serde roundtrip equality failed for ATierSnapshot"
        );
    }

    #[test]
    fn incident_metrics_serde_roundtrip() {
        let metrics = IncidentMetrics {
            frozen_lanes: 2,
            solvency_gate_failures_daily: 5,
            settlement_timeout_count_daily: 1,
            under_threshold_incidents_daily: 3,
            p1_incidents_monthly: 0,
        };
        let json = serde_json::to_string(&metrics).expect("serde_json serialization must succeed");
        let decoded: IncidentMetrics =
            serde_json::from_str(&json).expect("serde_json deserialization must succeed");
        assert_eq!(
            metrics, decoded,
            "serde roundtrip equality failed for IncidentMetrics"
        );
    }
}
