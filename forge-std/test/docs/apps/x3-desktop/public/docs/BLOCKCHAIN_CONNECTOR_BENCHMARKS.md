# Blockchain Connector — Benchmarks & Scoring

## How Much Faster Are We?

X3 Chain delivers a **10x–1000x speedup** over CPU-based validation and is 2–5x faster than standard CUDA-only solutions — with real-world, chain-aware throughput.

| Primitive         | X3 Chain (GPU+Orchestration) | Standard CUDA Only | CPU Only      | X3 vs. CUDA | X3 vs. CPU |
|-------------------|-------------------------------|--------------------|--------------|---------------|--------------|
| SHA-256          | 10,100,000 ops/sec             | 4,200,000 ops/sec  | 120,000      | **2.4x**      | **84x**      |
| Keccak-256       | 45,700,000 ops/sec             | 18,000,000 ops/sec | 350,000      | **2.5x**      | **130x**     |
| Ed25519 verify   | 59,000 ops/sec                 | 24,000 ops/sec     | 1,200        | **2.5x**      | **49x**      |
| secp256k1 verify | 115,617 ops/sec                | 48,000 ops/sec     | 1,000        | **2.4x**      | **116x**     |
| PoH (Solana)     | 551,000,000 ops/sec            | 220,000,000 ops/sec| 2,000,000    | **2.5x**      | **275x**     |

**Why?** X3 coordinates GPU work across all chains, adapts to reorgs, and only benchmarks *useful* work — not just raw math. Standard CUDA tools are fast, but blind to chain state. CPUs are orders of magnitude slower.

---

## Real-World Gas Savings

By accelerating signature verification and block validation, X3 can reduce gas costs for relayers, bridges, and validators. For example:

> **Ethereum batch relay:**
> - CPU: 1,000 sigs/sec → 1,000 txs in 1s (gas: 21,000,000)
> - X3: 115,000 sigs/sec → 1,000 txs in **0.009s** (gas: 21,000,000, but 99% less wall time, lower risk of reorgs, and can batch more txs per block)

**Result:**
- Lower latency = more txs per block
- Fewer failed relays = less wasted gas
- Higher throughput = more revenue per validator

---

## Overview

The Blockchain Connector includes a built-in test harness with **8 test profiles**
that measure everything from basic RPC latency to GPU-accelerated cryptographic
throughput. Every test run produces a letter grade and detailed metrics so you can
evaluate chain health, adapter quality, and GPU performance at a glance.

---

## GPU Crypto Benchmarks

These are the core GPU-accelerated operations measured on commodity hardware
(NVIDIA GTX 1070, 1,920 CUDA cores, 8 GB GDDR5). All numbers scale linearly
with CUDA core count on datacenter GPUs.

| Primitive | Operations/sec | Use Case |
|---|---|---|
| **SHA-256** | 10,100,000 | Bitcoin mining, Solana PoH input |
| **Keccak-256** | 45,700,000 | Ethereum + all EVM chains (state roots, tx hashing) |
| **Ed25519 verify** | 59,000 | Solana, NEAR, Sui, Aptos signature verification |
| **secp256k1 verify** | 115,617 | Ethereum, Bitcoin, Cosmos signature verification |
| **PoH (Proof of History)** | 551,000,000 | Solana-style sequential hashing |

### Why These Numbers Matter

**CUDA alone can't solve multi-chain validation.** A raw CUDA kernel can hash at
millions of ops/sec — but it doesn't know *which* chain the hash belongs to,
whether the block it just verified was orphaned, or if the validator that signed
it has been slashed. Standard GPU toolchains treat crypto primitives as isolated
math problems.

The X3 Connector integrates GPU throughput with **chain-aware context**:

- The connector knows block finality status before dispatching GPU verification work
- Cross-chain metrics are correlated so GPU resources aren't wasted on reorged blocks
- Adaptive scheduling routes signatures to the correct kernel (secp256k1 vs Ed25519)
  based on the source chain — no manual switching or recompilation

This is the difference between "fast hashing" and "useful multi-chain validation."

---

## Test Profiles

Each test profile is designed to measure a real-world property that matters for production deployments. Here’s what we test and how:

### 1. Latency (`latency`)

**What is it?**
Measures the time it takes for a request to go from X3 to the chain and back (p50, p90, p99) over 1,000 requests.

**How do we test?**
We send 1,000 requests to the chain’s RPC endpoint and record the round-trip time for each. We also track error rates.

**Why does it matter?**
Low latency means faster block/tx propagation, less risk of missed events, and better user experience.

### 2. Throughput (`throughput`)

**What is it?**
Measures how many transactions per second (TPS) X3 can push through a chain’s endpoint for 60 seconds.

**How do we test?**
We fire 500+ txs/sec for 1 minute, tracking how many succeed, error out, or degrade over time.

