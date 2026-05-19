mod dashboard;
mod feature_proof;
mod gap_proof;
mod proof;
mod receipt;
mod registry;
mod runners;
mod scoring;
mod todo_proof;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "x3-proof")]
#[command(about = "X3 ProofForge - Executable Truth Layer for X3")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to X3 codebase
    #[arg(global = true, long, default_value = ".")]
    workspace: PathBuf,

    /// Enable verbose output
    #[arg(global = true, short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify a claim with required proof
    Verify {
        /// Claim ID (e.g., x3.bridge.replay_protection)
        #[arg(value_name = "CLAIM_ID")]
        claim: String,

        /// Run with strict mode (require all proofs)
        #[arg(short, long)]
        strict: bool,
    },

    /// Run proof for a specific area
    Prove {
        /// Area to prove (asset-kernel, bridge, consensus, etc.)
        #[arg(value_name = "AREA")]
        area: String,

        /// Strict mode
        #[arg(short, long)]
        strict: bool,

        /// Dry run (show what would execute)
        #[arg(long)]
        dry_run: bool,
    },

    /// Run all proofs
    ProveAll {
        #[arg(short, long)]
        strict: bool,

        #[arg(long)]
        dry_run: bool,

        /// Run in parallel
        #[arg(long)]
        parallel: bool,
    },

    /// Check security gates (S0/S1 blockers)
    SecurityGate {
        #[arg(short, long)]
        fail_hard: bool,
    },

    /// Test hack resistance
    Hack {
        /// Specific area to test
        #[arg(value_name = "AREA")]
        area: Option<String>,

        #[arg(short, long)]
        strict: bool,
    },

    /// Test edge cases
    EdgeCase {
        /// Area to test
        #[arg(value_name = "AREA")]
        area: Option<String>,

        #[arg(short, long)]
        strict: bool,
    },

    /// Test degraded operation (limp to finish)
    Limp {
        /// Area to test
        #[arg(value_name = "AREA")]
        area: Option<String>,

        #[arg(short, long)]
        strict: bool,
    },

    /// Test operator safety (idiot-proof mode)
    Idiot {
        /// Command to test
        #[arg(value_name = "COMMAND")]
        command: String,

        #[arg(long)]
        dry_run: bool,
    },

    /// Check formal proofs
    Formal {
        /// Area to check
        #[arg(value_name = "AREA")]
        area: Option<String>,
    },

    /// Generate proof receipt
    Receipt {
        /// Receipt type (mainnet, testnet, upgrade, etc.)
        #[arg(value_name = "TYPE")]
        receipt_type: String,

        /// Areas to include in receipt
        #[arg(value_name = "AREAS")]
        areas: Vec<String>,
    },

    /// Check mainnet readiness
    MainnetGate {
        #[arg(short, long)]
        fail_hard: bool,

        #[arg(long)]
        strict: bool,
    },

    /// Check testnet readiness
    TestnetGate {
        #[arg(short, long)]
        fail_hard: bool,
    },

    /// Generate proof dashboard export
    Dashboard {
        /// Output file
        #[arg(short, long, default_value = "proof-score.json")]
        output: PathBuf,

        /// Include detailed reports
        #[arg(long)]
        detailed: bool,
    },

    /// Scan for unproven claims
    ScanClaims {
        /// File to scan (markdown/code)
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,

        #[arg(long)]
        fail_on_unproven: bool,
    },

    /// Check AI patch safety
    AiPatchFirewall {
        /// Git diff to check
        #[arg(value_name = "DIFF")]
        diff: Option<String>,

        #[arg(long)]
        fail_hard: bool,
    },

    /// Explain blockers for an area
    ExplainBlockers {
        /// Area
        #[arg(value_name = "AREA")]
        area: String,
    },

    /// List all claims and their status
    Claims,

    /// Run ALL critical proofs and gates - MUST PASS for mainnet
    ProveEverything {
        /// Strict mode (fail on any issue)
        #[arg(short, long)]
        strict: bool,

        /// Fail hard on blockers
        #[arg(long)]
        fail_hard: bool,

        /// Generate receipts
        #[arg(long)]
        receipts: bool,
    },

    /// Scan for TODO/FIXME/HACK/stub/mock/fake code
    TodoGate {
        /// Gate to check (mainnet, testnet)
        #[arg(value_name = "GATE", default_value = "mainnet")]
        gate: String,

        /// Fail on blockers
        #[arg(long)]
        fail_hard: bool,
    },

    /// Scan for missing implementations, tests, and wiring
    GapGate {
        /// Gate to check (mainnet, testnet)
        #[arg(value_name = "GATE", default_value = "mainnet")]
        gate: String,

        /// Fail on blockers
        #[arg(long)]
        fail_hard: bool,
    },

    /// Feature built proof - verify all features are truly built
    Features {
        #[command(subcommand)]
        command: Option<FeaturesCommand>,

        /// Strict mode (fail on any non-BUILT feature)
        #[arg(short, long)]
        strict: bool,

        /// Fail on blockers
        #[arg(long)]
        fail_hard: bool,
    },
}

