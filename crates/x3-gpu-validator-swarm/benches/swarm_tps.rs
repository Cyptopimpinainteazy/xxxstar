//! Real swarm TPS benchmark for hash-batch validation.
//!
//! This drives tasks through `SwarmOrchestrator` so the exported Prometheus TPS
//! gauges are populated by the same metrics path used by the runtime.
//!
//! Run CPU:
//! `cargo bench -p x3-gpu-validator-swarm --bench swarm_tps -- --nocapture`
//!
//! Run selected accelerator:
//! `X3_ACCEL=wgpu cargo bench -p x3-gpu-validator-swarm --features wgpu --bench swarm_tps -- --nocapture`
//!
//! Run a one-shot sustained scrape report before Criterion samples:
//! `X3_ACCEL=wgpu X3_SWARM_SOAK_SECS=30 cargo bench -p x3-gpu-validator-swarm --features wgpu --bench swarm_tps -- sustained --nocapture`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use x3_gpu_validator_swarm::{
    config::SwarmConfig,
    crypto::HashAlgorithm,
    deterministic::{DeterministicTask, TaskType},
    SwarmOrchestrator, Validator,
};

const DEFAULT_SOAK_TASKS_PER_ROUND: usize = 128;
const DEFAULT_SOAK_BATCH_SIZE: usize = 1024;

fn make_inputs(batch_size: usize, seed: usize) -> Vec<Vec<u8>> {
    let lengths = [32usize, 64, 120, 512, 1024, 4096];
    (0..batch_size)
        .map(|index| {
            let len = lengths[(seed + index) % lengths.len()];
            (0..len)
                .map(|byte_index| (seed.wrapping_add(index).wrapping_add(byte_index) & 0xff) as u8)
                .collect()
        })
        .collect()
}

fn create_orchestrator() -> SwarmOrchestrator {
    let config = SwarmConfig::default();
    let orchestrator = SwarmOrchestrator::new(config.clone());
    let validator = Arc::new(Validator::new(config, "swarm-tps-validator-1".to_string()));
    validator.initialize().expect("validator initializes");
    validator.enable_gpu_mode();
    orchestrator.register_validator(validator);
    orchestrator
}

fn run_swarm_hash_batches(task_count: usize, batch_size: usize) -> (usize, f64, f64, String) {
    let orchestrator = create_orchestrator();
    let tasks = (0..task_count)
        .map(|seed| {
            DeterministicTask::new(
                TaskType::BatchHash,
                make_inputs(batch_size, seed),
                HashAlgorithm::Sha256,
            )
        })
        .collect::<Vec<_>>();

    let total_hashes = task_count * batch_size;
    let started = Instant::now();
    orchestrator.submit_batch(tasks);
    let processed = orchestrator.process_pending_tasks();
    let elapsed_secs = started.elapsed().as_secs_f64();
    let metrics = orchestrator.get_swarm_metrics();
    let prometheus = orchestrator.export_metrics_prometheus();
    let hashes_per_sec = if elapsed_secs > 0.0 {
        total_hashes as f64 / elapsed_secs
    } else {
        0.0
    };

    assert_eq!(processed, task_count);
    assert_eq!(metrics.total_tasks, task_count as u64);
    assert_eq!(metrics.successful_tasks, task_count as u64);
    assert_eq!(metrics.failed_tasks, 0);
    assert_eq!(metrics.divergent_tasks, 0);
    assert_eq!(metrics.accelerator_fallbacks, 0);
    assert_eq!(metrics.accelerator_parity_mismatches, 0);

    (
        processed,
        metrics.tasks_per_second,
        hashes_per_sec,
        prometheus,
    )
}

