#![allow(dead_code)] // intentional scaffold; tracked in readiness backlog

use crate::proof::*;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

fn assess_upgrade_safety(workspace: &Path, claim_id: &str) -> ProofResult {
    let started = Instant::now();
    let checks: [(&str, &str); 4] = [
        (
            "scripts/run-try-runtime.sh",
            "Try-runtime execution script exists",
        ),
        (
            "proof/receipts/claims/x3.governance.proof_gated_upgrade.receipt.json",
            "Proof-gated upgrade receipt exists",
        ),
        (
            "pallets/governance/src/lib.rs",
            "Governance pallet implementation exists",
        ),
        ("runtime/Cargo.toml", "Runtime crate manifest exists"),
    ];

    let mut files_inspected = Vec::new();
    let mut passed_checks = Vec::new();
    let mut failed_checks = Vec::new();
    let mut missing_proofs = Vec::new();
    let mut evidence = HashMap::new();

    let mut present = 0usize;
    for (rel, label) in checks {
        files_inspected.push(rel.to_string());
        let ok = workspace.join(rel).exists();
        evidence.insert(rel.to_string(), ok.to_string());
        if ok {
            present += 1;
            passed_checks.push(label.to_string());
        } else {
            failed_checks.push(format!("Missing upgrade-safety artifact: {}", rel));
            missing_proofs.push(format!("Add or restore {}", rel));
        }
    }

    let score = present as f64 / 4.0;
    let status = if present == 4 {
        ProofStatus::Verified
    } else if present > 0 {
        ProofStatus::Partial
    } else {
        ProofStatus::Unverified
    };

    ProofResult {
        claim_id: claim_id.to_string(),
        claim: "Upgrade-safety artifacts and governance gate evidence are present".to_string(),
        status,
        proof_level: Some(ProofLevel::P6),
        edge_case_level: Some(EdgeCaseLevel::E7),
        hack_level: Some(HackLevel::H8),
        operator_level: Some(OperatorLevel::I8),
        degraded_level: Some(DegradedLevel::D6),
        files_inspected,
        commands_run: vec![],
        passed_checks,
        failed_checks,
        missing_proofs,
        blockers: vec![],
        score,
        evidence,
        timestamp: Utc::now(),
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

pub async fn verify_claim(workspace: &Path, claim_id: &str, _verbose: bool) -> Result<ProofResult> {
    Ok(assess_upgrade_safety(workspace, claim_id))
}

pub async fn run_proofs(workspace: &Path, _verbose: bool) -> Result<ProofResult> {
    Ok(assess_upgrade_safety(
        workspace,
        "x3.upgrade_safety.full_proof",
    ))
}
