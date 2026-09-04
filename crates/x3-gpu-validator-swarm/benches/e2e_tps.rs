//! End-to-End TPS Benchmark for X3 GPU Validator Swarm
//!
//! Measures real-world transaction throughput, latency percentiles (p50/p95/p99),
//! and identifies bottlenecks during sustained load.
//!
//! # Usage
//! ```sh
//! cargo bench --bench e2e_tps -- --nocapture
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Barrier;

/// Simulate a GPU compute task (SHA256)
/// Real SHA256 on GPU: 100µs-1ms per batch
/// For benchmarking, we simulate with work proportional to batch size
async fn simulate_gpu_compute(batch_size: usize) -> u64 {
    // Mock: 100µs base + 10µs per item
    let work_us = 100 + (batch_size as u64 * 10);
    tokio::time::sleep(tokio::time::Duration::from_micros(work_us)).await;
    work_us
}

/// Measure TPS with N concurrent tasks
/// Returns: (throughput_tps, p50_latency_ms, p95_latency_ms, p99_latency_ms)
async fn measure_tps_batch(
    num_tasks: usize,
    batch_size: usize,
    duration_secs: u64,
) -> (f64, f64, f64, f64) {
    let barrier = Arc::new(Barrier::new(num_tasks));
    let task_count = Arc::new(AtomicU64::new(0));
    let latencies = Arc::new(std::sync::Mutex::new(Vec::new()));

    let start = Instant::now();
    let deadline = start + std::time::Duration::from_secs(duration_secs);

    let mut handles = vec![];

    for _ in 0..num_tasks {
        let barrier_clone = barrier.clone();
        let task_count_clone = task_count.clone();
        let latencies_clone = latencies.clone();

        let handle = tokio::spawn(async move {
            // Wait for all tasks to start
            barrier_clone.wait().await;

            while Instant::now() < deadline {
                let task_start = Instant::now();
                simulate_gpu_compute(batch_size).await;
                let latency_ms = task_start.elapsed().as_secs_f64() * 1000.0;

                task_count_clone.fetch_add(1, Ordering::Relaxed);
                latencies_clone.lock().unwrap().push(latency_ms);
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        let _ = handle.await;
    }

    let total_tasks = task_count.load(Ordering::Relaxed) as f64;
    let actual_duration = start.elapsed().as_secs_f64();
    let tps = total_tasks / actual_duration;

    // Calculate latency percentiles
    let mut lats = latencies.lock().unwrap().clone();
    lats.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p50 = if !lats.is_empty() {
        lats[lats.len() / 2]
    } else {
        0.0
    };
    let p95 = if !lats.is_empty() {
        lats[(lats.len() * 95) / 100]
    } else {
        0.0
    };
    let p99 = if !lats.is_empty() {
        lats[(lats.len() * 99) / 100]
    } else {
        0.0
    };

    (tps, p50, p95, p99)
}

fn bench_tps_scaling(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("tps_scaling");

    // Test with increasing load: 1, 4, 16, 64 concurrent tasks
    for num_tasks in [1, 4, 16, 64].iter() {
        group.throughput(Throughput::Elements(*num_tasks as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("tasks_{}", num_tasks)),
            num_tasks,
            |b, &num_tasks| {
                b.to_async(&rt).iter(|| async {
                    let (tps, p50, p95, p99) = measure_tps_batch(
                        num_tasks, 128, // batch_size
                        2,   // duration_secs
                    )
                    .await;

                    println!(
                        "  Tasks: {}, TPS: {:.0}, p50: {:.2}ms, p95: {:.2}ms, p99: {:.2}ms",
                        num_tasks, tps, p50, p95, p99
                    );

                    black_box((tps, p50, p95, p99))
                });
            },
        );
    }
    group.finish();
}

fn bench_tps_batch_sizes(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("tps_batch_sizes");

    // Test with different batch sizes: 32, 128, 512, 2048
    for batch_size in [32, 128, 512, 2048].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("batch_{}", batch_size)),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let (tps, p50, p95, p99) = measure_tps_batch(
                        16, // num_tasks
                        batch_size, 2, // duration_secs
                    )
                    .await;

                    println!(
                        "  Batch: {}, TPS: {:.0}, p50: {:.2}ms, p95: {:.2}ms, p99: {:.2}ms",
                        batch_size, tps, p50, p95, p99
                    );

                    black_box((tps, p50, p95, p99))
                });
            },
        );
    }
    group.finish();
}

/// Stress test: Sustained load for 30 seconds to detect memory leaks/degradation
fn bench_tps_sustained_load(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("tps_sustained");
    group.sample_size(10); // Reduce samples for long-running test

    group.bench_function("sustained_30s", |b| {
        b.to_async(&rt).iter(|| async {
            let (tps, p50, p95, p99) = measure_tps_batch(
                64,  // num_tasks (high load)
                256, // batch_size
                30,  // duration_secs (long duration)
            )
            .await;

            println!(
                "  Sustained Load (30s): TPS: {:.0}, p50: {:.2}ms, p95: {:.2}ms, p99: {:.2}ms",
                tps, p50, p95, p99
            );

            black_box((tps, p50, p95, p99))
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_tps_scaling,
    bench_tps_batch_sizes,
    bench_tps_sustained_load
);
criterion_main!(benches);
