//! Smart-contracts claim runner.
//!
//! Dispatches on the claim suffix:
//!
//! * `x3.contracts.evm_svm_parity` — runs the EVM↔SVM parity harness in
//!   `X3-contracts/shared/parity-core` and checks that real EVM contracts
//!   and SVM programs are present (not just empty placeholder directories).
//!   This is the S0 claim; failing the harness means the stacks have
//!   drifted from the published parity vectors and MUST NOT ship.
//!
//! * any other `x3.contracts.*` claim — returns UNVERIFIED so it cannot
//!   silently pass `mainnet-gate`. Each such claim should grow its own
//!   real evidence-driven runner.
//!
//! The previous implementation returned a hard-coded `Verified` for every
//! claim id with synthetic "passed checks", which is the textbook
//! false-green pattern that the proof-forge truth principle is supposed
//! to prevent.

#![allow(dead_code)] // intentional scaffold; tracked in readiness backlog

use crate::proof::*;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tokio::process::Command;

const PARITY_CORE_REL: &str = "X3-contracts/shared/parity-core/Cargo.toml";
const PARITY_VECTORS_REL: &str = "X3-contracts/shared/test-vectors/flashloan_repay_or_revert.json";
const EVM_DIR_REL: &str = "X3-contracts/evm/contracts";
const SVM_DIR_REL: &str = "X3-contracts/svm/programs";

async fn run_parity_vector_harness(workspace: &Path) -> std::result::Result<(), String> {
    let manifest = workspace.join(PARITY_CORE_REL);
    if !manifest.exists() {
        return Err(format!(
            "parity-core manifest missing at {}",
            manifest.display()
        ));
    }

    let output = Command::new("cargo")
        .arg("test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--test")
        .arg("parity_vectors")
        .arg("--quiet")
        .output()
        .await
        .map_err(|e| format!("failed to spawn cargo: {e}"))?;

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

/// Count files with the given extension underneath `dir` (depth 4). Used to
/// verify EVM (`.sol`) and SVM (`.rs`) trees actually contain source, not
/// just placeholder READMEs.
fn count_files_with_ext(dir: &Path, ext: &str, depth_left: u32) -> usize {
    if depth_left == 0 || !dir.is_dir() {
        return 0;
    }
    let mut total = 0usize;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            total += count_files_with_ext(&p, ext, depth_left - 1);
        } else if p.extension().and_then(|s| s.to_str()) == Some(ext) {
            total += 1;
        }
    }
    total
}

