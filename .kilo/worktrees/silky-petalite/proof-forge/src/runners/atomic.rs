use crate::proof::*;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

/// Verify atomic cross-VM execution claims.
///
/// Checks:
///   one_terminal_state  — every atomic bundle ends in commit XOR rollback
///   rollback_safety     — failed legs trigger full rollback, no partial commits
pub async fn verify_claim(workspace: &Path, claim_id: &str, verbose: bool) -> Result<ProofResult> {
    let start = Instant::now();

    if verbose {
        println!("  → Verifying atomic cross-VM claim: {}", claim_id);
    }

    let mut files_inspected = vec![];
    let mut commands_run = vec![];
    let mut passed_checks = vec![];
    let mut failed_checks = vec![];
    let mut missing_proofs = vec![];
    let mut blockers = vec![];
    let mut evidence = HashMap::new();
    let strict_concurrency = std::env::var("PROOFFORGE_STRICT_CONCURRENCY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
        .unwrap_or(false);

    // ── 1. Check that the atomic-kernel source file exists ──────────────────
    let kernel_lib = workspace.join("pallets/x3-atomic-kernel/src/lib.rs");
    let kernel_tests = workspace.join("pallets/x3-atomic-kernel/src/tests.rs");

    if kernel_lib.exists() {
        files_inspected.push("pallets/x3-atomic-kernel/src/lib.rs".to_string());
        passed_checks.push("x3-atomic-kernel/src/lib.rs exists".to_string());
    } else {
        failed_checks.push("pallets/x3-atomic-kernel/src/lib.rs missing".to_string());
        blockers.push("Atomic kernel source not found".to_string());
    }

    if kernel_tests.exists() {
        files_inspected.push("pallets/x3-atomic-kernel/src/tests.rs".to_string());
        passed_checks.push("x3-atomic-kernel/src/tests.rs exists".to_string());
    } else {
        missing_proofs.push("No test file for x3-atomic-kernel".to_string());
    }

    // ── 2. Grep for one-terminal-state evidence ──────────────────────────────
    // The claim is that every bundle ends in exactly one terminal state.
    // We look for rollback / commit tests in the atomic kernel.
    let rollback_grep = Command::new("grep")
        .args([
            "-rn",
            "rollback\\|one_terminal\\|commit_or_rollback",
            "pallets/x3-atomic-kernel/src/",
        ])
        .current_dir(workspace)
        .output();

    match rollback_grep {
        Ok(out) => {
            let hits = String::from_utf8_lossy(&out.stdout);
            let count = hits.lines().count();
            evidence.insert("rollback_references".to_string(), count.to_string());
            if count > 0 {
                passed_checks.push(format!(
                    "{} rollback/commit references found in atomic kernel",
                    count
                ));
            } else {
                failed_checks.push("No rollback/commit references in atomic kernel".to_string());
                missing_proofs
                    .push("Need explicit rollback test cases in x3-atomic-kernel".to_string());
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("    Warning: grep failed: {}", e);
            }
        }
    }

    // ── 3. Run the atomic kernel test suite ─────────────────────────────────
    let pkg_name = "pallet-x3-atomic-kernel";
    let test_filter = if claim_id.contains("rollback") {
        "rollback"
    } else {
        "poae"
    };

    let cmd_str = format!("cargo test -p {} -- {}", pkg_name, test_filter);
    commands_run.push(cmd_str.clone());

    if verbose {
        println!("    Running: {}", cmd_str);
    }

    let test_out = Command::new("cargo")
        // Use --lib to avoid loom integration tests which require special env setup
        .args([
            "test",
            "-p",
            pkg_name,
            "--lib",
            "--",
            test_filter,
            "--quiet",
        ])
        .current_dir(workspace)
        .output();

    match test_out {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() {
                // Count how many tests passed
                let test_count = stdout
                    .lines()
                    .chain(stderr.lines())
                    .filter(|l| l.contains("test ") && l.contains("ok"))
                    .count();
                if test_count > 0 {
                    passed_checks.push(format!("cargo test passed ({} tests)", test_count));
                    evidence.insert("tests_passed".to_string(), test_count.to_string());
                } else {
                    failed_checks.push(format!(
                        "cargo test returned success but matched 0 tests for filter '{}'",
                        test_filter
                    ));
                    missing_proofs.push(format!(
                        "No executed tests matched filter '{}' in lib tests",
                        test_filter
                    ));
                }
            } else {
                let err_snippet = stderr.lines().take(5).collect::<Vec<_>>().join(" | ");
                failed_checks.push(format!("cargo test failed: {}", err_snippet));
                blockers.push(format!(
                    "x3-atomic-kernel tests failing for filter '{}'",
                    test_filter
                ));
            }
        }
        Err(e) => {
            failed_checks.push(format!("Could not run cargo test: {}", e));
            missing_proofs.push("cargo test could not be executed".to_string());
        }
    }

    // ── 4b. Concurrency rollback visibility probe (loom integration test) ───
    // This test previously failed and can invalidate rollback safety under contention.
    let loom_cmd = "cargo test -p pallet-x3-atomic-kernel --test loom_concurrency -- loom_rollback_visibility_across_threads --quiet";
    commands_run.push(loom_cmd.to_string());
    if verbose {
        println!("    Running: {}", loom_cmd);
    }
    let loom_out = Command::new("cargo")
        .args([
            "test",
            "-p",
            pkg_name,
            "--test",
            "loom_concurrency",
            "--",
            "loom_rollback_visibility_across_threads",
            "--quiet",
        ])
        .current_dir(workspace)
        .output();
    match loom_out {
        Ok(out) => {
            if out.status.success() {
                passed_checks.push("loom rollback visibility test passed".to_string());
                evidence.insert("loom_rollback_visibility".to_string(), "pass".to_string());
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let err_snippet = stderr.lines().take(5).collect::<Vec<_>>().join(" | ");
                failed_checks.push(format!(
                    "loom rollback visibility test failed: {}",
                    err_snippet
                ));
                evidence.insert("loom_rollback_visibility".to_string(), "fail".to_string());
                if strict_concurrency {
                    blockers.push(
                        "Concurrency rollback visibility failed (strict mode enabled via PROOFFORGE_STRICT_CONCURRENCY)"
                            .to_string(),
                    );
                } else {
                    missing_proofs.push(
                        "Concurrency rollback visibility is failing; set PROOFFORGE_STRICT_CONCURRENCY=1 to hard-fail this claim"
                            .to_string(),
                    );
                }
            }
        }
        Err(e) => {
            failed_checks.push(format!(
                "Could not run loom rollback visibility test: {}",
                e
            ));
            if strict_concurrency {
                blockers
                    .push("Could not execute loom concurrency probe in strict mode".to_string());
            } else {
                missing_proofs.push(
                    "Loom concurrency probe did not run; rollback safety under contention remains unproven"
                        .to_string(),
                );
            }
        }
    }

    // ── 5. Check cross-vm-router for single-outcome enforcement ─────────────
    let router_lib = workspace.join("pallets/x3-cross-vm-router/src/lib.rs");
    if router_lib.exists() {
        files_inspected.push("pallets/x3-cross-vm-router/src/lib.rs".to_string());

        let router_grep = Command::new("grep")
            .args([
                "-n",
                "atomic\\|rollback\\|one_state",
                "pallets/x3-cross-vm-router/src/lib.rs",
            ])
            .current_dir(workspace)
            .output();

        if let Ok(out) = router_grep {
            let hits = String::from_utf8_lossy(&out.stdout);
            let count = hits.lines().count();
            evidence.insert("router_atomic_references".to_string(), count.to_string());
            if count > 0 {
                passed_checks.push(format!(
                    "{} atomic enforcement references in cross-vm-router",
                    count
                ));
            } else {
                missing_proofs.push("No atomic enforcement visible in cross-vm-router".to_string());
            }
        }
    }

    // ── 6. Score and status ──────────────────────────────────────────────────
    let total_checks = passed_checks.len() + failed_checks.len();
    let score = if total_checks == 0 {
        0.0
    } else {
        passed_checks.len() as f64 / total_checks as f64
    };

    let status = if !blockers.is_empty() {
        ProofStatus::Failed
    } else if failed_checks.is_empty() && missing_proofs.is_empty() {
        ProofStatus::Verified
    } else if score >= 0.5 {
        ProofStatus::Partial
    } else {
        ProofStatus::Unverified
    };

    Ok(ProofResult {
        claim_id: claim_id.to_string(),
        claim: match claim_id {
            id if id.contains("rollback") => {
                "If any EVM/SVM/X3VM leg fails, the operation rolls back or refunds safely"
                    .to_string()
            }
            _ => "Every cross-VM atomic operation ends in exactly one terminal state".to_string(),
        },
        status,
        proof_level: None,
        edge_case_level: None,
        hack_level: None,
        operator_level: None,
        degraded_level: None,
        files_inspected,
        commands_run,
        passed_checks,
        failed_checks,
        missing_proofs,
        blockers,
        score,
        evidence,
        timestamp: Utc::now(),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

pub async fn run_proofs(workspace: &Path, verbose: bool) -> Result<ProofResult> {
    verify_claim(workspace, "x3.atomic.one_terminal_state", verbose).await
}
