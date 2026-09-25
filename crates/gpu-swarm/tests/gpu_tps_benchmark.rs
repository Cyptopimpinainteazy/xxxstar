//! GPU TPS (Transactions Per Second) Baseline Benchmark
//!
//! This test measures the current throughput of the Phase 3 GPU bytecode execution
//! implementation in CudaExecutor. It provides a baseline for measuring performance
//! improvements from optimization iterations.
//!
//! Usage:
//!   cargo test --package gpu-swarm --test gpu_tps_benchmark -- --nocapture
//!
//! The test executes 1000 SHA-256 and 1000 Keccak-256 GPU bytecode tasks,
//! measuring:
//! - Total execution time
//! - Tasks completed per second (TPS)
//! - Average per-task latency
//! - Success/failure rates

use gpu_swarm::{
    gpu_backends::{cuda::CudaExecutor, GpuExecutor},
    task::{Task, TaskType},
};
use std::time::Instant;

/// Helper to create a test submitter ID
fn test_submitter() -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = 0xFF;
    id
}

/// Create a SHA-256 GPU task with given payload
fn create_sha256_task(payload: Vec<u8>) -> Task {
    Task::new(
        TaskType::Custom {
            task_type: "sha256".to_string(),
            payload,
        },
        test_submitter(),
        100,
    )
}

/// Create a Keccak-256 GPU task with given payload
fn create_keccak256_task(payload: Vec<u8>) -> Task {
    Task::new(
        TaskType::Custom {
            task_type: "keccak256".to_string(),
            payload,
        },
        test_submitter(),
        100,
    )
}

#[tokio::test]
async fn bench_gpu_tps_baseline() {
    println!("\n========== GPU TPS Baseline Benchmark ==========\n");

    // Initialize CUDA executor
    let executor = match CudaExecutor::new().await {
        Ok(executor) => {
            println!("✓ CUDA executor initialized");
            executor
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize CUDA executor: {}", e);
            eprintln!("  (This test requires GPU support with x3-runtime feature)");
            return;
        }
    };

    // Check GPU availability
    if !executor.is_available().await {
        eprintln!("✗ GPU not available");
        return;
    }

    // Get device info
    let devices = match executor.list_devices().await {
        Ok(devices) => {
            println!("✓ Found {} GPU device(s)", devices.len());
            for (i, device) in devices.iter().enumerate() {
                println!(
                    "  Device {}: {} ({} GB VRAM)",
                    i,
                    device.name,
                    device.total_memory / (1024 * 1024 * 1024)
                );
            }
            devices
        }
        Err(e) => {
            eprintln!("✗ Failed to list GPU devices: {}", e);
            return;
        }
    };

    if devices.is_empty() {
        eprintln!("✗ No GPU devices available");
        return;
    }

    let device_id = devices[0].device_id;

    // Configuration
    let sha256_count = 1000;
    let keccak256_count = 1000;
    let total_tasks = sha256_count + keccak256_count;
    let test_payload_size = 64; // bytes (typical hash input)

    println!("\n--- Benchmark Configuration ---");
    println!("  SHA-256 tasks: {}", sha256_count);
    println!("  Keccak-256 tasks: {}", keccak256_count);
    println!("  Total tasks: {}", total_tasks);
    println!("  Test payload size: {} bytes", test_payload_size);
    println!("  Device: {} (ID: {})", devices[0].name, device_id);

    // Create test payloads
    let sha256_payload = vec![0x42u8; test_payload_size];
    let keccak256_payload = vec![0x84u8; test_payload_size];

    println!("\n--- Executing SHA-256 Batch ---");
    let sha256_start = Instant::now();
    let mut sha256_success = 0;
    let mut sha256_failures = 0;

    for i in 0..sha256_count {
        let task = create_sha256_task(sha256_payload.clone());
        match executor
            .execute(&task, device_id, std::time::Duration::from_secs(30))
            .await
        {
            Ok(_result) => {
                sha256_success += 1;
                if (i + 1) % 100 == 0 {
                    println!("  Completed {}/{} SHA-256 tasks", i + 1, sha256_count);
                }
            }
            Err(e) => {
                sha256_failures += 1;
                if sha256_failures <= 5 {
                    eprintln!("  SHA-256 task {} failed: {}", i, e);
                }
            }
        }
    }

    let sha256_elapsed = sha256_start.elapsed();
    let sha256_tps = sha256_success as f64 / sha256_elapsed.as_secs_f64();
    let sha256_avg_latency = sha256_elapsed.as_millis() as f64 / sha256_success.max(1) as f64;

    println!(
        "\nSHA-256 Results:\n  Success: {}/{}\n  Total time: {:.2}s\n  TPS: {:.2} tasks/sec\n  Avg latency: {:.2} ms/task",
        sha256_success, sha256_count, sha256_elapsed.as_secs_f64(), sha256_tps, sha256_avg_latency
    );

    println!("\n--- Executing Keccak-256 Batch ---");
    let keccak256_start = Instant::now();
    let mut keccak256_success = 0;
    let mut keccak256_failures = 0;

    for i in 0..keccak256_count {
        let task = create_keccak256_task(keccak256_payload.clone());
        match executor
            .execute(&task, device_id, std::time::Duration::from_secs(30))
            .await
        {
            Ok(_result) => {
                keccak256_success += 1;
                if (i + 1) % 100 == 0 {
                    println!("  Completed {}/{} Keccak-256 tasks", i + 1, keccak256_count);
                }
            }
            Err(e) => {
                keccak256_failures += 1;
                if keccak256_failures <= 5 {
                    eprintln!("  Keccak-256 task {} failed: {}", i, e);
                }
            }
        }
    }

    let keccak256_elapsed = keccak256_start.elapsed();
    let keccak256_tps = keccak256_success as f64 / keccak256_elapsed.as_secs_f64();
    let keccak256_avg_latency =
        keccak256_elapsed.as_millis() as f64 / keccak256_success.max(1) as f64;

    println!(
        "\nKeccak-256 Results:\n  Success: {}/{}\n  Total time: {:.2}s\n  TPS: {:.2} tasks/sec\n  Avg latency: {:.2} ms/task",
        keccak256_success, keccak256_count, keccak256_elapsed.as_secs_f64(), keccak256_tps, keccak256_avg_latency
    );

    // Aggregate results
    println!("\n========== BASELINE SUMMARY ==========\n");

    let total_success = sha256_success + keccak256_success;
    let total_failures = sha256_failures + keccak256_failures;
    let total_elapsed = sha256_elapsed + keccak256_elapsed;
    let overall_tps = total_success as f64 / total_elapsed.as_secs_f64();
    let overall_avg_latency = total_elapsed.as_millis() as f64 / total_success.max(1) as f64;

    println!("Total Tasks Completed: {}/{}", total_success, total_tasks);
    println!("Total Tasks Failed: {}", total_failures);
    println!("Total Execution Time: {:.2}s", total_elapsed.as_secs_f64());
    println!("\n>>> BASELINE TPS: {:.2} tasks/sec <<<", overall_tps);
    println!(
        ">>> BASELINE AVG LATENCY: {:.2} ms/task <<<\n",
        overall_avg_latency
    );

    // Success rate
    let success_rate = (total_success as f64 / total_tasks as f64) * 100.0;
    println!("Success Rate: {:.1}%", success_rate);

    println!("\n=========================================\n");

    // Verify success rate acceptable
    assert!(
        success_rate > 90.0,
        "Success rate should be > 90%, got {:.1}%",
        success_rate
    );
}
