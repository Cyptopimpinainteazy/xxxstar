# X3 Chain Finalized TPS Comparison Report

**Purpose:** Document real X3 `submitComitV2` finalized throughput vs. other chains' non-vote throughput.

## Baseline Measurements

### X3 Chain (Real submitComitV2, patched native runtime)

| Metric | Value | Unit | Notes |
|--------|-------|------|-------|
| **Finalized TPS** | _pending_ | tx/sec | Via nonce delta; primary metric |
| **In-Block TPS** | _pending_ | tx/sec | During submission phase |
| **Avg Block Time** | 200 | ms | MILLISECS_PER_BLOCK config |
| **Blocks/Sec** | 5 | blocks/sec | 200ms intervals |
| **Test Duration** | 180–600 | sec | Benchmark run length |
| **Concurrent Senders** | 240–1024 | accounts | Distributed load |
| **Total Submissions** | _pending_ | extrinsics | Over full duration |
| **Avg Txpool Depth** | _pending_ | pending tx | Queue during load |
| **Block Weight Used** | _pending_ | % | Per block; limited to 150ms |
| **Errors (Rate Limit)** | _pending_ | count | Expected behavior |
| **Errors (Nonce Conflict)** | _pending_ | count | Should be minimal |

**Benchmark Command:**
```bash
python3 run-multiprocess-load.py \
  --rpc-ws ws://127.0.0.1:9944 \
  --workers 8 \
  --senders 240 \
  --duration-sec 600 \
  --finality-wait-sec 45 \
  --concurrency-total 1024
```

**Benchmark Date:** _pending_  
**Benchmark Output:** `benchmarks/x3_chain_submit_comit_v2_tps_multiprocess.json`

---

## Known Benchmarks (For Comparison)

### Solana (Non-Vote Transactions)
| Metric | Value | Source |
|--------|-------|--------|
| **Non-Vote TPS** | 4,000–10,000 | Mainnet Beta (varies by network load) |
| **Block Time** | 400–800 | ms (average) |
| **Max Theoretical** | 65,000 | tx/sec (upper limit; rarely reached) |

**Notes:**
- Solana measures non-vote TPS (excludes consensus voting)
- TPS fluctuates based on network congestion
- Recent optimizations (Firedancer) target 1M TPS (experimental)

### Ethereum L1 (Post-Dencun)
| Metric | Value | Source |
|--------|-------|--------|
| **Mainnet TPS** | 15–25 | tx/sec (non-MEV bundles) |
| **Block Time** | 12 | sec (average) |
| **Target Gas** | 30M | units/block (flexible) |

**Notes:**
- Ethereum prioritizes security over throughput
- Layer 2s (Arbitrum, Optimism) achieve 4,000–7,000 TPS

### Polkadot Parachain (Typical)
| Metric | Value | Source |
|--------|-------|--------|
| **Relay Chain TPS** | 300–600 | non-relay extrinsics/sec |
| **Block Time** | 6 | sec |
| **Parachain TPS** | 1,000–3,000 | variable (app-dependent) |

**Notes:**
- Throughput depends on pallet logic and weight limits
- X3 chain aims to be a Polkadot parachain (future)

---

## Comparison Methodology

### Fairness Considerations

1. **Metric Alignment:** Compare X3 **finalized** TPS (via nonce delta) against other chains' **finalized** or **committed** transactions, not in-flight submissions.

2. **Payload Complexity:**
   - X3: submitComitV2 with real X3BC bytecode (~200 bytes) + potential EVM/SVM execution
   - Solana: Standard token transfers or program invocations
   - Ethereum: Standard ERC-20 transfers or contract calls
   - **Adjustment:** If X3 payloads are heavier, adjust TPS downward by (payload_size_ratio)

3. **Concurrency Model:**
   - X3: Async multi-process (8 workers, 240 signers, 1024 concurrent)
   - Solana: Parallel processing with leader rotation
   - Ethereum: Single-threaded consensus, but high parallelism in execution
   - **Adjustment:** X3 can scale concurrency; TPS growth should follow load increase

