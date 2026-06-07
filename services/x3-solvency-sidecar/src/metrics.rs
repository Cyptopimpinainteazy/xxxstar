//! Prometheus metrics for the X3 solvency sidecar.
//!
//! All gauge families are registered with the default global Prometheus
//! registry exactly once (via `lazy_static!`) so that the standard
//! `prometheus::gather()` → `TextEncoder` pipeline works without any
//! per-request setup.
//!
//! ## Gauge semantics
//!
//! | Metric | Type | Labels |
//! |--------|------|--------|
//! | `x3_vault_utilization_pct` | GaugeVec | `vault_id`, `chain_id`, `asset_id`, `vault_type` |
//! | `x3_vault_status` | GaugeVec | `vault_id`, `chain_id`, `asset_id` |
//! | `x3_lane_status` | GaugeVec | `lane_id`, `source_chain`, `dest_chain` |
//! | `x3_lane_unsettled_notional` | GaugeVec | `lane_id` |
//! | `x3_partner_health_score_bps` | GaugeVec | `partner_id` |
//! | `x3_partner_exposure_pct` | GaugeVec | `partner_id` |
//! | `x3_global_unsettled_notional` | Gauge | — |
//! | `x3_frozen_lane_count` | Gauge | — |
//! | `x3_treasury_at_risk` | Gauge | — |
//! | `x3_last_processed_block` | Gauge | — |
//!
//! Status encoding: `1.0` = Active, `0.5` = Degraded, `0.0` = Frozen.

use prometheus::{Encoder, GaugeVec, Gauge, TextEncoder};

use crate::state::SolvencyDashboard;

// ---------------------------------------------------------------------------
// Static gauge registrations
// ---------------------------------------------------------------------------

