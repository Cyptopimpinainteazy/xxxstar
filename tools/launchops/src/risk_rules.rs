//! Drift risk rules — translate a ChangedFileSet + diff text into DriftFlags.

use crate::models::{ChangedFileSet, DriftFlag, DriftSeverity};

const CONSENSUS_KEYWORDS: &[&str] = &[
    "invariant",
    "property",
    "proptest",
    "quickcheck",
    "finality",
    "fork",
    "reorg",
    "deterministic",
    "state_transition",
];

const BRIDGE_KEYWORDS: &[&str] = &[
    "replay",
    "nonce",
    "duplicate",
    "proof",
    "malformed",
    "withdrawal",
    "invalid_signature",
    "expired",
    "finality",
];

const MAINNET_REVIEW_MARKERS: &[&str] = &["MAINNET_REVIEW", "LAUNCH_REVIEW", "GENESIS_REVIEW"];

fn any_test_mentions(
    changed_tests: &[String],
    existing_tests: &[String],
    keywords: &[&str],
) -> bool {
    // If any changed or existing test path contains any keyword, we count it as evidence.
    let paths = changed_tests.iter().chain(existing_tests.iter());
    for p in paths {
        let lp = p.to_ascii_lowercase();
        if keywords.iter().any(|k| lp.contains(k)) {
            return true;
        }
    }
    false
}

fn any_test_content_mentions(
    root: &std::path::Path,
    changed_tests: &[String],
    keywords: &[&str],
) -> bool {
    for rel in changed_tests {
        let p = root.join(rel);
        if let Ok(c) = std::fs::read_to_string(&p) {
            let lc = c.to_ascii_lowercase();
            if keywords.iter().any(|k| lc.contains(k)) {
                return true;
            }
        }
    }
    false
}

pub struct DriftRulesInput<'a> {
    pub root: &'a std::path::Path,
    pub changed: &'a ChangedFileSet,
    pub diff_text: &'a str,
    pub existing_tests: &'a [String],
}

