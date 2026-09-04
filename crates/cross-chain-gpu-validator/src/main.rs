//! Cross-chain GPU validator service entry point

use cross_chain_gpu_validator::{dashboard::OperatorDashboard, Keccak256Kernel, Secp256k1Kernel};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_target(false).init();

    println!("Cross-Chain GPU Validator Service");
    println!("==================================\n");

    // Initialize components
    println!("Initializing validator components...");

    let _secp_kernel = Secp256k1Kernel::new(32, false);
    let keccak_kernel = Keccak256Kernel::new(32, false);
    let dashboard = Arc::new(OperatorDashboard::new(1000));

    println!("✓ Secp256k1 kernel initialized");
    println!("✓ Keccak256 kernel initialized");
    println!("✓ Dashboard initialized");

    // Test basic functionality
    println!("\nTesting kernel functionality...");

    let test_inputs = vec![b"test1".as_slice(), b"test2".as_slice()];
    match keccak_kernel.hash_batch_cpu(&test_inputs) {
        Ok((hashes, timing)) => {
            println!(
                "✓ Keccak256 batch hashing: {} hashes computed in {} ms",
                hashes.len(),
                timing
            );
        }
        Err(e) => eprintln!("✗ Keccak256 failed: {e}"),
    }

    // Test parity
    match keccak_kernel.verify_parity(&test_inputs) {
        Ok(true) => println!("✓ GPU/CPU parity check passed"),
        Ok(false) => println!("⚠ GPU/CPU parity check failed"),
        Err(e) => eprintln!("✗ Parity check failed: {e}"),
    }

    // Dashboard demo
    println!("\nRecording sample metrics...");
    dashboard.record_swap_success().await;
    dashboard.record_swap_success().await;
    dashboard.record_swap_rollback().await;
    dashboard.record_tps(1500.0, 25).await;
    dashboard.enable_gpu(false).await;
    dashboard.record_gpu_health(true).await;

    let metrics = dashboard.get_metrics().await;
    println!("✓ Total swaps: {}", metrics.total_swaps);
    let success_rate = {
        let denominator = metrics.total_swaps as f64;
        if denominator == 0.0 {
            "N/A".to_string()
        } else {
            format!(
                "{:.1}%",
                metrics.successful_commits as f64 / denominator * 100.0
            )
        }
    };
    println!("✓ Success rate: {}", success_rate);

    println!("\nCross-chain GPU validator ready!");

    Ok(())
}
