#![allow(dead_code)] // intentional scaffold; tracked in readiness backlog

use crate::proof::*;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

fn assess_x3language(workspace: &Path, claim_id: &str) -> ProofResult {
    let started = Instant::now();
    let checks: [(&str, &str); 5] = [
        ("x3-lang/Cargo.toml", "x3-lang workspace manifest exists"),
        (
            "x3-lang/compiler/Cargo.toml",
            "x3-lang compiler crate exists",
        ),
        ("x3-lang/vm/Cargo.toml", "x3-lang VM crate exists"),
        (
            "x3-lang/tests/verify_bytecode.rs",
            "x3-lang bytecode verification test exists",
        ),
        ("docs/x3-lang/README.md", "x3-lang docs entry exists"),
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
            failed_checks.push(format!("Missing x3language artifact: {}", rel));
            missing_proofs.push(format!("Add or restore {}", rel));
        }
    }

    let score = present as f64 / 5.0;
    let status = if present == 5 {
        ProofStatus::Verified
    } else if present > 0 {
        ProofStatus::Partial
    } else {
        ProofStatus::Unverified
    };

    ProofResult {
        claim_id: claim_id.to_string(),
        claim: "X3Language compiler/VM/test artifacts are present".to_string(),
        status,
        proof_level: Some(ProofLevel::P5),
        edge_case_level: Some(EdgeCaseLevel::E6),
        hack_level: Some(HackLevel::H7),
        operator_level: Some(OperatorLevel::I6),
        degraded_level: Some(DegradedLevel::D5),
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
    Ok(assess_x3language(workspace, claim_id))
}

pub async fn run_proofs(workspace: &Path, _verbose: bool) -> Result<ProofResult> {
    Ok(assess_x3language(workspace, "x3.x3language.full_proof"))
}