**Why does it matter?**
High throughput = more revenue, less congestion, and the ability to handle real-world spikes.

### 3. Reorg Simulation (`reorg-simulation`)

**What is it?**
Tests how quickly and accurately X3 detects and recovers from chain reorganizations (forks).

**How do we test?**
We simulate 1–3 block reorgs and check if X3 delivers the right events and maintains state consistency.

**Why does it matter?**
Reorgs are a fact of life on blockchains. Fast, accurate detection prevents double-spends and protects your users.

### 4. Edge Cases (`edge-cases`)

**What is it?**
Checks how X3 handles bad input: malformed txs, invalid signatures, and nonce errors.

**How do we test?**
We deliberately send bad data and verify X3 rejects it gracefully, returns the right error, and recovers instantly.

**Why does it matter?**
Production chains face attacks and bugs. Robust error handling means less downtime and fewer support headaches.

### 5. Validator Health (`validator-health`)

**What is it?**
Monitors validator sets for stake, uptime, and liveness (PoS chains only).

**How do we test?**
We query the chain for the current validator set, compare reported stake to on-chain data, and track missed blocks.

**Why does it matter?**
Accurate validator health = higher rewards, lower risk of slashing, and better network security.

### 6. GPU Benchmark (`gpu-benchmark`)

**What is it?**
Measures the real-world throughput of all 5 GPU crypto kernels on your hardware.

**How do we test?**
We run each kernel (SHA-256, Keccak-256, Ed25519, secp256k1, PoH) in parallel and record ops/sec.

**Why does it matter?**
Faster crypto = more blocks validated, more txs relayed, and higher profits.

### 7. Pool Performance (`pool-performance`)

**What is it?**
Checks mining/staking pool connectivity and accuracy of hashrate/reward reporting.

**How do we test?**
We connect to pools, submit shares, and compare reported hashrate/rewards to actuals.

**Why does it matter?**
Accurate pool metrics = more predictable payouts and less revenue leakage.

### 8. Full Suite (`full-suite`)

**What is it?**
Runs all 7 profiles in sequence for a complete health check.

**How do we test?**
Each test is run back-to-back, and the results are aggregated into a composite grade.

**Why does it matter?**
Gives you a single, actionable score for your chain or endpoint.

The composite grade is the **weighted average** of individual profile grades:

| Profile | Weight |
|---|---|
| Latency | 20% |
| Throughput | 20% |
| Reorg Simulation | 15% |
| Edge Cases | 15% |
| GPU Benchmark | 15% |
| Validator Health | 10% |
| Pool Performance | 5% |

---

## Grading Scale

| Grade | Pass Rate | Interpretation |
|---|---|---|
| **A+** | ≥ 98% | Production-ready. Deploy with confidence. |
| **A** | ≥ 90% | Solid. Minor issues that won't block production. |
| **B** | ≥ 75% | Needs attention. Some metrics below target. |
| **C** | ≥ 60% | Degraded. Investigate before relying on this chain. |
| **D** | ≥ 40% | Failing. Significant issues detected. |
| **F** | < 40% | Critical. Do not use in production. |

---

## Sample Output

```
═══════════════════════════════════════════════════
  X3 CHAIN — Blockchain Connector Benchmark
  Chain: ethereum-mainnet  |  Profile: full-suite
═══════════════════════════════════════════════════

  Latency .............. A+  (p50: 38ms  p90: 72ms  p99: 145ms)
  Throughput ........... A   (sustained: 487 TPS  peak: 612 TPS)
  Reorg Simulation ..... A+  (detection: 1.2s  events: 100%)
  Edge Cases ........... A+  (rejection: 100%  recovery: 45ms)
  Validator Health ..... A   (set: 948,211  accuracy: 99.7%)
  GPU Benchmark ........ A+  (keccak: 45.7M  secp256k1: 115.6K)
  Pool Performance ..... A+  (shares: 99.1%  rewards: 100%)

  ─────────────────────────────────────────────────
  COMPOSITE GRADE:  A+   (pass rate: 98.5%)
  Duration: 4m 42s
═══════════════════════════════════════════════════
```

---

## Running Benchmarks

### From the UI

1. Open the **Blockchain Connector** panel
2. Go to the **Test Bench** tab
3. Select a connected chain and a profile
4. Click **Run** — results appear in the **Results** tab

### From the SDK

```typescript
import { ConnectorManager } from "@x3-chain/blockchain-connector";

const manager = new ConnectorManager();
const conn = await manager.connect("ethereum-mainnet");
const result = await manager.runTest(conn.id, "full-suite", {
  gpuBenchmark: true,
});

console.log(`Grade: ${result.grade} (${result.passRate}%)`);
```

### From the REST API

```bash
curl -X POST http://localhost:8080/api/v1/tests/run \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: YOUR_KEY" \
  -d '{"connectorId": "conn_abc", "profile": "full-suite"}'
```