fn run_sustained_swarm_hash_batches(
    duration: Duration,
    tasks_per_round: usize,
    batch_size: usize,
) -> (u64, f64, f64, String) {
    let orchestrator = create_orchestrator();
    let started = Instant::now();
    let deadline = started + duration;
    let mut total_hashes = 0u64;
    let mut seed = 0usize;

    while Instant::now() < deadline {
        let tasks = (0..tasks_per_round)
            .map(|offset| {
                DeterministicTask::new(
                    TaskType::BatchHash,
                    make_inputs(batch_size, seed + offset),
                    HashAlgorithm::Sha256,
                )
            })
            .collect::<Vec<_>>();

        orchestrator.submit_batch(tasks);
        let processed = orchestrator.process_pending_tasks();
        assert_eq!(processed, tasks_per_round);

        total_hashes += (processed * batch_size) as u64;
        seed = seed.wrapping_add(tasks_per_round);
    }

    let elapsed_secs = started.elapsed().as_secs_f64();
    let metrics = orchestrator.get_swarm_metrics();
    assert_eq!(metrics.failed_tasks, 0);
    assert_eq!(metrics.divergent_tasks, 0);
    assert_eq!(metrics.accelerator_fallbacks, 0);
    assert_eq!(metrics.accelerator_parity_mismatches, 0);

    let hash_tps = if elapsed_secs > 0.0 {
        total_hashes as f64 / elapsed_secs
    } else {
        0.0
    };

    (
        metrics.total_tasks,
        metrics.tasks_per_second,
        hash_tps,
        orchestrator.export_metrics_prometheus(),
    )
}

fn bench_swarm_tps(c: &mut Criterion) {
    let mut group = c.benchmark_group("swarm_tps");

    for (task_count, batch_size) in [
        (16usize, 256usize),
        (64, 1024),
        (DEFAULT_SOAK_TASKS_PER_ROUND, DEFAULT_SOAK_BATCH_SIZE),
        (128, 2048),
    ] {
        group.throughput(Throughput::Elements((task_count * batch_size) as u64));
        group.bench_with_input(
            BenchmarkId::new(format!("tasks_{task_count}"), batch_size),
            &(task_count, batch_size),
            |bench, &(task_count, batch_size)| {
                bench.iter(|| {
                    let result = run_swarm_hash_batches(task_count, batch_size);
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn bench_swarm_sustained_report(c: &mut Criterion) {
    if let Ok(raw_secs) = env::var("X3_SWARM_SOAK_SECS") {
        let duration_secs = raw_secs
            .parse::<u64>()
            .expect("X3_SWARM_SOAK_SECS must be an integer number of seconds");
        let tasks_per_round = env::var("X3_SWARM_SOAK_TASKS")
            .ok()
            .map(|value| {
                value
                    .parse::<usize>()
                    .expect("X3_SWARM_SOAK_TASKS must be an integer")
            })
            .unwrap_or(DEFAULT_SOAK_TASKS_PER_ROUND);
        let batch_size = env::var("X3_SWARM_SOAK_BATCH")
            .ok()
            .map(|value| {
                value
                    .parse::<usize>()
                    .expect("X3_SWARM_SOAK_BATCH must be an integer")
            })
            .unwrap_or(DEFAULT_SOAK_BATCH_SIZE);

        let (processed, task_tps, hash_tps, prometheus) = run_sustained_swarm_hash_batches(
            Duration::from_secs(duration_secs),
            tasks_per_round,
            batch_size,
        );
        println!(
            "sustained_secs={duration_secs} tasks_per_round={tasks_per_round} batch_size={batch_size} processed={processed} task_tps={task_tps:.2} hash_tps={hash_tps:.2}\n{prometheus}"
        );
    }

    let mut group = c.benchmark_group("swarm_tps_sustained");
    group.bench_function("sustained_report", |bench| {
        bench.iter(|| black_box(()));
    });
    group.finish();
}

fn bench_swarm_tps_report(c: &mut Criterion) {
    let mut group = c.benchmark_group("swarm_tps_report");
    group.sample_size(10);

    group.bench_function("prometheus_export", |bench| {
        bench.iter(|| {
            let (processed, task_tps, hash_tps, prometheus) =
                run_swarm_hash_batches(DEFAULT_SOAK_TASKS_PER_ROUND, DEFAULT_SOAK_BATCH_SIZE);
            println!(
                "processed={processed} task_tps={task_tps:.2} hash_tps={hash_tps:.2}\n{prometheus}"
            );
            black_box((processed, task_tps, hash_tps, prometheus));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_swarm_tps,
    bench_swarm_tps_report,
    bench_swarm_sustained_report
);
criterion_main!(benches);
