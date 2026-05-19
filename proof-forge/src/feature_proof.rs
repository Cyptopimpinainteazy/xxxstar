use anyhow::{Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::receipt::Receipt;

/// Feature status classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureStatus {
    /// Fully built: all 10 criteria met
    #[serde(rename = "BUILT")]
    Built,

    /// Some parts exist but not all proof exists
    #[serde(rename = "PARTIAL")]
    Partial,

    /// Registry says required but no implementation found
    #[serde(rename = "MISSING")]
    Missing,

    /// Code exists but runtime/API/UI/deployment does not expose it
    #[serde(rename = "UNWIRED")]
    Unwired,

    /// Code exists and wired, but tests are missing
    #[serde(rename = "UNTESTED")]
    Untested,

    /// Only happy-path tests exist, negative tests missing
    #[serde(rename = "WEAK")]
    Weak,

    /// Receipt exists but touched files changed after receipt
    #[serde(rename = "STALE")]
    Stale,

    /// S0/S1 gap or T6+ TODO exists
    #[serde(rename = "BLOCKED")]
    Blocked,

    /// Feature was previously built but proof is now stale or failing
    #[serde(rename = "REVOKED")]
    Revoked,
}

impl FeatureStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeatureStatus::Built => "BUILT",
            FeatureStatus::Partial => "PARTIAL",
            FeatureStatus::Missing => "MISSING",
            FeatureStatus::Unwired => "UNWIRED",
            FeatureStatus::Untested => "UNTESTED",
            FeatureStatus::Weak => "WEAK",
            FeatureStatus::Stale => "STALE",
            FeatureStatus::Blocked => "BLOCKED",
            FeatureStatus::Revoked => "REVOKED",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            FeatureStatus::Built => "✅",
            FeatureStatus::Partial => "🟡",
            FeatureStatus::Missing => "❌",
            FeatureStatus::Unwired => "🔌",
            FeatureStatus::Untested => "🧪",
            FeatureStatus::Weak => "⚠️",
            FeatureStatus::Stale => "🕐",
            FeatureStatus::Blocked => "🚫",
            FeatureStatus::Revoked => "🔴",
        }
    }
}

/// Feature definition from feature_matrix.yml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub required_for: String,
    pub criticality: String,
    pub owner_area: String,
    pub docs: Vec<String>,
    pub code: Vec<String>,
    pub wiring: Vec<String>,
    pub tests: FeatureTests,
    pub proof_commands: Vec<String>,
    pub built_when: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureTests {
    #[serde(default)]
    pub unit: Vec<String>,
    #[serde(default)]
    pub integration: Vec<String>,
    #[serde(default)]
    pub negative: Vec<String>,
    #[serde(default)]
    pub fuzz: Vec<String>,
}

/// Feature matrix registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureMatrix {
    pub features: Vec<Feature>,
}

/// Feature proof result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureProofResult {
    pub feature_id: String,
    pub name: String,
    pub area: String,
    pub required_for: String,
    pub criticality: String,
    pub status: FeatureStatus,
    pub docs_found: usize,
    pub docs_missing: Vec<String>,
    pub code_found: usize,
    pub code_missing: Vec<String>,
    pub wiring_found: usize,
    pub wiring_missing: Vec<String>,
    pub tests_found: usize,
    pub tests_missing: Vec<String>,
    pub negative_tests_found: usize,
    pub negative_tests_missing: Vec<String>,
    pub receipt_exists: bool,
    pub receipt_fresh: bool,
    pub critical_todos: usize,
    pub blockers: Vec<String>,
    pub next_commands: Vec<String>,
}

/// Feature proof report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureReport {
    pub verdict: String,
    pub total_features: usize,
    pub built_count: usize,
    pub partial_count: usize,
    pub missing_count: usize,
    pub unwired_count: usize,
    pub untested_count: usize,
    pub weak_count: usize,
    pub stale_count: usize,
    pub blocked_count: usize,
    pub revoked_count: usize,
    pub results: Vec<FeatureProofResult>,
    pub top_blockers: Vec<String>,
}

/// Feature proof scanner
pub struct FeatureScanner {
    workspace: PathBuf,
}

struct FeatureStatusInputs {
    code_found: usize,
    code_missing: usize,
    wiring_found: usize,
    wiring_missing: usize,
    tests_found: usize,
    tests_missing: usize,
    negative_tests_found: usize,
    negative_tests_missing: usize,
    receipt_exists: bool,
    receipt_fresh: bool,
    critical_todos: usize,
}

