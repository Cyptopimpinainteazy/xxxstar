//! Trace command for transaction debugging.

use crate::error::Result;
use crate::project::Project;
use clap::Args;
use colored::Colorize;
use x3_sdk::AtlasClient;

#[derive(Args)]
pub struct TraceArgs {
    /// Transaction hash to trace
    pub tx_hash: String,

    /// Network to query
    #[arg(short, long)]
    pub network: Option<String>,

    /// Output format (json, human)
    #[arg(short, long, default_value = "human")]
    pub format: String,
}

pub async fn execute(args: TraceArgs) -> Result<()> {
    let endpoint = get_endpoint(args.network.as_deref());

    println!("{} Tracing transaction: {}", "→".blue(), args.tx_hash);
    println!("  Network: {}", endpoint);

    let client = AtlasClient::connect(&endpoint).await?;

    // Get chain info to verify connection
    let chain = client.chain_name().await?;
    println!("  Chain: {}", chain);

    println!();
    println!("{}", "Transaction Trace".bold());
    println!("{}", "─".repeat(50));

    // Parse the tx hash to verify it's valid
    let tx_hash_clean = args.tx_hash.trim_start_matches("0x");
    if tx_hash_clean.len() != 64 {
        println!("{} Invalid transaction hash length", "✗".red());
        return Ok(());
    }

    if hex::decode(tx_hash_clean).is_err() {
        println!("{} Invalid hex in transaction hash", "✗".red());
        return Ok(());
    }

    println!("  Hash: 0x{}", tx_hash_clean);

    // Note: Full trace functionality would require debug_traceTransaction RPC
    // which may not be enabled on all nodes
    println!();
    println!(
        "{} Note: Full tracing requires debug_traceTransaction RPC.",
        "!".yellow()
    );
    println!("  Enable with --rpc-methods=unsafe on the node.");
    println!();
    println!("  For basic receipt info, use: x3 query balance <address>");

    // Try to check if the address exists by querying EVM balance
    // This verifies RPC connectivity at least
    if let Ok(latest) = client.latest_block_hash().await {
        println!("  Latest block: 0x{}", hex::encode(&latest.0[..8]));
    }

    Ok(())
}

fn get_endpoint(network: Option<&str>) -> String {
    if let Ok(project) = Project::load_current() {
        project.config.network.get_endpoint(network).to_string()
    } else {
        network
            .map(|n| match n {
                "testnet" => x3_sdk::TESTNET_HTTP_ENDPOINT.to_string(),
                "mainnet" => x3_sdk::MAINNET_HTTP_ENDPOINT.to_string(),
                _ => x3_sdk::DEFAULT_HTTP_ENDPOINT.to_string(),
            })
            .unwrap_or_else(|| x3_sdk::DEFAULT_HTTP_ENDPOINT.to_string())
    }
}
