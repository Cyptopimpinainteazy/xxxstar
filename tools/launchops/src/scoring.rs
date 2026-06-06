//! Weighted readiness scoring — combines per-requirement scores, risk
//! multipliers, module weights, and downstream verify/audit multipliers.

use std::collections::BTreeMap;

use crate::evidence::{base_status_score, module_key, risk_multiplier};
use crate::models::{
    FeatureMatrixItem, Module, ReadinessMultipliers, ReadinessOutput, ScanStats, WeightsConfig,
};

fn module_weight(w: &WeightsConfig, key: &str) -> u32 {
    match key {
        "consensus" => w.consensus,
        "cross_vm" => w.cross_vm,
        "bridge" => w.bridge,
        "universal_asset_kernel" => w.universal_asset_kernel,
        "dex" => w.dex,
        "security" => w.security,
        "wallet_explorer" => w.wallet_explorer,
        "ops" => w.ops,
        "docs" => w.docs,
        _ => 1,
    }
}

pub struct ScoringOutput {
    pub scan_score: f64,
    pub module_breakdown: BTreeMap<String, f64>,
}

/// Compute per-module scores (0..100) and overall weighted scan score.
pub fn compute_scan_score(items: &[FeatureMatrixItem], weights: &WeightsConfig) -> ScoringOutput {
    let mut module_totals: BTreeMap<String, (f64, f64)> = BTreeMap::new(); // key -> (weighted_sum, count)

    for it in items {
        let base = base_status_score(&it.status) as f64;
        let rm = risk_multiplier(&it.risk);
        let score = base * rm;
        let key = module_key(&it.module).to_string();
        let entry = module_totals.entry(key).or_insert((0.0, 0.0));
        entry.0 += score;
        entry.1 += 1.0;
    }

    let mut module_breakdown: BTreeMap<String, f64> = BTreeMap::new();
    for (k, (sum, n)) in &module_totals {
        let avg = if *n > 0.0 { sum / n } else { 0.0 };
        module_breakdown.insert(k.clone(), (avg * 100.0).round() / 100.0);
    }

    // Weighted overall
    let mut total_weight = 0u32;
    let mut weighted_sum = 0.0f64;
    for (k, v) in &module_breakdown {
        let w = module_weight(weights, k);
        if w == 0 {
            continue;
        }
        weighted_sum += v * w as f64;
        total_weight += w;
    }
    let overall = if total_weight == 0 {
        0.0
    } else {
        weighted_sum / total_weight as f64
    };

    ScoringOutput {
        scan_score: (overall * 100.0).round() / 100.0,
        module_breakdown,
    }
}

pub fn build_readiness(
    scan_score: f64,
    module_breakdown: BTreeMap<String, f64>,
    stats: ScanStats,
    multipliers: ReadinessMultipliers,
    blocker_count: usize,
) -> ReadinessOutput {
    let final_readiness = scan_score
        * multipliers.command_multiplier
        * multipliers.gate_multiplier
        * multipliers.red_flag_multiplier
        * multipliers.audit_multiplier;
    let final_readiness = (final_readiness * 100.0).round() / 100.0;
    let status = if blocker_count == 0 && final_readiness >= 85.0 {
        "NOT_BLOCKED".to_string()
    } else {
        "BLOCKED".to_string()
    };

    ReadinessOutput {
        generated_at: chrono::Utc::now().to_rfc3339(),
        scan_score,
        final_readiness,
        status,
        module_breakdown,
        totals: stats,
        multipliers,
    }
}

/// Classify red flag severity impact into multiplier.
pub fn red_flag_multiplier_from(counts: &RedFlagCounts) -> f64 {
    if counts.critical > 0 {
        0.50
    } else if counts.high > 0 {
        0.75
    } else if counts.medium > 0 {
        0.90
    } else if counts.low > 0 {
        0.97
    } else {
        1.00
    }
}

#[derive(Default, Debug, Clone)]
pub struct RedFlagCounts {
    pub low: usize,
    pub medium: usize,
    pub high: usize,
    pub critical: usize,
}

/// Audit multiplier per spec (lowest applicable).
pub fn audit_multiplier_from(
    critical: usize,
    high: usize,
    medium: usize,
    low: usize,
    test_weakening: bool,
    consensus_without_invariants: bool,
    bridge_without_replay_tests: bool,
    mainnet_config_without_review: bool,
) -> f64 {
    let mut candidates: Vec<f64> = vec![1.0];
    if mainnet_config_without_review {
        candidates.push(0.20);
    }
    if test_weakening {
        candidates.push(0.25);
    }
    if consensus_without_invariants {
        candidates.push(0.25);
    }
    if bridge_without_replay_tests {
        candidates.push(0.25);
    }
    if critical > 0 {
        candidates.push(0.35);
    }
    if high > 0 {
        candidates.push(0.65);
    }
    if medium > 0 {
        candidates.push(0.85);
    }
    if low > 0 {
        candidates.push(0.95);
    }
    candidates.into_iter().fold(1.0f64, f64::min)
}

/// Command multiplier from pass/fail counts.
pub fn command_multiplier_from(passed: usize, failed: usize, ran: usize) -> f64 {
    if ran == 0 {
        return 0.60;
    }
    if failed > 0 && passed == 0 {
        return 0.50;
    }
    if failed > 0 {
        return 0.70;
    }
    1.0
}

/// Gate multiplier from required-fail count.
pub fn gate_multiplier_from(required_failed: usize, required_total: usize) -> f64 {
    if required_total == 0 {
        return 1.0;
    }
    if required_failed == 0 {
        1.0
    } else if required_failed as f64 / required_total as f64 >= 0.5 {
        0.55
    } else {
        0.70
    }
}

/// Expose for audit report labels.
pub fn module_display(m: &Module) -> &'static str {
    match m {
        Module::Consensus => "Consensus",
        Module::CrossVm => "Cross-VM",
        Module::Bridge => "Bridge",
        Module::UniversalAssetKernel => "Universal Asset Kernel",
        Module::Dex => "DEX",
        Module::GpuValidator => "GPU Validator",
        Module::WalletExplorer => "Wallet/Explorer",
        Module::LaunchOps => "LaunchOps",
        Module::Security => "Security",
        Module::Ops => "Ops",
        Module::Docs => "Docs",
        Module::Unknown => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_multiplier_picks_lowest() {
        let m = audit_multiplier_from(1, 1, 1, 1, true, false, true, false);
        assert!((m - 0.25).abs() < 1e-9, "got {m}");
    }

    #[test]
    fn audit_multiplier_none() {
        let m = audit_multiplier_from(0, 0, 0, 0, false, false, false, false);
        assert!((m - 1.0).abs() < 1e-9);
    }

    #[test]
    fn command_multiplier_fail_mix() {
        assert!((command_multiplier_from(2, 1, 3) - 0.70).abs() < 1e-9);
    }
}
