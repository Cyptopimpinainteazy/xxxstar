// Tests for x3-rpc-policy: score boundaries, failover logic, and status transitions.
//
// These tests are the normative check of §4.3 and §4.4 of
// `docs/RPC_OPERATIONAL_POLICY.md`.  Any change to the threshold constants or
// to `ProviderHealthScore::status` / `should_failover` must be accompanied by
// corresponding test updates.

use super::{
    ChainFamily, ProviderConfig, ProviderHealthScore, ProviderStatus, ProviderTier,
    DEGRADED_BLOCK_DRIFT, FAILOVER_THRESHOLD, FREEZE_THRESHOLD, MAX_BLOCK_DRIFT,
    MAX_ERROR_RATE_BPS,
};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Build a [`ProviderHealthScore`] with all signals set to zero except `score`.
fn score_only(score: u8) -> ProviderHealthScore {
    ProviderHealthScore {
        score,
        finality_lag_blocks: 0,
        error_rate_bps: 0,
        latency_ms: 0,
        block_drift: 0,
    }
}

// ---------------------------------------------------------------------------
// Test 1 — Status: Healthy at the exact failover threshold
// ---------------------------------------------------------------------------

/// A score exactly equal to `FAILOVER_THRESHOLD` (60) is Healthy.
///
/// Boundary value: the threshold is inclusive from the healthy side.
#[test]
fn status_at_failover_threshold_is_healthy() {
    let h = score_only(FAILOVER_THRESHOLD);
    assert_eq!(h.status(), ProviderStatus::Healthy);
}

// ---------------------------------------------------------------------------
// Test 2 — Status: Degraded one point below the failover threshold
// ---------------------------------------------------------------------------

/// A score of `FAILOVER_THRESHOLD - 1` (59) is Degraded, not Healthy.
#[test]
fn status_one_below_failover_threshold_is_degraded() {
    let h = score_only(FAILOVER_THRESHOLD - 1);
    assert_eq!(h.status(), ProviderStatus::Degraded);
}

// ---------------------------------------------------------------------------
// Test 3 — Status: Degraded at the exact freeze threshold
// ---------------------------------------------------------------------------

/// A score exactly equal to `FREEZE_THRESHOLD` (30) is Degraded, not Frozen.
///
/// The Frozen state begins below 30, so 30 itself must remain Degraded.
#[test]
fn status_at_freeze_threshold_is_degraded() {
    let h = score_only(FREEZE_THRESHOLD);
    assert_eq!(h.status(), ProviderStatus::Degraded);
}

// ---------------------------------------------------------------------------
// Test 4 — Status: Frozen one point below the freeze threshold
// ---------------------------------------------------------------------------

/// A score of `FREEZE_THRESHOLD - 1` (29) is Frozen.
#[test]
fn status_one_below_freeze_threshold_is_frozen() {
    let h = score_only(FREEZE_THRESHOLD - 1);
    assert_eq!(h.status(), ProviderStatus::Frozen);
}

// ---------------------------------------------------------------------------
// Test 5 — Status: Offline at score zero
// ---------------------------------------------------------------------------

/// A score of 0 maps to Offline regardless of signal fields.
#[test]
fn status_score_zero_is_offline() {
    let h = score_only(0);
    assert_eq!(h.status(), ProviderStatus::Offline);
}

// ---------------------------------------------------------------------------
// Test 6 — Failover: triggered by low score alone
// ---------------------------------------------------------------------------

/// A score below the failover threshold triggers failover even when
/// block_drift and error_rate are both zero.
#[test]
fn failover_triggered_by_low_score() {
    let h = score_only(FAILOVER_THRESHOLD - 1); // score = 59
    assert!(
        h.should_failover(),
        "score < FAILOVER_THRESHOLD must trigger failover"
    );
}

// ---------------------------------------------------------------------------
// Test 7 — Failover: triggered by block drift alone
// ---------------------------------------------------------------------------

/// An endpoint with a perfect score (100) but block drift exceeding
/// `MAX_BLOCK_DRIFT` (10) must still trigger failover.
#[test]
fn failover_triggered_by_excessive_block_drift() {
    let h = ProviderHealthScore {
        score: 100,                       // perfect score
        block_drift: MAX_BLOCK_DRIFT + 1, // 11 blocks → exceeds limit
        error_rate_bps: 0,
        finality_lag_blocks: 0,
        latency_ms: 0,
    };
    assert!(
        h.should_failover(),
        "block_drift > MAX_BLOCK_DRIFT must trigger failover"
    );
}

// ---------------------------------------------------------------------------
// Test 8 — Failover: triggered by error rate alone
// ---------------------------------------------------------------------------

