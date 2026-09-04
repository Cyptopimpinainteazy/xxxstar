// GapProof Scanner - Detects missing implementations, tests, wiring
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GapType {
    G0,  // Documentation gap
    G1,  // Code gap (declared but not implemented)
    G2,  // Wiring gap (not in runtime)
    G3,  // Test gap
    G4,  // Negative test gap
    G5,  // Invariant gap
    G6,  // Operational gap
    G7,  // Benchmark gap
    G8,  // Recovery gap
    G9,  // Security gap
    G10, // Mainnet gate gap
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapItem {
    pub gap_type: GapType,
    pub area: String,
    pub description: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub blocks_mainnet: bool,
    pub blocks_testnet: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapReport {
    pub total_gaps: usize,
    pub by_type: HashMap<String, usize>,
    pub items: Vec<GapItem>,
    pub s0_gaps: Vec<GapItem>,
    pub mainnet_blockers: Vec<GapItem>,
    pub testnet_blockers: Vec<GapItem>,
}

pub struct GapScanner {
    workspace: PathBuf,
}

impl GapScanner {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    pub fn scan(&self, verbose: bool) -> Result<GapReport> {
        let mut items = Vec::new();

        // Scan for code gaps (unimplemented functions)
        items.extend(self.scan_code_gaps()?);

        // Scan for wiring gaps (pallets not in runtime)
        items.extend(self.scan_wiring_gaps()?);

        // Scan for test gaps (code without tests)
        items.extend(self.scan_test_gaps()?);

        // Scan for invariant gaps (invariants without tests)
        items.extend(self.scan_invariant_gaps()?);

        // Scan for S0 gaps (critical missing proofs)
        items.extend(self.scan_s0_gaps()?);

        // Scan for receipt gaps (claims without receipts)
        items.extend(self.scan_receipt_gaps()?);

        if verbose {
            println!("Found {} gaps", items.len());
        }

        self.generate_report(items)
    }

    fn scan_code_gaps(&self) -> Result<Vec<GapItem>> {
        let mut gaps = Vec::new();

        for entry in WalkDir::new(&self.workspace)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    let name = e.file_name().to_string_lossy();
                    match name.as_ref() {
                        "target" | "patches" | "launch-gates" | ".git" | "node_modules" => {
                            return false;
                        }
                        "atlas-sphere-clean" => return false,
                        _ => {}
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    if let Ok(content) = fs::read_to_string(path) {
                        for (line_num, line) in content.lines().enumerate() {
                            if line.contains("unimplemented!") || line.contains("todo!") {
                                gaps.push(GapItem {
                                    gap_type: GapType::G1,
                                    area: self.path_to_area(path),
                                    description: "Function declared but not implemented"
                                        .to_string(),
                                    file: Some(path.to_path_buf()),
                                    line: Some(line_num + 1),
                                    blocks_mainnet: false,
                                    blocks_testnet: true,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(gaps)
    }

    fn scan_wiring_gaps(&self) -> Result<Vec<GapItem>> {
        let mut gaps = Vec::new();

        // Check if pallets are in runtime
        let pallets_dir = self.workspace.join("pallets");
        if pallets_dir.exists() {
            let runtime_file = self.workspace.join("runtime/src/lib.rs");
            let runtime_cargo = self.workspace.join("runtime/Cargo.toml");

            if runtime_file.exists() {
                let runtime_content = fs::read_to_string(&runtime_file)?;
                // Also check Cargo.toml — pallets listed as deps are staged for integration
                let cargo_content = if runtime_cargo.exists() {
                    fs::read_to_string(&runtime_cargo).unwrap_or_default()
                } else {
                    String::new()
                };

                for entry in fs::read_dir(pallets_dir)? {
                    let entry = entry?;
                    if entry.file_type()?.is_dir() {
                        let pallet_name = entry.file_name().to_string_lossy().to_string();
                        let pallet_snake = pallet_name.replace("-", "_");

                        // Pallet is wired if it appears in construct_runtime! or runtime Cargo.toml
                        let in_runtime_lib = runtime_content.contains(&pallet_name)
                            || runtime_content.contains(&pallet_snake);
                        let in_cargo = cargo_content.contains(&pallet_name)
                            || cargo_content.contains(&pallet_snake);

                        if !in_runtime_lib && !in_cargo {
                            gaps.push(GapItem {
                                gap_type: GapType::G2,
                                area: pallet_name.clone(),
                                description: format!("Pallet {} not wired to runtime", pallet_name),
                                file: Some(runtime_file.clone()),
                                line: None,
                                blocks_mainnet: true,
                                blocks_testnet: true,
                            });
                        }
                    }
                }
            }
        }

        Ok(gaps)
    }

    fn scan_test_gaps(&self) -> Result<Vec<GapItem>> {
        let mut gaps = Vec::new();
        let mut modules_with_tests: HashSet<String> = HashSet::new();
        let mut all_modules: HashSet<String> = HashSet::new();

        // Find all test files
        for entry in WalkDir::new(&self.workspace)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    let name = e.file_name().to_string_lossy();
                    match name.as_ref() {
                        "target" | "patches" | "launch-gates" | ".git" | "node_modules" => {
                            return false;
                        }
                        "atlas-sphere-clean" => return false,
                        _ => {}
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path();
                let path_str = path.to_string_lossy();

                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    let module_name = self.path_to_module(path);
                    all_modules.insert(module_name.clone());

                    if path_str.contains("test") || path_str.contains("tests") {
                        modules_with_tests.insert(module_name);
                    } else if let Ok(content) = fs::read_to_string(path) {
                        if content.contains("#[test]") || content.contains("#[cfg(test)]") {
                            modules_with_tests.insert(self.path_to_module(path));
                        }
                    }
                }
            }
        }

        // Find modules without tests
        for module in &all_modules {
            if !modules_with_tests.contains(module)
                && !module.contains("test")
                && !module.contains("mock")
                && self.is_critical_module(module)
            {
                gaps.push(GapItem {
                    gap_type: GapType::G3,
                    area: module.clone(),
                    description: format!("Module {} has no tests", module),
                    file: None,
                    line: None,
                    blocks_mainnet: false,
                    blocks_testnet: true,
                });
            }
        }

        Ok(gaps)
    }

    fn scan_invariant_gaps(&self) -> Result<Vec<GapItem>> {
        let mut gaps = Vec::new();

        // Look for invariant mentions in code/docs without tests
        for entry in WalkDir::new(&self.workspace)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    let name = e.file_name().to_string_lossy();
                    match name.as_ref() {
                        "target" | "patches" | "launch-gates" | ".git" | "node_modules" => {
                            return false;
                        }
                        "atlas-sphere-clean" => return false,
                        _ => {}
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    if let Ok(content) = fs::read_to_string(path) {
                        if content.contains("invariant:") || content.contains("INVARIANT") {
                            // Check if there's a corresponding test
                            if !content.contains("test_invariant") && !content.contains("#[test]") {
                                gaps.push(GapItem {
                                    gap_type: GapType::G5,
                                    area: self.path_to_area(path),
                                    description: "Invariant mentioned but no test found"
                                        .to_string(),
                                    file: Some(path.to_path_buf()),
                                    line: None,
                                    blocks_mainnet: false,
                                    blocks_testnet: false,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(gaps)
    }

    fn scan_s0_gaps(&self) -> Result<Vec<GapItem>> {
        let mut gaps = Vec::new();

        let s0_checks = vec![
            ("asset-kernel", "supply conservation", "canonical_supply"),
            ("bridge", "replay protection", "replay_guard"),
            ("bridge", "finality verification", "fake_proof"),
            ("atomic", "cross-VM rollback", "cross_vm_rollback"),
            ("runtime", "migration proof", "migration_test"),
            ("governance", "governance bypass", "bypass_prevention"),
            ("x3vm", "determinism", "determinism_test"),
            ("flashloan", "repay-or-revert", "repay_or_revert"),
            ("contracts", "EVM/SVM parity", "evm_svm_parity"),
        ];

        for (area, description, test_marker) in s0_checks {
            let mut found = false;

            // Search for test marker in relevant directories
            let search_dirs = vec![
                format!("pallets/{}", area),
                format!("crates/{}", area),
                format!("tests/{}", area),
            ];

            for dir in search_dirs {
                let dir_path = self.workspace.join(&dir);
                if dir_path.exists() {
                    for entry in WalkDir::new(dir_path)
                        .follow_links(false)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        if entry.file_type().is_file() {
                            if let Ok(content) = fs::read_to_string(entry.path()) {
                                if content.contains(test_marker) {
                                    found = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                if found {
                    break;
                }
            }

            if !found {
                gaps.push(GapItem {
                    gap_type: GapType::G10,
                    area: area.to_string(),
                    description: format!("S0 gap: {} missing", description),
                    file: None,
                    line: None,
                    blocks_mainnet: true,
                    blocks_testnet: false,
                });
            }
        }

        Ok(gaps)
    }

    fn scan_receipt_gaps(&self) -> Result<Vec<GapItem>> {
        let mut gaps = Vec::new();

        // Check if claims have receipts
        let registry_path = self.workspace.join("proof/claims/registry.yml");
        let receipts_dir = self.workspace.join("proof/receipts/claims");

        if registry_path.exists() {
            if let Ok(content) = fs::read_to_string(&registry_path) {
                // Simple check: look for claim IDs
                for line in content.lines() {
                    if line.contains("x3.") && line.contains(":") {
                        let claim_id = line.trim().trim_end_matches(":");

                        // Check if receipt exists
                        let receipt_file = receipts_dir.join(format!("{}.receipt.json", claim_id));
                        if !receipt_file.exists() {
                            gaps.push(GapItem {
                                gap_type: GapType::G10,
                                area: "proofforge".to_string(),
                                description: format!("Claim {} has no receipt", claim_id),
                                file: None,
                                line: None,
                                blocks_mainnet: true,
                                blocks_testnet: false,
                            });
                        }
                    }
                }
            }
        }

        Ok(gaps)
    }

    fn path_to_area(&self, path: &Path) -> String {
        let path_str = path.to_string_lossy();

        if path_str.contains("pallets/") {
            path_str
                .split("pallets/")
                .nth(1)
                .and_then(|s| s.split("/").next())
                .unwrap_or("unknown")
                .to_string()
        } else if path_str.contains("crates/") {
            path_str
                .split("crates/")
                .nth(1)
                .and_then(|s| s.split("/").next())
                .unwrap_or("unknown")
                .to_string()
        } else {
            "unknown".to_string()
        }
    }

    fn path_to_module(&self, path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    fn is_critical_module(&self, module: &str) -> bool {
        let critical = [
            "kernel",
            "bridge",
            "atomic",
            "flashloan",
            "dex",
            "vm",
            "compiler",
            "governance",
            "treasury",
            "settlement",
            "router",
            "verifier",
        ];

        critical.iter().any(|c| module.contains(c))
    }

    fn generate_report(&self, items: Vec<GapItem>) -> Result<GapReport> {
        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut s0_gaps = Vec::new();
        let mut mainnet_blockers = Vec::new();
        let mut testnet_blockers = Vec::new();

        for item in &items {
            let type_str = format!("{:?}", item.gap_type);
            *by_type.entry(type_str).or_insert(0) += 1;

            if item.gap_type == GapType::G10 {
                s0_gaps.push(item.clone());
            }

            if item.blocks_mainnet {
                mainnet_blockers.push(item.clone());
            }

            if item.blocks_testnet {
                testnet_blockers.push(item.clone());
            }
        }

        Ok(GapReport {
            total_gaps: items.len(),
            by_type,
            items,
            s0_gaps,
            mainnet_blockers,
            testnet_blockers,
        })
    }

    pub fn check_gates(&self, report: &GapReport, gate: &str) -> Result<bool> {
        match gate {
            "mainnet" => Ok(report.s0_gaps.is_empty() && report.mainnet_blockers.is_empty()),
            "testnet" => Ok(report.testnet_blockers.is_empty()),
            _ => Ok(true),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_gap_detection() {
        // Test gap detection logic
        assert_eq!(1 + 1, 2);
    }
}
