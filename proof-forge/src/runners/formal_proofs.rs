#![allow(dead_code)] // intentional scaffold; tracked in readiness backlog

use crate::proof::*;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::process::Command;

#[derive(Debug, Clone)]
struct BackendOutcome {
    name: &'static str,
    passed: usize,
    failed: usize,
    missing: usize,
}

fn collect_files_with_ext(root: &Path, ext: &str, depth_left: u32, out: &mut Vec<PathBuf>) {
    if depth_left == 0 || !root.is_dir() {
        return;
    }

    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_ext(&path, ext, depth_left - 1, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some(ext) {
            out.push(path);
        }
    }
}

fn rel(workspace: &Path, p: &Path) -> String {
    p.strip_prefix(workspace)
        .unwrap_or(p)
        .to_string_lossy()
        .to_string()
}

fn tool_exists(cmd: &str) -> bool {
    std::process::Command::new("bash")
        .args(["-lc", &format!("command -v {} >/dev/null 2>&1", cmd)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn resolve_tool(workspace: &Path, cmd: &str) -> Option<PathBuf> {
    if tool_exists(cmd) {
        return Some(PathBuf::from(cmd));
    }

    let local = workspace.join("tools").join(cmd);
    if local.is_file() {
        return Some(local);
    }

    None
}

async fn run_tla_specs(
    workspace: &Path,
    commands_run: &mut Vec<String>,
    passed_checks: &mut Vec<String>,
    failed_checks: &mut Vec<String>,
    missing_proofs: &mut Vec<String>,
) -> BackendOutcome {
    let mut tla_specs = Vec::new();
    collect_files_with_ext(
        &workspace.join("formal-proofs/tla"),
        "tla",
        4,
        &mut tla_specs,
    );
    tla_specs.retain(|spec| {
        let rel_path = rel(workspace, spec);
        rel_path.contains("formal-proofs/tla/consensus/")
            || rel_path.contains("formal-proofs/tla/asset_kernel/")
            || rel_path.contains("formal-proofs/tla/contracts/")
    });
    tla_specs.sort();

    if tla_specs.is_empty() {
        missing_proofs.push("No TLA+ specs found under formal-proofs/tla".to_string());
        return BackendOutcome {
            name: "tla",
            passed: 0,
            failed: 0,
            missing: 1,
        };
    }

    let jar = workspace.join("tools/tla2tools.jar");
    let system_jar = PathBuf::from("/opt/tla2tools.jar");
    let tla_jar = if jar.exists() {
        jar
    } else if system_jar.exists() {
        system_jar
    } else {
        missing_proofs.push(
            "TLA+ tool missing (expected tools/tla2tools.jar or /opt/tla2tools.jar)".to_string(),
        );
        return BackendOutcome {
            name: "tla",
            passed: 0,
            failed: 0,
            missing: tla_specs.len(),
        };
    };

    let tla_jar_abs = std::fs::canonicalize(&tla_jar).unwrap_or(tla_jar.clone());

    let mut passed = 0usize;
    let mut failed = 0usize;

    for (idx, spec) in tla_specs.iter().enumerate() {
        let spec_dir = spec.parent().unwrap_or(workspace);
        let module_name = spec
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Spec")
            .to_string();
        let cfg = spec.with_extension("cfg");
        let spec_rel = rel(workspace, spec);
        // Per-spec unique metadir avoids TLC collisions when multiple checks
        // happen in the same second.
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let metadir = workspace.join("target/tlc").join(format!(
            "{}-{}-{}-{}",
            spec.file_stem().and_then(|s| s.to_str()).unwrap_or("spec"),
            std::process::id(),
            idx,
            now_ns
        ));
        let _ = std::fs::create_dir_all(&metadir);
        let cmd_text = if cfg.exists() {
            format!(
                "(cd {} && java -cp {} tlc2.TLC -deadlock -metadir {} -config {} {})",
                rel(workspace, spec_dir),
                tla_jar_abs.display(),
                metadir.display(),
                cfg.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("spec.cfg"),
                module_name
            )
        } else {
            format!(
                "(cd {} && java -cp {} tlc2.TLC -deadlock -metadir {} {})",
                rel(workspace, spec_dir),
                tla_jar_abs.display(),
                metadir.display(),
                module_name
            )
        };
        commands_run.push(cmd_text);

        let mut cmd = Command::new("java");
        cmd.current_dir(spec_dir)
            .arg("-cp")
            .arg(&tla_jar_abs)
            .arg("tlc2.TLC")
            .arg("-deadlock")
            .arg("-metadir")
            .arg(&metadir);
        if cfg.exists() {
            cmd.arg("-config").arg(cfg.file_name().unwrap_or_default());
        }
        cmd.arg(&module_name);

        match cmd.output().await {
            Ok(output) if output.status.success() => {
                passed += 1;
                passed_checks.push(format!("TLA+ model check passed: {}", spec_rel));
            }
            Ok(output) => {
                failed += 1;
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                // TLC prints invariant violations and parse errors to stdout,
                // not stderr — fall back to the first non-empty stdout line so
                // the gate failure carries an actionable diagnostic.
                let snippet = stderr
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .or_else(|| stdout.lines().find(|l| l.contains("Error")))
                    .or_else(|| stdout.lines().find(|l| !l.trim().is_empty()))
                    .unwrap_or("no stderr/stdout");
                failed_checks.push(format!(
                    "TLA+ model check failed: {} ({})",
                    spec_rel, snippet
                ));
            }
            Err(e) => {
                failed += 1;
                failed_checks.push(format!("TLA+ invocation error for {}: {}", spec_rel, e));
            }
        }
    }

    BackendOutcome {
        name: "tla",
        passed,
        failed,
        missing: 0,
    }
}

async fn run_coq_specs(
    workspace: &Path,
    commands_run: &mut Vec<String>,
    passed_checks: &mut Vec<String>,
    failed_checks: &mut Vec<String>,
    missing_proofs: &mut Vec<String>,
) -> BackendOutcome {
    let mut specs = Vec::new();
    collect_files_with_ext(&workspace.join("formal-proofs/coq"), "v", 4, &mut specs);
    specs.sort();

    if specs.is_empty() {
        missing_proofs.push("No Coq specs found under formal-proofs/coq".to_string());
        return BackendOutcome {
            name: "coq",
            passed: 0,
            failed: 0,
            missing: 1,
        };
    }

    let coqc = match resolve_tool(workspace, "coqc") {
        Some(path) => path,
        None => {
            let local = workspace.join("tools").join("coqc");
            missing_proofs.push(format!(
                "coqc not found in PATH (local check: {} exists={} file={})",
                local.display(),
                local.exists(),
                local.is_file()
            ));
            return BackendOutcome {
                name: "coq",
                passed: 0,
                failed: 0,
                missing: specs.len(),
            };
        }
    };

    let mut passed = 0usize;
    let mut failed = 0usize;

    for spec in specs {
        let spec_rel = rel(workspace, &spec);
        commands_run.push(format!("{} {}", coqc.display(), spec_rel));

        match Command::new(&coqc)
            .current_dir(workspace)
            .arg(spec.clone())
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                passed += 1;
                passed_checks.push(format!("Coq proof compiled: {}", spec_rel));
            }
            Ok(output) => {
                failed += 1;
                let stderr = String::from_utf8_lossy(&output.stderr);
                failed_checks.push(format!(
                    "Coq proof failed: {} ({})",
                    spec_rel,
                    stderr.lines().next().unwrap_or("no stderr")
                ));
            }
            Err(e) => {
                failed += 1;
                failed_checks.push(format!("Coq invocation error for {}: {}", spec_rel, e));
            }
        }
    }

    BackendOutcome {
        name: "coq",
        passed,
        failed,
        missing: 0,
    }
}

async fn run_k_specs(
    workspace: &Path,
    commands_run: &mut Vec<String>,
    passed_checks: &mut Vec<String>,
    failed_checks: &mut Vec<String>,
    missing_proofs: &mut Vec<String>,
) -> BackendOutcome {
    let mut specs = Vec::new();
    collect_files_with_ext(&workspace.join("formal-proofs/k"), "k", 4, &mut specs);
    specs.retain(|spec| {
        spec.file_stem()
            .and_then(|s| s.to_str())
            .map(|stem| !stem.ends_with("-claims"))
            .unwrap_or(true)
    });
    specs.sort();

    if specs.is_empty() {
        missing_proofs.push("No K specs found under formal-proofs/k".to_string());
        return BackendOutcome {
            name: "k",
            passed: 0,
            failed: 0,
            missing: 1,
        };
    }

    let kprove = match resolve_tool(workspace, "kprove") {
        Some(path) => path,
        None => {
            let local = workspace.join("tools").join("kprove");
            missing_proofs.push(format!(
                "kprove not found in PATH (local check: {} exists={} file={})",
                local.display(),
                local.exists(),
                local.is_file()
            ));
            return BackendOutcome {
                name: "k",
                passed: 0,
                failed: 0,
                missing: specs.len(),
            };
        }
    };

    let kompile = match resolve_tool(workspace, "kompile") {
        Some(path) => path,
        None => {
            let local = workspace.join("tools").join("kompile");
            missing_proofs.push(format!(
                "kompile not found in PATH (local check: {} exists={} file={})",
                local.display(),
                local.exists(),
                local.is_file()
            ));
            return BackendOutcome {
                name: "k",
                passed: 0,
                failed: 0,
                missing: specs.len(),
            };
        }
    };

    let mut passed = 0usize;
    let mut failed = 0usize;

    for spec in specs {
        let spec_rel = rel(workspace, &spec);
        let claim_file = {
            let stem = spec
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("k-spec")
                .to_string();
            let candidate_name = if stem.ends_with("-spec") {
                format!("{}-claims.k", stem.trim_end_matches("-spec"))
            } else {
                format!("{}-claims.k", stem)
            };
            let candidate = spec.with_file_name(candidate_name);
            if candidate.is_file() {
                candidate
            } else {
                spec.clone()
            }
        };
        let prove_rel = rel(workspace, &claim_file);

        let stem = spec
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("k-spec");
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let definition_dir = workspace.join("target/k").join(format!(
            "{}-kompiled-{}-{}",
            stem,
            std::process::id(),
            now_ns
        ));
        let _ = std::fs::create_dir_all(definition_dir.parent().unwrap_or(workspace));

        commands_run.push(format!(
            "{} {} --backend haskell -o {}",
            kompile.display(),
            spec_rel,
            rel(workspace, &definition_dir)
        ));

        let kompile_output = Command::new(&kompile)
            .current_dir(workspace)
            .arg(spec.clone())
            .arg("--backend")
            .arg("haskell")
            .arg("-o")
            .arg(definition_dir.clone())
            .output()
            .await;

        match kompile_output {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                failed += 1;
                let stderr = String::from_utf8_lossy(&output.stderr);
                failed_checks.push(format!(
                    "K kompile failed: {} ({})",
                    spec_rel,
                    stderr.lines().next().unwrap_or("no stderr")
                ));
                continue;
            }
            Err(e) => {
                failed += 1;
                failed_checks.push(format!(
                    "K kompile invocation error for {}: {}",
                    spec_rel, e
                ));
                continue;
            }
        }

        commands_run.push(format!(
            "{} {} --definition {}",
            kprove.display(),
            prove_rel,
            rel(workspace, &definition_dir)
        ));

        match Command::new(&kprove)
            .current_dir(workspace)
            .arg(claim_file)
            .arg("--definition")
            .arg(definition_dir)
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                passed += 1;
                passed_checks.push(format!("K proof passed: {}", spec_rel));
            }
            Ok(output) => {
                failed += 1;
                let stderr = String::from_utf8_lossy(&output.stderr);
                failed_checks.push(format!(
                    "K proof failed: {} ({})",
                    spec_rel,
                    stderr.lines().next().unwrap_or("no stderr")
                ));
            }
            Err(e) => {
                failed += 1;
                failed_checks.push(format!("K invocation error for {}: {}", spec_rel, e));
            }
        }
    }

    BackendOutcome {
        name: "k",
        passed,
        failed,
        missing: 0,
    }
}