impl FeatureScanner {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    /// Load feature matrix from YAML
    pub fn load_matrix(&self) -> Result<FeatureMatrix> {
        let matrix_path = self.workspace.join("proof/features/feature_matrix.yml");
        let yaml_content = fs::read_to_string(&matrix_path)
            .context(format!("Failed to read {}", matrix_path.display()))?;

        let matrix: FeatureMatrix =
            serde_yaml::from_str(&yaml_content).context("Failed to parse feature_matrix.yml")?;

        Ok(matrix)
    }

    /// Scan all features
    pub fn scan(&self, verbose: bool) -> Result<FeatureReport> {
        let matrix = self.load_matrix()?;

        if verbose {
            println!("Scanning {} features...", matrix.features.len());
        }

        let mut results = Vec::new();

        for feature in &matrix.features {
            if verbose {
                println!("  → {}", feature.id);
            }

            let result = self.scan_feature(feature, verbose)?;
            results.push(result);
        }

        // Compute report statistics
        let total_features = results.len();
        let mut built_count = 0;
        let mut partial_count = 0;
        let mut missing_count = 0;
        let mut unwired_count = 0;
        let mut untested_count = 0;
        let mut weak_count = 0;
        let mut stale_count = 0;
        let mut blocked_count = 0;
        let mut revoked_count = 0;

        for result in &results {
            match result.status {
                FeatureStatus::Built => built_count += 1,
                FeatureStatus::Partial => partial_count += 1,
                FeatureStatus::Missing => missing_count += 1,
                FeatureStatus::Unwired => unwired_count += 1,
                FeatureStatus::Untested => untested_count += 1,
                FeatureStatus::Weak => weak_count += 1,
                FeatureStatus::Stale => stale_count += 1,
                FeatureStatus::Blocked => blocked_count += 1,
                FeatureStatus::Revoked => revoked_count += 1,
            }
        }

        // Collect top blockers
        let mut top_blockers = Vec::new();
        for result in &results {
            if !result.blockers.is_empty() {
                for blocker in &result.blockers {
                    top_blockers.push(format!("{}: {}", result.feature_id, blocker));
                }
            }
        }
        top_blockers.sort();
        top_blockers.truncate(50);

        // Determine verdict
        let verdict = if blocked_count > 0 || missing_count > 0 {
            "BLOCKED"
        } else if partial_count > 0 || unwired_count > 0 || untested_count > 0 {
            "PARTIAL"
        } else if built_count == total_features {
            "BUILT"
        } else {
            "INCOMPLETE"
        };

        Ok(FeatureReport {
            verdict: verdict.to_string(),
            total_features,
            built_count,
            partial_count,
            missing_count,
            unwired_count,
            untested_count,
            weak_count,
            stale_count,
            blocked_count,
            revoked_count,
            results,
            top_blockers,
        })
    }

