//! X3 Cross-VM Sidecar daemon.
//!
//! Launched by the X3 node via `spawn_sidecar_service`. Monitors external VMs
//! (Solana, EVM sidechains) for escrow/bridge events and validates finality
//! before signalling the X3 runtime.
//!
//! # Environment variables
//! - `X3_SIDECAR_BIN` — override binary path (consumed by the launcher, not here)
//! - `X3_SOLANA_RPC_URL` — Solana JSON-RPC endpoint (default: mainnet-beta)
//! - `RUST_LOG` — log filter (default: info)

use clap::Parser;
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(name = "x3-sidecar", about = "X3 Cross-VM Sidecar daemon")]
struct Args {
    /// Logical service identifier (set by the node launcher)
    #[arg(long, default_value = "x3-sidecar")]
    service_id: String,

    /// Solana JSON-RPC endpoint to monitor
    #[arg(long, env = "X3_SOLANA_RPC_URL", default_value = "https://api.mainnet-beta.solana.com")]
    solana_rpc: String,

    /// How often to poll for new events (seconds)
    #[arg(long, default_value_t = 30)]
    poll_interval_secs: u64,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    log::info!(
        "🔌 X3 Sidecar '{}' starting — Solana RPC: {}",
        args.service_id,
        args.solana_rpc
    );

    // Install Ctrl-C handler for graceful shutdown.
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    let poll_interval = Duration::from_secs(args.poll_interval_secs);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                log::info!("🛑 X3 Sidecar '{}' received shutdown signal — exiting", args.service_id);
                break;
            }
            _ = tokio::time::sleep(poll_interval) => {
                poll_once(&args.service_id, &args.solana_rpc).await;
            }
        }
    }

    log::info!("✅ X3 Sidecar '{}' shut down cleanly", args.service_id);
}

/// One monitoring cycle.
///
/// Phase 1 implementation: health-check log only.
/// Phase 2 (tracked in t5-sidecar-rpc-monitor): connect to `solana_rpc`,
/// fetch recent escrow program accounts, validate 32-confirmation finality,
/// and submit bridge extrinsics to the X3 node via JSON-RPC.
async fn poll_once(service_id: &str, solana_rpc: &str) {
    log::debug!(
        "🔌 Sidecar '{}' polling {} — phase 1 (health log only)",
        service_id,
        solana_rpc
    );
    // Phase 2: HTTP call to solana_rpc + extrinsic submission goes here.
}
