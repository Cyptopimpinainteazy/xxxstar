// crates/gpu-swarm/src/cli/mod.rs
// CLI tooling for GPU Swarm management

pub mod swarm_cli;
pub mod swarm_inspect;

pub use swarm_cli::SwarmCLI;
pub use swarm_inspect::SwarmInspect;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "swarm")]
#[command(about = "GPU Swarm management and monitoring CLI", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