    /// Scan a single feature
    fn scan_feature(&self, feature: &Feature, verbose: bool) -> Result<FeatureProofResult> {
        let mut docs_found = 0;
        let mut docs_missing = Vec::new();
        let mut code_found = 0;
        let mut code_missing = Vec::new();
        let mut wiring_found = 0;
        let mut wiring_missing = Vec::new();
        let mut tests_found = 0;
        let mut tests_missing = Vec::new();
        let mut negative_tests_found = 0;
        let mut negative_tests_missing = Vec::new();
        let mut blockers = Vec::new();
        let mut next_commands = Vec::new();

        // Check docs
        for doc_path in &feature.docs {
            let full_path = self.workspace.join(doc_path);
            if full_path.exists() {
                docs_found += 1;
            } else {
                docs_missing.push(doc_path.clone());
            }
        }

        // Check code files
        for code_path in &feature.code {
            let full_path = self.workspace.join(code_path);
            if full_path.exists() {
                code_found += 1;
            } else {
                code_missing.push(code_path.clone());
            }
        }

        // Check wiring (search for evidence in codebase)
        for wiring_desc in &feature.wiring {
            let found = self.check_wiring_evidence(wiring_desc, verbose)?;
            if found {
                wiring_found += 1;
            } else {
                wiring_missing.push(wiring_desc.clone());
            }
        }

        // Check unit tests
        for test_name in &feature.tests.unit {
            let found = self.check_test_exists(test_name, &feature.code)?;
            if found {
                tests_found += 1;
            } else {
                tests_missing.push(format!("unit: {}", test_name));
            }
        }

        // Check integration tests
        for test_name in &feature.tests.integration {
            let found = self.check_test_exists(test_name, &feature.code)?;
            if found {
                tests_found += 1;
            } else {
                tests_missing.push(format!("integration: {}", test_name));
            }
        }

        // Check negative tests
        for test_name in &feature.tests.negative {
            let found = self.check_test_exists(test_name, &feature.code)?;
            if found {
                negative_tests_found += 1;
            } else {
                negative_tests_missing.push(test_name.clone());
            }
        }

        // Check receipt
        let receipt_path = self
            .workspace
            .join(format!("proof/receipts/claims/{}.receipt.json", feature.id));
        let receipt_exists = receipt_path.exists();
        let receipt_fresh = self.receipt_is_fresh_and_valid(&receipt_path, receipt_exists)?;

        // Check for critical TODOs in code
        let critical_todos = self.count_critical_todos(&feature.code)?;

        // Build blockers list
        if !code_missing.is_empty() {
            blockers.push(format!("{} code files missing", code_missing.len()));
        }
        if !wiring_missing.is_empty() {
            blockers.push(format!("{} wiring checks failed", wiring_missing.len()));
        }
        if !tests_missing.is_empty() {
            blockers.push(format!("{} tests missing", tests_missing.len()));
        }
        if !negative_tests_missing.is_empty() {
            blockers.push(format!(
                "{} negative tests missing",
                negative_tests_missing.len()
            ));
        }
        if !receipt_exists {
            blockers.push("proof receipt missing".to_string());
        } else if !receipt_fresh {
            blockers.push("proof receipt stale".to_string());
        }
        if critical_todos > 0 {
            blockers.push(format!("{} critical TODOs found", critical_todos));
        }

        // Build next commands
        if !tests_missing.is_empty() {
            for cmd in &feature.proof_commands {
                next_commands.push(cmd.clone());
            }
        }
        if !receipt_exists {
            next_commands.push(format!("x3-proof claim prove {}", feature.id));
        }

        // Determine status
        let status = self.determine_status(FeatureStatusInputs {
            code_found,
            code_missing: code_missing.len(),
            wiring_found,
            wiring_missing: wiring_missing.len(),
            tests_found,
            tests_missing: tests_missing.len(),
            negative_tests_found,
            negative_tests_missing: negative_tests_missing.len(),
            receipt_exists,
            receipt_fresh,
            critical_todos,
        });

        Ok(FeatureProofResult {
            feature_id: feature.id.clone(),
            name: feature.name.clone(),
            area: feature.owner_area.clone(),
            required_for: feature.required_for.clone(),
            criticality: feature.criticality.clone(),
            status,
            docs_found,
            docs_missing,
            code_found,
            code_missing,
            wiring_found,
            wiring_missing,
            tests_found,
            tests_missing,
            negative_tests_found,
            negative_tests_missing,
            receipt_exists,
            receipt_fresh,
            critical_todos,
            blockers,
            next_commands,
        })
    }

    /// A receipt is fresh only when it loads, passes integrity, is not stale, and is <24h old.
    fn receipt_is_fresh_and_valid(
        &self,
        receipt_path: &Path,
        receipt_exists: bool,
    ) -> Result<bool> {
        if !receipt_exists {
            return Ok(false);
        }

        let receipt = match Receipt::load(receipt_path) {
            Ok(receipt) => receipt,
            Err(_) => {
                // Legacy or malformed receipts are treated as not fresh until migrated.
                return Ok(false);
            }
        };

        if !receipt.verify_integrity()? {
            return Ok(false);
        }

        if receipt.is_stale()? {
            return Ok(false);
        }

        Ok(receipt.is_fresh())
    }

