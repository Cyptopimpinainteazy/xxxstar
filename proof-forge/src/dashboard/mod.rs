#![allow(dead_code)] // intentional dashboard scaffold; tracked in readiness backlog

// Dashboard module for proof metrics export and visualization

use crate::proof::ProofResult;
use crate::scoring::ScoreGrade;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub timestamp: String,
    pub overall_status: String,
    pub overall_score: f64,
    pub grade: String,
    pub areas_proven: Vec<AreaMetrics>,
    pub blockers: Vec<BlockerInfo>,
    pub proof_distribution: HashMap<String, u32>,
    pub test_coverage: TestCoverageMetrics,
    /// Why the score is what it is. Added because a dashboard with no evidence
    /// used to be indistinguishable from one where everything passed.
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaMetrics {
    pub area: String,
    pub score: f64,
    pub grade: String,
    pub status: String,
    pub proven_claims: u32,
    pub total_claims: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockerInfo {
    pub claim_id: String,
    pub severity: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCoverageMetrics {
    pub compile_checks_pass: bool,
    pub unit_tests_pass: u32,
    pub integration_tests_pass: u32,
    pub invariant_tests_pass: u32,
    pub adversarial_tests_pass: u32,
    pub benchmark_avg_ms: f64,
    pub wiring_verified: bool,
    pub drift_detected: bool,
}

impl Default for Dashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            overall_status: "Pending".to_string(),
            overall_score: 0.0,
            grade: "F".to_string(),
            areas_proven: vec![],
            blockers: vec![],
            proof_distribution: HashMap::new(),
            test_coverage: TestCoverageMetrics {
                compile_checks_pass: false,
                unit_tests_pass: 0,
                integration_tests_pass: 0,
                invariant_tests_pass: 0,
                adversarial_tests_pass: 0,
                benchmark_avg_ms: 0.0,
                wiring_verified: false,
                drift_detected: false,
            },
            reason: String::new(),
        }
    }

    pub fn add_area(&mut self, area: AreaMetrics) {
        self.areas_proven.push(area);
    }

    pub fn add_blocker(&mut self, blocker: BlockerInfo) {
        self.blockers.push(blocker);
    }

    pub fn set_score(&mut self, score: f64) {
        self.overall_score = score;
        self.grade = ScoreGrade::from_score(score).as_str().to_string();
        self.overall_status = if score >= 0.95 {
            "Excellent".to_string()
        } else if score >= 0.85 {
            "Good".to_string()
        } else if score >= 0.70 {
            "Acceptable".to_string()
        } else if score >= 0.50 {
            "Poor".to_string()
        } else {
            "Critical".to_string()
        };
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    pub fn readiness_assessment(&self) -> ReadinessAssessment {
        ReadinessAssessment {
            testnet_ready: self.overall_score >= 0.85,
            mainnet_ready: self.overall_score >= 0.95,
            critical_blockers: self.blockers.len(),
            all_areas_covered: self.areas_proven.iter().all(|a| a.proven_claims > 0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessAssessment {
    pub testnet_ready: bool,
    pub mainnet_ready: bool,
    pub critical_blockers: usize,
    pub all_areas_covered: bool,
}

/// Export proof metrics in machine-readable JSON format
pub struct MetricsExporter;

impl MetricsExporter {
    pub fn export_summary(results: &[ProofResult]) -> serde_json::Result<String> {
        let mut dashboard = Dashboard::new();

        let avg_score: f64 = results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64;
        dashboard.set_score(avg_score);

        for result in results {
            dashboard
                .proof_distribution
                .entry(result.claim_id.clone())
                .or_insert(1);

            if !result.blockers.is_empty() {
                for blocker in &result.blockers {
                    dashboard.add_blocker(BlockerInfo {
                        claim_id: result.claim_id.clone(),
                        severity: "High".to_string(),
                        description: blocker.clone(),
                    });
                }
            }
        }

        dashboard.to_json()
    }

    pub fn export_detailed(result: &ProofResult) -> serde_json::Result<String> {
        serde_json::to_string_pretty(&serde_json::json!({
            "claim_id": result.claim_id,
            "claim": result.claim,
            "status": format!("{:?}", result.status),
            "proof_level": result.proof_level.as_ref().map(|l| format!("{:?}", l)),
            "score": result.score,
            "passed_checks": result.passed_checks.len(),
            "failed_checks": result.failed_checks.len(),
            "blockers": result.blockers.len(),
            "evidence_keys": result.evidence.keys().collect::<Vec<_>>(),
            "timestamp": result.timestamp.to_rfc3339(),
            "duration_ms": result.duration_ms,
        }))
    }
}

/// Generate dashboard report and export to JSON file
pub async fn generate_dashboard(
    workspace: &Path,
    output_file: &Path,
    _detailed: bool,
    _verbose: bool,
) -> Result<()> {
    // This used to be:
    //
    //     let mut dashboard = Dashboard::new();
    //     dashboard.set_score(0.92);
    //
    // which wrote "Good / A- / 0.92" for *any* workspace — including one where
    // nothing had been verified, which is the state the same file records:
    // `compile_checks_pass: false`, `unit_tests_pass: 0`,
    // `integration_tests_pass: 0`, `invariant_tests_pass: 0`,
    // `adversarial_tests_pass: 0`, `wiring_verified: false`, `areas_proven: []`.
    // A grade is a claim about evidence, and a constant is not evidence.
    //
    // The score has to come from recorded proof results
    // (`Registry::record_result`), and nothing persists a registry — the CLI's
    // own `Registry` is built in memory per run (`Registry::new()`). So there is
    // no stored evidence to read, and the truthful dashboard says so instead of
    // inventing a number.
    if !workspace.is_dir() {
        anyhow::bail!(
            "workspace {} does not exist; a dashboard describes a workspace's evidence",
            workspace.display()
        );
    }

    let mut dashboard = Dashboard::new();
    dashboard.overall_score = 0.0;
    dashboard.overall_status = "Unverified".to_string();
    dashboard.grade = "Not assessed".to_string();
    dashboard.reason = format!(
        "No proof results are recorded for {}: `x3-proof verify` records them per run and the \
         registry is in-memory, so there is nothing to score yet. The previous version of this \
         function wrote a constant 0.92 / A- here regardless of the evidence.",
        workspace.display()
    );

    eprintln!(
        "warning: the dashboard carries no evidence — no proof results are recorded for {} \
         (score reported as 0.0, grade \"Not assessed\")",
        workspace.display()
    );

    let json_output = dashboard.to_json()?;
    std::fs::write(output_file, json_output)?;

    println!("Dashboard exported to: {}", output_file.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("x3-dashboard-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[tokio::test]
    async fn the_dashboard_does_not_invent_a_grade() {
        // This function used to write a constant 0.92 / "A-" / "Good" for any
        // workspace, including one where nothing had been verified. The three
        // fields it now writes are the honest ones, and the published artifacts
        // in the repository root carried that constant for months.
        let dir = temp_dir("grade");
        let out = dir.join("proof-score.json");
        generate_dashboard(&dir, &out, false, false)
            .await
            .expect("writes a dashboard");

        let text = std::fs::read_to_string(&out).expect("read back");
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(parsed["overall_status"], "Unverified");
        assert_eq!(parsed["overall_score"], 0.0);
        assert_eq!(parsed["grade"], "Not assessed");
        assert!(
            !text.contains("\"A-\""),
            "the dashboard must not carry a constant grade: {text}"
        );
        assert!(
            parsed["reason"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty()),
            "an unverified dashboard has to say why"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_missing_workspace_is_refused() {
        // The workspace argument used to be ignored (`_workspace`); a dashboard
        // describes a workspace's evidence, so a workspace that is not there is
        // an error rather than an occasion for a default score.
        let out =
            std::env::temp_dir().join(format!("x3-dashboard-missing-{}.json", std::process::id()));
        let result = generate_dashboard(
            std::path::Path::new("/nonexistent-x3-workspace"),
            &out,
            false,
            false,
        )
        .await;
        assert!(result.is_err(), "a missing workspace must be refused");
        assert!(
            !out.exists(),
            "nothing may be written for a missing workspace"
        );
    }
}
