use crate::proof::*;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// Verify operational / evidence-based claims:
///
///   onboarding.developer_first_value  — dev quickstart path exists and is measurable
///   funding.milestone_receipts        — funding asks map to milestones with deliverables
///   evolution.no_regression           — S0/S1 claim registry shows no new regressions
pub async fn verify_claim(workspace: &Path, claim_id: &str, verbose: bool) -> Result<ProofResult> {
    if claim_id.contains("onboarding") {
        verify_onboarding(workspace, claim_id, verbose).await
    } else if claim_id.contains("funding") {
        verify_funding(workspace, claim_id, verbose).await
    } else if claim_id.contains("evolution") {
        verify_evolution(workspace, claim_id, verbose).await
    } else if claim_id.contains("observability") {
        verify_observability(workspace, claim_id, verbose).await
    } else {
        // Shouldn't happen, but handle gracefully
        Ok(unrecognized_claim(claim_id))
    }
}

async fn verify_observability(
    workspace: &Path,
    claim_id: &str,
    verbose: bool,
) -> Result<ProofResult> {
    let start = Instant::now();

    if verbose {
        println!("  → Checking observability telemetry evidence...");
    }

    let mut files_inspected = vec![];
    let mut passed_checks = vec![];
    let mut failed_checks = vec![];
    let mut missing_proofs = vec![];
    let mut evidence = HashMap::new();

    let required = [
        (
            "docs/testnet-config/prometheus-config.json",
            "Prometheus scrape config present",
        ),
        (
            "docs/testnet-config/grafana-dashboards.json",
            "Grafana dashboard config present",
        ),
        (
            "crates/x3-indexer/src/metrics.rs",
            "Indexer metrics surface implemented",
        ),
        (
            "crates/x3-sidecar/src/telemetry.rs",
            "Sidecar telemetry surface implemented",
        ),
        (
            "tests_phase4/x3_operator/test_telemetry.py",
            "Operator telemetry test exists",
        ),
    ];

    for (rel, label) in required {
        files_inspected.push(rel.to_string());
        let exists = workspace.join(rel).exists();
        evidence.insert(rel.to_string(), exists.to_string());
        if exists {
            passed_checks.push(label.to_string());
        } else {
            failed_checks.push(format!("Missing observability artifact: {}", rel));
            missing_proofs.push(format!("Add {}", rel));
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
        claim: "Observability telemetry and dashboards are configured with executable evidence"
            .to_string(),
        status,
        proof_level: None,
        edge_case_level: None,
        hack_level: None,
        operator_level: None,
        degraded_level: None,
        files_inspected,
        commands_run: vec![],
        passed_checks,
        failed_checks,
        missing_proofs,
        blockers: vec![],
        score,
        evidence,
        timestamp: Utc::now(),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

async fn verify_onboarding(workspace: &Path, claim_id: &str, verbose: bool) -> Result<ProofResult> {
    let start = Instant::now();

    if verbose {
        println!("  → Checking developer onboarding evidence...");
    }

    let mut files_inspected = vec![];
    let mut passed_checks = vec![];
    let mut failed_checks = vec![];
    let mut missing_proofs = vec![];
    let mut evidence = HashMap::new();

    // Quick-start guide
    let quickstart_candidates = [
        "MAINNET_QUICK_START.md",
        "OPTION_D_LAUNCH_GUIDE.md",
        "docs/BLOCKCHAIN_STARTUP_GUIDE.md",
        "README.md",
    ];
    let mut found_quickstart = false;
    for candidate in &quickstart_candidates {
        if workspace.join(candidate).exists() {
            files_inspected.push(candidate.to_string());
            if !found_quickstart {
                passed_checks.push(format!("Quickstart guide found: {}", candidate));
                evidence.insert("quickstart_doc".to_string(), candidate.to_string());
                found_quickstart = true;
            }
        }
    }
    if !found_quickstart {
        failed_checks.push("No developer quickstart guide found".to_string());
        missing_proofs
            .push("Add QUICKSTART.md or README with time-to-first-value steps".to_string());
    }

    // Makefile or script with first-run target
    if workspace.join("Makefile").exists() {
        files_inspected.push("Makefile".to_string());
        passed_checks.push("Makefile present (dev entry point)".to_string());
        evidence.insert("makefile".to_string(), "present".to_string());
    } else {
        missing_proofs
            .push("No Makefile — hard for a developer to know what to run first".to_string());
    }

    // Chain node binary is buildable (check Cargo workspace references it)
    let cargo_toml = workspace.join("Cargo.toml");
    if cargo_toml.exists() {
        files_inspected.push("Cargo.toml".to_string());
        // Check if node member is listed
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("node") || content.contains("x3-chain") {
                passed_checks.push("Node crate referenced in workspace Cargo.toml".to_string());
            } else {
                missing_proofs
                    .push("Node binary not clearly referenced in root Cargo.toml".to_string());
            }
        }
    }

    // Measure: time-to-first-value is S1 — we can only prove the DOCUMENTATION exists,
    // not the measured time. Flag as partial if we have docs but no benchmark.
    // ----------------------------------------------------------------------
    // Real time-to-first-value gate.
    //
    // The measurement is produced by `scripts/onboarding/measure_ttfv.sh`,
    // which times an actual fresh-clone first-value flow (build + two
    // verify calls) and writes a structured benchmark to
    //   proof/onboarding/ttfv_benchmark.json.
    // This runner reads that file and asserts:
    //   1. it parses,
    //   2. every step succeeded,
    //   3. total_seconds <= budget_seconds (within_budget == true),
    //   4. it was measured within TTFV_FRESHNESS_DAYS (default 30) days.
    // ----------------------------------------------------------------------
    let ttfv_path = workspace.join("proof/onboarding/ttfv_benchmark.json");
    if ttfv_path.exists() {
        files_inspected.push("proof/onboarding/ttfv_benchmark.json".to_string());
        match std::fs::read_to_string(&ttfv_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        {
            Some(json) => {
                let total_seconds = json
                    .get("total_seconds")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(f64::INFINITY);
                let budget_seconds = json
                    .get("budget_seconds")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let within_budget = json
                    .get("within_budget")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let passed = json
                    .get("passed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let measured_at = json
                    .get("measured_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                evidence.insert(
                    "ttfv_total_seconds".to_string(),
                    format!("{:.3}", total_seconds),
                );
                evidence.insert(
                    "ttfv_budget_seconds".to_string(),
                    format!("{:.0}", budget_seconds),
                );
                evidence.insert("ttfv_measured_at".to_string(), measured_at.clone());

                // Freshness: parse as RFC3339 and require <= 30 days old.
                let freshness_days: i64 = std::env::var("TTFV_FRESHNESS_DAYS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30);
                let fresh = chrono::DateTime::parse_from_rfc3339(&measured_at)
                    .map(|t| {
                        let age = Utc::now().signed_duration_since(t.with_timezone(&Utc));
                        age.num_days() <= freshness_days
                    })
                    .unwrap_or(false);

                if passed {
                    passed_checks.push("All TTFV benchmark steps succeeded".to_string());
                } else {
                    failed_checks.push("TTFV benchmark recorded a failed step".to_string());
                }
                if within_budget {
                    passed_checks.push(format!(
                        "TTFV {:.2}s ≤ budget {:.0}s",
                        total_seconds, budget_seconds
                    ));
                } else {
                    failed_checks.push(format!(
                        "TTFV {:.2}s exceeds budget {:.0}s",
                        total_seconds, budget_seconds
                    ));
                }
                if fresh {
                    passed_checks.push(format!(
                        "TTFV benchmark fresh (≤ {} days, measured {})",
                        freshness_days, measured_at
                    ));
                } else {
                    missing_proofs.push(format!(
                        "TTFV benchmark stale (> {} days old or unparseable timestamp: '{}'); rerun scripts/onboarding/measure_ttfv.sh",
                        freshness_days, measured_at
                    ));
                }
            }
            None => {
                failed_checks.push(
                    "proof/onboarding/ttfv_benchmark.json exists but did not parse as JSON"
                        .to_string(),
                );
            }
        }
    } else {
        missing_proofs.push(
            "No measured time-to-first-value benchmark — \
             run scripts/onboarding/measure_ttfv.sh to populate proof/onboarding/ttfv_benchmark.json"
                .to_string(),
        );
    }

    let total_checks = passed_checks.len() + failed_checks.len();
    let score = if total_checks == 0 {
        0.0
    } else {
        passed_checks.len() as f64 / total_checks as f64
    };

    let status = if !failed_checks.is_empty() {
        ProofStatus::Unverified
    } else if missing_proofs.is_empty() {
        ProofStatus::Verified
    } else {
        ProofStatus::Partial
    };

    Ok(ProofResult {
        claim_id: claim_id.to_string(),
        claim:
            "A fresh developer can deploy and test a first X3 app with measured time-to-first-value"
                .to_string(),
        status,
        proof_level: None,
        edge_case_level: None,
        hack_level: None,
        operator_level: None,
        degraded_level: None,
        files_inspected,
        commands_run: vec![],
        passed_checks,
        failed_checks,
        missing_proofs,
        blockers: vec![],
        score,
        evidence,
        timestamp: Utc::now(),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

async fn verify_funding(workspace: &Path, claim_id: &str, verbose: bool) -> Result<ProofResult> {
    let start = Instant::now();

    if verbose {
        println!("  → Checking funding milestone evidence...");
    }

    let mut files_inspected = vec![];
    let mut passed_checks = vec![];
    let mut failed_checks = vec![];
    let mut missing_proofs = vec![];
    let mut evidence = HashMap::new();

    // Look for milestone tracking files
    let milestone_candidates = [
        "docs/ATLAS_SPHERE_ROADMAP.md",
        "docs/BUILD_PHASES.md",
        "MASTER_STATUS.md",
        "PHASE_1_2_KICKOFF.md",
    ];
    let mut milestone_count = 0;
    for candidate in &milestone_candidates {
        if workspace.join(candidate).exists() {
            files_inspected.push(candidate.to_string());
            milestone_count += 1;
        }
    }
    evidence.insert("milestone_docs".to_string(), milestone_count.to_string());
    if milestone_count > 0 {
        passed_checks.push(format!(
            "{} milestone/phase documents found",
            milestone_count
        ));
    } else {
        failed_checks.push("No milestone tracking documents found".to_string());
    }

    // Look for any ProofForge receipts — they serve as proof of deliverable completion
    let receipts_dir = workspace.join("proof/receipts/claims");
    if receipts_dir.exists() {
        let receipt_count = std::fs::read_dir(&receipts_dir)
            .map(|rd| rd.filter_map(|e| e.ok()).count())
            .unwrap_or(0);
        evidence.insert("proof_receipts".to_string(), receipt_count.to_string());
        if receipt_count > 0 {
            passed_checks.push(format!(
                "{} proof receipts exist as deliverable evidence",
                receipt_count
            ));
            files_inspected.push("proof/receipts/claims/".to_string());
        } else {
            missing_proofs
                .push("No proof receipts — no machine-verifiable deliverable evidence".to_string());
        }
    } else {
        missing_proofs.push("proof/receipts/claims/ directory missing".to_string());
    }

    // Hard gate: parse the explicit funding→milestone→receipt linkage file and
    // assert every referenced receipt actually exists and is `verified`.
    let map_path = workspace.join("proof/funding/milestone-receipt-map.yml");
    if !map_path.exists() {
        missing_proofs.push(
            "No proof/funding/milestone-receipt-map.yml — \
             create a file linking each funding ask to a milestone ID, deliverable, \
             budget and a proof-forge receipt"
                .to_string(),
        );
    } else {
        files_inspected.push("proof/funding/milestone-receipt-map.yml".to_string());
        match std::fs::read_to_string(&map_path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_yaml::from_str::<serde_yaml::Value>(&s).map_err(|e| e.to_string()))
        {
            Err(e) => {
                failed_checks.push(format!(
                    "proof/funding/milestone-receipt-map.yml failed to parse: {}",
                    e
                ));
            }
            Ok(doc) => {
                let milestones = doc.get("milestones").and_then(|v| v.as_sequence());
                match milestones {
                    None => failed_checks.push(
                        "milestone-receipt-map.yml missing top-level `milestones:` sequence"
                            .to_string(),
                    ),
                    Some(items) if items.is_empty() => {
                        failed_checks.push(
                            "milestone-receipt-map.yml has zero milestones — at least one \
                             funding ask must be mapped"
                                .to_string(),
                        );
                    }
                    Some(items) => {
                        let mut seen_ids = std::collections::HashSet::new();
                        let mut total_budget: u64 = 0;
                        let mut bad_items = 0usize;
                        let mut verified_receipt_count = 0usize;
                        for (idx, m) in items.iter().enumerate() {
                            let id = m.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let deliverable =
                                m.get("deliverable").and_then(|v| v.as_str()).unwrap_or("");
                            let budget = m
                                .get("funding_ask_usd")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            let receipt_rel =
                                m.get("receipt").and_then(|v| v.as_str()).unwrap_or("");
                            if id.is_empty() || deliverable.is_empty() || receipt_rel.is_empty() {
                                failed_checks.push(format!(
                                    "milestones[{}]: missing required field (id/deliverable/receipt)",
                                    idx
                                ));
                                bad_items += 1;
                                continue;
                            }
                            if !seen_ids.insert(id.to_string()) {
                                failed_checks
                                    .push(format!("milestones[{}]: duplicate id `{}`", idx, id));
                                bad_items += 1;
                                continue;
                            }
                            if budget == 0 {
                                failed_checks.push(format!(
                                    "milestones[{}] (`{}`): funding_ask_usd must be > 0",
                                    idx, id
                                ));
                                bad_items += 1;
                                continue;
                            }
                            let receipt_path = workspace.join(receipt_rel);
                            if !receipt_path.exists() {
                                failed_checks.push(format!(
                                    "milestones[{}] (`{}`): receipt `{}` does not exist",
                                    idx, id, receipt_rel
                                ));
                                bad_items += 1;
                                continue;
                            }
                            // The receipt must itself be `verified`.
                            let receipt_status = std::fs::read_to_string(&receipt_path)
                                .ok()
                                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                                .and_then(|j| {
                                    j.get("result")
                                        .and_then(|r| r.get("status"))
                                        .and_then(|s| s.as_str())
                                        .map(|s| s.to_string())
                                });
                            match receipt_status.as_deref() {
                                Some("verified") => {
                                    verified_receipt_count += 1;
                                    total_budget = total_budget.saturating_add(budget);
                                }
                                Some(other) => {
                                    failed_checks.push(format!(
                                        "milestones[{}] (`{}`): receipt status is `{}`, not `verified`",
                                        idx, id, other
                                    ));
                                    bad_items += 1;
                                }
                                None => {
                                    failed_checks.push(format!(
                                        "milestones[{}] (`{}`): receipt `{}` has no result.status field",
                                        idx, id, receipt_rel
                                    ));
                                    bad_items += 1;
                                }
                            }
                        }
                        evidence.insert(
                            "funding_milestones_total".to_string(),
                            items.len().to_string(),
                        );
                        evidence.insert(
                            "funding_milestones_verified".to_string(),
                            verified_receipt_count.to_string(),
                        );
                        evidence.insert(
                            "funding_total_budget_usd".to_string(),
                            total_budget.to_string(),
                        );
                        if bad_items == 0 {
                            passed_checks.push(format!(
                                "All {} funding milestones map to existing verified receipts",
                                items.len()
                            ));
                            passed_checks
                                .push(format!("Funding map total budget = ${} USD", total_budget));
                        }
                    }
                }
            }
        }
    }

    let total_checks = passed_checks.len() + failed_checks.len();
    let score = if total_checks == 0 {
        0.0
    } else {
        passed_checks.len() as f64 / total_checks as f64
    };

    let status = if !failed_checks.is_empty() {
        ProofStatus::Unverified
    } else if missing_proofs.is_empty() {
        ProofStatus::Verified
    } else {
        ProofStatus::Partial
    };

    Ok(ProofResult {
        claim_id: claim_id.to_string(),
        claim: "Every funding ask maps to a milestone, deliverable, budget, and proof receipt"
            .to_string(),
        status,
        proof_level: None,
        edge_case_level: None,
        hack_level: None,
        operator_level: None,
        degraded_level: None,
        files_inspected,
        commands_run: vec![],
        passed_checks,
        failed_checks,
        missing_proofs,
        blockers: vec![],
        score,
        evidence,
        timestamp: Utc::now(),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

async fn verify_evolution(workspace: &Path, claim_id: &str, verbose: bool) -> Result<ProofResult> {
    let start = Instant::now();

    if verbose {
        println!("  → Checking evolution / no-regression evidence...");
    }

    let mut files_inspected = vec![];
    let mut passed_checks = vec![];
    let mut failed_checks = vec![];
    let mut missing_proofs = vec![];
    let mut evidence = HashMap::new();

    // Read the claims registry to check S0/S1 status
    let registry = workspace.join("proof/claims/registry.yml");
    if !registry.exists() {
        return Ok(unrecognized_claim(claim_id));
    }
    files_inspected.push("proof/claims/registry.yml".to_string());

    let registry_content = std::fs::read_to_string(&registry)?;

    // Count S0/S1 claims and how many are VERIFIED vs UNVERIFIED
    let mut s0_total = 0usize;
    let mut s0_verified = 0usize;
    let mut s1_total = 0usize;
    let mut s1_verified = 0usize;

    let mut current_criticality = "";
    for line in registry_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("criticality: S0") {
            current_criticality = "S0";
        } else if trimmed.starts_with("criticality: S1") {
            current_criticality = "S1";
        } else if trimmed.starts_with("status:") {
            let status_val = trimmed.trim_start_matches("status:").trim();
            match current_criticality {
                "S0" => {
                    s0_total += 1;
                    if status_val == "VERIFIED" {
                        s0_verified += 1;
                    }
                }
                "S1" => {
                    s1_total += 1;
                    if status_val == "VERIFIED" {
                        s1_verified += 1;
                    }
                }
                _ => {}
            }
            current_criticality = "";
        }
    }

    evidence.insert("s0_total".to_string(), s0_total.to_string());
    evidence.insert("s0_verified".to_string(), s0_verified.to_string());
    evidence.insert("s1_total".to_string(), s1_total.to_string());
    evidence.insert("s1_verified".to_string(), s1_verified.to_string());

    let s0_unverified = s0_total.saturating_sub(s0_verified);
    let s1_unverified = s1_total.saturating_sub(s1_verified);

    if s0_unverified == 0 {
        passed_checks.push(format!(
            "All {} S0 claims verified — no S0 regressions",
            s0_total
        ));
    } else {
        failed_checks.push(format!(
            "{}/{} S0 claims are unverified — S0 regressions present",
            s0_unverified, s0_total
        ));
    }

    if s1_unverified == 0 {
        passed_checks.push(format!(
            "All {} S1 claims verified — no S1 regressions",
            s1_total
        ));
    } else {
        missing_proofs.push(format!(
            "{}/{} S1 claims are unverified",
            s1_unverified, s1_total
        ));
    }

    // Check evolution pallet exists
    if workspace.join("pallets/evolution-core/src/lib.rs").exists() {
        files_inspected.push("pallets/evolution-core/src/lib.rs".to_string());
        passed_checks.push("evolution-core pallet exists".to_string());
    } else {
        missing_proofs.push(
            "pallets/evolution-core/ not found — no automated regression detection pallet"
                .to_string(),
        );
    }

    // ----------------------------------------------------------------------
    // Real executable no-regression gate.
    //
    // For every prior claim receipt under proof/receipts/claims/, compare the
    // current score against a persisted baseline at
    //   proof/baselines/claim_scores.yml
    // If a claim's score has dropped below its baseline, that is a regression
    // and this check fails. Missing baselines are bootstrapped at current
    // score (first-run pin) but flagged so the bootstrap is auditable.
    //
    // The own receipt for x3.evolution.no_regression is excluded to avoid
    // self-reference loops.
    // ----------------------------------------------------------------------
    let receipts_dir = workspace.join("proof/receipts/claims");
    let baseline_path = workspace.join("proof/baselines/claim_scores.yml");
    files_inspected.push("proof/baselines/claim_scores.yml".to_string());

    let mut baseline: HashMap<String, f64> = HashMap::new();
    if baseline_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&baseline_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once(':') {
                    if let Ok(score) = v.trim().parse::<f64>() {
                        baseline.insert(k.trim().to_string(), score);
                    }
                }
            }
        }
    }

    let mut current_scores: HashMap<String, f64> = HashMap::new();
    let mut receipts_seen = 0usize;
    if receipts_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&receipts_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
                    continue;
                };
                let Some(cid) = json
                    .get("result")
                    .and_then(|r| r.get("claim_id"))
                    .and_then(|v| v.as_str())
                else {
                    continue;
                };
                if cid == claim_id {
                    continue; // skip self
                }
                let Some(score) = json
                    .get("result")
                    .and_then(|r| r.get("score"))
                    .and_then(|v| v.as_f64())
                else {
                    continue;
                };
                current_scores.insert(cid.to_string(), score);
                receipts_seen += 1;
            }
        }
    }

    evidence.insert("receipts_compared".to_string(), receipts_seen.to_string());
    evidence.insert("baseline_entries".to_string(), baseline.len().to_string());

    let mut regressions: Vec<String> = vec![];
    let mut new_pins: Vec<(String, f64)> = vec![];
    // Tolerance: treat tiny floating-point differences as no-change.
    const EPS: f64 = 1e-9;
    for (cid, current) in &current_scores {
        match baseline.get(cid) {
            Some(prev) => {
                if *current + EPS < *prev {
                    regressions.push(format!(
                        "{}: score regressed {:.3} -> {:.3}",
                        cid, prev, current
                    ));
                }
            }
            None => {
                new_pins.push((cid.clone(), *current));
            }
        }
    }

    if regressions.is_empty() {
        passed_checks.push(format!(
            "No score regressions across {} claim receipts (vs {} baseline pins)",
            receipts_seen,
            baseline.len()
        ));
    } else {
        for r in &regressions {
            failed_checks.push(r.clone());
        }
    }

    // Persist (or bootstrap) the baseline so the next run has something to
    // compare against. We only ever raise the floor: take max(prev, current)
    // for known claims, and pin first-seen claims at their current score.
    if !current_scores.is_empty() {
        let mut merged: Vec<(String, f64)> =
            baseline.iter().map(|(k, v)| (k.clone(), *v)).collect();
        for (cid, score) in &current_scores {
            if let Some(entry) = merged.iter_mut().find(|(k, _)| k == cid) {
                entry.1 = entry.1.max(*score);
            } else {
                merged.push((cid.clone(), *score));
            }
        }
        merged.sort_by(|a, b| a.0.cmp(&b.0));
        if let Some(parent) = baseline_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut out = String::new();
        out.push_str("# Auto-maintained by proof-forge / evolution.no_regression runner.\n");
        out.push_str("# Each line pins the historical floor score for a claim. Scores can\n");
        out.push_str("# only ratchet upward; any drop becomes a regression failure.\n");
        for (k, v) in &merged {
            out.push_str(&format!("{}: {:.6}\n", k, v));
        }
        let _ = std::fs::write(&baseline_path, out);
    }

    if !new_pins.is_empty() {
        missing_proofs.push(format!(
            "{} new claim receipt(s) bootstrapped into baseline this run (auditable; future runs will gate against them)",
            new_pins.len()
        ));
    }

    let total_checks = passed_checks.len() + failed_checks.len();
    let score = if total_checks == 0 {
        0.0
    } else {
        passed_checks.len() as f64 / total_checks as f64
    };

    let status = if !failed_checks.is_empty() {
        ProofStatus::Unverified
    } else if missing_proofs.is_empty() {
        ProofStatus::Verified
    } else {
        ProofStatus::Partial
    };

    Ok(ProofResult {
        claim_id: claim_id.to_string(),
        claim: "New generations must beat prior generations without S0/S1 regressions".to_string(),
        status,
        proof_level: None,
        edge_case_level: None,
        hack_level: None,
        operator_level: None,
        degraded_level: None,
        files_inspected,
        commands_run: vec![],
        passed_checks,
        failed_checks,
        missing_proofs,
        blockers: vec![],
        score,
        evidence,
        timestamp: Utc::now(),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn unrecognized_claim(claim_id: &str) -> ProofResult {
    ProofResult {
        claim_id: claim_id.to_string(),
        claim: "Unrecognized operational claim".to_string(),
        status: ProofStatus::Unverified,
        proof_level: None,
        edge_case_level: None,
        hack_level: None,
        operator_level: None,
        degraded_level: None,
        files_inspected: vec![],
        commands_run: vec![],
        passed_checks: vec![],
        failed_checks: vec!["Claim ID not handled by operational runner".to_string()],
        missing_proofs: vec![],
        blockers: vec!["Operational runner does not handle this claim".to_string()],
        score: 0.0,
        evidence: HashMap::new(),
        timestamp: Utc::now(),
        duration_ms: 0,
    }
}
