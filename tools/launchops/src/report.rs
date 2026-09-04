//! Deterministic Markdown/JSON report renderers.

use std::fmt::Write;

use crate::models::{
    AuditReport, BlockerItem, DriftFlag, DriftSeverity, FeatureMatrixItem, GateResult, GateStatus,
    ReadinessOutput, RedFlag, RedFlagSeverity, VerifyReport,
};
use crate::scoring::module_display;

pub fn scan_report_md(
    readiness: &ReadinessOutput,
    blockers: &[BlockerItem],
    features: &[FeatureMatrixItem],
) -> String {
    let mut s = String::new();
    writeln!(s, "# X3 LaunchOps Scan Report").ok();
    writeln!(s).ok();
    writeln!(s, "- Generated at: {}", readiness.generated_at).ok();
    writeln!(s, "- Scan score: {:.2}%", readiness.scan_score).ok();
    writeln!(s, "- Final readiness: {:.2}%", readiness.final_readiness).ok();
    writeln!(s, "- Status: **{}**", readiness.status).ok();
    writeln!(s).ok();

    writeln!(s, "## Module Breakdown").ok();
    writeln!(s).ok();
    writeln!(s, "| Module | Score |").ok();
    writeln!(s, "|---|---:|").ok();
    for (k, v) in &readiness.module_breakdown {
        writeln!(s, "| {} | {:.2}% |", k, v).ok();
    }
    writeln!(s).ok();

    writeln!(s, "## Totals").ok();
    writeln!(s).ok();
    writeln!(
        s,
        "- Markdown files scanned: {}",
        readiness.totals.total_md_files
    )
    .ok();
    writeln!(s, "- Requirements: {}", readiness.totals.total_requirements).ok();
    writeln!(s, "- Complete: {}", readiness.totals.complete_items).ok();
    writeln!(s, "- Partial: {}", readiness.totals.partial_items).ok();
    writeln!(s, "- Blocked: {}", readiness.totals.blocked_items).ok();
    writeln!(s, "- Needs tests: {}", readiness.totals.needs_test_items).ok();
    writeln!(s).ok();

    writeln!(s, "## Top Blockers").ok();
    writeln!(s).ok();
    if blockers.is_empty() {
        writeln!(s, "_No blockers detected._").ok();
    } else {
        for b in blockers.iter().take(20) {
            writeln!(
                s,
                "- **{:?}** [{}] `{}:{}` — {}",
                b.severity,
                module_display(&b.module),
                b.source_file,
                b.line,
                b.reason
            )
            .ok();
        }
    }
    writeln!(s).ok();

    writeln!(s, "## Feature Matrix (top 30 by risk)").ok();
    writeln!(s).ok();
    writeln!(
        s,
        "| Feature | Module | Status | Risk | Score | Code | Tests |"
    )
    .ok();
    writeln!(s, "|---|---|---|---|---:|---:|---:|").ok();
    let mut sorted: Vec<&FeatureMatrixItem> = features.iter().collect();
    sorted.sort_by(|a, b| {
        risk_order(&a.risk)
            .cmp(&risk_order(&b.risk))
            .reverse()
            .then(a.feature.cmp(&b.feature))
    });
    for it in sorted.iter().take(30) {
        writeln!(
            s,
            "| {} | {} | {:?} | {:?} | {} | {} | {} |",
            it.feature,
            module_display(&it.module),
            it.status,
            it.risk,
            it.score,
            it.code_evidence.len(),
            it.test_evidence.len(),
        )
        .ok();
    }
    s
}

fn risk_order(r: &crate::models::RiskLevel) -> u8 {
    match r {
        crate::models::RiskLevel::Low => 0,
        crate::models::RiskLevel::Medium => 1,
        crate::models::RiskLevel::High => 2,
        crate::models::RiskLevel::Critical => 3,
    }
}

