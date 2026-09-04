//! Evidence mapper — searches production code and tests for keywords
//! derived from a requirement, then derives a FeatureStatus.

use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::models::{AuditPathConfig, FeatureStatus, Module, Requirement, RequirementStatus};
use crate::scanner::collect_by_globs;

const STOP_WORDS: &[&str] = &[
    "the",
    "a",
    "an",
    "and",
    "or",
    "but",
    "for",
    "to",
    "of",
    "in",
    "on",
    "with",
    "without",
    "by",
    "at",
    "is",
    "are",
    "be",
    "been",
    "have",
    "has",
    "should",
    "must",
    "will",
    "needs",
    "need",
    "add",
    "remove",
    "implement",
    "implementation",
    "support",
    "supports",
    "this",
    "that",
    "these",
    "those",
    "from",
    "into",
    "it",
    "its",
    "all",
    "any",
    "some",
    "each",
    "every",
    "new",
    "ensure",
    "allow",
    "provide",
    "handle",
    "update",
    "make",
    "build",
];

fn tokenize(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() || c == '_' {
            cur.push(c.to_ascii_lowercase());
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out.into_iter()
        .filter(|t| t.len() >= 4 && !STOP_WORDS.contains(&t.as_str()))
        .collect()
}

/// Extract significant keywords from a requirement.
pub fn keywords_from(req: &Requirement) -> Vec<String> {
    let mut set: HashSet<String> = HashSet::new();
    for t in tokenize(&req.text) {
        set.insert(t);
    }
    let mut v: Vec<String> = set.into_iter().collect();
    v.sort();
    v
}

/// Case-insensitive substring search of any keyword in the file contents.
fn file_matches(path: &Path, keywords: &[String]) -> bool {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return false,
    };
    let content = String::from_utf8_lossy(&data).to_ascii_lowercase();
    keywords.iter().any(|k| content.contains(k))
}

pub struct EvidenceResult {
    pub code_evidence: Vec<String>,
    pub test_evidence: Vec<String>,
}

pub fn find_evidence(
    root: &Path,
    req: &Requirement,
    paths: &AuditPathConfig,
    code_files_cache: &[PathBuf],
    test_files_cache: &[PathBuf],
) -> Result<EvidenceResult> {
    let _ = paths;
    let keywords = keywords_from(req);
    // Cap keyword count to avoid runaway greps — top 8 alphabetical slice.
    let keywords: Vec<String> = keywords.into_iter().take(12).collect();

    if keywords.is_empty() {
        return Ok(EvidenceResult {
            code_evidence: vec![],
            test_evidence: vec![],
        });
    }

    let mut code: Vec<String> = Vec::new();
    let mut tests: Vec<String> = Vec::new();

    for p in code_files_cache {
        if file_matches(p, &keywords) {
            if let Ok(rel) = p.strip_prefix(root) {
                code.push(rel.to_string_lossy().into_owned());
            }
        }
        if code.len() >= 10 {
            break;
        }
    }
    for p in test_files_cache {
        if file_matches(p, &keywords) {
            if let Ok(rel) = p.strip_prefix(root) {
                tests.push(rel.to_string_lossy().into_owned());
            }
        }
        if tests.len() >= 10 {
            break;
        }
    }

    code.sort();
    tests.sort();
    Ok(EvidenceResult {
        code_evidence: code,
        test_evidence: tests,
    })
}

/// Pre-collect the code/test file lists so we don't rewalk the tree per requirement.
pub fn build_caches(root: &Path, paths: &AuditPathConfig) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let code = collect_by_globs(root, &paths.production_code)?;
    let tests = collect_by_globs(root, &paths.tests)?;
    Ok((code, tests))
}

pub fn derive_feature_status(req: &Requirement, ev: &EvidenceResult) -> FeatureStatus {
    if matches!(req.status, RequirementStatus::Blocker) {
        return FeatureStatus::Blocked;
    }
    let has_code = !ev.code_evidence.is_empty();
    let has_tests = !ev.test_evidence.is_empty();
    match req.status {
        RequirementStatus::Complete => {
            if has_code && has_tests {
                FeatureStatus::Tested
            } else if has_code {
                FeatureStatus::Implemented
            } else {
                FeatureStatus::SpecifiedOnly
            }
        }
        RequirementStatus::Partial => FeatureStatus::Partial,
        RequirementStatus::NeedsTest => {
            if has_code {
                FeatureStatus::Implemented
            } else {
                FeatureStatus::SpecifiedOnly
            }
        }
        RequirementStatus::Incomplete => {
            if has_code && has_tests {
                FeatureStatus::Tested
            } else if has_code {
                FeatureStatus::Implemented
            } else {
                FeatureStatus::Missing
            }
        }
        RequirementStatus::Blocker => FeatureStatus::Blocked,
    }
}

/// A score derived purely from FeatureStatus (pre-risk multiplier).
pub fn base_status_score(status: &FeatureStatus) -> u32 {
    match status {
        FeatureStatus::Missing => 0,
        FeatureStatus::SpecifiedOnly => 20,
        FeatureStatus::Partial => 40,
        FeatureStatus::Implemented => 60,
        FeatureStatus::Tested => 80,
        FeatureStatus::Verified => 100,
        FeatureStatus::Blocked => 0,
    }
}

pub fn risk_multiplier(risk: &crate::models::RiskLevel) -> f64 {
    match risk {
        crate::models::RiskLevel::Low => 1.00,
        crate::models::RiskLevel::Medium => 0.90,
        crate::models::RiskLevel::High => 0.70,
        crate::models::RiskLevel::Critical => 0.45,
    }
}

pub fn module_key(m: &Module) -> &'static str {
    match m {
        Module::Consensus => "consensus",
        Module::CrossVm => "cross_vm",
        Module::Bridge => "bridge",
        Module::UniversalAssetKernel => "universal_asset_kernel",
        Module::Dex => "dex",
        Module::GpuValidator => "gpu_validator",
        Module::WalletExplorer => "wallet_explorer",
        Module::LaunchOps => "launch_ops",
        Module::Security => "security",
        Module::Ops => "ops",
        Module::Docs => "docs",
        Module::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Module, RequirementStatus, RiskLevel};

    fn req(text: &str, status: RequirementStatus) -> Requirement {
        Requirement {
            id: "x".into(),
            text: text.into(),
            source_file: "a.md".into(),
            line: 1,
            status,
            tags: vec![],
            module: Module::Bridge,
            risk: RiskLevel::High,
        }
    }

    #[test]
    fn keywords_skip_stop_words() {
        let r = req("Add bridge replay protection", RequirementStatus::Complete);
        let k = keywords_from(&r);
        assert!(k.contains(&"bridge".into()));
        assert!(k.contains(&"replay".into()));
        assert!(k.contains(&"protection".into()));
        assert!(!k.contains(&"add".into()));
    }

    #[test]
    fn status_when_code_and_tests() {
        let r = req("replay nonce", RequirementStatus::Complete);
        let ev = EvidenceResult {
            code_evidence: vec!["a.rs".into()],
            test_evidence: vec!["b.rs".into()],
        };
        assert_eq!(derive_feature_status(&r, &ev), FeatureStatus::Tested);
    }

    #[test]
    fn status_blocker_wins() {
        let r = req("x", RequirementStatus::Blocker);
        let ev = EvidenceResult {
            code_evidence: vec!["a".into()],
            test_evidence: vec!["b".into()],
        };
        assert_eq!(derive_feature_status(&r, &ev), FeatureStatus::Blocked);
    }
}
