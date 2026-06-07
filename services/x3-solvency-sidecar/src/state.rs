//! Shared in-memory dashboard state for the X3 solvency sidecar.
//!
//! All fields are updated by the subscriber task and read concurrently by the
//! metrics and API tasks via the `SharedDashboard` arc-lock.
//!
//! Uses `std::sync::RwLock` (not tokio's) so that lock acquisition in axum
//! handlers does not require an async context and avoids hidden latency spikes.

use std::sync::{Arc, RwLock};

/// Top-level snapshot of the solvency control plane.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct SolvencyDashboard {
    /// Block number at which this snapshot was last refreshed.
    pub last_updated_block: u64,
    /// Per-vault liquidity summary.
    pub vaults: Vec<VaultSummary>,
    /// Cross-chain lane summary.
    pub lanes: Vec<LaneSummary>,
    /// Partner / counterparty summary.
    pub partners: Vec<PartnerSummary>,
    /// Sum of all unsettled notional across every active lane (raw chain units).
    pub global_unsettled_notional: u128,
    /// Number of lanes currently in Frozen status.
    pub frozen_lane_count: u32,
    /// Total vault balance currently at risk of under-collateralization.
    pub treasury_at_risk: u128,
    /// Ring-buffer of recent system alerts; capped at 100 entries.
    pub recent_alerts: Vec<Alert>,
}

/// Liquidity status for a single vault instance.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VaultSummary {
    /// Hex-encoded `[u8; 32]` vault identifier.
    pub vault_id: String,
    /// Chain this vault is deployed on.
    pub chain_id: u32,
    /// Asset held by the vault.
    pub asset_id: u32,
    /// Vault role: `"Gas"`, `"SettlementFloat"`, `"TreasuryReserve"`, or `"InsuranceLoss"`.
    pub vault_type: String,
    /// Balance available for immediate use (raw chain units).
    pub available_balance: u128,
    /// Balance locked as reserve margin (raw chain units).
    pub reserved_balance: u128,
    /// Balance committed to pending outbound settlements (raw chain units).
    pub pending_out_balance: u128,
    /// Minimum balance below which the vault immediately Freezes.
    pub critical_min: u128,
    /// Lower bound of the "healthy" band; violations trigger Degraded status.
    pub min_band: u128,
    /// Operational status: `"Active"`, `"Degraded"`, or `"Frozen"`.
    pub status: String,
    /// `available / (available + reserved + pending_out) * 100.0`,
    /// or `0.0` when the total is zero.
    pub utilization_pct: f64,
}

/// Operational status for a cross-chain settlement lane.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LaneSummary {
    /// Opaque lane identifier string.
    pub lane_id: String,
    /// Origin chain identifier.
    pub source_chain: u32,
    /// Destination chain identifier.
    pub dest_chain: u32,
    /// Traffic class: `"A"` (priority), `"B"` (standard), or `"C"` (best-effort).
    pub lane_class: String,
    /// Operational status: `"Active"`, `"Degraded"`, or `"Frozen"`.
    pub status: String,
    /// Total unconfirmed settlement value in-flight on this lane (raw units).
    pub unsettled_notional: u128,
    /// Maximum in-flight notional permitted before automatic freeze.
    pub exposure_cap: u128,
}

/// Credit and exposure summary for a registered counterparty.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PartnerSummary {
    /// Opaque partner identifier string.
    pub partner_id: String,
    /// Operational status: `"Active"`, `"Degraded"`, or `"Frozen"`.
    pub status: String,
    /// Composite health score in basis points (0–10 000).
    pub health_score_bps: u32,
    /// Current exposure to this partner (raw units).
    pub current_exposure: u128,
    /// Maximum exposure permitted before restriction (raw units).
    pub exposure_limit: u128,
}

/// A solvency alert emitted by the subscriber task.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Alert {
    /// Severity classification.
    pub level: AlertLevel,
    /// Human-readable description of the alert condition.
    pub message: String,
    /// Block number at which the alert was raised.
    pub block: u64,
    /// RFC 3339 timestamp string.
    pub timestamp: String,
}

/// Alert severity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// Thread-safe handle to the shared dashboard.
pub type SharedDashboard = Arc<RwLock<SolvencyDashboard>>;

/// Construct a new, empty dashboard wrapped in the shared-state container.
pub fn new_dashboard() -> SharedDashboard {
    Arc::new(RwLock::new(SolvencyDashboard::default()))
}

/// Append `alert` to the dashboard's ring-buffer, evicting the oldest entry
/// when the buffer would exceed 100 items.
pub fn push_alert(dashboard: &SharedDashboard, alert: Alert) {
    let mut dash = dashboard
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    dash.recent_alerts.push(alert);
    if dash.recent_alerts.len() > 100 {
        dash.recent_alerts.remove(0);
    }
}

/// RFC 3339 timestamp of the current UTC instant (no external crate).
pub fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    epoch_to_rfc3339(secs)
}

/// Convert Unix seconds to a `YYYY-MM-DDTHH:MM:SSZ` string.
pub fn epoch_to_rfc3339(secs: u64) -> String {
    let s = secs % 60;
    let total_min = secs / 60;
    let m = total_min % 60;
    let total_h = total_min / 60;
    let h = total_h % 24;
    let total_days = total_h / 24;
    let (year, month, day) = days_to_ymd(total_days as u32);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

fn days_to_ymd(mut days: u32) -> (u32, u32, u32) {
    let mut year = 1970u32;
    loop {
        let y_days = if is_leap(year) { 366 } else { 365 };
        if days < y_days {
            break;
        }
        days -= y_days;
        year += 1;
    }
    let month_lengths: [u32; 12] = [
        31,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u32;
    for &ml in &month_lengths {
        if days < ml {
            break;
        }
        days -= ml;
        month += 1;
    }
    (year, month.min(12), days + 1)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dashboard_is_zeroed() {
        let dash = SolvencyDashboard::default();
        assert_eq!(dash.last_updated_block, 0);
        assert!(dash.vaults.is_empty());
        assert!(dash.lanes.is_empty());
        assert!(dash.partners.is_empty());
        assert_eq!(dash.global_unsettled_notional, 0);
        assert_eq!(dash.frozen_lane_count, 0);
        assert!(dash.recent_alerts.is_empty());
    }

    #[test]
    fn push_alert_caps_at_100() {
        let dashboard = new_dashboard();
        for i in 0..105u64 {
            push_alert(
                &dashboard,
                Alert {
                    level: AlertLevel::Info,
                    message: format!("alert {i}"),
                    block: i,
                    timestamp: "2026-01-01T00:00:00Z".to_string(),
                },
            );
        }
        let dash = dashboard.read().unwrap();
        assert_eq!(dash.recent_alerts.len(), 100);
        // First 5 entries evicted; block 5 is now the earliest.
        assert_eq!(dash.recent_alerts[0].block, 5);
    }

    #[test]
    fn shared_dashboard_clone_shares_arc() {
        let d1 = new_dashboard();
        let d2 = Arc::clone(&d1);
        {
            let mut w = d1.write().unwrap();
            w.last_updated_block = 999;
        }
        let r = d2.read().unwrap();
        assert_eq!(r.last_updated_block, 999);
    }

    #[test]
    fn epoch_to_rfc3339_known_value() {
        // Unix epoch zero should format to the start of 1970.
        assert_eq!(epoch_to_rfc3339(0), "1970-01-01T00:00:00Z");
        // 1746748800 seconds since epoch = 2025-05-09T00:00:00Z
        assert_eq!(epoch_to_rfc3339(1_746_748_800), "2025-05-09T00:00:00Z");
    }
}
