//! X3 Autonomic CLI Binary
//! 
//! Main entry point for the x3-autonomic command-line tool.

use clap::{Parser, Subcommand};
use x3_autonomic_cli::{CliConfig, Command};
use x3_autonomic_types::AutonomyLevel;

#[derive(Parser)]
#[command(name = "x3-autonomic")]
#[command(about = "X3 Autonomic Core CLI", long_about = None)]
struct Cli {
    /// RPC endpoint
    #[arg(short, long, default_value = "ws://localhost:9944")]
    rpc: String,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Set the autonomy level
    SetAutonomy {
        /// Autonomy level (0-5 or manual/auto/self-improving/self-governing)
        level: String,
    },
    /// Check health status
    Health,
    /// Run audit
    Audit,
    /// List invariants
    Invariants,
    /// View metrics
    Metrics,
}

fn main() {
    let cli = Cli::parse();
    
    let config = CliConfig::default()
        .with_verbose(cli.verbose)
        .with_autonomy(AutonomyLevel::Manual);

    match &cli.command {
        Some(Commands::SetAutonomy { level }) => {
            println!("Setting autonomy level to: {}", level);
        }
        Some(Commands::Health) => {
            println!("Checking health status...");
        }
        Some(Commands::Audit) => {
            println!("Running audit...");
        }
        Some(Commands::Invariants) => {
            println!("Listing invariants...");
        }
        Some(Commands::Metrics) => {
            println!("Viewing metrics...");
        }
        None => {
            println!("X3 Autonomic Core CLI");
            println!("Use --help for usage information");
        }
    }
}