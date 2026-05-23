//! X3 Wallet GPU Sync Tool
//!
//! Enables users to sync their GPU with the swarm through wallet integration.

use std::sync::Arc;
use x3_gpu_validator_swarm::{
    config::SwarmConfig,
    payment::{wallet_sync, PaymentSystem},
};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "detect" => detect_gpu(),
        "benchmark" => run_benchmark(),
        "register" => register_provider(),
        "status" => show_payment_status(),
        "sync" => wallet_sync_flow(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
        }
    }
}

fn print_usage() {
    println!("X3 Wallet GPU Sync");
    println!();
    println!("Usage:");
    println!("  wallet-sync detect    - Detect and report GPU capabilities");
    println!("  wallet-sync benchmark - Run performance benchmark");
    println!("  wallet-sync register  - Register as a provider");
    println!("  wallet-sync status    - Show payment status");
    println!("  wallet-sync sync     - Full sync flow (detect + register)");
    println!();
    println!("Options:");
    println!("  --wallet <address>   - Wallet address");
    println!("  --stake <amount>     - Stake amount (default: 1000000)");
}

fn detect_gpu() {
    println!("Detecting GPU...");

    if let Some(report) = wallet_sync::detect_gpu() {
        println!("✓ GPU Detected!");
        println!();
        println!("GPU Information:");
        println!("  Model: {}", report.gpu_model);
        println!("  Memory: {} MB", report.memory_mb);
        println!(
            "  Compute Capability: {}.{}",
            report.compute_capability.0, report.compute_capability.1
        );
        println!("  CUDA Cores: {}", report.cuda_cores);
        println!("  Supported Operations: {:?}", report.supported_ops);

        // Run benchmark
        println!();
        println!("Running benchmark...");
        let score = wallet_sync::run_benchmark();
        println!("  Benchmark Score: {} ops/sec", score);
    } else {
        println!("✗ No GPU detected");
        println!("Running in CPU-only mode");
    }
}

fn run_benchmark() {
    println!("Running GPU Benchmark...");
    println!();

    let score = wallet_sync::run_benchmark();

    println!("Benchmark Results:");
    println!("  Score: {} operations/second", score);
    println!();

    // Calculate estimated earnings
    let rate_per_sec = 1000u64; // Base rate
    let daily_earnings = score * 60 * 60 * 24 / rate_per_sec;
    println!("Estimated Daily Earnings: {} X3", daily_earnings);
}

fn register_provider() {
    println!("Registering as Provider...");

    // Get wallet from args
    let wallet = std::env::var("WALLET_ADDRESS")
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());

    let stake = std::env::var("STAKE_AMOUNT")
        .unwrap_or_else(|_| "1000000".to_string())
        .parse::<u64>()
        .unwrap_or(1000000);

    let config = SwarmConfig::default();
    let payment = Arc::new(PaymentSystem::new(config));

    let provider_id = uuid::Uuid::new_v4().to_string();

    match payment.register_provider(provider_id, wallet, stake) {
        Ok(account) => {
            println!("✓ Provider Registered!");
            println!();
            println!("Provider Details:");
            println!("  Provider ID: {}", account.provider_id);
            println!("  Wallet: {}", account.wallet_address);
            println!("  Stake: {} X3", account.stake);
            println!("  Status: {:?}", account.status);
        }
        Err(e) => {
            println!("✗ Registration failed: {}", e);
        }
    }
}

fn show_payment_status() {
    println!("Payment Status");
    println!("==============");

    let config = SwarmConfig::default();
    let payment = PaymentSystem::new(config);

    // Show reward rates
    println!();
    println!("Reward Rates:");
    println!("  Base rate: 0.001 X3 per work unit");
    println!("  Verification bonus: 1.5x");
    println!("  Divergence penalty: 0.5x");
    println!("  Minimum stake: 1 X3");

    // Show providers
    let providers = payment.get_all_providers();
    println!();
    println!("Providers: {}", providers.len());

    for provider in providers {
        println!();
        println!("Provider: {}", provider.provider_id);
        println!("  Status: {:?}", provider.status);
        println!("  Total Earned: {} X3", provider.total_earned);
        println!("  Pending: {} X3", provider.total_pending);
        println!("  Withdrawn: {} X3", provider.total_withdrawn);
        println!("  Reputation: {:.1}", provider.reputation);
        println!("  Tasks Completed: {}", provider.tasks_completed);
        println!("  Divergences: {}", provider.divergence_count);
    }
}

fn wallet_sync_flow() {
    println!("╔════════════════════════════════════════╗");
    println!("║     X3 Wallet GPU Sync Flow           ║");
    println!("╚════════════════════════════════════════╝");
    println!();

    // Step 1: Detect GPU
    println!("Step 1: Detecting GPU...");
    let report = match wallet_sync::detect_gpu() {
        Some(r) => {
            println!("  ✓ GPU detected: {}", r.gpu_model);
            r
        }
        None => {
            println!("  ✗ No GPU detected, running in CPU mode");
            return;
        }
    };

    // Step 2: Run benchmark
    println!();
    println!("Step 2: Running benchmark...");
    let score = wallet_sync::run_benchmark();
    println!("  ✓ Benchmark score: {} ops/sec", score);

    // Step 3: Get wallet address
    println!();
    println!("Step 3: Wallet Connection");
    let wallet = std::env::var("WALLET_ADDRESS").unwrap_or_else(|_| {
        println!("  Enter wallet address (or set WALLET_ADDRESS env var):");
        println!("  > ");
        "0x0000000000000000000000000000000000000000".to_string()
    });
    println!("  ✓ Connected: {}", wallet);

    // Step 4: Register
    println!();
    println!("Step 4: Registering provider...");
    let config = SwarmConfig::default();
    let _payment = Arc::new(PaymentSystem::new(config));

    let request = wallet_sync::WalletSyncRequest {
        wallet_address: wallet.clone(),
        signature: vec![1, 2, 3], // Would be real signature
        gpu_report: report,
    };

    let response = wallet_sync::sync_wallet(request);

    if response.success {
        println!("  ✓ Provider registered!");
        if let Some(provider_id) = &response.provider_id {
            println!("  ✓ Provider ID: {}", provider_id);
        }
        if let Some(node_info) = &response.node_info {
            println!("  ✓ Node address: {}:{}", node_info.address, node_info.port);
        }
    } else {
        println!("  ✗ Registration failed: {:?}", response.error);
    }

    // Step 5: Show earnings estimate
    println!();
    println!("Step 5: Earnings Estimate");
    println!("  Daily estimate: {} X3", score * 60 * 60 * 24 / 1000);
    println!();
    println!("═══════════════════════════════════════════");
    println!("GPU Sync Complete!");
    println!("═══════════════════════════════════════════");
}