#[derive(Subcommand)]
enum FeaturesCommand {
    /// List all features
    List,

    /// Scan all features
    Scan,

    /// Show feature status summary
    Status,

    /// Show missing features
    Missing,

    /// Show partial features
    Partial,

    /// Show unwired features
    Unwired,

    /// Show untested features
    Untested,

    /// Show stale features
    Stale,

    /// Show blocked features
    Blockers,

    /// Generate full report
    Report,
}

// ============================================================================
// Feature Command Handlers
// ============================================================================

fn run_features_list(workspace: &PathBuf, verbose: bool) -> Result<()> {
    let scanner = feature_proof::FeatureScanner::new(workspace.clone());
    let matrix = scanner.load_matrix()?;
    println!("{}", "X3 Feature Registry".bold());
    println!();

    for feature in &matrix.features {
        println!(
            "  {} {}",
            feature.id,
            format!("({})", feature.criticality).dimmed()
        );
        if verbose {
            println!("    {}", feature.name);
        }
    }

    println!();
    println!("Total features: {}", matrix.features.len());
    Ok(())
}

fn run_features_scan(workspace: &PathBuf, verbose: bool) -> Result<()> {
    let scanner = feature_proof::FeatureScanner::new(workspace.clone());
    let report = scanner.scan(verbose)?;
    scanner.save_report(&report)?;
    println!("{}", "Feature scan complete".green());
    println!("Reports saved to proof/reports/");
    Ok(())
}

fn run_features_status(workspace: &PathBuf, _verbose: bool) -> Result<()> {
    let scanner = feature_proof::FeatureScanner::new(workspace.clone());
    let report = scanner.scan(false)?;
    println!("{}", "Feature Status Summary".bold());
    println!();
    println!("Built:     {}", report.built_count.to_string().green());
    println!("Partial:   {}", report.partial_count.to_string().yellow());
    println!("Missing:   {}", report.missing_count.to_string().red());
    println!("Unwired:   {}", report.unwired_count.to_string().yellow());
    println!("Untested:  {}", report.untested_count.to_string().yellow());
    println!("Weak:      {}", report.weak_count.to_string().yellow());
    println!("Stale:     {}", report.stale_count.to_string().yellow());
    println!("Blocked:   {}", report.blocked_count.to_string().red());
    Ok(())
}

fn run_features_missing(workspace: &PathBuf, verbose: bool) -> Result<()> {
    let scanner = feature_proof::FeatureScanner::new(workspace.clone());
    let report = scanner.scan(false)?;
    println!("{}", "Missing Features".bold().red());
    println!();
    for result in &report.results {
        if result.status == feature_proof::FeatureStatus::Missing {
            println!("  ❌ {}", result.feature_id);
            if verbose {
                for missing in &result.code_missing {
                    println!("      - {}", missing.dimmed());
                }
            }
        }
    }
    Ok(())
}

