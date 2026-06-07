//! Machine-readable report types for the X3 Phase 1 proving harness.
//!
//! All types implement `serde::Serialize` and `serde::Deserialize` so they can
//! be round-tripped through the JSON compatibility scorecard unchanged.

// ─────────────────────────────────────────────────────────────────────────────
// Core result types
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a single named RPC check against one chain endpoint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckResult {
    /// Short name of the check (e.g. `"quoting"`, `"bundle_construction"`).
    pub check: String,
    /// Chain identifier this result belongs to (e.g. `"x3-native"`).
    pub chain_id: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Numeric score: `1.0` if passed, `0.0` if failed.
    pub score: f64,
    /// Wall-clock round-trip time for the RPC probe in milliseconds.
    pub latency_ms: u64,
    /// Human-readable error message when `passed == false`, otherwise `None`.
    pub error: Option<String>,
    /// RFC-3339 UTC timestamp at which this result was recorded.
    pub timestamp: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Chain-level aggregation
// ─────────────────────────────────────────────────────────────────────────────

/// Summary status of a single chain after all six checks have been evaluated
/// against the operator-configured thresholds.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ChainStatus {
    /// All three threshold metrics were met.
    Passing,
    /// At least one check passed but at least one threshold was not met.
    Degraded,
    /// Every single check failed — node is unreachable or chain unsupported.
    Unsupported,
}

/// Per-chain aggregated results and derived metrics.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChainReport {
    pub chain_id: String,
    /// Raw results for the six individual checks, in probe order.
    pub checks: Vec<CheckResult>,
    /// Score of the `bundle_construction` check (1.0 or 0.0).
    pub bundle_success_rate: f64,
    /// Score of the `rollback` check (1.0 or 0.0).
    pub rollback_correctness: f64,
    /// Score of the `reconciliation` check (1.0 or 0.0).
    pub reconciliation_accuracy: f64,
    /// Computed status after comparing metrics against `ThresholdConfig`.
    pub overall_status: ChainStatus,
}

// ─────────────────────────────────────────────────────────────────────────────
// Configuration and summary
// ─────────────────────────────────────────────────────────────────────────────

/// Operator-configurable pass/fail thresholds.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ThresholdConfig {
    /// Minimum `bundle_success_rate` to consider a chain Passing.
    pub bundle_success: f64,
    /// Minimum `rollback_correctness` to consider a chain Passing.
    pub rollback: f64,
    /// Minimum `reconciliation_accuracy` to consider a chain Passing.
    pub reconciliation: f64,
}

/// Cross-chain summary counts included at the top of the scorecard.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HarnessSummary {
    pub total_chains: usize,
    pub passing: usize,
    pub degraded: usize,
    pub unsupported: usize,
    /// `true` iff every chain is `Passing` and at least one chain was probed.
    pub overall_pass: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Root scorecard document
// ─────────────────────────────────────────────────────────────────────────────

/// Root machine-readable compatibility scorecard emitted to `--output`.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CompatibilityMatrix {
    /// Unique identifier for this run (e.g. `"x3-prove-1746789600000"`).
    pub run_id: String,
    /// RFC-3339 UTC timestamp at which this scorecard was generated.
    pub generated_at: String,
    /// Threshold values that were active during this run.
    pub thresholds: ThresholdConfig,
    /// One entry per probed chain, in the order chains were specified.
    pub chains: Vec<ChainReport>,
    /// Cross-chain summary counts and overall pass/fail decision.
    pub summary: HarnessSummary,
}
