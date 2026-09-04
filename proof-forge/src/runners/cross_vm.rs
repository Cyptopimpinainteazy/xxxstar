#![allow(dead_code)] // intentional scaffold; tracked in readiness backlog

use crate::proof::*;
use crate::runners::run_cargo_test_and_parse;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

fn expected_attack_tests() -> Vec<&'static str> {
    vec!["cross_vm_arbitrage_blocked_by_atomic_state"]
}

async fn run_cross_vm_attack_suite(
    claim_id: String,
    claim: String,
    workspace: &Path,
) -> Result<ProofResult> {
    let started = Instant::now();
    let expected = expected_attack_tests();
    let (passed_tests, failed_tests) =
        run_cargo_test_and_parse(workspace, "x3-cross-vm-bridge", "attack_arbitrage").await?;

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
            failed_checks.push(format!("{} missing/ignored (cross-VM atomicity gap)", t));
        }
    }

    for extra in failed_tests {
        if !failed_checks.iter().any(|f| f.contains(&extra)) {
            failed_checks.push(format!("test failure: {}", extra));
        }
    }

    let passed_expected = passed_checks.len();
    let total_expected = expected.len();
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
        total_expected.to_string(),
    );
    evidence.insert(
        "passed_attack_tests".to_string(),
        passed_expected.to_string(),
    );
    evidence.insert(
        "failed_attack_tests".to_string(),
        failed_checks.len().to_string(),
    );

    Ok(ProofResult {
        claim_id,
        claim,
        status,
        proof_level: Some(ProofLevel::P6),
        edge_case_level: Some(EdgeCaseLevel::E7),
        hack_level: Some(if blocked {
            HackLevel::H0
        } else {
            HackLevel::H8
        }),
        operator_level: Some(OperatorLevel::I7),
        degraded_level: Some(DegradedLevel::D6),
        files_inspected: vec![
            "crates/cross-vm-bridge/src/lib.rs".to_string(),
            "crates/cross-vm-bridge/src/tests/attack_arbitrage.rs".to_string(),
        ],
        commands_run: vec![
            "cargo test -p x3-cross-vm-bridge attack_arbitrage -- --nocapture".to_string(),
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

pub async fn verify_claim(workspace: &Path, claim_id: &str, _verbose: bool) -> Result<ProofResult> {
    run_cross_vm_attack_suite(
        claim_id.to_string(),
        "Cross-VM arbitrage resistance".to_string(),
        workspace,
    )
    .await
}

pub async fn run_proofs(workspace: &Path, _verbose: bool) -> Result<ProofResult> {
    run_cross_vm_attack_suite(
        "x3.cross_vm.full_proof".to_string(),
        "Cross-VM economic attack suite".to_string(),
        workspace,
    )
    .await
}