fn run_features_partial(workspace: &PathBuf, verbose: bool) -> Result<()> {
    let scanner = feature_proof::FeatureScanner::new(workspace.clone());
    let report = scanner.scan(false)?;
    println!("{}", "Partial Features".bold().yellow());
    println!();
    for result in &report.results {
        if result.status == feature_proof::FeatureStatus::Partial {
            println!("  🟡 {}", result.feature_id);
            if verbose {
                for blocker in &result.blockers {
                    println!("      - {}", blocker.dimmed());
                }
            }
        }
    }
    Ok(())
}

fn run_features_unwired(workspace: &PathBuf, verbose: bool) -> Result<()> {
    let scanner = feature_proof::FeatureScanner::new(workspace.clone());
    let report = scanner.scan(false)?;
    println!("{}", "Unwired Features".bold().yellow());
    println!();
    for result in &report.results {
        if result.status == feature_proof::FeatureStatus::Unwired {
            println!("  🔌 {}", result.feature_id);
            if verbose {
                for missing in &result.wiring_missing {
                    println!("      - {}", missing.dimmed());
                }
            }
        }
    }
    Ok(())
}

fn run_features_untested(workspace: &PathBuf, verbose: bool) -> Result<()> {
    let scanner = feature_proof::FeatureScanner::new(workspace.clone());
    let report = scanner.scan(false)?;
    println!("{}", "Untested Features".bold().yellow());
    println!();
    for result in &report.results {
        if result.status == feature_proof::FeatureStatus::Untested {
            println!("  🧪 {}", result.feature_id);
            if verbose {
                for missing in &result.tests_missing {
                    println!("      - {}", missing.dimmed());
                }
            }
        }
    }
    Ok(())
}

fn run_features_stale(workspace: &PathBuf, verbose: bool) -> Result<()> {
    let scanner = feature_proof::FeatureScanner::new(workspace.clone());
    let report = scanner.scan(false)?;
    println!("{}", "Stale Features".bold().yellow());
    println!();
    for result in &report.results {
        if result.status == feature_proof::FeatureStatus::Stale {
            println!("  🕐 {}", result.feature_id);
            if verbose {
                println!("      Receipt needs regeneration");
            }
        }
    }
    Ok(())
}

fn run_features_blockers(workspace: &PathBuf, verbose: bool) -> Result<()> {
    let scanner = feature_proof::FeatureScanner::new(workspace.clone());
    let report = scanner.scan(false)?;
    println!("{}", "Blocked Features".bold().red());
    println!();
    for result in &report.results {
        if result.status == feature_proof::FeatureStatus::Blocked {
            println!("  🚫 {}", result.feature_id);
            if verbose {
                for blocker in &result.blockers {
                    println!("      - {}", blocker.red());
                }
            }
        }
    }
    Ok(())
}

