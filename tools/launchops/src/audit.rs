//! Orchestration helpers for the audit command.

use anyhow::Result;
use std::path::Path;

use crate::conflict::detect_conflicts;
use crate::drift::classify;
use crate::gitdiff::diff_against;
use crate::models::{AuditConfig, AuditReport, DriftReport, FeatureMatrixItem, Requirement};
use crate::risk_rules::{evaluate, tally, DriftRulesInput};
use crate::scanner::collect_by_globs;
use crate::scoring::audit_multiplier_from;
use crate::stale_docs::detect as detect_stale;
use crate::test_weaken::detect as detect_weaken;

pub struct AuditOutput {
    pub report: AuditReport,
    pub multiplier: f64,
}

pub fn run_audit(
    root: &Path,
    cfg: &AuditConfig,
    requirements: &[Requirement],
    features: &[FeatureMatrixItem],
) -> Result<AuditOutput> {
    let diff = diff_against(root, &cfg.baseline_branch)?;
    let changed = classify(&cfg.paths, &diff.changed_files)?;

    let existing_tests: Vec<String> = collect_by_globs(root, &cfg.paths.tests)?
        .into_iter()
        .filter_map(|p| {
            p.strip_prefix(root)
                .ok()
                .map(|r| r.to_string_lossy().into_owned())
        })
        .collect();

    let mut flags = evaluate(&DriftRulesInput {
        root,
        changed: &changed,
        diff_text: &diff.diff_text,
        existing_tests: &existing_tests,
    });
    if let Some(weak) = detect_weaken(&diff.diff_text) {
        flags.push(weak);
    }
    flags.sort_by(|a, b| a.flag_type.cmp(&b.flag_type));

    let conflicts = detect_conflicts(requirements);
    let stale = detect_stale(root, features, cfg.stale_doc_days);
    let t = tally(&flags);
    let mult = audit_multiplier_from(
        t.critical,
        t.high,
        t.medium,
        t.low,
        t.test_weakening,
        t.consensus_without_invariants,
        t.bridge_without_replay_tests,
        t.mainnet_config_without_review,
    );
    let blocked = t.critical > 0
        || (cfg.fail_on_test_weakening && t.test_weakening)
        || (cfg.fail_on_consensus_without_invariants && t.consensus_without_invariants)
        || (cfg.fail_on_bridge_without_replay_tests && t.bridge_without_replay_tests)
        || (cfg.fail_on_mainnet_config_without_review && t.mainnet_config_without_review);

    let drift_report = DriftReport {
        baseline_branch: diff.baseline.clone(),
        changed_files: changed,
        drift_flags: flags.clone(),
    };
    let report = AuditReport {
        overall_status: if blocked {
            "BLOCKED".into()
        } else {
            "NOT_BLOCKED".into()
        },
        drift_report,
        deep_red_flags: flags,
        requirement_conflicts: conflicts,
        stale_docs: stale,
        audit_multiplier: mult,
    };
    Ok(AuditOutput {
        report,
        multiplier: mult,
    })
}