pub fn verify_report_md(v: &VerifyReport) -> String {
    let mut s = String::new();
    writeln!(s, "# X3 LaunchOps Verify Report").ok();
    writeln!(s).ok();
    writeln!(s, "## Overall Status").ok();
    writeln!(s).ok();
    writeln!(s, "**{}**", v.overall_status).ok();
    writeln!(s).ok();

    writeln!(s, "## Command Results").ok();
    writeln!(s).ok();
    writeln!(s, "| Command | Status | Exit | Duration |").ok();
    writeln!(s, "|---|---|---:|---:|").ok();
    for c in &v.command_results {
        writeln!(
            s,
            "| `{}` | {:?} | {} | {} ms |",
            c.command,
            c.status,
            c.exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".into()),
            c.duration_ms
        )
        .ok();
    }
    writeln!(s).ok();

    writeln!(s, "## Gates").ok();
    writeln!(s).ok();
    writeln!(s, "| Gate | Status | Required | Source | Reason |").ok();
    writeln!(s, "|---|---|:-:|---|---|").ok();
    for g in &v.gates {
        writeln!(
            s,
            "| {} | {} | {} | {} | {} |",
            g.name,
            gate_emoji(&g.status),
            if g.required { "yes" } else { "no" },
            g.source,
            g.reason.clone().unwrap_or_else(|| "-".into())
        )
        .ok();
    }
    writeln!(s).ok();

    writeln!(s, "## Top Red Flags").ok();
    writeln!(s).ok();
    if v.red_flags.is_empty() {
        writeln!(s, "_No red flags detected._").ok();
    } else {
        let mut sorted: Vec<&RedFlag> = v.red_flags.iter().collect();
        sorted.sort_by(|a, b| {
            red_order(&a.severity)
                .cmp(&red_order(&b.severity))
                .reverse()
        });
        for r in sorted.iter().take(30) {
            writeln!(
                s,
                "- **{:?}** `{}:{}` [{}] — {}",
                r.severity, r.file, r.line, r.pattern, r.reason
            )
            .ok();
        }
    }
    writeln!(s).ok();

    writeln!(s, "## Multipliers").ok();
    writeln!(s).ok();
    writeln!(s, "- command: {:.2}", v.command_multiplier).ok();
    writeln!(s, "- gate: {:.2}", v.gate_multiplier).ok();
    writeln!(s, "- red_flag: {:.2}", v.red_flag_multiplier).ok();
    s
}

fn gate_emoji(g: &GateStatus) -> &'static str {
    match g {
        GateStatus::Pass => "PASS",
        GateStatus::Fail => "FAIL",
        GateStatus::Warn => "WARN",
        GateStatus::Skipped => "SKIP",
    }
}

fn red_order(r: &RedFlagSeverity) -> u8 {
    match r {
        RedFlagSeverity::Low => 0,
        RedFlagSeverity::Medium => 1,
        RedFlagSeverity::High => 2,
        RedFlagSeverity::Critical => 3,
    }
}

fn drift_order(r: &DriftSeverity) -> u8 {
    match r {
        DriftSeverity::Low => 0,
        DriftSeverity::Medium => 1,
        DriftSeverity::High => 2,
        DriftSeverity::Critical => 3,
    }
}

pub fn audit_report_md(a: &AuditReport) -> String {
    let mut s = String::new();
    writeln!(s, "# X3 LaunchOps Audit Report").ok();
    writeln!(s).ok();
    writeln!(s, "## Overall Audit Status").ok();
    writeln!(s).ok();
    writeln!(s, "**{}**", a.overall_status).ok();
    writeln!(s, "- audit_multiplier: {:.2}", a.audit_multiplier).ok();
    writeln!(s).ok();

    writeln!(s, "## Changed File Summary").ok();
    writeln!(s).ok();
    writeln!(s, "| Category | Count |").ok();
    writeln!(s, "|---|---:|").ok();
    let cf = &a.drift_report.changed_files;
    writeln!(s, "| docs | {} |", cf.docs.len()).ok();
    writeln!(s, "| code | {} |", cf.code.len()).ok();
    writeln!(s, "| tests | {} |", cf.tests.len()).ok();
    writeln!(s, "| consensus | {} |", cf.consensus.len()).ok();
    writeln!(s, "| bridge | {} |", cf.bridge.len()).ok();
    writeln!(s, "| cross_vm | {} |", cf.cross_vm.len()).ok();
    writeln!(s, "| dex | {} |", cf.dex.len()).ok();
    writeln!(s, "| gpu | {} |", cf.gpu.len()).ok();
    writeln!(s, "| ops | {} |", cf.ops.len()).ok();
    writeln!(s, "| mainnet_config | {} |", cf.mainnet_config.len()).ok();
    writeln!(s).ok();

    writeln!(s, "## Drift Flags").ok();
    writeln!(s).ok();
    if a.drift_report.drift_flags.is_empty() {
        writeln!(s, "_No drift detected._").ok();
    } else {
        let mut sorted: Vec<&DriftFlag> = a.drift_report.drift_flags.iter().collect();
        sorted.sort_by(|x, y| {
            drift_order(&x.severity)
                .cmp(&drift_order(&y.severity))
                .reverse()
        });
        for f in sorted {
            writeln!(s, "### [{:?}] {}", f.severity, f.flag_type).ok();
            writeln!(s).ok();
            writeln!(s, "{}", f.reason).ok();
            writeln!(s).ok();
            if !f.files.is_empty() {
                writeln!(s, "Files:").ok();
                for file in &f.files {
                    writeln!(s, "- `{}`", file).ok();
                }
                writeln!(s).ok();
            }
        }
    }

    writeln!(s, "## Requirement Conflicts").ok();
    writeln!(s).ok();
    if a.requirement_conflicts.is_empty() {
        writeln!(s, "_No conflicts detected._").ok();
    } else {
        for c in &a.requirement_conflicts {
            writeln!(
                s,
                "- **{:?}** {} — `{}:{}` vs `{}:{}`",
                c.severity,
                c.conflict_type,
                c.requirement_a.file,
                c.requirement_a.line,
                c.requirement_b.file,
                c.requirement_b.line
            )
            .ok();
        }
    }
    writeln!(s).ok();

    writeln!(s, "## Stale Docs").ok();
    writeln!(s).ok();
    if a.stale_docs.is_empty() {
        writeln!(s, "_No stale docs detected._").ok();
    } else {
        for d in &a.stale_docs {
            writeln!(s, "- `{}` — {}", d.file, d.reason).ok();
        }
    }
    s
}