fn run_features_report(workspace: &PathBuf, _verbose: bool) -> Result<()> {
    let scanner = feature_proof::FeatureScanner::new(workspace.clone());
    let report = scanner.scan(false)?;
    scanner.save_report(&report)?;
    println!("{}", "Full Feature Report Generated".bold().green());
    println!();
    println!("  - proof/reports/feature_status.json");
    println!("  - proof/reports/features_report.md");
    println!();
    println!("Verdict: {}", report.verdict.bold());
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        println!(
            "{}",
            "X3 ProofForge v1.0.0 - Executable Truth Layer"
                .bold()
                .cyan()
        );
        println!("Workspace: {}", cli.workspace.display());
        println!();
    }

    match cli.command {
        Commands::Verify { claim, strict } => {
            runners::verify_claim(&cli.workspace, &claim, strict, cli.verbose).await?
        }

        Commands::Prove {
            area,
            strict,
            dry_run,
        } => runners::prove_area(&cli.workspace, &area, strict, dry_run, cli.verbose).await?,

        Commands::ProveAll {
            strict,
            dry_run,
            parallel,
        } => runners::prove_all(&cli.workspace, strict, dry_run, parallel, cli.verbose).await?,

        Commands::SecurityGate { fail_hard } => {
            runners::check_security_gate(&cli.workspace, fail_hard, cli.verbose).await?
        }

        Commands::Hack { area, strict } => {
            runners::test_hack_resistance(&cli.workspace, area, strict, cli.verbose).await?
        }

        Commands::EdgeCase { area, strict } => {
            runners::test_edge_cases(&cli.workspace, area, strict, cli.verbose).await?
        }

        Commands::Limp { area, strict } => {
            runners::test_limp_mode(&cli.workspace, area, strict, cli.verbose).await?
        }

        Commands::Idiot { command, dry_run } => {
            runners::test_idiot_proof(&cli.workspace, &command, dry_run, cli.verbose).await?
        }

        Commands::Formal { area } => {
            runners::check_formal_proofs(&cli.workspace, area, false, false, cli.verbose).await?
        }

        Commands::Receipt {
            receipt_type,
            areas,
        } => runners::generate_receipt(&cli.workspace, &receipt_type, &areas, cli.verbose).await?,

        Commands::MainnetGate { fail_hard, strict } => {
            runners::check_mainnet_readiness(&cli.workspace, fail_hard, strict, cli.verbose).await?
        }

        Commands::TestnetGate { fail_hard } => {
            runners::check_testnet_readiness(&cli.workspace, fail_hard, cli.verbose).await?
        }

        Commands::Dashboard { output, detailed } => {
            dashboard::generate_dashboard(&cli.workspace, &output, detailed, cli.verbose).await?
        }

        Commands::ScanClaims {
            file,
            fail_on_unproven,
        } => runners::scan_claims(&cli.workspace, file, fail_on_unproven, cli.verbose).await?,

        Commands::AiPatchFirewall { diff, fail_hard } => {
            runners::check_ai_patch(&cli.workspace, diff, fail_hard, cli.verbose).await?
        }

        Commands::ExplainBlockers { area } => {
            runners::explain_blockers(&cli.workspace, &area, cli.verbose).await?
        }

        Commands::Claims => runners::list_all_claims(&cli.workspace, cli.verbose).await?,

        Commands::ProveEverything {
            strict,
            fail_hard,
            receipts,
        } => prove_everything(&cli.workspace, strict, fail_hard, receipts, cli.verbose).await?,

        Commands::TodoGate { gate, fail_hard } => {
            run_todo_gate(&cli.workspace, &gate, fail_hard, cli.verbose).await?
        }

        Commands::GapGate { gate, fail_hard } => {
            run_gap_gate(&cli.workspace, &gate, fail_hard, cli.verbose).await?
        }

        Commands::Features {
            command,
            strict,
            fail_hard,
        } => {
            match command {
                None => {
                    // Default: run full feature gate
                    feature_proof::run_feature_gate(&cli.workspace, strict, fail_hard, cli.verbose)?;
                }
                Some(FeaturesCommand::List) => {
                    run_features_list(&cli.workspace, cli.verbose)?;
                }
                Some(FeaturesCommand::Scan) => {
                    run_features_scan(&cli.workspace, cli.verbose)?;
                }
                Some(FeaturesCommand::Status) => {
                    run_features_status(&cli.workspace, cli.verbose)?;
                }
                Some(FeaturesCommand::Missing) => {
                    run_features_missing(&cli.workspace, cli.verbose)?;
                }
                Some(FeaturesCommand::Partial) => {
                    run_features_partial(&cli.workspace, cli.verbose)?;
                }
                Some(FeaturesCommand::Unwired) => {
                    run_features_unwired(&cli.workspace, cli.verbose)?;
                }
                Some(FeaturesCommand::Untested) => {
                    run_features_untested(&cli.workspace, cli.verbose)?;
                }
                Some(FeaturesCommand::Stale) => {
                    run_features_stale(&cli.workspace, cli.verbose)?;
                }
                Some(FeaturesCommand::Blockers) => {
                    run_features_blockers(&cli.workspace, cli.verbose)?;
                }
                Some(FeaturesCommand::Report) => {
                    run_features_report(&cli.workspace, cli.verbose)?;
                }
            }
        }
    }

    Ok(())
}

