use crate::proof::*;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tokio::process::Command;

async fn assess_treasury(workspace: &Path, claim_id: &str) -> Result<ProofResult> {
    let started = Instant::now();
    let checks: [(&str, &str); 4] = [
        (
            "pallets/treasury/Cargo.toml",
            "Treasury pallet manifest exists",
        ),
        (
            "pallets/treasury/src/lib.rs",
            "Treasury pallet implementation exists",
        ),
        (
            "proof/funding/milestone-receipt-map.yml",
            "Funding milestone-to-receipt map exists",
        ),
        (
            "proof/receipts/claims/x3.funding.milestone_receipts.receipt.json",
            "Funding milestone receipt exists",
        ),
    ];

    let mut files_inspected = Vec::new();
    let mut passed_checks = Vec::new();
    let mut failed_checks = Vec::new();
    let mut missing_proofs = Vec::new();
    let mut evidence = HashMap::new();
    let mut commands_run = Vec::new();

    for (rel, label) in checks {
        files_inspected.push(rel.to_string());
        let ok = workspace.join(rel).exists();
        evidence.insert(rel.to_string(), ok.to_string());
        if ok {
            passed_checks.push(label.to_string());
        } else {
            failed_checks.push(format!("Missing treasury artifact: {}", rel));
            missing_proofs.push(format!("Add or restore {}", rel));
        }
    }

    // Executable gate: treasury pallet should resolve and type-check.
    let treasury_manifest = workspace.join("pallets/treasury/Cargo.toml");
    if treasury_manifest.exists() {
        let cmd = "cargo check -p pallet-treasury --quiet";
        commands_run.push(cmd.to_string());
        match Command::new("cargo")
            .current_dir(workspace)
            .arg("check")
            .arg("-p")
            .arg("pallet-treasury")
            .arg("--quiet")
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                passed_checks.push("Treasury executable check passed".to_string());
                evidence.insert("treasury_check".to_string(), "true".to_string());
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                failed_checks.push(format!(
                    "Treasury executable check failed: {}",
                    stderr
                        .lines()
                        .find(|l| !l.trim().is_empty())
                        .unwrap_or("unknown error")
                ));
                evidence.insert("treasury_check".to_string(), "false".to_string());
            }
            Err(e) => {
                failed_checks.push(format!("Treasury executable check could not run: {}", e));
                evidence.insert("treasury_check".to_string(), "false".to_string());
            }
        }
    }

    let total_checks = passed_checks.len() + failed_checks.len();
    let score = if total_checks == 0 {
        0.0
    } else {
        passed_checks.len() as f64 / total_checks as f64
    };
    let status = if failed_checks.is_empty() && missing_proofs.is_empty() {
        ProofStatus::Verified
    } else if !failed_checks.is_empty() || !passed_checks.is_empty() {
        ProofStatus::Partial
    } else {
        ProofStatus::Unverified
    };

    Ok(ProofResult {
        claim_id: claim_id.to_string(),
        claim: "Treasury and funding receipt artifacts are present".to_string(),
        status,
        proof_level: Some(ProofLevel::P5),
        edge_case_level: Some(EdgeCaseLevel::E6),
        hack_level: Some(HackLevel::H7),
        operator_level: Some(OperatorLevel::I6),
        degraded_level: Some(DegradedLevel::D5),
        files_inspected,
        commands_run,
        passed_checks,
        failed_checks,
        missing_proofs,
        blockers: vec![],
        score,
        evidence,
        timestamp: Utc::now(),
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

pub async fn verify_claim(workspace: &Path, claim_id: &str, _verbose: bool) -> Result<ProofResult> {
    assess_treasury(workspace, claim_id).await
}

pub async fn run_proofs(workspace: &Path, _verbose: bool) -> Result<ProofResult> {
    assess_treasury(workspace, "x3.treasury.full_proof").await
}