async fn verify_evm_svm_parity(workspace: &Path, claim_id: &str) -> Result<ProofResult> {
    let started = Instant::now();
    let mut passed_checks: Vec<String> = Vec::new();
    let mut failed_checks: Vec<String> = Vec::new();
    let mut missing_proofs: Vec<String> = Vec::new();
    let mut blockers: Vec<String> = Vec::new();
    let mut evidence: HashMap<String, String> = HashMap::new();

    // 1. EVM source presence: .sol files under contracts/.
    let evm_dir = workspace.join(EVM_DIR_REL);
    let sol_count = count_files_with_ext(&evm_dir, "sol", 4);
    evidence.insert("evm_sol_files".to_string(), sol_count.to_string());
    if sol_count > 0 {
        passed_checks.push(format!("EVM contracts present ({} .sol files)", sol_count));
    } else {
        missing_proofs.push(format!("no .sol files under {}", evm_dir.display()));
    }

    // 2. SVM source presence: .rs files under programs/.
    let svm_dir = workspace.join(SVM_DIR_REL);
    let svm_rs_count = count_files_with_ext(&svm_dir, "rs", 4);
    evidence.insert("svm_rs_files".to_string(), svm_rs_count.to_string());
    if svm_rs_count > 0 {
        passed_checks.push(format!("SVM programs present ({} .rs files)", svm_rs_count));
    } else {
        missing_proofs.push(format!("no .rs files under {}", svm_dir.display()));
    }

    // 3. Parity vectors document present and non-empty.
    let vectors = workspace.join(PARITY_VECTORS_REL);
    if vectors.exists() {
        let len = std::fs::metadata(&vectors).map(|m| m.len()).unwrap_or(0);
        evidence.insert("vectors_bytes".to_string(), len.to_string());
        if len > 0 {
            passed_checks.push(format!("Parity vectors present ({} bytes)", len));
        } else {
            failed_checks.push(format!(
                "parity vectors file is empty: {}",
                vectors.display()
            ));
        }
    } else {
        failed_checks.push(format!("parity vectors missing at {}", vectors.display()));
    }

    // 4. Run the executable parity harness (single source of truth).
    match run_parity_vector_harness(workspace).await {
        Ok(()) => {
            passed_checks.push("EVM↔SVM parity vector harness PASSED".to_string());
            evidence.insert("parity_harness".to_string(), "ok".to_string());
        }
        Err(detail) => {
            blockers.push("parity vector harness failure (EVM/SVM math drift)".to_string());
            failed_checks.push(detail);
            evidence.insert("parity_harness".to_string(), "failed".to_string());
        }
    }

    // Status logic: harness pass + both stacks present = VERIFIED.
    let test_passed = evidence
        .get("parity_harness")
        .map(|s| s == "ok")
        .unwrap_or(false);
    let stacks_present = sol_count > 0 && svm_rs_count > 0;

    let status = if !blockers.is_empty() {
        ProofStatus::Failed
    } else if test_passed && stacks_present {
        ProofStatus::Verified
    } else if test_passed || stacks_present {
        ProofStatus::Partial
    } else {
        ProofStatus::Unverified
    };

    let score = if test_passed && stacks_present {
        // 0.7 base for executable harness pass + 0.15 EVM + 0.15 SVM presence.
        0.7 + (if sol_count > 0 { 0.15 } else { 0.0 }) + (if svm_rs_count > 0 { 0.15 } else { 0.0 })
    } else if test_passed {
        0.5
    } else {
        0.0
    };

    Ok(ProofResult {
        claim_id: claim_id.to_string(),
        claim: "Core EVM contracts and SVM programs implement equivalent X3 behavior".to_string(),
        status,
        proof_level: Some(ProofLevel::P5),
        edge_case_level: Some(EdgeCaseLevel::E6),
        hack_level: Some(HackLevel::H7),
        operator_level: Some(OperatorLevel::I5),
        degraded_level: Some(DegradedLevel::D4),
        files_inspected: vec![
            EVM_DIR_REL.to_string(),
            SVM_DIR_REL.to_string(),
            PARITY_VECTORS_REL.to_string(),
            "X3-contracts/shared/parity-core/src/lib.rs".to_string(),
            "X3-contracts/shared/parity-core/tests/parity_vectors.rs".to_string(),
        ],
        commands_run: vec![format!(
            "cargo test --manifest-path {} --test parity_vectors",
            PARITY_CORE_REL
        )],
        passed_checks,
        failed_checks,
        missing_proofs,
        blockers,
        score,
        evidence,
        timestamp: Utc::now(),
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Default response for any `x3.contracts.*` claim that does not yet have
/// a real runner. Returning `Unverified` (rather than fabricating a green
/// pass) prevents `mainnet-gate` from accepting an unsubstantiated claim.
fn unverified_stub(claim_id: &str) -> ProofResult {
    let mut evidence = HashMap::new();
    evidence.insert(
        "reason".to_string(),
        "no runner implemented for this claim id yet".to_string(),
    );
    ProofResult {
        claim_id: claim_id.to_string(),
        claim: format!("{} (no evidence-driven runner)", claim_id),
        status: ProofStatus::Unverified,
        proof_level: None,
        edge_case_level: None,
        hack_level: None,
        operator_level: None,
        degraded_level: None,
        files_inspected: vec![],
        commands_run: vec![],
        passed_checks: vec![],
        failed_checks: vec![format!(
            "{}: no runner implemented; refusing to fabricate Verified",
            claim_id
        )],
        missing_proofs: vec![format!(
            "Add a runner for {} that drives executable evidence",
            claim_id
        )],
        blockers: vec![],
        score: 0.0,
        evidence,
        timestamp: Utc::now(),
        duration_ms: 0,
    }
}

pub async fn verify_claim(workspace: &Path, claim_id: &str, _verbose: bool) -> Result<ProofResult> {
    let suffix = claim_id.rsplit('.').next().unwrap_or("");
    match suffix {
        "evm_svm_parity" => verify_evm_svm_parity(workspace, claim_id).await,
        _ => Ok(unverified_stub(claim_id)),
    }
}

pub async fn run_proofs(workspace: &Path, _verbose: bool) -> Result<ProofResult> {
    // The roll-up `prove-all` lane uses the canonical S0 claim.
    verify_evm_svm_parity(workspace, "x3.contracts.evm_svm_parity").await
}
