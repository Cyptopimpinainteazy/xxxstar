#![allow(dead_code)] // intentional scaffold; tracked in readiness backlog

use crate::proof::*;
use crate::runners::run_cargo_test_and_parse;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tokio::process::Command;

/// Run the EVM/SVM parity vector harness in
/// `X3-contracts/shared/parity-core` and return `Ok(())` iff every published
/// flashloan vector matches the pure simulator.
async fn run_parity_vector_harness(workspace: &Path) -> std::result::Result<(), String> {
    let manifest = workspace
        .join("X3-contracts")
        .join("shared")
        .join("parity-core")
        .join("Cargo.toml");
    if !manifest.exists() {
        return Err(format!(
            "parity-core manifest missing at {}",
            manifest.display()
        ));
    }

    let output = Command::new("cargo")
        .arg("test")
        .arg("--target-dir")
        .arg(workspace.join("target/gates/economic-attack"))
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--test")
        .arg("parity_vectors")
        .arg("--quiet")
        .output()
        .await
        .map_err(|e| format!("failed to spawn cargo for parity-core: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "parity vector harness failed:\n--- stdout ---\n{}\n--- stderr ---\n{}",
            stdout.trim(),
            stderr.trim()
        ))
    }
}

fn expected_attack_tests() -> Vec<&'static str> {
    vec![
        "flash_loan_oracle_manipulation_fails",
        "flash_loan_reentrancy_rejected",
        "flash_loan_repayment_bypass_reverts",
    ]
}

async fn run_flashloan_attack_suite(
    claim_id: String,
    claim: String,
    workspace: &Path,
) -> Result<ProofResult> {
    let started = Instant::now();
    let expected = expected_attack_tests();
    let (passed_tests, failed_tests) =
        run_cargo_test_and_parse(workspace, "x3-flashloan", "attack_").await?;

    let mut passed_checks: Vec<String> = Vec::new();
    let mut failed_checks: Vec<String> = Vec::new();

    for t in &expected {
        let found_pass = passed_tests.iter().any(|p| p.contains(t));
        let found_fail = failed_tests.iter().any(|f| f.contains(t));
        if found_pass {
            passed_checks.push(format!("{} passed", t));
        } else if found_fail {
            failed_checks.push(format!("{} failed", t));
        } else {
            failed_checks.push(format!("{} missing/ignored (mitigation gap)", t));
        }
    }

    for extra in failed_tests {
        if !failed_checks.iter().any(|f| f.contains(&extra)) {
            failed_checks.push(format!("test failure: {}", extra));
        }
    }

    // Dual-stack parity gate: every flashloan vector in
    // `X3-contracts/shared/test-vectors/*.json` must match the pure simulator
    // in `X3-contracts/shared/parity-core`. Failure here means EVM and SVM
    // implementations are pinned against drifting math and MUST NOT ship.
    let parity_label = "X3-contracts parity vectors (flashloan/repay_or_revert)";
    let parity_ok = match run_parity_vector_harness(workspace).await {
        Ok(()) => {
            passed_checks.push(format!("{} ok", parity_label));
            true
        }
        Err(detail) => {
            failed_checks.push(format!("{}: {}", parity_label, detail));
            false
        }
    };

    let passed_expected = passed_checks.len();
    let total_expected = expected.len() + 1; // attack tests + parity gate
    let score = if total_expected == 0 {
        0.0
    } else {
        passed_expected as f64 / total_expected as f64
    };

    let blocked = !failed_checks.is_empty();
    let status = if blocked {
        ProofStatus::Blocked
    } else {
        ProofStatus::Verified
    };

    let mut evidence = HashMap::new();
    evidence.insert(
        "expected_attack_tests".to_string(),
        expected.len().to_string(),
    );
    evidence.insert(
        "passed_attack_tests".to_string(),
        // Subtract the parity entry if it passed; its bookkeeping lives below.
        (passed_expected.saturating_sub(if parity_ok { 1 } else { 0 })).to_string(),
    );
    evidence.insert(
        "failed_attack_tests".to_string(),
        failed_checks.len().to_string(),
    );
    evidence.insert(
        "parity_vector_harness".to_string(),
        if parity_ok {
            "ok".to_string()
        } else {
            "failed".to_string()
        },
    );

    Ok(ProofResult {
        claim_id,
        claim,
        status,
        proof_level: Some(ProofLevel::P6),
        edge_case_level: Some(EdgeCaseLevel::E7),
        hack_level: Some(if blocked { HackLevel::H0 } else { HackLevel::H8 }),
        operator_level: Some(OperatorLevel::I6),
        degraded_level: Some(DegradedLevel::D5),
        files_inspected: vec![
            "crates/x3-flashloan/src/lib.rs".to_string(),
            "crates/x3-flashloan/src/tests/attack_oracle_manipulation.rs".to_string(),
            "crates/x3-flashloan/src/tests/attack_reentrancy.rs".to_string(),
            "crates/x3-flashloan/src/tests/attack_repayment_bypass.rs".to_string(),
            "X3-contracts/shared/parity-core/src/lib.rs".to_string(),
            "X3-contracts/shared/parity-core/tests/parity_vectors.rs".to_string(),
            "X3-contracts/shared/test-vectors/flashloan_repay_or_revert.json".to_string(),
        ],
        commands_run: vec![
            "cargo test -p x3-flashloan attack_ -- --nocapture".to_string(),
            "cargo test --manifest-path X3-contracts/shared/parity-core/Cargo.toml --test parity_vectors".to_string(),
        ],
        passed_checks,
        failed_checks,
        missing_proofs: vec![],
        blockers: vec![],
        score,
        evidence,
        timestamp: Utc::now(),
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

pub async fn verify_claim(
    _workspace: &Path,
    claim_id: &str,
    _verbose: bool,
) -> Result<ProofResult> {
    run_flashloan_attack_suite(
        claim_id.to_string(),
        "Flashloan attack resistance".to_string(),
        _workspace,
    )
    .await
}

pub async fn run_proofs(workspace: &Path, _verbose: bool) -> Result<ProofResult> {
    run_flashloan_attack_suite(
        "x3.flashloans.full_proof".to_string(),
        "Flashloan economic attack suite".to_string(),
        workspace,
    )
    .await
}
