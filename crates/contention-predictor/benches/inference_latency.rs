//! Benchmark: inference latency for contention prediction pipeline.

use criterion::{criterion_group, criterion_main, Criterion};

fn bench_predict(c: &mut Criterion) {
    c.bench_function("predict_batch_8", |b| {
        b.iter(|| {
            // Placeholder: real benchmark will create ContentionPredictor
            // and call predict_and_shard with sample transactions.
            std::hint::black_box(42u64)
        })
    });
}

criterion_group!(benches, bench_predict);
criterion_main!(benches);