    /// Determine feature status based on checks
    fn determine_status(&self, inputs: FeatureStatusInputs) -> FeatureStatus {
        // BLOCKED: critical TODOs or major issues
        if inputs.critical_todos > 0 {
            return FeatureStatus::Blocked;
        }

        // MISSING: no code found
        if inputs.code_found == 0 {
            return FeatureStatus::Missing;
        }

        // UNWIRED: code exists but wiring missing
        if inputs.code_found > 0 && inputs.wiring_found == 0 {
            return FeatureStatus::Unwired;
        }

        // UNTESTED: code and wiring exist but no tests
        if inputs.code_found > 0 && inputs.wiring_found > 0 && inputs.tests_found == 0 {
            return FeatureStatus::Untested;
        }

        // WEAK: tests exist but negative tests missing
        if inputs.tests_found > 0
            && inputs.negative_tests_found == 0
            && inputs.negative_tests_missing > 0
        {
            return FeatureStatus::Weak;
        }

        // STALE: receipt exists but not fresh
        if inputs.receipt_exists && !inputs.receipt_fresh {
            return FeatureStatus::Stale;
        }

        // BUILT: all criteria met
        if inputs.code_missing == 0
            && inputs.wiring_missing == 0
            && inputs.tests_missing == 0
            && inputs.negative_tests_missing == 0
            && inputs.receipt_exists
            && inputs.receipt_fresh
            && inputs.critical_todos == 0
        {
            return FeatureStatus::Built;
        }

        // PARTIAL: some criteria met but not all
        FeatureStatus::Partial
    }

