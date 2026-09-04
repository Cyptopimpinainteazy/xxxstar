//! Requirement conflict / duplicate detector.
//!
//! Strategy: normalize checklist text (lowercase, strip stop words), compute
//! Jaccard similarity over token sets, flag near-duplicates; then cross-check
//! statuses for direct contradictions.

use std::collections::HashSet;

use crate::evidence::module_key;
use crate::models::{
    DriftSeverity, Module, Requirement, RequirementConflict, RequirementRef, RequirementStatus,
};

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
    "implement",
    "support",
    "from",
    "into",
    "it",
    "its",
    "all",
    "any",
    "some",
    "new",
    "ensure",
    "allow",
    "provide",
    "handle",
    "update",
    "make",
    "build",
    "this",
    "that",
];

fn tokens(text: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut cur = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() || c == '_' {
            cur.push(c.to_ascii_lowercase());
        } else if !cur.is_empty() {
            out.insert(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.insert(cur);
    }
    out.retain(|t| t.len() >= 4 && !STOP_WORDS.contains(&t.as_str()));
    out
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let inter: usize = a.intersection(b).count();
    let union: usize = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

fn status_str(s: &RequirementStatus) -> &'static str {
    match s {
        RequirementStatus::Complete => "complete",
        RequirementStatus::Incomplete => "incomplete",
        RequirementStatus::Partial => "partial",
        RequirementStatus::Blocker => "blocker",
        RequirementStatus::NeedsTest => "needs_test",
    }
}

fn mk_ref(r: &Requirement) -> RequirementRef {
    RequirementRef {
        file: r.source_file.clone(),
        line: r.line,
        text: r.text.clone(),
        status: status_str(&r.status).to_string(),
        module: module_key(&r.module).to_string(),
    }
}

fn conflicting(a: &RequirementStatus, b: &RequirementStatus) -> bool {
    matches!(
        (a, b),
        (RequirementStatus::Complete, RequirementStatus::Incomplete)
            | (RequirementStatus::Incomplete, RequirementStatus::Complete)
            | (RequirementStatus::Complete, RequirementStatus::Blocker)
            | (RequirementStatus::Blocker, RequirementStatus::Complete)
    )
}

#[allow(dead_code)]
fn same_module(a: &Module, b: &Module) -> bool {
    a == b
}

pub fn detect_conflicts(reqs: &[Requirement]) -> Vec<RequirementConflict> {
    // Bucket by module to avoid O(n^2) across the whole corpus.
    let mut buckets: std::collections::HashMap<Module, Vec<usize>> =
        std::collections::HashMap::new();
    for (idx, r) in reqs.iter().enumerate() {
        buckets.entry(r.module).or_default().push(idx);
    }

    let token_sets: Vec<HashSet<String>> = reqs.iter().map(|r| tokens(&r.text)).collect();
    let mut out: Vec<RequirementConflict> = Vec::new();
    const MAX_CONFLICTS: usize = 2000;
    const MAX_BUCKET: usize = 4000;

    for (_, indices) in buckets.iter() {
        // If a bucket is extremely large, cap pairwise comparison window to keep runtime bounded.
        let slice: &[usize] = if indices.len() > MAX_BUCKET {
            &indices[..MAX_BUCKET]
        } else {
            &indices[..]
        };
        for (ii, &i) in slice.iter().enumerate() {
            if out.len() >= MAX_CONFLICTS {
                break;
            }
            for &j in &slice[ii + 1..] {
                // Only emit actual status conflicts — duplicates are too noisy
                // in doc corpora of this scale.
                if !conflicting(&reqs[i].status, &reqs[j].status) {
                    continue;
                }
                let sim = jaccard(&token_sets[i], &token_sets[j]);
                if sim < 0.55 {
                    continue;
                }
                out.push(RequirementConflict {
                    severity: DriftSeverity::High,
                    conflict_type: "conflicting_requirement".to_string(),
                    requirement_a: mk_ref(&reqs[i]),
                    requirement_b: mk_ref(&reqs[j]),
                    reason: format!(
                        "Requirements describe the same feature (sim={:.2}) with opposing statuses",
                        sim
                    ),
                });
                if out.len() >= MAX_CONFLICTS {
                    break;
                }
            }
        }
        if out.len() >= MAX_CONFLICTS {
            break;
        }
    }

    out.sort_by(|a, b| {
        a.requirement_a
            .file
            .cmp(&b.requirement_a.file)
            .then(a.requirement_a.line.cmp(&b.requirement_a.line))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Module, RiskLevel};

    fn r(text: &str, status: RequirementStatus, module: Module) -> Requirement {
        Requirement {
            id: "x".into(),
            text: text.into(),
            source_file: "a.md".into(),
            line: 1,
            status,
            tags: vec![],
            module,
            risk: RiskLevel::High,
        }
    }

    #[test]
    fn detects_near_duplicate() {
        let a = r(
            "bridge replay protection implemented",
            RequirementStatus::Complete,
            Module::Bridge,
        );
        let b = r(
            "implement replay protection for bridge messages",
            RequirementStatus::Incomplete,
            Module::Bridge,
        );
        let out = detect_conflicts(&[a, b]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].conflict_type, "conflicting_requirement");
    }
}
