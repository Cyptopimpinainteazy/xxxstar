//! X3 GPU Validator Binary
//!
//! Main entry point for running an X3 GPU Validator.

use std::sync::Arc;
use x3_gpu_validator_swarm::{
    config::SwarmConfig,
    crypto::HashAlgorithm,
    deterministic::{DeterministicTask, TaskType},
    validator::Validator,
};

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "run" => run_validator().await,
        "benchmark" => run_benchmark().await,
        "status" => show_status().await,
        "test" => test_validator().await,
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
        }
    }
}

fn print_usage() {
    println!("X3 GPU Validator");
    println!();
    println!("Usage:");
    println!("  x3-validator run        - Run the validator");
    println!("  x3-validator benchmark  - Run benchmarks");
    println!("  x3-validator status    - Show validator status");
    println!("  x3-validator test      - Run tests");
    println!();
    println!("Options:");
    println!("  --config <path>   - Path to config file");
    println!("  --validator-id    - Validator ID");
    println!("  --cpu-only        - Run in CPU-only mode");
}

async fn run_validator() {
    println!("Starting X3 GPU Validator...");

    // Load config
    let config = SwarmConfig::default();

    // Create validator
    let validator = Arc::new(Validator::new(config.clone(), "validator-1".to_string()));

    // Initialize
    if let Err(e) = validator.initialize() {
        eprintln!("Failed to initialize validator: {}", e);
        return;
    }

    println!("Validator initialized successfully");
    println!("Validator ID: {}", validator.id());
    println!("State: {:?}", validator.state());

    // Run some test tasks
    let test_data = vec![
        b"hello world".to_vec(),
        b"test data for hashing".to_vec(),
        b"x3 gpu validator swarm".to_vec(),
    ];

    let task = DeterministicTask::new(TaskType::BatchHash, test_data, HashAlgorithm::Keccak256);

    println!("\nProcessing test task...");
    let result = validator.process_task(task);

    println!("Task completed:");
    println!("  Verification: {:?}", result.verification);
    println!("  Outputs: {}", result.outputs.len());
    println!("  Execution mode: {:?}", result.execution_mode);
    println!("  CPU fallback: {}", result.cpu_fallback_used);

    if result.divergence_detected {
        println!("  ⚠ Divergence detected!");
    }

    // Show metrics
    let metrics = validator.get_metrics();
    println!("\nMetrics:");
    println!("  Total tasks: {}", metrics.total_tasks);
    println!("  Successful: {}", metrics.successful_tasks);
    println!("  Failed: {}", metrics.failed_tasks);
    println!("  Divergent: {}", metrics.divergent_tasks);

    println!("\nValidator running. Press Ctrl+C to stop.");

    // Keep running
    tokio::signal::ctrl_c().await.unwrap();

    println!("Shutting down validator...");
    validator.shutdown();
    println!("Validator stopped.");
}

async fn run_benchmark() {
    println!("Running X3 GPU Validator Benchmark...");

    let config = SwarmConfig::default();
    let validator = Arc::new(Validator::new(
        config.clone(),
        "benchmark-validator".to_string(),
    ));

    validator.initialize().unwrap();

    // Benchmark different batch sizes
    let batch_sizes = vec![1, 10, 100, 1000, 10000];

    println!("\nBenchmarking batch hash operations:");
    println!(
        "{:<10} {:<15} {:<15} {:<15}",
        "Batch Size", "Total Time (ms)", "Avg Time (us)", "Throughput/s"
    );
    println!("{}", "-".repeat(55));

    for batch_size in batch_sizes {
        let inputs: Vec<Vec<u8>> = (0..batch_size)
            .map(|i| format!("test data {}", i).into_bytes())
            .collect();

        let task = DeterministicTask::new(TaskType::BatchHash, inputs, HashAlgorithm::Keccak256);

        let start = std::time::Instant::now();
        let _result = validator.process_task(task);
        let elapsed = start.elapsed();

        let throughput = batch_size as f64 / elapsed.as_secs_f64();

        println!(
            "{:<10} {:<15.2} {:<15.2} {:<15.0}",
            batch_size,
            elapsed.as_millis() as f64,
            elapsed.as_micros() as f64 / batch_size as f64,
            throughput
        );
    }

    println!("\nBenchmark complete.");
}

async fn show_status() {
    println!("X3 GPU Validator Status");
    println!("========================");

    let config = SwarmConfig::default();
    let validator = Arc::new(Validator::new(config, "status-validator".to_string()));

    println!("Validator ID: {}", validator.id());
    println!("State: {:?}", validator.state());
    println!("Uptime: {:?}", validator.uptime());
    println!("Health: {:?}", validator.health_status());

    if let Some(qs) = validator.get_quarantine_status() {
        println!("Quarantined: {}", qs.is_quarantined);
        println!("  Reason: {:?}", qs.reason);
    }

    let metrics = validator.get_metrics();
    println!("\nMetrics:");
    println!("  Total tasks: {}", metrics.total_tasks);
    println!("  Successful: {}", metrics.successful_tasks);
    println!("  Failed: {}", metrics.failed_tasks);
    println!("  Divergent: {}", metrics.divergent_tasks);
    println!("  CPU fallbacks: {}", metrics.cpu_fallbacks);
}

async fn test_validator() {
    println!("Running X3 GPU Validator Tests...");

    let config = SwarmConfig::default();
    let validator = Arc::new(Validator::new(config, "test-validator".to_string()));

    validator.initialize().unwrap();

    // Test 1: Basic hash
    println!("\nTest 1: Basic hash");
    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"hello world".to_vec()],
        HashAlgorithm::Keccak256,
    );
    let result = validator.process_task(task);
    assert!(result.verification == x3_gpu_validator_swarm::crypto::VerificationResult::Valid);
    println!("  ✓ Passed");

    // Test 2: Multiple hashes
    println!("Test 2: Multiple hashes");
    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"hello".to_vec(), b"world".to_vec(), b"test".to_vec()],
        HashAlgorithm::Keccak256,
    );
    let result = validator.process_task(task);
    assert!(result.outputs.len() == 3);
    println!("  ✓ Passed");

    // Test 3: Different algorithms
    println!("Test 3: SHA256");
    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"test data".to_vec()],
        HashAlgorithm::Sha256,
    );
    let result = validator.process_task(task);
    assert!(result.verification == x3_gpu_validator_swarm::crypto::VerificationResult::Valid);
    println!("  ✓ Passed");

    println!("\nAll tests passed!");
}