pub async fn verify_claim(workspace: &Path, claim_id: &str, verbose: bool) -> Result<ProofResult> {
    let mut result = run_proofs(workspace, verbose).await?;
    result.claim_id = claim_id.to_string();
    result.claim = "Consensus-critical formal verification".to_string();
    Ok(result)
}

pub async fn run_proofs(workspace: &Path, verbose: bool) -> Result<ProofResult> {
    let started = Instant::now();

    let mut commands_run = Vec::new();
    let mut passed_checks = Vec::new();
    let mut failed_checks = Vec::new();
    let mut missing_proofs = Vec::new();

    let tla = run_tla_specs(
        workspace,
        &mut commands_run,
        &mut passed_checks,
        &mut failed_checks,
        &mut missing_proofs,
    )
    .await;
    let coq = run_coq_specs(
        workspace,
        &mut commands_run,
        &mut passed_checks,
        &mut failed_checks,
        &mut missing_proofs,
    )
    .await;
    let k = run_k_specs(
        workspace,
        &mut commands_run,
        &mut passed_checks,
        &mut failed_checks,
        &mut missing_proofs,
    )
    .await;

    let inspected = vec![
        "formal-proofs/tla".to_string(),
        "formal-proofs/coq".to_string(),
        "formal-proofs/k".to_string(),
    ];

    let mut evidence = HashMap::new();
    for outcome in [&tla, &coq, &k] {
        evidence.insert(
            format!("{}_passed", outcome.name),
            outcome.passed.to_string(),
        );
        evidence.insert(
            format!("{}_failed", outcome.name),
            outcome.failed.to_string(),
        );
        evidence.insert(
            format!("{}_missing", outcome.name),
            outcome.missing.to_string(),
        );
    }

    if verbose {
        passed_checks.push(format!(
            "backend summary: TLA(p={},f={},m={}) Coq(p={},f={},m={}) K(p={},f={},m={})",
            tla.passed,
            tla.failed,
            tla.missing,
            coq.passed,
            coq.failed,
            coq.missing,
            k.passed,
            k.failed,
            k.missing
        ));
    }

    let backend_failures = tla.failed + coq.failed + k.failed;
    let backend_missing = tla.missing + coq.missing + k.missing;
    let mut blockers = Vec::new();

    if backend_failures > 0 {
        blockers.push(format!(
            "S0: formal verification backend failures detected ({})",
            backend_failures
        ));
    }
    if backend_missing > 0 {
        blockers.push(format!(
            "S0: formal verification missing specs/tooling ({})",
            backend_missing
        ));
    }

    let status = if backend_failures == 0 && backend_missing == 0 {
        ProofStatus::Verified
    } else if backend_failures == 0 {
        ProofStatus::Partial
    } else {
        ProofStatus::Blocked
    };

    let total_signals = (passed_checks.len() + failed_checks.len() + missing_proofs.len()) as f64;
    let score = if total_signals > 0.0 {
        passed_checks.len() as f64 / total_signals
    } else {
        0.0
    };

    Ok(ProofResult {
        claim_id: "x3.formal_proofs.consensus_critical".to_string(),
        claim: "Consensus-critical paths formally verified".to_string(),
        status,
        proof_level: Some(ProofLevel::P7),
        edge_case_level: Some(EdgeCaseLevel::E9),
        hack_level: Some(HackLevel::H9),
        operator_level: Some(OperatorLevel::I8),
        degraded_level: Some(DegradedLevel::D7),
        files_inspected: inspected,
        commands_run,
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
