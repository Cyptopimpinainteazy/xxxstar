//! Type definitions for readiness reports.
//!
//! Every check is tri-state (`Pass`, `Fail`, `Unknown`).  `Unknown` means the
//! collector could not prove the check; it must never be silently treated as
//! `Pass`.  The overall ready flag requires *all* checks to be `Pass`.

use serde::{Deserialize, Serialize};

/// Tri-state check status.  Never collapse `Unknown` into `Pass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Fail,
    Unknown,
}

impl CheckStatus {
    pub fn is_pass(self) -> bool {
        matches!(self, CheckStatus::Pass)
    }
}

/// A single readiness check with status + a human-readable reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessCheck {
    pub status: CheckStatus,
    pub reason: String,
}

impl ReadinessCheck {
    pub fn pass(reason: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Pass,
            reason: reason.into(),
        }
    }
    pub fn fail(reason: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Fail,
            reason: reason.into(),
        }
    }
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Unknown,
            reason: reason.into(),
        }
    }
    pub fn is_pass(&self) -> bool {
        self.status.is_pass()
    }
}

impl Default for ReadinessCheck {
    fn default() -> Self {
        Self::unknown("not yet evaluated")
    }
}

/// Snapshot of kernel state at report time.  All fields `Option<_>` because
/// the collector may be unable to prove them; never substitute defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KernelStatus {
    pub supply: Option<u128>,
    pub account_count: Option<usize>,
    pub halted: Option<bool>,
    pub total_locked: Option<u128>,
}

impl KernelStatus {
    pub fn unknown() -> Self {
        Self::default()
    }
}

/// Comprehensive readiness report for X3 Atomic Star v0.4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReport {
    pub timestamp: String,
    pub version: String,
    pub kernel_status: KernelStatus,
    pub supply_invariant: ReadinessCheck,
    pub halt_functional: ReadinessCheck,
    pub permissions_enforced: ReadinessCheck,
    pub balance_reconciliation: ReadinessCheck,
    // ── RC-1 specific gates ──────────────────────────────────────────────
    /// IXL bundle execution path: planner + interpreter + rollback wired.
    pub ixl_bundle_gate: ReadinessCheck,
    /// Packet lifecycle: replay guard, commitment, timeout wired in router.
    pub packet_lifecycle_gate: ReadinessCheck,
    /// LiquidityCore settlement: spot AMM swap callback wired to IXL.
    pub liquidity_core_gate: ReadinessCheck,
    /// External bridges: must be DISABLED at genesis (scope-freeze rule).
    pub external_bridges_disabled: ReadinessCheck,
    /// Kernel invariant: supply ledger invariant check wired after every bundle.
    pub kernel_invariant_gate: ReadinessCheck,
    /// Computed; only `true` when every check is `Pass`.
    pub overall_ready: bool,
}

impl Default for ReadinessReport {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadinessReport {
    pub fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "0.4.0".to_string(),
            kernel_status: KernelStatus::unknown(),
            supply_invariant: ReadinessCheck::default(),
            halt_functional: ReadinessCheck::default(),
            permissions_enforced: ReadinessCheck::default(),
            balance_reconciliation: ReadinessCheck::default(),
            ixl_bundle_gate: ReadinessCheck::default(),
            packet_lifecycle_gate: ReadinessCheck::default(),
            liquidity_core_gate: ReadinessCheck::default(),
            external_bridges_disabled: ReadinessCheck::default(),
            kernel_invariant_gate: ReadinessCheck::default(),
            overall_ready: false,
        }
    }

    /// Recompute the overall flag.  `true` only when *all* checks are `Pass`.
    pub fn recompute_overall(&mut self) {
        self.overall_ready = self.supply_invariant.is_pass()
            && self.halt_functional.is_pass()
            && self.permissions_enforced.is_pass()
            && self.balance_reconciliation.is_pass()
            && self.ixl_bundle_gate.is_pass()
            && self.packet_lifecycle_gate.is_pass()
            && self.liquidity_core_gate.is_pass()
            && self.external_bridges_disabled.is_pass()
            && self.kernel_invariant_gate.is_pass();
    }

    pub fn is_ready(&self) -> bool {
        self.overall_ready
    }

    pub fn readiness_percentage(&self) -> u32 {
        let checks = [
            self.supply_invariant.is_pass(),
            self.halt_functional.is_pass(),
            self.permissions_enforced.is_pass(),
            self.balance_reconciliation.is_pass(),
            self.ixl_bundle_gate.is_pass(),
            self.packet_lifecycle_gate.is_pass(),
            self.liquidity_core_gate.is_pass(),
            self.external_bridges_disabled.is_pass(),
            self.kernel_invariant_gate.is_pass(),
        ];
        let passed = checks.iter().filter(|&&x| x).count() as u32;
        (passed * 100) / checks.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_report_is_not_ready() {
        let r = ReadinessReport::new();
        assert!(!r.is_ready());
        assert_eq!(r.readiness_percentage(), 0);
    }

    #[test]
    fn unknown_blocks_ready() {
        let mut r = ReadinessReport::new();
        r.supply_invariant = ReadinessCheck::pass("ok");
        r.halt_functional = ReadinessCheck::pass("ok");
        r.permissions_enforced = ReadinessCheck::pass("ok");
        r.balance_reconciliation = ReadinessCheck::unknown("not yet wired");
        r.recompute_overall();
        assert!(!r.is_ready());
    }

    #[test]
    fn all_pass_is_ready() {
        let mut r = ReadinessReport::new();
        r.supply_invariant = ReadinessCheck::pass("ok");
        r.halt_functional = ReadinessCheck::pass("ok");
        r.permissions_enforced = ReadinessCheck::pass("ok");
        r.balance_reconciliation = ReadinessCheck::pass("ok");
        r.recompute_overall();
        assert!(r.is_ready());
        assert_eq!(r.readiness_percentage(), 100);
    }
}
