//! CPU vs wgpu SHA256 accelerator throughput benchmark.
//!
//! Run CPU only:
//! `cargo bench -p x3-gpu-validator-swarm --bench accel_sha256`
//!
//! Run CPU + wgpu:
//! `X3_ACCEL=wgpu cargo bench -p x3-gpu-validator-swarm --features wgpu --bench accel_sha256 -- --nocapture`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use x3_accel::{AccelBackend, CpuBackend};
use x3_gpu_validator_swarm::MetricsCollector;

fn make_inputs(batch_size: usize) -> Vec<Vec<u8>> {
    let lengths = [0usize, 1, 3, 31, 55, 56, 63, 64, 65, 120, 512, 1024, 4096];
    (0..batch_size)
        .map(|index| {
            let len = lengths[index % lengths.len()];
            (0..len)
                .map(|byte_index| (index.wrapping_add(byte_index) & 0xff) as u8)
                .collect()
        })
        .collect()
}

fn bench_cpu_vs_wgpu_sha256(c: &mut Criterion) {
    let cpu = CpuBackend::new();
    let metrics = Arc::new(MetricsCollector::new());
    let mut group = c.benchmark_group("accel_sha256");

    for batch_size in [1usize, 16, 256, 1024, 4096, 8192] {
        let inputs = make_inputs(batch_size);
        group.throughput(Throughput::Bytes(
            inputs.iter().map(|input| input.len() as u64).sum(),
        ));

        group.bench_with_input(
            BenchmarkId::new("cpu", batch_size),
            &inputs,
            |bench, inputs| {
                bench.iter(|| {
                    let outputs = cpu.sha256_batch(black_box(inputs)).unwrap();
                    black_box(outputs);
                });
            },
        );

        #[cfg(feature = "wgpu")]
        {
            let Ok(wgpu) = x3_accel::WgpuBackend::try_new() else {
                metrics.set_accelerator_backend("wgpu-unavailable");
                metrics.record_accelerator_fallback();
                continue;
            };
            metrics.set_accelerator_backend(wgpu.name());
            let cpu_truth = cpu.sha256_batch(&inputs).unwrap();

            group.bench_with_input(
                BenchmarkId::new("wgpu", batch_size),
                &inputs,
                |bench, inputs| {
                    bench.iter(|| match wgpu.sha256_batch(black_box(inputs)) {
                        Ok(outputs) => {
                            if outputs != cpu_truth {
                                metrics.record_accelerator_parity_mismatch();
                            }
                            black_box(outputs);
                        }
                        Err(_) => {
                            metrics.record_accelerator_fallback();
                            black_box(cpu.sha256_batch(inputs).unwrap());
                        }
                    });
                },
            );
        }
    }

    group.finish();
    println!("{}", metrics.export_prometheus());
}

criterion_group!(benches, bench_cpu_vs_wgpu_sha256);
criterion_main!(benches);
