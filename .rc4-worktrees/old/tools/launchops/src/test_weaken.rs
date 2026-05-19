//! Test weakening detector — scans a unified diff for removed strong
//! assertions and added weakness markers.

use regex::Regex;

use crate::models::{DriftFlag, DriftSeverity};

pub fn detect(diff_text: &str) -> Option<DriftFlag> {
    let removed_assert = Regex::new(
        r"^-\s*(assert!|assert_eq!|assert_ne!|prop_assert!|prop_assert_eq!|proptest!|quickcheck!)",
    )
    .unwrap();
    let added_weakness = Regex::new(
        r"^\+\s*(#\[ignore\]|return Ok\(\(\)\);?|assert!\(true\)|todo!\(\)|unimplemented!\(\))",
    )
    .unwrap();
    let self_compare_assert = Regex::new(
        r"^\+\s*assert_eq!\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*,\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)",
    )
    .unwrap();

    let mut files: Vec<String> = Vec::new();
    let mut current_file: Option<String> = None;
    let mut hit = false;

    for line in diff_text.lines() {
        if let Some(stripped) = line.strip_prefix("+++ b/") {
            current_file = Some(stripped.to_string());
            continue;
        }
        if let Some(stripped) = line.strip_prefix("+++ ") {
            current_file = Some(stripped.to_string());
            continue;
        }
        if removed_assert.is_match(line) || added_weakness.is_match(line) {
            hit = true;
            if let Some(f) = &current_file {
                if !files.contains(f) {
                    files.push(f.clone());
                }
            }
        } else if let Some(caps) = self_compare_assert.captures(line) {
            if caps.get(1).map(|m| m.as_str()) == caps.get(2).map(|m| m.as_str()) {
                hit = true;
                if let Some(f) = &current_file {
                    if !files.contains(f) {
                        files.push(f.clone());
                    }
                }
            }
        }
    }

    if !hit {
        return None;
    }
    files.sort();
    Some(DriftFlag {
        severity: DriftSeverity::Critical,
        flag_type: "test_weakened".into(),
        files,
        reason: "Assertions removed or weakness markers added in test diff".into(),
        required_evidence: vec![
            "Restore removed assertions".into(),
            "Remove #[ignore] or replace with invariant/property tests".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_removed_assert() {
        let diff = "\
+++ b/crates/x3-bridge/tests/replay.rs
-    assert_eq!(actual, expected);
+    // replaced
";
        let f = detect(diff).unwrap();
        assert_eq!(f.flag_type, "test_weakened");
        assert!(f.files.iter().any(|x| x.contains("replay.rs")));
    }

    #[test]
    fn detects_ignore_addition() {
        let diff = "\
+++ b/tests/x.rs
+#[ignore]
";
        assert!(detect(diff).is_some());
    }

    #[test]
    fn ignores_clean_diff() {
        let diff = "\
+++ b/x.rs
+let x = 1;
";
        assert!(detect(diff).is_none());
    }
}