/// Prove Everything - The Ultimate Gate
async fn prove_everything(
    workspace: &PathBuf,
    strict: bool,
    fail_hard: bool,
    receipts: bool,
    verbose: bool,
) -> Result<()> {
    println!(
        "{}",
        "🔥 PROVE EVERYTHING - Ultimate X3 Proof Gauntlet"
            .bold()
            .red()
    );
    println!();

    let mut all_pass = true;
    let mut failures = Vec::new();

    // 1. TodoGate
    println!("{}", "▸ Running TodoGate...".cyan());
    if let Err(e) = run_todo_gate(workspace, "mainnet", true, verbose).await {
        all_pass = false;
        failures.push(format!("TodoGate: {}", e));
    }

    // 2. GapGate
    println!("{}", "▸ Running GapGate...".cyan());
    if let Err(e) = run_gap_gate(workspace, "mainnet", true, verbose).await {
        all_pass = false;
        failures.push(format!("GapGate: {}", e));
    }

    // 3. Security Gate
    println!("{}", "▸ Running SecurityGate...".cyan());
    if let Err(e) = runners::check_security_gate(workspace, true, verbose).await {
        all_pass = false;
        failures.push(format!("SecurityGate: {}", e));
    }

    // 4. Mainnet Gate
    println!("{}", "▸ Running MainnetGate...".cyan());
    if let Err(e) = runners::check_mainnet_readiness(workspace, true, true, verbose).await {
        all_pass = false;
        failures.push(format!("MainnetGate: {}", e));
    }

    // 5. Critical Claims
    println!("{}", "▸ Verifying Critical Claims...".cyan());
    let critical_claims = vec![
        "x3.asset_kernel.supply_conservation",
        "x3.bridge.replay_protection",
        "x3.bridge.finality_verification",
        "x3.atomic.one_terminal_state",
        "x3.atomic.rollback_safety",
        "x3.flashloan.repay_or_revert",
        "x3.x3vm.determinism",
        "x3.x3lang.compiler_reproducibility",
        "x3.contracts.evm_svm_parity",
        "x3.governance.proof_gated_upgrade",
        "x3.proofforge.receipt_integrity",
    ];

    for claim in critical_claims {
        if let Err(e) = runners::verify_claim(workspace, claim, strict, false).await {
            all_pass = false;
            failures.push(format!("Claim {}: {}", claim, e));
        }
    }

    println!();
    if all_pass {
        println!(
            "{}",
            "✓ PROVE EVERYTHING PASSED - X3 is proof-ready"
                .bold()
                .green()
        );

        if receipts {
            println!("Generating master receipt...");
            runners::generate_receipt(workspace, "mainnet", &vec![], verbose).await?;
        }

        Ok(())
    } else {
        println!("{}", "✗ PROVE EVERYTHING FAILED".bold().red());
        println!();
        println!("{}", "Failures:".bold());
        for failure in &failures {
            println!("  - {}", failure.red());
        }
        println!();

        if fail_hard {
            anyhow::bail!("prove-everything failed with {} blockers", failures.len());
        }

        Ok(())
    }
}