/// An endpoint with a perfect score (100) but an error rate exceeding
/// `MAX_ERROR_RATE_BPS` (500) must still trigger failover.
#[test]
fn failover_triggered_by_high_error_rate() {
    let h = ProviderHealthScore {
        score: 100,                             // perfect score
        error_rate_bps: MAX_ERROR_RATE_BPS + 1, // 501 BPS → exceeds limit
        block_drift: 0,
        finality_lag_blocks: 0,
        latency_ms: 0,
    };
    assert!(
        h.should_failover(),
        "error_rate_bps > MAX_ERROR_RATE_BPS must trigger failover"
    );
}

// ---------------------------------------------------------------------------
// Test 9 — No failover: all signals within acceptable bounds
// ---------------------------------------------------------------------------

/// An endpoint with a healthy score and all signals at the boundary of
/// acceptability must NOT trigger failover.
///
/// Boundary values used:
/// - score = FAILOVER_THRESHOLD (60) — exactly healthy
/// - block_drift = MAX_BLOCK_DRIFT (10) — at the limit, not beyond
/// - error_rate_bps = MAX_ERROR_RATE_BPS (500) — at the limit, not beyond
#[test]
fn no_failover_when_all_signals_at_or_within_bounds() {
    let h = ProviderHealthScore {
        score: FAILOVER_THRESHOLD,          // 60 — healthy
        block_drift: MAX_BLOCK_DRIFT,       // 10 — at limit, still OK
        error_rate_bps: MAX_ERROR_RATE_BPS, // 500 — at limit, still OK
        finality_lag_blocks: 0,
        latency_ms: 0,
    };
    assert!(
        !h.should_failover(),
        "no failover expected when all signals are within bounds"
    );
}

// ---------------------------------------------------------------------------
// Test 10 — Constants match policy document values
// ---------------------------------------------------------------------------

/// Verify that the five authoritative constants match the values documented in
/// `docs/RPC_OPERATIONAL_POLICY.md §4 Appendix A`.
///
/// This test fails intentionally when a constant is changed without updating
/// the policy document, acting as a documentation coupling guard.
#[test]
fn policy_constants_match_documented_values() {
    assert_eq!(
        FAILOVER_THRESHOLD, 60,
        "FAILOVER_THRESHOLD must be 60 (§4.3)"
    );
    assert_eq!(FREEZE_THRESHOLD, 30, "FREEZE_THRESHOLD must be 30 (§4.3)");
    assert_eq!(
        MAX_BLOCK_DRIFT, 10,
        "MAX_BLOCK_DRIFT must be 10 blocks (§4.4 / §6.5)"
    );
    assert_eq!(
        MAX_ERROR_RATE_BPS, 500,
        "MAX_ERROR_RATE_BPS must be 500 BPS = 5% (§4.4)"
    );
    assert_eq!(
        DEGRADED_BLOCK_DRIFT, 5,
        "DEGRADED_BLOCK_DRIFT must be 5 blocks (§6.5)"
    );
}

// ---------------------------------------------------------------------------
// Test 11 — ProviderConfig: field round-trip encode/decode
// ---------------------------------------------------------------------------

/// Verify that [`ProviderConfig`] round-trips through SCALE encoding without
/// data loss.  This guards the wire format used by on-chain storage and the
/// solvency sidecar serialization path.
#[test]
fn provider_config_scale_round_trip() {
    use codec::{Decode, Encode};

    let original = ProviderConfig {
        tier: ProviderTier::ManagedProvider,
        chain_family: ChainFamily::Evm,
        failover_priority: 1,
        max_requests_per_second: 300,
        cache_ttl_seconds: 1,
    };
    let encoded = original.encode();
    let decoded = ProviderConfig::decode(&mut &encoded[..])
        .expect("ProviderConfig must decode without error");
    assert_eq!(
        original, decoded,
        "SCALE round-trip must preserve all fields"
    );
}

// ---------------------------------------------------------------------------
// Test 12 — ProviderHealthScore: full status transition sequence
// ---------------------------------------------------------------------------

/// Walk a provider from Healthy through Degraded → Frozen → Offline by
/// decrementing the score, verifying that status changes at the correct
/// boundary values.
#[test]
fn status_transitions_follow_documented_boundaries() {
    // 100 → Healthy
    assert_eq!(score_only(100).status(), ProviderStatus::Healthy);
    // 60  → Healthy (inclusive boundary)
    assert_eq!(score_only(60).status(), ProviderStatus::Healthy);
    // 59  → Degraded
    assert_eq!(score_only(59).status(), ProviderStatus::Degraded);
    // 30  → Degraded (inclusive boundary)
    assert_eq!(score_only(30).status(), ProviderStatus::Degraded);
    // 29  → Frozen
    assert_eq!(score_only(29).status(), ProviderStatus::Frozen);
    // 1   → Frozen
    assert_eq!(score_only(1).status(), ProviderStatus::Frozen);
    // 0   → Offline
    assert_eq!(score_only(0).status(), ProviderStatus::Offline);
}
