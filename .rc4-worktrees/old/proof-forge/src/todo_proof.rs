// TodoProof Scanner - Detects TODO/FIXME/HACK/stub/mock/fake code
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TodoSeverity {
    T0, // Harmless note
    T1, // Cleanup
    T2, // Test debt
    T3, // Stub debt
    T4, // Mock debt
    T5, // Panic debt
    T6, // Security debt
    T7, // Funds-at-risk debt
    T8, // Launch blocker
    T9, // False completion marker
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub file: PathBuf,
    pub line: usize,
    pub content: String,
    pub severity: TodoSeverity,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoReport {
    pub total_todos: usize,
    pub by_severity: HashMap<String, usize>,
    pub items: Vec<TodoItem>,
    pub mainnet_blockers: Vec<TodoItem>,
    pub testnet_blockers: Vec<TodoItem>,
    pub critical_paths_affected: Vec<String>,
}

pub struct TodoScanner {
    workspace: PathBuf,
    critical_paths: Vec<String>,
    exempt_paths: Vec<String>,
}

impl TodoScanner {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            critical_paths: vec![
                "pallets/x3-kernel".to_string(),
                "pallets/atomic-trade-engine".to_string(),
                "pallets/x3-settlement-engine".to_string(),
                "pallets/x3-atomic-kernel".to_string(),
                "crates/x3-bridge".to_string(),
                "crates/cross-vm-bridge".to_string(),
                "crates/evm-integration".to_string(),
                "crates/svm-integration".to_string(),
                "crates/x3-atomic-trade".to_string(),
                "crates/x3-flashloan".to_string(),
                "crates/x3-dex".to_string(),
                "crates/x3-vm".to_string(),
                "crates/x3-compiler".to_string(),
                "pallets/governance".to_string(),
                "pallets/treasury".to_string(),
                "runtime".to_string(),
                "X3-contracts/evm/contracts".to_string(),
                "X3-contracts/svm/programs".to_string(),
            ],
            exempt_paths: vec![
                "tests/".to_string(),
                "benches/".to_string(),
                "examples/".to_string(),
                "docs/".to_string(),
                "scripts/".to_string(),
                ".md".to_string(),
                "test_".to_string(),
                "_test.rs".to_string(),
                "tests.rs".to_string(),
                "mock_".to_string(),
                "proof-forge/".to_string(),
                "fuzz/".to_string(),
                "fuzz_targets/".to_string(),
            ],
        }
    }

    pub fn scan(&self, verbose: bool) -> Result<TodoReport> {
        let mut items = Vec::new();

        // Keywords to search for
        let keywords = vec![
            "TODO",
            "FIXME",
            "HACK",
            "XXX",
            "stub",
            "mock",
            "placeholder",
            "temporary",
            "for now",
            "later",
            "not implemented",
            "unimplemented!",
            "todo!",
            "panic!",
            "unwrap(",
            "expect(",
            "Ok(true)",
            "Ok(())",
            "Ok(vec![])",
            "Default::default()",
            "fake finality",
            "mock verifier",
            "dummy signature",
            "hardcoded timestamp",
            "hardcoded price",
            "hardcoded admin",
            "FAKE_",
            "MOCK_",
            "TEST_ONLY",
        ];

        for entry in WalkDir::new(&self.workspace)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                // Skip noisy build / vendor / duplicate directories
                if e.file_type().is_dir() {
                    let name = e.file_name().to_string_lossy();
                    match name.as_ref() {
                        "target" | "patches" | "launch-gates" | ".git" | "node_modules" => {
                            return false;
                        }
                        // atlas-sphere-clean is a full duplicate of the pallets tree
                        "atlas-sphere-clean" => return false,
                        _ => {}
                    }
                }
                !self.is_exempt(e.path())
            })
        {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();

                // Only scan code files
                if let Some(ext) = path.extension() {
                    if !["rs", "sol", "ts", "js", "toml"].contains(&ext.to_str().unwrap_or("")) {
                        continue;
                    }
                } else {
                    continue;
                }

                if let Ok(content) = fs::read_to_string(path) {
                    for (line_num, line) in content.lines().enumerate() {
                        for keyword in &keywords {
                            if line.contains(keyword) {
                                let severity = self.classify_severity(line, path);
                                let item = TodoItem {
                                    file: path.to_path_buf(),
                                    line: line_num + 1,
                                    content: line.trim().to_string(),
                                    severity: severity.clone(),
                                    reason: format!("Keyword '{}' found", keyword),
                                };
                                items.push(item);
                                break; // Only report once per line
                            }
                        }
                    }
                }
            }
        }

        if verbose {
            println!("Found {} TODO items", items.len());
        }

        self.generate_report(items)
    }

    fn is_exempt(&self, path: &std::path::Path) -> bool {
        let path_str = path.to_string_lossy();
        self.exempt_paths
            .iter()
            .any(|exempt| path_str.contains(exempt))
    }

    fn is_critical_path(&self, path: &std::path::Path) -> bool {
        let path_str = path.to_string_lossy();
        self.critical_paths
            .iter()
            .any(|critical| path_str.contains(critical))
    }

    fn classify_severity(&self, line: &str, path: &std::path::Path) -> TodoSeverity {
        let lower = line.to_lowercase();
        let in_critical = self.is_critical_path(path);

        // T9: False completion markers
        if lower.contains("fake finality")
            || lower.contains("mock verifier")
            || lower.contains("dummy signature")
        {
            return TodoSeverity::T9;
        }

        // T8: Launch blockers — unimplemented! stubs in critical paths only
        if lower.contains("unimplemented!") && in_critical {
            return TodoSeverity::T8;
        }

        // T7: Funds-at-risk
        if in_critical
            && (lower.contains("vault") || lower.contains("custody") || lower.contains("treasury"))
        {
            return TodoSeverity::T7;
        }

        // T6: Security debt — specific dangerous patterns only (not broad 'security' keyword)
        if lower.contains("hardcoded admin")
            || lower.contains("hardcoded timestamp")
            || lower.contains("hardcoded price")
            || lower.contains("dummy signature")
            || (in_critical && lower.contains("fake finality"))
        {
            return TodoSeverity::T6;
        }

        // T5: Panic debt — only a mainnet blocker in critical consensus/financial paths
        // Skip test-assertion patterns: `other => panic!(...)` and `_ => panic!(...)`
        if lower.contains("panic!") && in_critical {
            let trimmed = line.trim();
            if !trimmed.starts_with("other =>") && !trimmed.starts_with("_ =>") {
                return TodoSeverity::T5;
            }
        }

        // T4: Mock debt in test code, or panic! in non-critical app code
        if lower.contains("mock")
            && (path.to_string_lossy().contains("test") || path.to_string_lossy().contains("mock"))
        {
            return TodoSeverity::T4;
        }
        if lower.contains("panic!") {
            return TodoSeverity::T4;
        }

        // T3: Stub debt
        if lower.contains("stub") || lower.contains("placeholder") {
            return TodoSeverity::T3;
        }

        // T2: Test debt
        if lower.contains("test") {
            return TodoSeverity::T2;
        }

        // T1: Cleanup
        if lower.contains("cleanup") || lower.contains("refactor") {
            return TodoSeverity::T1;
        }

        // T0: Harmless note
        TodoSeverity::T0
    }

    fn generate_report(&self, items: Vec<TodoItem>) -> Result<TodoReport> {
        let mut by_severity: HashMap<String, usize> = HashMap::new();
        let mut mainnet_blockers = Vec::new();
        let mut testnet_blockers = Vec::new();
        let mut critical_paths_affected = Vec::new();

        for item in &items {
            let severity_str = format!("{:?}", item.severity);
            *by_severity.entry(severity_str).or_insert(0) += 1;

            // Check if blocks mainnet
            if matches!(
                item.severity,
                TodoSeverity::T5
                    | TodoSeverity::T6
                    | TodoSeverity::T7
                    | TodoSeverity::T8
                    | TodoSeverity::T9
            ) {
                mainnet_blockers.push(item.clone());
            }

            // Check if blocks testnet
            if matches!(
                item.severity,
                TodoSeverity::T6 | TodoSeverity::T7 | TodoSeverity::T8 | TodoSeverity::T9
            ) {
                testnet_blockers.push(item.clone());
            }

            // Check if in critical path
            if self.is_critical_path(&item.file) {
                let path_str = item.file.to_string_lossy().to_string();
                if !critical_paths_affected.contains(&path_str) {
                    critical_paths_affected.push(path_str);
                }
            }
        }

        Ok(TodoReport {
            total_todos: items.len(),
            by_severity,
            items,
            mainnet_blockers,
            testnet_blockers,
            critical_paths_affected,
        })
    }

    pub fn check_gates(&self, report: &TodoReport, gate: &str) -> Result<bool> {
        match gate {
            "mainnet" => Ok(report.mainnet_blockers.is_empty()),
            "testnet" => Ok(report.testnet_blockers.is_empty()),
            _ => Ok(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_classification() {
        let scanner = TodoScanner::new(PathBuf::from("."));

        assert_eq!(
            scanner.classify_severity("// TODO: fake finality", &PathBuf::from("test.rs")),
            TodoSeverity::T9
        );

        assert_eq!(
            scanner.classify_severity("unimplemented!()", &PathBuf::from("test.rs")),
            TodoSeverity::T8
        );
    }
}
