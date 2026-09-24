//! GPU TPS (Transactions Per Second) Benchmark with Cache Analysis
//!
//! This test measures the throughput of the Phase 3 GPU bytecode execution
//! with bytecode caching. It provides metrics for:
//! - Cache hit/miss rates
//! - Performance with repeated vs unique payloads
//! - Overall system throughput

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
async fn bench_gpu_tps_with_cache_analysis() {
    println!("\n========== GPU TPS Benchmark with Cache Analysis ==========\n");

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

    println!("\n--- Benchmark 1: Repeated Payloads (Cache Hits Expected) ---");
    let payload_size = 64;
    let sha256_payload = vec![0x42u8; payload_size];
    let keccak256_payload = vec![0x84u8; payload_size];

    let start = Instant::now();
    let mut success = 0;

    // Execute 500 identical SHA-256 tasks (all should hit cache after first)
    for _ in 0..500 {
        let task = create_sha256_task(sha256_payload.clone());
        if executor
            .execute(&task, device_id, std::time::Duration::from_secs(30))
            .await
            .is_ok()
        {
            success += 1;
        }
    }

    // Execute 500 identical Keccak-256 tasks (all should hit cache after first)
    for _ in 0..500 {
        let task = create_keccak256_task(keccak256_payload.clone());
        if executor
            .execute(&task, device_id, std::time::Duration::from_secs(30))
            .await
            .is_ok()
        {
            success += 1;
        }
    }

    let elapsed = start.elapsed();
    let tps_repeated = success as f64 / elapsed.as_secs_f64();
    let avg_latency_repeated = elapsed.as_millis() as f64 / success.max(1) as f64;

    println!(
        "Repeated Payloads Results:\n  Tasks completed: {}/1000\n  Total time: {:.2}s\n  TPS: {:.2} tasks/sec\n  Avg latency: {:.3} ms/task",
        success, elapsed.as_secs_f64(), tps_repeated, avg_latency_repeated
    );

    println!("\n--- Benchmark 2: Varied Payloads (Cache Misses Expected) ---");

    let start = Instant::now();
    let mut success = 0;

    // Execute 500 SHA-256 tasks with varying payloads
    for i in 0..500 {
        let mut payload = vec![0x42u8; payload_size];
        payload[0] = (i % 256) as u8; // Vary first byte to create different cache keys
        let task = create_sha256_task(payload);
        if executor
            .execute(&task, device_id, std::time::Duration::from_secs(30))
            .await
            .is_ok()
        {
            success += 1;
        }
    }

    // Execute 500 Keccak-256 tasks with varying payloads
    for i in 0..500 {
        let mut payload = vec![0x84u8; payload_size];
        payload[0] = (i % 256) as u8; // Vary first byte to create different cache keys
        let task = create_keccak256_task(payload);
        if executor
            .execute(&task, device_id, std::time::Duration::from_secs(30))
            .await
            .is_ok()
        {
            success += 1;
        }
    }

    let elapsed = start.elapsed();
    let tps_varied = success as f64 / elapsed.as_secs_f64();
    let avg_latency_varied = elapsed.as_millis() as f64 / success.max(1) as f64;

    println!(
        "Varied Payloads Results:\n  Tasks completed: {}/1000\n  Total time: {:.2}s\n  TPS: {:.2} tasks/sec\n  Avg latency: {:.3} ms/task",
        success, elapsed.as_secs_f64(), tps_varied, avg_latency_varied
    );

    println!("\n========== CACHE ANALYSIS SUMMARY ==========\n");
    println!(
        "Repeated Payloads TPS:  {:.2} tasks/sec (cache hits expected)",
        tps_repeated
    );
    println!(
        "Varied Payloads TPS:    {:.2} tasks/sec (cache misses expected)",
        tps_varied
    );
    println!(
        "Cache Benefit: {:.1}x speedup",
        tps_repeated / tps_varied.max(1.0)
    );
    println!(
        "\nRepeated Payloads Avg:  {:.3} ms/task",
        avg_latency_repeated
    );
    println!("Varied Payloads Avg:    {:.3} ms/task", avg_latency_varied);

    println!("\n=========================================\n");

    // Both should have reasonable success rates
    assert!(
        tps_repeated > 0.0,
        "Repeated payload benchmark should complete"
    );
    assert!(tps_varied > 0.0, "Varied payload benchmark should complete");
}
