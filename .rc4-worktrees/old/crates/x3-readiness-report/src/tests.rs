//! Integration tests for readiness reporting.
//!
//! These tests verify the *contract* of the readiness pipeline:
//!   * the offline collector never claims readiness;
//!   * Unknown checks block overall_ready;
//!   * formatters round-trip JSON.
//!   * RC-1 specific gates are present and default to Unknown (never Pass).
//!
//! Live RPC tests live in collector.rs as unit tests.

use crate::types::{CheckStatus, ReadinessCheck, ReadinessReport};
use crate::{Collector, JsonFormatter, TextFormatter};

#[test]
fn collector_offline_emits_unknown_only() {
    let r = Collector::collect_offline();
    assert!(!r.overall_ready);
    assert_eq!(r.supply_invariant.status, CheckStatus::Unknown);
    assert_eq!(r.balance_reconciliation.status, CheckStatus::Unknown);
    assert!(r.kernel_status.supply.is_none());
    assert!(r.kernel_status.halted.is_none());
}

#[test]
fn rc1_gates_present_and_default_unknown() {
    let r = ReadinessReport::new();
    assert_eq!(r.ixl_bundle_gate.status, CheckStatus::Unknown);
    assert_eq!(r.packet_lifecycle_gate.status, CheckStatus::Unknown);
    assert_eq!(r.liquidity_core_gate.status, CheckStatus::Unknown);
    assert_eq!(r.external_bridges_disabled.status, CheckStatus::Unknown);
    assert_eq!(r.kernel_invariant_gate.status, CheckStatus::Unknown);
}

#[test]
fn rc1_gates_block_overall_ready() {
    // All original gates pass but RC-1 gates are Unknown — must block.
    let mut r = ReadinessReport::new();
    r.supply_invariant = ReadinessCheck::pass("ok");
    r.halt_functional = ReadinessCheck::pass("ok");
    r.permissions_enforced = ReadinessCheck::pass("ok");
    r.balance_reconciliation = ReadinessCheck::pass("ok");
    // RC-1 gates left Unknown.
    r.recompute_overall();
    assert!(!r.is_ready(), "RC-1 Unknown gates must block overall_ready");
}

#[test]
fn external_bridges_fail_blocks_launch() {
    let mut r = ReadinessReport::new();
    r.supply_invariant = ReadinessCheck::pass("ok");
    r.halt_functional = ReadinessCheck::pass("ok");
    r.permissions_enforced = ReadinessCheck::pass("ok");
    r.balance_reconciliation = ReadinessCheck::pass("ok");
    r.ixl_bundle_gate = ReadinessCheck::pass("ok");
    r.packet_lifecycle_gate = ReadinessCheck::pass("ok");
    r.liquidity_core_gate = ReadinessCheck::pass("ok");
    // LAUNCH BLOCKER: bridges enabled
    r.external_bridges_disabled =
        ReadinessCheck::fail("LAUNCH BLOCKER: external bridges are ENABLED");
    r.kernel_invariant_gate = ReadinessCheck::pass("ok");
    r.recompute_overall();
    assert!(!r.is_ready());
}

#[test]
fn all_rc1_pass_means_ready() {
    let mut r = ReadinessReport::new();
    r.supply_invariant = ReadinessCheck::pass("ok");
    r.halt_functional = ReadinessCheck::pass("ok");
    r.permissions_enforced = ReadinessCheck::pass("ok");
    r.balance_reconciliation = ReadinessCheck::pass("ok");
    r.ixl_bundle_gate = ReadinessCheck::pass("IXL wired");
    r.packet_lifecycle_gate = ReadinessCheck::pass("replay guard live");
    r.liquidity_core_gate = ReadinessCheck::pass("spot AMM active");
    r.external_bridges_disabled = ReadinessCheck::pass("bridges disabled");
    r.kernel_invariant_gate = ReadinessCheck::pass("invariant holds");
    r.recompute_overall();
    assert!(r.is_ready(), "all gates pass → must be ready");
    assert_eq!(r.readiness_percentage(), 100);
}

#[test]
fn unknown_blocks_ready_even_if_others_pass() {
    let mut r = ReadinessReport::new();
    r.supply_invariant = ReadinessCheck::pass("ok");
    r.halt_functional = ReadinessCheck::pass("ok");
    r.permissions_enforced = ReadinessCheck::pass("ok");
    r.balance_reconciliation = ReadinessCheck::unknown("not wired");
    r.recompute_overall();
    assert!(!r.is_ready());
}

#[test]
fn fail_blocks_ready() {
    let mut r = ReadinessReport::new();
    r.supply_invariant = ReadinessCheck::fail("locked > supply");
    r.halt_functional = ReadinessCheck::pass("ok");
    r.permissions_enforced = ReadinessCheck::pass("ok");
    r.balance_reconciliation = ReadinessCheck::pass("ok");
    r.recompute_overall();
    assert!(!r.is_ready());
}

#[test]
fn json_roundtrip_preserves_check_state() {
    let mut original = ReadinessReport::new();
    original.supply_invariant = ReadinessCheck::pass("supply=10, locked=5");
    original.kernel_status.supply = Some(10);
    original.kernel_status.account_count = Some(42);
    original.ixl_bundle_gate = ReadinessCheck::pass("IXL wired");
    original.recompute_overall();

    let json = JsonFormatter::format(&original);
    let back: ReadinessReport = serde_json::from_str(&json).expect("valid json");
    assert_eq!(back.supply_invariant.status, CheckStatus::Pass);
    assert_eq!(back.supply_invariant.reason, "supply=10, locked=5");
    assert_eq!(back.kernel_status.supply, Some(10));
    assert_eq!(back.kernel_status.account_count, Some(42));
    assert_eq!(back.ixl_bundle_gate.status, CheckStatus::Pass);
}

#[test]
fn text_formatter_marks_unknown_distinctly() {
    let mut r = ReadinessReport::new();
    r.supply_invariant = ReadinessCheck::pass("ok");
    r.halt_functional = ReadinessCheck::fail("bad");
    // permissions + balance left as default (Unknown)
    r.recompute_overall();
    let t = TextFormatter::format(&r);
    assert!(t.contains("PASS"), "must show PASS marker:\n{}", t);
    assert!(t.contains("FAIL"));
    assert!(t.contains("UNK"));
    assert!(t.contains("NOT READY"));
}