lazy_static::lazy_static! {
    /// Vault utilization percentage (available / total capacity × 100).
    static ref VAULT_UTILIZATION_PCT: GaugeVec = prometheus::register_gauge_vec!(
        "x3_vault_utilization_pct",
        "Vault utilization percentage: available_balance / (available + reserved + pending_out) * 100",
        &["vault_id", "chain_id", "asset_id", "vault_type"]
    ).expect("register x3_vault_utilization_pct");

    /// Vault operational status encoded as a float: 1.0=Active, 0.5=Degraded, 0.0=Frozen.
    static ref VAULT_STATUS: GaugeVec = prometheus::register_gauge_vec!(
        "x3_vault_status",
        "Vault operational status: 1.0=Active, 0.5=Degraded, 0.0=Frozen",
        &["vault_id", "chain_id", "asset_id"]
    ).expect("register x3_vault_status");

    /// Lane operational status encoded as a float: 1.0=Active, 0.5=Degraded, 0.0=Frozen.
    static ref LANE_STATUS: GaugeVec = prometheus::register_gauge_vec!(
        "x3_lane_status",
        "Lane operational status: 1.0=Active, 0.5=Degraded, 0.0=Frozen",
        &["lane_id", "source_chain", "dest_chain"]
    ).expect("register x3_lane_status");

    /// Unsettled notional (raw chain units) currently in-flight on a lane.
    static ref LANE_UNSETTLED_NOTIONAL: GaugeVec = prometheus::register_gauge_vec!(
        "x3_lane_unsettled_notional",
        "Unsettled notional in-flight on a lane (raw chain units)",
        &["lane_id"]
    ).expect("register x3_lane_unsettled_notional");

    /// Partner composite health score in basis points (0–10 000).
    static ref PARTNER_HEALTH_SCORE_BPS: GaugeVec = prometheus::register_gauge_vec!(
        "x3_partner_health_score_bps",
        "Partner health score in basis points (0=worst, 10000=best)",
        &["partner_id"]
    ).expect("register x3_partner_health_score_bps");

    /// Partner current exposure as a percentage of the exposure limit.
    static ref PARTNER_EXPOSURE_PCT: GaugeVec = prometheus::register_gauge_vec!(
        "x3_partner_exposure_pct",
        "Partner current_exposure / exposure_limit * 100",
        &["partner_id"]
    ).expect("register x3_partner_exposure_pct");

    /// Total unsettled notional across all lanes (raw chain units).
    static ref GLOBAL_UNSETTLED_NOTIONAL: Gauge = prometheus::register_gauge!(
        "x3_global_unsettled_notional",
        "Total unsettled notional across all active lanes (raw chain units)"
    ).expect("register x3_global_unsettled_notional");

    /// Number of lanes currently in Frozen status.
    static ref FROZEN_LANE_COUNT: Gauge = prometheus::register_gauge!(
        "x3_frozen_lane_count",
        "Number of cross-chain lanes currently in the Frozen state"
    ).expect("register x3_frozen_lane_count");

    /// Treasury balance currently at risk of under-collateralization (raw units).
    static ref TREASURY_AT_RISK: Gauge = prometheus::register_gauge!(
        "x3_treasury_at_risk",
        "Treasury balance at risk of under-collateralization (raw chain units)"
    ).expect("register x3_treasury_at_risk");

    /// Most recent block number processed by the subscriber task.
    static ref LAST_PROCESSED_BLOCK: Gauge = prometheus::register_gauge!(
        "x3_last_processed_block",
        "Most recent block number processed by the solvency subscriber"
    ).expect("register x3_last_processed_block");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Encode an operational status string as a Prometheus-friendly float.
///
/// Returns `1.0` for `"Active"`, `0.5` for `"Degraded"`, and `0.0` for
/// `"Frozen"` (or any unrecognised status).
fn status_to_f64(status: &str) -> f64 {
    match status {
        "Active" => 1.0,
        "Degraded" => 0.5,
        _ => 0.0, // "Frozen" and anything unknown → 0
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Push the current `SolvencyDashboard` snapshot into all registered gauges.
///
/// This function is called by the subscriber task after each block update and
/// is also called inline by the metrics HTTP handler so scrape responses always
/// reflect the latest state even when no block has arrived recently.
pub fn update_metrics(dashboard: &SolvencyDashboard) {
    // Force lazy_static initialisation so every gauge family exists in the
    // registry before any encode call, even if `update_metrics` is the first
    // function the caller invokes.
    lazy_static::initialize(&VAULT_UTILIZATION_PCT);
    lazy_static::initialize(&VAULT_STATUS);
    lazy_static::initialize(&LANE_STATUS);
    lazy_static::initialize(&LANE_UNSETTLED_NOTIONAL);
    lazy_static::initialize(&PARTNER_HEALTH_SCORE_BPS);
    lazy_static::initialize(&PARTNER_EXPOSURE_PCT);
    lazy_static::initialize(&GLOBAL_UNSETTLED_NOTIONAL);
    lazy_static::initialize(&FROZEN_LANE_COUNT);
    lazy_static::initialize(&TREASURY_AT_RISK);
    lazy_static::initialize(&LAST_PROCESSED_BLOCK);

    // Per-vault gauges.
    for vault in &dashboard.vaults {
        let chain = vault.chain_id.to_string();
        let asset = vault.asset_id.to_string();

        VAULT_UTILIZATION_PCT
            .with_label_values(&[&vault.vault_id, &chain, &asset, &vault.vault_type])
            .set(vault.utilization_pct);

        VAULT_STATUS
            .with_label_values(&[&vault.vault_id, &chain, &asset])
            .set(status_to_f64(&vault.status));
    }

    // Per-lane gauges.
    for lane in &dashboard.lanes {
        let src = lane.source_chain.to_string();
        let dst = lane.dest_chain.to_string();

        LANE_STATUS
            .with_label_values(&[&lane.lane_id, &src, &dst])
            .set(status_to_f64(&lane.status));

        LANE_UNSETTLED_NOTIONAL
            .with_label_values(&[&lane.lane_id])
            .set(lane.unsettled_notional as f64);
    }

    // Per-partner gauges.
    for partner in &dashboard.partners {
        PARTNER_HEALTH_SCORE_BPS
            .with_label_values(&[&partner.partner_id])
            .set(f64::from(partner.health_score_bps));

        let exposure_pct = if partner.exposure_limit > 0 {
            partner.current_exposure as f64 / partner.exposure_limit as f64 * 100.0
        } else {
            0.0
        };
        PARTNER_EXPOSURE_PCT
            .with_label_values(&[&partner.partner_id])
            .set(exposure_pct);
    }

    // Scalar gauges.
    GLOBAL_UNSETTLED_NOTIONAL.set(dashboard.global_unsettled_notional as f64);
    FROZEN_LANE_COUNT.set(f64::from(dashboard.frozen_lane_count));
    TREASURY_AT_RISK.set(dashboard.treasury_at_risk as f64);
    LAST_PROCESSED_BLOCK.set(dashboard.last_updated_block as f64);
}

/// Collect all metric families from the default Prometheus registry.
///
/// Provided as a convenience for operators and integration tests that want
/// programmatic access to the raw metric family objects.
#[allow(dead_code)]
pub fn gather_metrics() -> Vec<prometheus::proto::MetricFamily> {
    prometheus::gather()
}

/// Render all metrics to the Prometheus text exposition format (UTF-8 string).
///
/// Returns an empty string if the encoder fails — callers should serve whatever
/// is returned so the scrape endpoint never returns a hard error.
pub fn render_metrics() -> String {
    let encoder = TextEncoder::new();
    let families = prometheus::gather();
    let mut buf = Vec::with_capacity(8192);
    if let Err(e) = encoder.encode(&families, &mut buf) {
        tracing::error!("Prometheus text encoding failed: {e}");
        return String::new();
    }
    String::from_utf8(buf).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{LaneSummary, SolvencyDashboard, VaultSummary};

    fn make_dashboard() -> SolvencyDashboard {
        SolvencyDashboard {
            last_updated_block: 100,
            global_unsettled_notional: 5_000,
            frozen_lane_count: 1,
            treasury_at_risk: 200,
            vaults: vec![VaultSummary {
                vault_id: "0xdeadbeef".to_string(),
                chain_id: 1,
                asset_id: 10,
                vault_type: "Gas".to_string(),
                available_balance: 900,
                reserved_balance: 80,
                pending_out_balance: 20,
                critical_min: 100,
                min_band: 200,
                status: "Active".to_string(),
                utilization_pct: 90.0,
            }],
            lanes: vec![LaneSummary {
                lane_id: "lane-1-2".to_string(),
                source_chain: 1,
                dest_chain: 2,
                lane_class: "A".to_string(),
                status: "Frozen".to_string(),
                unsettled_notional: 1_000,
                exposure_cap: 5_000,
            }],
            ..SolvencyDashboard::default()
        }
    }

    #[test]
    fn status_encoding_is_correct() {
        assert_eq!(status_to_f64("Active"), 1.0);
        assert_eq!(status_to_f64("Degraded"), 0.5);
        assert_eq!(status_to_f64("Frozen"), 0.0);
        assert_eq!(status_to_f64("Unknown"), 0.0);
    }

    #[test]
    fn update_and_render_produce_non_empty_output() {
        let dash = make_dashboard();
        update_metrics(&dash);
        let output = render_metrics();
        // Output should contain our metric names.
        assert!(output.contains("x3_vault_utilization_pct"));
        assert!(output.contains("x3_frozen_lane_count"));
        assert!(output.contains("x3_last_processed_block"));
    }

    #[test]
    fn gather_returns_metric_families() {
        let dash = make_dashboard();
        update_metrics(&dash);
        let families = gather_metrics();
        // At least our 10 families should be present.
        assert!(!families.is_empty());
    }
}
