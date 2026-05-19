//! x3-prove — X3 Phase 1 testnet proving harness.
//!
//! Probes each chain concurrently with six RPC checks, computes per-chain
//! metrics, prints a human-readable table, and writes a machine-readable
//! JSON compatibility scorecard.  Exits 0 when all chains pass their
//! configured thresholds, exits 1 otherwise.
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc)] // binary entry point; errors printed & exit 2

mod checks;
mod report;

use std::{path::PathBuf, process};

use anyhow::Result;
use chrono::Utc;
use clap::Parser;

use checks::{
    chain_config_for, check_bundle_construction, check_quoting, check_reconciliation,
    check_rollback, check_state_verification, check_submission,
};
use report::{ChainReport, ChainStatus, CompatibilityMatrix, HarnessSummary, ThresholdConfig};

// ─────────────────────────────────────────────────────────────────────────────
// CLI definition
// ─────────────────────────────────────────────────────────────────────────────

/// X3 Phase-1 testnet proving harness.
///
/// Probes each chain concurrently with six RPC checks and emits a
/// machine-readable JSON compatibility scorecard.
/// Exits 0 when all chains pass their thresholds, 1 otherwise.
#[derive(Parser, Debug)]
#[command(name = "x3-prove", version = "0.1.0")]
struct Args {
    /// Comma-separated list of chain IDs to probe.
    #[arg(long, default_value = "x3-native,evm-testnet,svm-testnet")]
    chains: String,

    /// Output path for the JSON compatibility matrix.
    #[arg(long, default_value = "proof/reports/compatibility_matrix.json")]
    output: PathBuf,

    /// Minimum bundle-construction success rate required to mark a chain Passing.
    #[arg(long, default_value_t = 0.95)]
    threshold_bundle_success: f64,

    /// Minimum rollback-correctness rate required to mark a chain Passing.
    #[arg(long, default_value_t = 1.0)]
    threshold_rollback: f64,

    /// Minimum reconciliation-accuracy rate required to mark a chain Passing.
    #[arg(long, default_value_t = 0.99)]
    threshold_reconciliation: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let thresholds = ThresholdConfig {
        bundle_success: args.threshold_bundle_success,
        rollback: args.threshold_rollback,
        reconciliation: args.threshold_reconciliation,
    };

    // Parse chain list — ignore empty segments from trailing commas.
    let chain_ids: Vec<&str> = args
        .chains
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let run_id = format!("x3-prove-{}", Utc::now().timestamp_millis());
    let generated_at = Utc::now().to_rfc3339();

    println!("X3 PROVING HARNESS — Phase 1 Compatibility Run");
    println!("Run ID    : {run_id}");
    println!("Generated : {generated_at}");
    println!(
        "Thresholds: bundle≥{:.0}%  rollback≥{:.0}%  reconciliation≥{:.0}%",
        thresholds.bundle_success * 100.0,
        thresholds.rollback * 100.0,
        thresholds.reconciliation * 100.0
    );
    println!("Chains    : {}", chain_ids.join(", "));
    println!();

    // ── Probe each chain, running all 6 checks concurrently per chain ────────
    let mut chain_reports: Vec<ChainReport> = Vec::with_capacity(chain_ids.len());

    for &chain_id in &chain_ids {
        let config = chain_config_for(chain_id);
        eprintln!("  probing {} ({}) …", config.chain_id, config.rpc_url);

        // All six checks run concurrently inside a single async task.
        let (r_quoting, r_bundle, r_submit, r_rollback, r_recon, r_state) = tokio::join!(
            check_quoting(&config),
            check_bundle_construction(&config),
            check_submission(&config),
            check_rollback(&config),
            check_reconciliation(&config),
            check_state_verification(&config),
        );

        // Extract the three metric scores before moving the results into the vec.
        let bundle_success_rate = r_bundle.score;
        let rollback_correctness = r_rollback.score;
        let reconciliation_accuracy = r_recon.score;

        let all_checks = vec![
            r_quoting,
            r_bundle,
            r_submit,
            r_rollback,
            r_recon,
            r_state,
        ];

        // Determine per-chain status from metrics vs. thresholds.
        let all_failed = all_checks.iter().all(|c| !c.passed);

        let overall_status = if all_failed {
            ChainStatus::Unsupported
        } else if bundle_success_rate >= thresholds.bundle_success
            && rollback_correctness >= thresholds.rollback
            && reconciliation_accuracy >= thresholds.reconciliation
        {
            ChainStatus::Passing
        } else {
            ChainStatus::Degraded
        };

        chain_reports.push(ChainReport {
            chain_id: chain_id.to_string(),
            checks: all_checks,
            bundle_success_rate,
            rollback_correctness,
            reconciliation_accuracy,
            overall_status,
        });
    }