    /// Check if wiring evidence exists
    fn check_wiring_evidence(&self, wiring_desc: &str, _verbose: bool) -> Result<bool> {
        // Simple heuristic: search for key phrases in relevant files
        let search_terms: Vec<&str> = if wiring_desc.contains("runtime") {
            vec!["construct_runtime!", "pallet", "Runtime"]
        } else if wiring_desc.contains("pallet") {
            vec!["impl", "Config", "trait"]
        } else {
            vec![wiring_desc]
        };

        for term in search_terms {
            let output = Command::new("grep")
                .args(["-r", term, "runtime/", "--include=*.rs"])
                .current_dir(&self.workspace)
                .output();

            if let Ok(output) = output {
                if !output.stdout.is_empty() {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if a test exists in code files
    fn check_test_exists(&self, test_name: &str, code_files: &[String]) -> Result<bool> {
        for code_file in code_files {
            let full_path = self.workspace.join(code_file);
            if !full_path.exists() {
                continue;
            }

            let output = Command::new("grep")
                .args(["-l", test_name, &full_path.to_string_lossy()])
                .output();

            if let Ok(output) = output {
                if output.status.success() && !output.stdout.is_empty() {
                    return Ok(true);
                }
            }

            // Also check parent directory for test files
            if let Some(parent) = full_path.parent() {
                let output = Command::new("find")
                    .args([
                        parent.to_string_lossy().as_ref(),
                        "-name",
                        "*.rs",
                        "-type",
                        "f",
                    ])
                    .output();

                if let Ok(output) = output {
                    let test_files = String::from_utf8_lossy(&output.stdout);
                    for test_file in test_files.lines() {
                        let grep_output = Command::new("grep")
                            .args(["-l", test_name, test_file])
                            .output();

                        if let Ok(grep_output) = grep_output {
                            if grep_output.status.success() && !grep_output.stdout.is_empty() {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Count critical TODOs in code files
    fn count_critical_todos(&self, code_files: &[String]) -> Result<usize> {
        let mut count = 0;

        let critical_patterns = vec![
            "TODO: CRITICAL",
            "TODO: BLOCKER",
            "FIXME: CRITICAL",
            "XXX: CRITICAL",
            "unimplemented!()",
            "todo!()",
        ];

        for code_file in code_files {
            let full_path = self.workspace.join(code_file);
            if !full_path.exists() {
                continue;
            }

            for pattern in &critical_patterns {
                let output = Command::new("grep")
                    .args(["-c", pattern, &full_path.to_string_lossy()])
                    .output();

                if let Ok(output) = output {
                    if output.status.success() {
                        let count_str = String::from_utf8_lossy(&output.stdout);
                        if let Ok(file_count) = count_str.trim().parse::<usize>() {
                            count += file_count;
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    /// Generate markdown report
    pub fn generate_markdown_report(&self, report: &FeatureReport) -> Result<String> {
        let mut md = String::new();

        md.push_str("# X3 FeatureBuiltProof Report\n\n");
        md.push_str(&format!("## Verdict\n**{}**\n\n", report.verdict));

        md.push_str("## Summary\n");
        md.push_str("| Status | Count |\n");
        md.push_str("|---|---:|\n");
        md.push_str(&format!("| BUILT | {} |\n", report.built_count));
        md.push_str(&format!("| PARTIAL | {} |\n", report.partial_count));
        md.push_str(&format!("| MISSING | {} |\n", report.missing_count));
        md.push_str(&format!("| UNWIRED | {} |\n", report.unwired_count));
        md.push_str(&format!("| UNTESTED | {} |\n", report.untested_count));
        md.push_str(&format!("| WEAK | {} |\n", report.weak_count));
        md.push_str(&format!("| STALE | {} |\n", report.stale_count));
        md.push_str(&format!("| BLOCKED | {} |\n", report.blocked_count));
        md.push_str(&format!("| REVOKED | {} |\n", report.revoked_count));
        md.push('\n');

        md.push_str("## Top Blockers\n");
        for (i, blocker) in report.top_blockers.iter().enumerate().take(10) {
            md.push_str(&format!("{}. {}\n", i + 1, blocker));
        }
        md.push('\n');

        let mut top_50_gaps: Vec<&FeatureProofResult> = report
            .results
            .iter()
            .filter(|r| {
                matches!(
                    r.status,
                    FeatureStatus::Missing
                        | FeatureStatus::Partial
                        | FeatureStatus::Unwired
                        | FeatureStatus::Untested
                )
            })
            .collect();
        top_50_gaps.sort_by(|a, b| {
            let ac = criticality_rank(&a.criticality);
            let bc = criticality_rank(&b.criticality);
            ac.cmp(&bc)
                .then(status_rank(a.status).cmp(&status_rank(b.status)))
                .then(a.feature_id.cmp(&b.feature_id))
        });

        md.push_str("## Top 50 Missing/Partial/Unwired/Untested Features\n");
        md.push_str("| Feature | Area | Criticality | Status | Blockers |\n");
        md.push_str("|---|---|---|---|---|\n");
        for result in top_50_gaps.into_iter().take(50) {
            let blockers = if result.blockers.is_empty() {
                "none".to_string()
            } else {
                result.blockers.join(", ")
            };
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                result.feature_id,
                result.area,
                result.criticality,
                result.status.as_str(),
                blockers
            ));
        }
        md.push('\n');

        md.push_str("## Built Features\n");
        md.push_str("| Feature | Area | Status | Exact Files Proving Build |\n");
        md.push_str("|---|---|---|---|\n");
        for result in &report.results {
            if result.status == FeatureStatus::Built {
                let files = self.get_feature_evidence_files(&result.feature_id)?;
                let joined = if files.is_empty() {
                    "none".to_string()
                } else {
                    files.join(", ")
                };
                md.push_str(&format!(
                    "| {} | {} | {} {} | {} |\n",
                    result.feature_id,
                    result.area,
                    result.status.emoji(),
                    result.status.as_str(),
                    joined
                ));
            }
        }
        md.push('\n');

        md.push_str("## Partial Features\n");
        md.push_str("| Feature | Blockers | Next Command |\n");
        md.push_str("|---|---|---|\n");
        for result in &report.results {
            if result.status == FeatureStatus::Partial {
                let blockers = result.blockers.join(", ");
                let next_cmd = result
                    .next_commands
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("N/A");
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    result.feature_id, blockers, next_cmd
                ));
            }
        }
        md.push('\n');

        md.push_str("## Missing Features\n");
        md.push_str("| Feature | Missing Code |\n");
        md.push_str("|---|---|\n");
        for result in &report.results {
            if result.status == FeatureStatus::Missing {
                let missing = result.code_missing.join(", ");
                md.push_str(&format!("| {} | {} |\n", result.feature_id, missing));
            }
        }
        md.push('\n');

        md.push_str("## Exact Commands Required To Prove Non-BUILT Features\n");
        md.push_str("| Feature | Status | Next Commands |\n");
        md.push_str("|---|---|---|\n");
        for result in &report.results {
            if result.status != FeatureStatus::Built {
                let cmds = if result.next_commands.is_empty() {
                    "none".to_string()
                } else {
                    result.next_commands.join(" ; ")
                };
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    result.feature_id,
                    result.status.as_str(),
                    cmds
                ));
            }
        }
        md.push('\n');

        md.push_str("## Exact Blockers Preventing Completion\n");
        md.push_str("| Feature | Status | Blockers |\n");
        md.push_str("|---|---|---|\n");
        for result in &report.results {
            if result.status != FeatureStatus::Built {
                let blockers = if result.blockers.is_empty() {
                    "none".to_string()
                } else {
                    result.blockers.join(" ; ")
                };
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    result.feature_id,
                    result.status.as_str(),
                    blockers
                ));
            }
        }
        md.push('\n');

        let mut burndown: Vec<&FeatureProofResult> = report
            .results
            .iter()
            .filter(|r| r.status != FeatureStatus::Built)
            .collect();
        burndown.sort_by(|a, b| {
            criticality_rank(&a.criticality)
                .cmp(&criticality_rank(&b.criticality))
                .then(status_rank(a.status).cmp(&status_rank(b.status)))
                .then(a.feature_id.cmp(&b.feature_id))
        });

        md.push_str("## Burn-Down Order By Criticality\n");
        md.push_str("| Order | Feature | Criticality | Status | Required For |\n");
        md.push_str("|---:|---|---|---|---|\n");
        for (idx, result) in burndown.iter().enumerate() {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                idx + 1,
                result.feature_id,
                result.criticality,
                result.status.as_str(),
                result.required_for
            ));
        }
        md.push('\n');

        Ok(md)
    }

    fn get_feature_evidence_files(&self, feature_id: &str) -> Result<Vec<String>> {
        let matrix = self.load_matrix()?;
        let files = matrix
            .features
            .iter()
            .find(|f| f.id == feature_id)
            .map(|f| f.code.clone())
            .unwrap_or_default();
        Ok(files)
    }

    /// Save report to files
    pub fn save_report(&self, report: &FeatureReport) -> Result<()> {
        // Save JSON
        let json_path = self.workspace.join("proof/reports/feature_status.json");
        let json = serde_json::to_string_pretty(report)?;
        fs::write(&json_path, json)?;

        // Save Markdown
        let md_path = self.workspace.join("proof/reports/features_report.md");
        let md = self.generate_markdown_report(report)?;
        fs::write(&md_path, md)?;

        Ok(())
    }
}

/// Run feature gate check
pub fn run_feature_gate(
    workspace: &Path,
    strict: bool,
    fail_hard: bool,
    verbose: bool,
) -> Result<()> {
    println!("{}", "🔥 X3 FEATUREBUILTPROOF GATE".bold().red());
    println!();

    let scanner = FeatureScanner::new(workspace.to_path_buf());
    let report = scanner.scan(verbose)?;

    // Save reports
    scanner.save_report(&report)?;

    // Print summary
    println!("{}", format!("Verdict: {}", report.verdict).bold());
    println!();
    println!("Built:     {}", report.built_count.to_string().green());
    println!("Partial:   {}", report.partial_count.to_string().yellow());
    println!("Missing:   {}", report.missing_count.to_string().red());
    println!("Unwired:   {}", report.unwired_count.to_string().yellow());
    println!("Untested:  {}", report.untested_count.to_string().yellow());
    println!("Weak:      {}", report.weak_count.to_string().yellow());
    println!("Stale:     {}", report.stale_count.to_string().yellow());
    println!("Blocked:   {}", report.blocked_count.to_string().red());
    println!("Revoked:   {}", report.revoked_count.to_string().red());
    println!();

    if !report.top_blockers.is_empty() {
        println!("{}", "Top blockers:".bold());
        for (i, blocker) in report.top_blockers.iter().enumerate().take(5) {
            println!("  {}. {}", i + 1, blocker.red());
        }
        println!();
    }

    println!("Reports saved:");
    println!("  - proof/reports/feature_status.json");
    println!("  - proof/reports/features_report.md");
    println!();

    if strict && report.built_count != report.total_features {
        anyhow::bail!(
            "Feature gate FAILED (--strict): {} of {} features are not BUILT",
            report.total_features - report.built_count,
            report.total_features
        );
    }

    if fail_hard
        && (report.blocked_count > 0 || report.missing_count > 0 || report.revoked_count > 0)
    {
        anyhow::bail!(
            "Feature gate FAILED (--fail-hard): {} blocked, {} missing, {} revoked",
            report.blocked_count,
            report.missing_count,
            report.revoked_count
        );
    }

    Ok(())
}

fn criticality_rank(criticality: &str) -> usize {
    match criticality.to_ascii_lowercase().as_str() {
        "catastrophic" => 0,
        "critical" => 1,
        "high" => 2,
        "medium" => 3,
        "low" => 4,
        _ => 5,
    }
}

fn status_rank(status: FeatureStatus) -> usize {
    match status {
        FeatureStatus::Blocked => 0,
        FeatureStatus::Missing => 1,
        FeatureStatus::Unwired => 2,
        FeatureStatus::Untested => 3,
        FeatureStatus::Weak => 4,
        FeatureStatus::Stale => 5,
        FeatureStatus::Revoked => 6,
        FeatureStatus::Partial => 7,
        FeatureStatus::Built => 8,
    }
}