/// Run TODO/FIXME/HACK scanner
async fn run_todo_gate(
    workspace: &PathBuf,
    gate: &str,
    fail_hard: bool,
    verbose: bool,
) -> Result<()> {
    use todo_proof::TodoScanner;

    println!(
        "{}",
        format!("📋 TODO Gate: {} readiness", gate).bold().yellow()
    );
    println!();

    let scanner = TodoScanner::new(workspace.clone());
    let report = scanner.scan(verbose)?;

    println!("Total TODOs found: {}", report.total_todos);
    println!();

    println!("By severity:");
    for (severity, count) in &report.by_severity {
        println!("  {}: {}", severity, count);
    }
    println!();

    let mainnet_blockers = report.mainnet_blockers.len();
    let testnet_blockers = report.testnet_blockers.len();

    println!(
        "Mainnet blockers (T5+): {}",
        if mainnet_blockers > 0 {
            mainnet_blockers.to_string().red()
        } else {
            mainnet_blockers.to_string().green()
        }
    );

    println!(
        "Testnet blockers (T6+): {}",
        if testnet_blockers > 0 {
            testnet_blockers.to_string().red()
        } else {
            testnet_blockers.to_string().green()
        }
    );

    println!();

    if !report.mainnet_blockers.is_empty() && verbose {
        println!("{}", "Mainnet Blockers:".bold().red());
        for item in &report.mainnet_blockers {
            println!(
                "  {} (line {}) - {:?}",
                item.file.display(),
                item.line,
                item.severity
            );
            println!("    {}", item.content);
        }
        println!();
    }

    // Check gate
    let passes = scanner.check_gates(&report, gate)?;

    if passes {
        println!("{}", format!("✓ {} gate PASSED", gate).bold().green());
        Ok(())
    } else {
        println!("{}", format!("✗ {} gate FAILED", gate).bold().red());

        if fail_hard {
            anyhow::bail!(
                "{} gate failed: {} blockers found",
                gate,
                if gate == "mainnet" {
                    mainnet_blockers
                } else {
                    testnet_blockers
                }
            );
        }

        Ok(())
    }
}

/// Run Gap scanner
async fn run_gap_gate(
    workspace: &PathBuf,
    gate: &str,
    fail_hard: bool,
    verbose: bool,
) -> Result<()> {
    use gap_proof::GapScanner;

    println!(
        "{}",
        format!("🔍 Gap Gate: {} readiness", gate).bold().yellow()
    );
    println!();

    let scanner = GapScanner::new(workspace.clone());
    let report = scanner.scan(verbose)?;

    println!("Total gaps found: {}", report.total_gaps);
    println!();

    println!("By type:");
    for (gap_type, count) in &report.by_type {
        println!("  {}: {}", gap_type, count);
    }
    println!();

    let s0_gaps = report.s0_gaps.len();
    let mainnet_blockers = report.mainnet_blockers.len();
    let testnet_blockers = report.testnet_blockers.len();

    println!(
        "S0 gaps (critical): {}",
        if s0_gaps > 0 {
            s0_gaps.to_string().red()
        } else {
            s0_gaps.to_string().green()
        }
    );

    println!(
        "Mainnet blockers: {}",
        if mainnet_blockers > 0 {
            mainnet_blockers.to_string().red()
        } else {
            mainnet_blockers.to_string().green()
        }
    );

    println!(
        "Testnet blockers: {}",
        if testnet_blockers > 0 {
            testnet_blockers.to_string().red()
        } else {
            testnet_blockers.to_string().green()
        }
    );

    println!();

    if !report.s0_gaps.is_empty() && verbose {
        println!("{}", "S0 Gaps (CRITICAL):".bold().red());
        for item in &report.s0_gaps {
            println!(
                "  [{}] {}: {}",
                item.area,
                format!("{:?}", item.gap_type).red(),
                item.description
            );
        }
        println!();
    }

    // Check gate
    let passes = scanner.check_gates(&report, gate)?;

    if passes {
        println!("{}", format!("✓ {} gate PASSED", gate).bold().green());
        Ok(())
    } else {
        println!("{}", format!("✗ {} gate FAILED", gate).bold().red());

        if fail_hard {
            anyhow::bail!(
                "{} gate failed: {} S0 gaps, {} blockers",
                gate,
                s0_gaps,
                mainnet_blockers
            );
        }

        Ok(())
    }
}
