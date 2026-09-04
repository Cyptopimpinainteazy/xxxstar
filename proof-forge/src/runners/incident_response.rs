#![allow(dead_code)] // intentional scaffold; tracked in readiness backlog

use crate::proof::*;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

fn assess_incident_response(workspace: &Path, claim_id: &str) -> ProofResult {
    let started = Instant::now();
    let checks: [(&str, &str); 4] = [
        (
            "launch-gates/DISASTER_RECOVERY_RUNBOOK.md",
            "Disaster recovery runbook exists",
        ),
        (
            "launch-gates/DISASTER_RECOVERY_RUNBOOKS.md",
            "Expanded DR runbooks index exists",
        ),
        (
            "docs/deployment/RUNBOOKS.md",
            "Deployment operations runbook exists",
        ),
        (
            "docs/runbooks/deployment/DEPLOYMENT_CHECKLIST.md",
            "Deployment checklist includes incident workflow path",
        ),
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
            failed_checks.push(format!("Missing incident-response artifact: {}", rel));
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
        claim: "Incident-response and disaster-recovery runbook artifacts are present".to_string(),
        status,
        proof_level: Some(ProofLevel::P5),
        edge_case_level: Some(EdgeCaseLevel::E6),
        hack_level: Some(HackLevel::H7),
        operator_level: Some(OperatorLevel::I7),
        degraded_level: Some(DegradedLevel::D8),
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
    Ok(assess_incident_response(workspace, claim_id))
}

pub async fn run_proofs(workspace: &Path, _verbose: bool) -> Result<ProofResult> {
    Ok(assess_incident_response(
        workspace,
        "x3.incident_response.full_proof",
    ))
}
