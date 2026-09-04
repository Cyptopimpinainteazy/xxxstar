# X3 Chain — Benchmarking Stack

The X3 benchmarking stack measures: runtime weights, TPS/latency, storage I/O, RPC/WebSocket load, cross-VM latency, GPU/offload performance, and end-to-end mainnet launch gate readiness.

## Quick Start

```bash
# Quick benchmark (reduced samples)
make bench

# Full benchmark suite
make bench-all

# Individual suites
make bench-criterion    # Rust microbenchmarks
make bench-pallets      # FRAME pallet weights
make bench-k6           # k6 RPC/WebSocket load test
make bench-report       # Generate report from collected data
```

## Architecture

```
├── benches/                          # Criterion.rs microbenchmarks
│   ├── Cargo.toml                    # Bench-only crate manifest
│   ├── atomic_swap_bench.rs          # HTLC hashlock, timelock, swap state transitions
│   ├── dex_route_bench.rs            # AMM quotes, route search, anti-rug checks
│   ├── bridge_proof_bench.rs         # Merkle proof verify, replay guard, deserialize
│   ├── vm_dispatch_bench.rs          # Opcode decode, gas metering, VM loop
│   ├── rpc_encoding_bench.rs         # JSON-RPC serialize, hex encode, parse
│   └── signature_verify_bench.rs     # ed25519/sr25519 sign, verify, batch
│
├── benchmarks/
│   ├── k6/
│   │   └── x3_rpc_load.js            # k6 RPC/WebSocket load test script
│   └── zombienet/
│       └── x3-benchmark-network.toml # 5-validator testnet finality benchmark
│
├── scripts/
│   ├── run_all_benchmarks.sh         # Orchestrator: runs suites A-E, generates report
│   ├── benchmark_pallet_weights.sh   # FRAME pallet weight benchmark runner
│   └── benchmark_report.py           # Aggregates Criterion + k6, detects regressions
│
├── monitoring/grafana/dashboards/
│   └── x3-benchmark-dashboard.json   # Grafana dashboard for benchmark metrics
│
├── .github/workflows/
│   └── benchmark-regression.yml      # CI regression gate workflow
│
└── reports/benchmarks/               # Output directory
    ├── x3-benchmark-report-YYYY-MM-DD.json
    └── x3-benchmark-baseline.json
```

## Suites

### Suite A: Criterion Microbenchmarks (Rust)

Measures nanosecond-level hot paths in the critical subsystems.

| Benchmark | Key Metrics | Target |
|-----------|------------|--------|
| `atomic_swap_bench` | sha256/blake2 hashlock, timelock ops, swap state transitions, fee math, intent serialize | Cross-chain atomic swap core |
| `dex_route_bench` | AMM constant product quote, route search (1-100 pools), route scoring, pool update, anti-rug score | DEX routing engine |
| `bridge_proof_bench` | Proof deserialize (tiny-xlarge), merkle verify (depth 4-32), replay guard (cold/hot 10k) | Bridge proof verification |
| `vm_dispatch_bench` | Opcode decode, gas metering, stack push/pop, VM execution loop (5-200 instr) | VM runtime dispatch |
| `rpc_encoding_bench` | Receipt serialize, block header serialize, hex encode/decode, JSON parse (200 logs) | RPC serialization |
| `signature_verify_bench` | ed25519 sign/verify, sr25519 sign/verify, batch verify (1-128), address derivation | Validator crypto |

### Suite B: FRAME Pallet Weight Benchmarks

Generates real `weights.rs` files for 27 X3 pallets using `frame-benchmarking-cli`.

```bash
# Build node with benchmarks
cargo build -p node --release --features runtime-benchmarks

# Run all pallet weight benchmarks
make bench-pallets
```

Weight files are written to `runtime/src/weights/`.

**Pallet list**: x3-kernel, atomic-trade-engine, x3-dex, x3-atomic-kernel, x3-custody, x3-cross-vm-router, x3-supply-ledger, x3-asset-registry, x3-token-factory, x3-staking, x3-oracle, x3-governance, x3-slash, x3-launchpad, x3-flashloan, x3-auction, x3-compute-market, x3-sequencer, x3-da, x3-invariants, x3-lp-locker, x3-wrapped, x3-reconciliation, x3-solvency, x3-inventory, x3-reservation, x3-rebalance

### Suite C: k6 RPC / WebSocket Load Test

Tests 9 RPC endpoints under concurrent load with latency histograms, error rates, and WebSocket reconnect tracking.

