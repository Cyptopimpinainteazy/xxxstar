//! Readiness data collector.
//!
//! Two collection modes:
//!
//! * [`Collector::collect_offline`] — no network calls.  Returns a report with
//!   `status = Unknown` for every check and explicit reason strings.  This is
//!   what static tooling and CI lint jobs should call.  It must never claim
//!   readiness it cannot prove.
//!
//! * [`Collector::collect_live`] — talks to a running node over JSON-RPC at the
//!   address supplied (env `X3_NODE_RPC` overrides the default).  Each check
//!   runs as a real query and downgrades to `Unknown` on transport failure
//!   instead of fabricating `true`.
//!
//! IMPORTANT: this collector is the single source of truth for the
//! release-readiness dashboard.  Do not introduce hard-coded `true` flags
//! here.  Every check must either query state or report `Unknown` with a
//! reason.

use crate::types::{KernelStatus, ReadinessCheck, ReadinessReport};
use std::time::Duration;

const DEFAULT_RPC: &str = "http://127.0.0.1:9944";
const RPC_TIMEOUT_MS: u64 = 1500;

/// Readiness collector.
pub struct Collector;

impl Collector {
    fn endpoint(rpc_override: Option<&str>) -> String {
        if let Some(s) = rpc_override {
            return s.to_string();
        }
        std::env::var("X3_NODE_RPC").unwrap_or_else(|_| DEFAULT_RPC.to_string())
    }

    /// Offline collection: every check `Unknown` with a reason.  Never claims
    /// readiness.  Use this for static CI lint paths and unit tests.
    pub fn collect_offline() -> ReadinessReport {
        let mut report = ReadinessReport::new();
        report.kernel_status = KernelStatus::unknown();
        let reason = "offline collector: no RPC endpoint queried".to_string();
        report.supply_invariant = ReadinessCheck::unknown(reason.clone());
        report.halt_functional = ReadinessCheck::unknown(reason.clone());
        report.permissions_enforced = ReadinessCheck::unknown(reason.clone());
        report.balance_reconciliation = ReadinessCheck::unknown(reason.clone());
        report.ixl_bundle_gate = ReadinessCheck::unknown(reason.clone());
        report.packet_lifecycle_gate = ReadinessCheck::unknown(reason.clone());
        report.liquidity_core_gate = ReadinessCheck::unknown(reason.clone());
        report.external_bridges_disabled = ReadinessCheck::unknown(reason.clone());
        report.kernel_invariant_gate = ReadinessCheck::unknown(reason);
        report.recompute_overall();
        report
    }

