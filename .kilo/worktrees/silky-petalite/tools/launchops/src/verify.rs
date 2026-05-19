//! Verify command — runs red-flag scanner over production vs. test code
//! and distinguishes severity by critical path.

use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};

use crate::models::{AuditPathConfig, RedFlag, RedFlagSeverity};
use crate::scanner::collect_by_globs;
use crate::scoring::RedFlagCounts;

struct Pattern {
    name: &'static str,
    re: Regex,
    prod_severity: RedFlagSeverity,
    test_severity: Option<RedFlagSeverity>, // None = not flagged in tests
    reason: &'static str,
}

fn build_patterns() -> Vec<Pattern> {
    vec![
        Pattern {
            name: "unimplemented!",
            re: Regex::new(r"\bunimplemented!\s*\(").unwrap(),
            prod_severity: RedFlagSeverity::Critical,
            test_severity: Some(RedFlagSeverity::High),
            reason: "Unimplemented! in production code path",
        },
        Pattern {
            name: "todo!",
            re: Regex::new(r"\btodo!\s*\(").unwrap(),
            prod_severity: RedFlagSeverity::Critical,
            test_severity: Some(RedFlagSeverity::Medium),
            reason: "todo!() placeholder in code path",
        },
        Pattern {
            name: "panic!",
            re: Regex::new(r"\bpanic!\s*\(").unwrap(),
            prod_severity: RedFlagSeverity::High,
            test_severity: None,
            reason: "panic! in production code",
        },
        Pattern {
            name: "unwrap()",
            re: Regex::new(r"\.unwrap\s*\(\s*\)").unwrap(),
            prod_severity: RedFlagSeverity::High,
            test_severity: None,
            reason: "Unsafe unwrap in production code",
        },
        Pattern {
            name: "expect()",
            re: Regex::new(r"\.expect\s*\(").unwrap(),
            prod_severity: RedFlagSeverity::Medium,
            test_severity: None,
            reason: ".expect() in production code",
        },
        Pattern {
            name: "#[ignore]",
            re: Regex::new(r"#\[ignore").unwrap(),
            prod_severity: RedFlagSeverity::Medium,
            test_severity: Some(RedFlagSeverity::High),
            reason: "Ignored test",
        },
        Pattern {
            name: "TODO",
            re: Regex::new(r"//\s*TODO\b|//\s*FIXME\b").unwrap(),
            prod_severity: RedFlagSeverity::Low,
            test_severity: Some(RedFlagSeverity::Low),
            reason: "TODO/FIXME comment",
        },
        Pattern {
            name: "stub",
            re: Regex::new(r"(?i)\b(stub|placeholder|mock_impl|hardcoded)\b").unwrap(),
            prod_severity: RedFlagSeverity::Medium,
            test_severity: None,
            reason: "Stub/placeholder marker in production code",
        },
    ]
}

fn is_critical_path(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    [
        "consensus",
        "finality",
        "bridge",
        "cross-vm",
        "cross_vm",
        "atomic",
        "universal-asset",
        "universal_asset",
        "asset-kernel",
        "asset_kernel",
        "dex",
        "gpu-validator",
        "gpu_validator",
        "runtime/",
    ]
    .iter()
    .any(|n| p.contains(n))
}

fn upgrade(sev: RedFlagSeverity) -> RedFlagSeverity {
    match sev {
        RedFlagSeverity::Low => RedFlagSeverity::Medium,
        RedFlagSeverity::Medium => RedFlagSeverity::High,
        RedFlagSeverity::High => RedFlagSeverity::Critical,
        RedFlagSeverity::Critical => RedFlagSeverity::Critical,
    }
}

pub fn scan_red_flags(root: &Path, paths: &AuditPathConfig) -> Result<Vec<RedFlag>> {
    let prod: Vec<PathBuf> = collect_by_globs(root, &paths.production_code)?;
    let tests: Vec<PathBuf> = collect_by_globs(root, &paths.tests)?;
    let patterns = build_patterns();

    let mut flags: Vec<RedFlag> = Vec::new();

    for (files, is_test) in [(&prod, false), (&tests, true)] {
        for f in files {
            let content = match std::fs::read_to_string(f) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let rel = match f.strip_prefix(root) {
                Ok(p) => p.to_string_lossy().into_owned(),
                Err(_) => f.to_string_lossy().into_owned(),
            };
            let critical = is_critical_path(&rel);
            for (lineno, line) in content.lines().enumerate() {
                // Skip commented-out lines for big patterns (keep TODO handled by its own regex)
                for p in &patterns {
                    if !p.re.is_match(line) {
                        continue;
                    }
                    let base = if is_test {
                        match &p.test_severity {
                            Some(s) => s.clone(),
                            None => continue,
                        }
                    } else {
                        p.prod_severity.clone()
                    };
                    let sev = if critical && !is_test {
                        upgrade(base)
                    } else {
                        base
                    };
                    flags.push(RedFlag {
                        severity: sev,
                        file: rel.clone(),
                        line: lineno + 1,
                        pattern: p.name.to_string(),
                        reason: p.reason.to_string(),
                    });
                }
            }
        }
    }

    // Deterministic order
    flags.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.pattern.cmp(&b.pattern))
    });
    // Cap to 500 so output stays manageable
    flags.truncate(500);
    Ok(flags)
}

pub fn count_red_flags(flags: &[RedFlag]) -> RedFlagCounts {
    let mut c = RedFlagCounts::default();
    for f in flags {
        match f.severity {
            RedFlagSeverity::Low => c.low += 1,
            RedFlagSeverity::Medium => c.medium += 1,
            RedFlagSeverity::High => c.high += 1,
            RedFlagSeverity::Critical => c.critical += 1,
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_path_detection() {
        assert!(is_critical_path("crates/x3-bridge/src/lib.rs"));
        assert!(is_critical_path("runtime/src/lib.rs"));
        assert!(!is_critical_path("crates/x3-wallet/src/ui.rs"));
    }

    #[test]
    fn upgrade_chain() {
        assert_eq!(upgrade(RedFlagSeverity::High), RedFlagSeverity::Critical);
        assert_eq!(upgrade(RedFlagSeverity::Low), RedFlagSeverity::Medium);
    }
}