| Endpoint | Type |
|----------|------|
| `eth_call` | Read-only |
| `eth_sendRawTransaction` | Write |
| `eth_getLogs` | Historical query |
| `x3_submitAtomicSwap` | Cross-chain write |
| `x3_quoteMultiVmSwap` | Quote |
| `x3_getBridgeStatus` | Bridge query |
| `x3_estimateAtomicFee` | Fee estimation |
| `x3_getValidatorMetrics` | Metrics |
| WebSocket `chain_subscribeNewHeads` | Subscription |

```bash
# Start node first, then:
RPC_URL=http://127.0.0.1:9933 WS_URL=ws://127.0.0.1:9944 make bench-k6
```

### Suite D: Zombienet Network Benchmark

Deploys a 5-validator testnet and measures:
- Finality time under load
- Block import/sync time after catch-up
- Network partition recovery convergence
- Validator count rampdown finality

```bash
zombienet spawn benchmarks/zombienet/x3-benchmark-network.toml
```

### Suite E: Storage & Network I/O

Uses `fio` for disk IOPS/latency and `iperf3` for network throughput.

## Report Format

Benchmark reports follow a standardized JSON schema at `reports/benchmarks/x3-benchmark-report-YYYY-MM-DD.json`:

```json
{
  "schema_version": "1.0.0",
  "commit": "git_sha",
  "branch": "main",
  "timestamp": "2026-04-24T00:00:00Z",
  "machine": {
    "cpu": "Threadripper 1900X",
    "ram_gb": 64,
    "os": "Ubuntu 22.04"
  },
  "criteria": { "...": { "mean_ns": 1234, "p95_ns": 1500 } },
  "k6": { "...": { "eth_call_p95": 120 } },
  "regressions": [
    { "metric": "atomic_swap_p99_ns", "previous_mean_ns": 420, "current_mean_ns": 610, "delta_percent": 45.2, "status": "fail" }
  ],
  "verdict": "pass|fail"
}
```

## CI Regression Gate

The `.github/workflows/benchmark-regression.yml` workflow runs on:
- Push to `main` (benchmark-sensitive paths)
- Pull requests (benchmark-sensitive paths)
- Daily at 4am UTC (`cron: 0 4 * * *`)
- Manual dispatch (`workflow_dispatch`)

It fails the gate if:
- Any Criterion benchmark degrades > threshold (10-25% depending on metric)
- Any RPC latency threshold is exceeded (e.g., eth_call p95 > 500ms)
- The overall verdict is `fail`

## Regression Thresholds

| Metric | Threshold | Direction |
|--------|-----------|-----------|
| atomic_swap_p99_ns | 20% | ↑ slower |
| dex_route_p99_ns | 15% | ↑ slower |
| bridge_proof_verify_ns | 20% | ↑ slower |
| vm_dispatch_ns | 15% | ↑ slower |
| rpc_encode_p99_ns | 20% | ↑ slower |
| sig_verify_p99_ns | 10% | ↑ slower |
| eth_call_p95_ms | 30% | ↑ slower |
| eth_call_p99_ms | 30% | ↑ slower |
| atomic_cross_vm_p99_ms | 25% | ↑ slower |
| block_import_p99_ms | 20% | ↑ slower |
| bridge_verify_per_sec | -15% | ↓ slower |

## Grafana Dashboard

Import `monitoring/grafana/dashboards/x3-benchmark-dashboard.json` into Grafana for a live dashboard with:
- Criterion benchmark bargauges (atomic swap, DEX route, bridge proof)
- RPC latency graphs (p50/p95/p99 for eth_call, quoteMultiVmSwap)
- RPC throughput and WebSocket connection stats
- Block production finality lag and import time
- Regression heatmap table
- Pass/fail verdict indicator

## Adding New Benchmarks

1. **Criterion bench**: Add a `.rs` file in `benches/`, add `[[bench]]` entry in `benches/Cargo.toml`
2. **Pallet weight**: Add entry to `PALLETS` array in `scripts/benchmark_pallet_weights.sh`
3. **k6 endpoint**: Add function in `benchmarks/k6/x3_rpc_load.js`, update `export default` to call it
4. **Regression threshold**: Add entry to `REGRESSION_THRESHOLDS` in `scripts/benchmark_report.py`

## Priority Order

1. FRAME weights for every pallet — no placeholder weights at mainnet
2. Criterion benches for atomic swap, DEX routing, bridge proof verification
3. k6/Goose RPC load tests
4. Zombienet validator/finality benchmark
5. Prometheus/Grafana benchmark dashboard
6. Regression gate in CI
7. Public benchmark report generator

## Rule

No X3 commit is "better" unless the benchmark report proves it.