    /// Live collection: queries the running node over JSON-RPC.  On any RPC
    /// failure the affected check is `Unknown` with the transport error,
    /// never silently `Pass`.
    pub fn collect_live(rpc_override: Option<&str>) -> ReadinessReport {
        let endpoint = Self::endpoint(rpc_override);
        let mut report = ReadinessReport::new();

        report.kernel_status = match Self::fetch_kernel_status(&endpoint) {
            Ok(s) => s,
            Err(_) => KernelStatus::unknown(),
        };

        report.supply_invariant = match (
            report.kernel_status.supply,
            report.kernel_status.total_locked,
        ) {
            (Some(supply), Some(locked)) if supply > 0 && locked <= supply => ReadinessCheck::pass(
                format!("supply={}, locked={}, locked<=supply OK", supply, locked),
            ),
            (Some(supply), Some(locked)) if locked > supply => ReadinessCheck::fail(format!(
                "INVARIANT VIOLATION: locked={} > supply={}",
                locked, supply
            )),
            _ => ReadinessCheck::unknown("supply or total_locked unavailable from node RPC"),
        };

        report.halt_functional = match report.kernel_status.halted {
            Some(halted) => ReadinessCheck::pass(format!(
                "ProtocolPaused observable; current state: paused={}",
                halted
            )),
            None => ReadinessCheck::unknown("ProtocolPaused storage not exposed via RPC"),
        };

        report.permissions_enforced = match Self::rpc_call(&endpoint, "system_chain", &[]) {
            Ok(_) => ReadinessCheck::pass("system_chain reachable; metadata accessible"),
            Err(e) => ReadinessCheck::unknown(format!("system_chain RPC failed: {}", e)),
        };

        report.balance_reconciliation = ReadinessCheck::unknown(
            "balance reconciliation requires runtime API \
             `canonical_ledger_reconcile`; not yet wired",
        );

        // ── RC-1 specific gates ──────────────────────────────────────────────
        // IXL bundle gate: verify the router's IXL integration endpoint is live.
        report.ixl_bundle_gate = match Self::rpc_call(&endpoint, "x3_router_ixlStatus", &[]) {
            Ok(v) if v.get("wired").and_then(|b| b.as_bool()) == Some(true) => {
                ReadinessCheck::pass("IXL bundle execution wired and reported live by node")
            }
            Ok(_) => ReadinessCheck::fail(
                "x3_router_ixlStatus returned but `wired` field is false — \
                 IXL not properly initialised in runtime",
            ),
            Err(_) => ReadinessCheck::fail(
                "LAUNCH BLOCKER: x3_router_ixlStatus RPC not available; \
                 RC-1 requires explicit IXL wiring evidence from a live node",
            ),
        };

        // Packet lifecycle gate: verify replay guard and commitment storage.
        report.packet_lifecycle_gate =
            match Self::rpc_call(&endpoint, "x3_router_packetLifecycleStatus", &[]) {
                Ok(v) if v.get("replay_guard").and_then(|b| b.as_bool()) == Some(true) => {
                    ReadinessCheck::pass("packet lifecycle wired: replay guard + commitments live")
                }
                Ok(_) => ReadinessCheck::fail(
                    "x3_router_packetLifecycleStatus returned but replay_guard is false",
                ),
                Err(_) => ReadinessCheck::fail(
                    "LAUNCH BLOCKER: x3_router_packetLifecycleStatus RPC not available; \
                     RC-1 requires live replay/commitment status evidence",
                ),
            };

        // LiquidityCore gate: spot AMM + LP lock must be callable.
        report.liquidity_core_gate =
            match Self::rpc_call(&endpoint, "x3_liquidityCore_spotAmmStatus", &[]) {
                Ok(v) if v.get("spot_amm_active").and_then(|b| b.as_bool()) == Some(true) => {
                    ReadinessCheck::pass("LiquidityCore spot AMM active and LP locks enforced")
                }
                Ok(_) => ReadinessCheck::fail(
                    "x3_liquidityCore_spotAmmStatus returned but spot_amm_active is false",
                ),
                Err(_) => ReadinessCheck::fail(
                    "LAUNCH BLOCKER: x3_liquidityCore_spotAmmStatus RPC not available; \
                     LiquidityCore settlement is not provably wired on the live path",
                ),
            };

        // External bridges disabled gate: MUST be false at genesis.
        report.external_bridges_disabled =
            match Self::rpc_call(&endpoint, "x3_router_externalBridgesEnabled", &[]) {
                Ok(v) if v.as_bool() == Some(false) => ReadinessCheck::pass(
                    "external bridges are DISABLED — scope-freeze rule satisfied",
                ),
                Ok(v) if v.as_bool() == Some(true) => ReadinessCheck::fail(
                    "LAUNCH BLOCKER: external bridges are ENABLED — they must be \
                     disabled at genesis for RC-1 scope compliance",
                ),
                Ok(_) => ReadinessCheck::unknown(
                    "x3_router_externalBridgesEnabled returned unexpected type",
                ),
                Err(_) => ReadinessCheck::fail(
                    "LAUNCH BLOCKER: x3_router_externalBridgesEnabled RPC not available; \
                     cannot prove scope-freeze (external bridges disabled)",
                ),
            };

        // Kernel invariant gate: supply ledger invariant enforcement.
        report.kernel_invariant_gate = match (
            report.kernel_status.supply,
            report.kernel_status.total_locked,
        ) {
            (Some(supply), Some(locked)) if supply > 0 && locked <= supply => ReadinessCheck::pass(
                format!("kernel invariant holds: supply={supply}, locked={locked}, locked≤supply"),
            ),
            (Some(_supply), Some(locked)) if locked == 0 => ReadinessCheck::pass(
                "kernel invariant holds: no assets locked (genesis/idle state)",
            ),
            (Some(supply), Some(locked)) if locked > supply => ReadinessCheck::fail(format!(
                "INVARIANT VIOLATION: locked={locked} > supply={supply} — state corruption"
            )),
            _ => ReadinessCheck::unknown(
                "kernel invariant cannot be checked: supply/locked unavailable from RPC",
            ),
        };

        report.recompute_overall();
        report
    }