pub fn final_report_md(
    readiness: &ReadinessOutput,
    verify: Option<&VerifyReport>,
    audit: Option<&AuditReport>,
    gates: &[GateResult],
) -> String {
    let mut s = String::new();
    writeln!(s, "# X3 LaunchOps Final Report").ok();
    writeln!(s).ok();
    writeln!(s, "- Generated at: {}", readiness.generated_at).ok();
    writeln!(
        s,
        "- **X3 Mainnet Readiness: {:.2}% — Status: {}**",
        readiness.final_readiness, readiness.status
    )
    .ok();
    writeln!(s, "- Scan score: {:.2}%", readiness.scan_score).ok();
    writeln!(
        s,
        "- command_multiplier: {:.2}",
        readiness.multipliers.command_multiplier
    )
    .ok();
    writeln!(
        s,
        "- gate_multiplier: {:.2}",
        readiness.multipliers.gate_multiplier
    )
    .ok();
    writeln!(
        s,
        "- red_flag_multiplier: {:.2}",
        readiness.multipliers.red_flag_multiplier
    )
    .ok();
    writeln!(
        s,
        "- audit_multiplier: {:.2}",
        readiness.multipliers.audit_multiplier
    )
    .ok();
    writeln!(s).ok();

    writeln!(s, "## Failed Required Gates").ok();
    writeln!(s).ok();
    let failed: Vec<&GateResult> = gates
        .iter()
        .filter(|g| g.required && matches!(g.status, GateStatus::Fail))
        .collect();
    if failed.is_empty() {
        writeln!(s, "_None._").ok();
    } else {
        for g in failed {
            writeln!(
                s,
                "- **{}** — {}",
                g.name,
                g.reason.clone().unwrap_or_default()
            )
            .ok();
        }
    }
    writeln!(s).ok();

    if let Some(v) = verify {
        writeln!(s, "## Verify Summary").ok();
        writeln!(s).ok();
        writeln!(s, "- Status: {}", v.overall_status).ok();
        writeln!(s, "- Commands ran: {}", v.command_results.len()).ok();
        writeln!(s, "- Red flags: {}", v.red_flags.len()).ok();
        writeln!(s).ok();
    }
    if let Some(a) = audit {
        writeln!(s, "## Audit Summary").ok();
        writeln!(s).ok();
        writeln!(s, "- Status: {}", a.overall_status).ok();
        writeln!(s, "- Drift flags: {}", a.drift_report.drift_flags.len()).ok();
        writeln!(
            s,
            "- Requirement conflicts: {}",
            a.requirement_conflicts.len()
        )
        .ok();
        writeln!(s, "- Stale docs: {}", a.stale_docs.len()).ok();
    }
    s
}