    // ── Human-readable table ─────────────────────────────────────────────────
    print_table(&chain_reports);

    // ── Cross-chain summary ──────────────────────────────────────────────────
    let passing = chain_reports
        .iter()
        .filter(|r| matches!(r.overall_status, ChainStatus::Passing))
        .count();
    let degraded = chain_reports
        .iter()
        .filter(|r| matches!(r.overall_status, ChainStatus::Degraded))
        .count();
    let unsupported = chain_reports
        .iter()
        .filter(|r| matches!(r.overall_status, ChainStatus::Unsupported))
        .count();
    let total_chains = chain_reports.len();
    // overall_pass requires at least one chain and all chains Passing.
    let overall_pass = total_chains > 0 && passing == total_chains;

    println!();
    println!(
        "Summary : {passing}/{total_chains} chains Passing  \
         ({degraded} Degraded, {unsupported} Unsupported)"
    );
    println!("Overall : {}", if overall_pass { "PASS" } else { "FAIL" });

    // ── Write JSON scorecard ─────────────────────────────────────────────────
    let matrix = CompatibilityMatrix {
        run_id,
        generated_at,
        thresholds,
        chains: chain_reports,
        summary: HarnessSummary {
            total_chains,
            passing,
            degraded,
            unsupported,
            overall_pass,
        },
    };

    let json_output = serde_json::to_string_pretty(&matrix)?;

    // Create parent directories if they do not yet exist.
    if let Some(parent) = args.output.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    tokio::fs::write(&args.output, json_output.as_bytes()).await?;

    println!();
    println!("Scorecard written to: {}", args.output.display());

    if !overall_pass {
        process::exit(1);
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Table printer
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a boolean pass result to a fixed-width label.
fn pass_label(passed: bool) -> &'static str {
    if passed {
        "PASS"
    } else {
        "FAIL"
    }
}

/// Print a human-readable tabular summary of all chain probes to stdout.
fn print_table(reports: &[ChainReport]) {
    // Column widths chosen to fit the widest expected values.
    const SEP: &str =
        "─────────────────────────────────────────────────────────────────────────────────────────────────────────";

    println!("{SEP}");
    println!(
        "{:<15}  {:<8} {:<8} {:<8} {:<9} {:<8} {:<8}  {:>8} {:>10} {:>8}   {}",
        "Chain",
        "Quoting",
        "Bundle",
        "Submit",
        "Rollback",
        "Recon",
        "State",
        "Bundle%",
        "Rollback%",
        "Recon%",
        "Status"
    );
    println!("{SEP}");

    for report in reports {
        let c = &report.checks;
        // Use `get` so the function is safe even if the vec is shorter than expected.
        let get = |i: usize| c.get(i).map_or(false, |r| r.passed);

        let status_str = match report.overall_status {
            ChainStatus::Passing => "Passing",
            ChainStatus::Degraded => "Degraded",
            ChainStatus::Unsupported => "Unsupported",
        };

        println!(
            "{:<15}  {:<8} {:<8} {:<8} {:<9} {:<8} {:<8}  {:>7.0}% {:>9.0}% {:>7.0}%   {}",
            report.chain_id,
            pass_label(get(0)), // quoting
            pass_label(get(1)), // bundle_construction
            pass_label(get(2)), // submission
            pass_label(get(3)), // rollback
            pass_label(get(4)), // reconciliation
            pass_label(get(5)), // state_verification
            report.bundle_success_rate * 100.0,
            report.rollback_correctness * 100.0,
            report.reconciliation_accuracy * 100.0,
            status_str,
        );
    }

    println!("{SEP}");
}