pub fn evaluate(input: &DriftRulesInput<'_>) -> Vec<DriftFlag> {
    let mut out = Vec::new();

    // Rule 1: docs changed but code did not
    if !input.changed.docs.is_empty() && input.changed.code.is_empty() {
        let has_critical_tag = input.changed.docs.iter().any(|f| {
            std::fs::read_to_string(input.root.join(f))
                .map(|c| {
                    let c = c.to_ascii_uppercase();
                    c.contains("[MAINNET]")
                        || c.contains("[CONSENSUS]")
                        || c.contains("[BRIDGE]")
                        || c.contains("[CROSS_VM]")
                        || c.contains("[DEX]")
                        || c.contains("[GPU]")
                })
                .unwrap_or(false)
        });
        out.push(DriftFlag {
            severity: if has_critical_tag {
                DriftSeverity::High
            } else {
                DriftSeverity::Medium
            },
            flag_type: "docs_without_code".into(),
            files: input.changed.docs.clone(),
            reason: "Markdown requirements changed without linked implementation changes".into(),
            required_evidence: vec!["Update production code or downgrade doc claims".into()],
        });
    }

    // Rule 2: code changed but tests did not
    if !input.changed.code.is_empty() && input.changed.tests.is_empty() {
        let touches_critical = !input.changed.consensus.is_empty()
            || !input.changed.bridge.is_empty()
            || !input.changed.cross_vm.is_empty()
            || !input.changed.mainnet_config.is_empty();
        out.push(DriftFlag {
            severity: if touches_critical {
                DriftSeverity::Critical
            } else {
                DriftSeverity::High
            },
            flag_type: "code_without_tests".into(),
            files: input.changed.code.clone(),
            reason: "Production code changed without corresponding tests".into(),
            required_evidence: vec!["Add/extend tests covering changed code".into()],
        });
    }

    // Rule 3: consensus changed without invariants
    if !input.changed.consensus.is_empty() {
        let in_changed =
            any_test_content_mentions(input.root, &input.changed.tests, CONSENSUS_KEYWORDS)
                || any_test_mentions(&input.changed.tests, &[], CONSENSUS_KEYWORDS);
        let in_existing = any_test_mentions(&[], input.existing_tests, CONSENSUS_KEYWORDS);
        if !(in_changed || in_existing) {
            out.push(DriftFlag {
                severity: DriftSeverity::Critical,
                flag_type: "consensus_without_invariants".into(),
                files: input.changed.consensus.clone(),
                reason: "Consensus-critical files changed without invariant/property test updates"
                    .into(),
                required_evidence: CONSENSUS_KEYWORDS
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            });
        }
    }

    // Rule 4: bridge changed without replay tests
    if !input.changed.bridge.is_empty() {
        let in_changed =
            any_test_content_mentions(input.root, &input.changed.tests, BRIDGE_KEYWORDS)
                || any_test_mentions(&input.changed.tests, &[], BRIDGE_KEYWORDS);
        let in_existing = any_test_mentions(&[], input.existing_tests, BRIDGE_KEYWORDS);
        if !(in_changed || in_existing) {
            out.push(DriftFlag {
                severity: DriftSeverity::Critical,
                flag_type: "bridge_without_replay_tests".into(),
                files: input.changed.bridge.clone(),
                reason:
                    "Bridge/cross-chain files changed without replay, nonce, proof, or malformed-message tests"
                        .into(),
                required_evidence: BRIDGE_KEYWORDS.iter().map(|s| (*s).to_string()).collect(),
            });
        }
    }

    // Rule 5: mainnet config changed without review
    if !input.changed.mainnet_config.is_empty() {
        let review_file_paths = [
            ".launchops/mainnet-review.md",
            "docs/mainnet-launch-review.md",
            "docs/launch-gates.md",
        ];
        let review_file_present = review_file_paths
            .iter()
            .any(|p| input.root.join(p).exists());
        let marker_in_diff = MAINNET_REVIEW_MARKERS
            .iter()
            .any(|m| input.diff_text.contains(m));
        if !(review_file_present || marker_in_diff) {
            out.push(DriftFlag {
                severity: DriftSeverity::Critical,
                flag_type: "mainnet_config_without_review".into(),
                files: input.changed.mainnet_config.clone(),
                reason: "Mainnet/testnet config changed without launch review evidence".into(),
                required_evidence: vec![
                    ".launchops/mainnet-review.md".into(),
                    "docs/mainnet-launch-review.md".into(),
                    "docs/launch-gates.md".into(),
                    "MAINNET_REVIEW marker".into(),
                ],
            });
        }
    }

    // Deterministic
    out.sort_by(|a, b| a.flag_type.cmp(&b.flag_type));
    out
}

pub struct SeverityTally {
    pub low: usize,
    pub medium: usize,
    pub high: usize,
    pub critical: usize,
    pub test_weakening: bool,
    pub consensus_without_invariants: bool,
    pub bridge_without_replay_tests: bool,
    pub mainnet_config_without_review: bool,
}

pub fn tally(flags: &[DriftFlag]) -> SeverityTally {
    let mut t = SeverityTally {
        low: 0,
        medium: 0,
        high: 0,
        critical: 0,
        test_weakening: false,
        consensus_without_invariants: false,
        bridge_without_replay_tests: false,
        mainnet_config_without_review: false,
    };
    for f in flags {
        match f.severity {
            DriftSeverity::Low => t.low += 1,
            DriftSeverity::Medium => t.medium += 1,
            DriftSeverity::High => t.high += 1,
            DriftSeverity::Critical => t.critical += 1,
        }
        match f.flag_type.as_str() {
            "test_weakened" => t.test_weakening = true,
            "consensus_without_invariants" => t.consensus_without_invariants = true,
            "bridge_without_replay_tests" => t.bridge_without_replay_tests = true,
            "mainnet_config_without_review" => t.mainnet_config_without_review = true,
            _ => {}
        }
    }
    t
}
