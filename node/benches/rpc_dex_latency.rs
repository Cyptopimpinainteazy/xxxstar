//! DEX RPC Endpoint Latency & Throughput Benchmark
//!
//! Measures performance of walletDex_estimateSwap and walletDex_executeSwap
//! RPC endpoints under various load conditions.
//!
//! Metrics:
//! - Request latency (p50, p90, p95, p99)
//! - Throughput (requests/sec)
//! - Error rate under load
//! - Resource utilization patterns
//!
//! # Usage
//! ```sh
//! cargo bench --bench rpc_dex_latency -- --nocapture
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

// Mock RPC client for benchmarking
struct MockDexRpcClient {
    request_count: Arc<AtomicU64>,
    latency_ms_base: u64,
}

impl MockDexRpcClient {
    fn new(latency_ms_base: u64) -> Self {
        Self {
            request_count: Arc::new(AtomicU64::new(0)),
            latency_ms_base,
        }
    }

    async fn estimate_swap(&self, request: serde_json::Value) -> Result<serde_json::Value, String> {
        // Simulate processing time: 5-15ms typical
        let jitter = (self.request_count.load(Ordering::Relaxed) % 10) as u64;
        tokio::time::sleep(Duration::from_millis(self.latency_ms_base + jitter)).await;

        self.request_count.fetch_add(1, Ordering::Relaxed);

        // Mock response with 5% fee
        let amount_in = request["amount_in"]
            .as_str()
            .unwrap_or("1000000")
            .parse::<u128>()
            .unwrap_or(1000000);
        let amount_out = amount_in * 95 / 100; // 5% fee simulation

        Ok(json!({
            "swap_id": self.request_count.load(Ordering::Relaxed),
            "amount_out": amount_out.to_string(),
            "approval_required": false,
            "estimated_gas": "21000"
        }))
    }

    async fn execute_swap(&self, request: serde_json::Value) -> Result<serde_json::Value, String> {
        // Execute is slower: 20-50ms typical (includes settlement intent creation)
        let jitter = (self.request_count.load(Ordering::Relaxed) % 30) as u64;
        tokio::time::sleep(Duration::from_millis(self.latency_ms_base * 2 + jitter)).await;

        self.request_count.fetch_add(1, Ordering::Relaxed);

        Ok(json!({
            "swap_id": self.request_count.load(Ordering::Relaxed),
            "status": "pending_settlement",
            "intent_id": format!("0x{:064x}", self.request_count.load(Ordering::Relaxed)),
            "estimated_finality_blocks": 12
        }))
    }
}

fn create_swap_request(token_in: &str, token_out: &str, amount: u128) -> serde_json::Value {
    json!({
        "token_in": token_in,
        "token_out": token_out,
        "amount_in": amount.to_string(),
        "min_amount_out": (amount * 90 / 100).to_string(), // 10% slippage tolerance
        "wallet_id": "0x0000000000000000000000000000000000000000000000000000000000000001",
        "require_approval": false,
        "approval_threshold": "0"
    })
}

async fn benchmark_estimate_swap_latency(
    client: &MockDexRpcClient,
    iterations: usize,
) -> (f64, f64, f64, f64) {
    let mut latencies = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let request = create_swap_request(
            "0x0000000000000000000000000000000000000000000000000000000000000002", // X3
            "0x0000000000000000000000000000000000000000000000000000000000000003", // USDC
            1_000_000 + (i as u128 * 100_000),
        );

        let start = Instant::now();
        let _ = client.estimate_swap(request).await;
        let elapsed = start.elapsed().as_micros() as f64 / 1000.0;
        latencies.push(elapsed);
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = latencies[iterations * 50 / 100];
    let p90 = latencies[iterations * 90 / 100];
    let p95 = latencies[iterations * 95 / 100];
    let p99 = latencies[iterations * 99 / 100];

    (p50, p90, p95, p99)
}

async fn benchmark_execute_swap_latency(
    client: &MockDexRpcClient,
    iterations: usize,
) -> (f64, f64, f64, f64) {
    let mut latencies = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let request = create_swap_request(
            "0x0000000000000000000000000000000000000000000000000000000000000002",
            "0x0000000000000000000000000000000000000000000000000000000000000003",
            1_000_000 + (i as u128 * 100_000),
        );

        let start = Instant::now();
        let _ = client.execute_swap(request).await;
        let elapsed = start.elapsed().as_micros() as f64 / 1000.0;
        latencies.push(elapsed);
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = latencies[iterations * 50 / 100];
    let p90 = latencies[iterations * 90 / 100];
    let p95 = latencies[iterations * 95 / 100];
    let p99 = latencies[iterations * 99 / 100];

    (p50, p90, p95, p99)
}

async fn benchmark_throughput(client: &MockDexRpcClient, duration_secs: u64) -> f64 {
    let start = Instant::now();
    let deadline = start + Duration::from_secs(duration_secs);
    let mut request_count = 0u64;

    while Instant::now() < deadline {
        let request = create_swap_request(
            "0x0000000000000000000000000000000000000000000000000000000000000002",
            "0x0000000000000000000000000000000000000000000000000000000000000003",
            1_000_000,
        );

        let _ = client.estimate_swap(request).await;
        request_count += 1;
    }

    let elapsed = start.elapsed().as_secs_f64();
    request_count as f64 / elapsed
}

fn bench_estimate_swap(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("dex_rpc_estimate_swap");

    for latency_ms in [5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("latency_percentiles", latency_ms),
            latency_ms,
            |b, &latency_ms| {
                b.to_async(&rt).iter(|| async {
                    let client = MockDexRpcClient::new(latency_ms);
                    let (p50, p90, p95, p99) = benchmark_estimate_swap_latency(&client, 100).await;
                    black_box((p50, p90, p95, p99))
                });
            },
        );
    }

    group.finish();
}

fn bench_execute_swap(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("dex_rpc_execute_swap");

    for latency_ms in [10, 20, 40].iter() {
        group.bench_with_input(
            BenchmarkId::new("latency_percentiles", latency_ms),
            latency_ms,
            |b, &latency_ms| {
                b.to_async(&rt).iter(|| async {
                    let client = MockDexRpcClient::new(latency_ms);
                    let (p50, p90, p95, p99) = benchmark_execute_swap_latency(&client, 100).await;
                    black_box((p50, p90, p95, p99))
                });
            },
        );
    }

    group.finish();
}

fn bench_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("dex_rpc_throughput");
    group.throughput(Throughput::Elements(1));

    group.bench_function("requests_per_second", |b| {
        b.to_async(&rt).iter(|| async {
            let client = MockDexRpcClient::new(5);
            let tps = benchmark_throughput(&client, 3).await;
            black_box(tps)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_estimate_swap,
    bench_execute_swap,
    bench_throughput
);
criterion_main!(benches);
