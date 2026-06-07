//! Gate evaluator — turns command outcomes, feature matrix, blocker list,
//! and red-flag list into deterministic pass/fail gate results.

use std::collections::BTreeMap;

use crate::models::{
    BlockerItem, CommandResult, CommandStatus, FeatureMatrixItem, FeatureStatus, GateConfig,
    GateResult, GateStatus, Module, RedFlag, RedFlagSeverity,
};

fn module_matches(m: &Module, key: &str) -> bool {
    matches!(
        (m, key),
        (Module::Bridge, "bridge")
            | (Module::CrossVm, "cross_vm")
            | (Module::Dex, "dex")
            | (Module::Consensus, "consensus")
            | (Module::UniversalAssetKernel, "universal_asset_kernel")
            | (Module::GpuValidator, "gpu_validator")
            | (Module::Security, "security")
            | (Module::Ops, "ops")
            | (Module::WalletExplorer, "wallet_explorer")
            | (Module::Docs, "docs")
            | (Module::LaunchOps, "launch_ops")
    )
}

fn feature_has_keywords(
    items: &[FeatureMatrixItem],
    module_key: &str,
    keywords: &[String],
) -> bool {
    let kw_lower: Vec<String> = keywords.iter().map(|k| k.to_ascii_lowercase()).collect();
    items.iter().any(|it| {
        if !module_matches(&it.module, module_key) {
            return false;
        }
        if !matches!(
            it.status,
            FeatureStatus::Tested | FeatureStatus::Verified | FeatureStatus::Implemented
        ) {
            return false;
        }
        let hay: Vec<String> = it
            .test_evidence
            .iter()
            .chain(it.code_evidence.iter())
            .map(|p| p.to_ascii_lowercase())
            .collect();
        kw_lower.iter().any(|kw| hay.iter().any(|p| p.contains(kw)))
    })
}

pub fn evaluate_gates(
    gates: &BTreeMap<String, GateConfig>,
    commands: &[CommandResult],
    features: &[FeatureMatrixItem],
    blockers: &[BlockerItem],
    red_flags: &[RedFlag],
) -> Vec<GateResult> {
    let mut out = Vec::new();
    for (id, cfg) in gates {
        let (status, reason, source) = if let Some(cmd_name) = &cfg.command {
            let source = format!("command:{cmd_name}");
            let found = commands.iter().find(|c| &c.name == cmd_name);
            match found {
                Some(c) => match c.status {
                    CommandStatus::Passed => (GateStatus::Pass, None, source),
                    CommandStatus::Failed => (
                        GateStatus::Fail,
                        Some(format!("{} exited with {:?}", c.command, c.exit_code)),
                        source,
                    ),
                    CommandStatus::MissingTool => (
                        GateStatus::Skipped,
                        Some(format!("tool missing for `{cmd_name}`")),
                        source,
                    ),
                    CommandStatus::Skipped => (
                        GateStatus::Skipped,
                        Some(format!("command `{cmd_name}` skipped")),
                        source,
                    ),
                },
                None => (
                    GateStatus::Skipped,
                    Some(format!("command `{cmd_name}` not configured")),
                    source,
                ),
            }
        } else {
            match cfg.source.as_deref() {
                Some("blockers") => {
                    let p0_count = blockers
                        .iter()
                        .filter(|b| matches!(b.severity, crate::models::DriftSeverity::Critical))
                        .count();
                    if p0_count == 0 {
                        (GateStatus::Pass, None, "source:blockers".into())
                    } else {
                        (
                            GateStatus::Fail,
                            Some(format!("{p0_count} P0 blockers present")),
                            "source:blockers".into(),
                        )
                    }
                }
                Some("red_flags") => {
                    let critical = red_flags
                        .iter()
                        .filter(|r| matches!(r.severity, RedFlagSeverity::Critical))
                        .count();
                    if critical == 0 {
                        (GateStatus::Pass, None, "source:red_flags".into())
                    } else {
                        (
                            GateStatus::Fail,
                            Some(format!("{critical} critical red flags in production code")),
                            "source:red_flags".into(),
                        )
                    }
                }
                Some("feature_matrix") => {
                    let module_key = cfg.module.clone().unwrap_or_default();
                    let ok = feature_has_keywords(features, &module_key, &cfg.keywords);
                    if ok {
                        (
                            GateStatus::Pass,
                            None,
                            format!("source:feature_matrix:{module_key}"),
                        )
                    } else {
                        (
                            GateStatus::Fail,
                            Some(format!(
                                "No evidence for keywords {:?} in module `{module_key}`",
                                cfg.keywords
                            )),
                            format!("source:feature_matrix:{module_key}"),
                        )
                    }
                }
                _ => (
                    GateStatus::Skipped,
                    Some("gate has no source or command".to_string()),
                    "source:unknown".into(),
                ),
            }
        };
        out.push(GateResult {
            id: id.clone(),
            name: humanize(id),
            status,
            required: cfg.required,
            weight: cfg.weight,
            source,
            reason,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn humanize(id: &str) -> String {
    id.replace('_', " ")
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