4. **Rate Limiting:**
   - X3: 10 submissions per account per block (enforced)
   - Solana: No per-account limit; rate limit at network level
   - Ethereum: No per-account limit; rate limit at validator level
   - **Adjustment:** If X3 is rate-limited, increase senders count to overcome limit

### Expected Outcome

**Hypothesis:** X3 finalized TPS should be in range:

- **Conservative (baseline):** 100–500 tx/sec
  - If block weights are tight
  - If finality time is long (45+ sec wait)
  - If adapters (EVM/SVM) have startup overhead

- **Optimistic (tuned):** 500–2,000 tx/sec
  - With perf-mode block weight tuning
  - With lower finality wait (rollback guarantees)
  - With optimized adapters

- **Theoretical Max:** 5,000+ tx/sec
  - If block weights are increased
  - If finality is instant (unsafe)
  - If adapters run in parallel per extrinsic

---

## Results Template

**Fill in after running benchmark:**

```json
{
  "benchmark_run": {
    "timestamp_utc": "2026-04-04T12:00:00Z",
    "chain": "x3-chain-native",
    "runtime_version": "1.0",
    "network": "local|testnet|mainnet",
    "patched": true,
    "native_adapters_used": true
  },
  "test_parameters": {
    "workers": 8,
    "senders": 240,
    "concurrency_total": 1024,
    "duration_sec": 600,
    "finality_wait_sec": 45
  },
  "results": {
    "finalized_tps": null,
    "in_block_tps": null,
    "avg_block_time_ms": null,
    "blocks_per_sec": null,
    "total_submissions": null,
    "finalized_extrinsics": null,
    "avg_txpool_depth": null,
    "max_txpool_depth": null,
    "block_weight_utilization_pct": null,
    "error_rate_pct": null,
    "rate_limit_errors": null,
    "nonce_conflicts": null
  },
  "comparison": {
    "vs_solana_non_vote_tps": "x3_finalized_tps / solana_typical_non_vote",
    "vs_ethereum_mainnet_tps": "x3_finalized_tps / ethereum_mainnet_typical",
    "vs_polkadot_parachain_tps": "x3_finalized_tps / polkadot_parachain_typical"
  },
  "bottleneck_analysis": {
    "block_weight_limit": null,
    "block_length_limit": null,
    "finality_latency": null,
    "adapter_overhead": null,
    "rate_limit_impact": null
  },
  "observations": []
}
```

---

## Tuning Guide

If finalized TPS is below expected range:

### 1. **Increase Block Weight Budget**
   - Current: 150ms ref time per block
   - Try: 200ms or 250ms
   - File: `runtime/src/lib.rs:173–175`
   - Trade-off: Longer finality time

### 2. **Increase Block Length**
   - Current: 5 MB hard cap
   - Try: 10 MB
   - File: `runtime/src/lib.rs:178–180`
   - Trade-off: Higher bandwidth requirement

### 3. **Reduce Rate Limit**
   - Current: 10 submissions per account per block
   - Try: 20 or unlimited (if governance allows)
   - File: `pallets/x3-kernel/src/lib.rs:1143`
   - Trade-off: Risk of spam

### 4. **Optimize Adapters**
   - Profile EVM/SVM/X3 adapter execution time
   - Consider parallel execution if safe
   - May require runtime changes

### 5. **Use More Signers**
   - Current: 240 signers
   - Try: 1,000+ signers
   - Overcomes per-account rate limits
   - Cost: Authorization overhead

---

## Reporting

When publishing results, include:

1. **Test Setup:** Workers, senders, concurrency, duration
2. **Finalized TPS:** Primary metric with 95% confidence interval
3. **Bottleneck:** What limited throughput?
4. **Comparison:** How does X3 compare to known chains?
5. **Reproducibility:** Command and JSON output for replication
6. **Limitations:** What assumptions were made? What was not tested?

---

**Report Date:** _pending_  
**Analyst:** X3 Chain Benchmark Team  
**Status:** Work in Progress