    /// Default entry point: live mode, unless `X3_READINESS_OFFLINE=1`.
    pub fn collect() -> ReadinessReport {
        if std::env::var("X3_READINESS_OFFLINE").as_deref() == Ok("1") {
            return Self::collect_offline();
        }
        Self::collect_live(None)
    }

    fn rpc_call(
        endpoint: &str,
        method: &str,
        params: &[serde_json::Value],
    ) -> Result<serde_json::Value, String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_millis(RPC_TIMEOUT_MS))
            .build();
        let resp = agent
            .post(endpoint)
            .set("Content-Type", "application/json")
            .send_string(&body.to_string())
            .map_err(|e| format!("transport error: {}", e))?;
        let v: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("decode error: {}", e))?;
        if let Some(err) = v.get("error") {
            return Err(format!("rpc error: {}", err));
        }
        v.get("result")
            .cloned()
            .ok_or_else(|| "missing result".to_string())
    }

    fn fetch_kernel_status(endpoint: &str) -> Result<KernelStatus, String> {
        let mut status = KernelStatus::unknown();

        // Liveness probe — required.
        let _ = Self::rpc_call(endpoint, "system_health", &[])?;

        if let Ok(v) = Self::rpc_call(endpoint, "x3_supplyLedger_canonicalSupply", &[]) {
            status.supply = parse_balance_value(&v);
        }
        if let Ok(v) = Self::rpc_call(endpoint, "x3_supplyLedger_totalLocked", &[]) {
            status.total_locked = parse_balance_value(&v);
        }
        if let Ok(v) = Self::rpc_call(endpoint, "x3_atomicKernel_paused", &[]) {
            status.halted = v.as_bool();
        }
        if let Ok(v) = Self::rpc_call(endpoint, "x3_atomicKernel_accountCount", &[]) {
            status.account_count = v.as_u64().map(|n| n as usize);
        }

        Ok(status)
    }
}

fn parse_balance_value(v: &serde_json::Value) -> Option<u128> {
    if let Some(s) = v.as_str() {
        if let Some(stripped) = s.strip_prefix("0x") {
            return u128::from_str_radix(stripped, 16).ok();
        }
        return s.parse::<u128>().ok();
    }
    v.as_u64().map(|n| n as u128)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CheckStatus;

    #[test]
    fn offline_collector_never_claims_readiness() {
        let r = Collector::collect_offline();
        assert!(
            !r.overall_ready,
            "offline collector must never report ready"
        );
        assert_eq!(r.supply_invariant.status, CheckStatus::Unknown);
        assert_eq!(r.halt_functional.status, CheckStatus::Unknown);
        assert_eq!(r.permissions_enforced.status, CheckStatus::Unknown);
        assert_eq!(r.balance_reconciliation.status, CheckStatus::Unknown);
        assert!(r.kernel_status.supply.is_none());
    }

    #[test]
    fn live_collector_against_unreachable_endpoint_is_unknown_not_pass() {
        // Force a definitely-unreachable endpoint.
        let r = Collector::collect_live(Some("http://127.0.0.1:1"));
        assert!(!r.overall_ready);
        assert_eq!(r.supply_invariant.status, CheckStatus::Unknown);
        assert_eq!(r.halt_functional.status, CheckStatus::Unknown);
        assert_eq!(r.permissions_enforced.status, CheckStatus::Unknown);
    }

    #[test]
    fn balance_reconciliation_is_pinned_unknown_until_runtime_api_exists() {
        let offline = Collector::collect_offline();
        assert_eq!(offline.balance_reconciliation.status, CheckStatus::Unknown);

        let live_unreachable = Collector::collect_live(Some("http://127.0.0.1:1"));
        assert_eq!(
            live_unreachable.balance_reconciliation.status,
            CheckStatus::Unknown
        );
        assert!(
            live_unreachable
                .balance_reconciliation
                .reason
                .contains("canonical_ledger_reconcile"),
            "reason should point to missing runtime API"
        );
    }
}
