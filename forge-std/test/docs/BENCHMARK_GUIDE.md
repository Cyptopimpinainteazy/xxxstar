# Real X3 Kernel Benchmark Guide

This guide explains how to deploy and run the real X3 `submitComitV2` TPS benchmark, replacing the old synthetic `system.remark` benchmark.

## Overview

The benchmark harness submits real `x3Kernel.submitComitV2` extrinsics with:
- **Real X3BC payloads** generated via `assemble_simple_module()` (200+ byte bytecode)
- **Valid authorization** via `authorize_account` (governance-only, requires sudo)
- **Correct prepare_root computation** using Blake2-256 as per kernel spec
- **Rate limit distribution** across many signers (10 submissions/block per account)
- **Rich telemetry** measuring block times, signed extrinsics, in-block TPS, finalized TPS, txpool depth

## Quick Start

### Prerequisites

1. **Running X3 Chain validator** with the patched native runtime:
   - Patches: runtime EVM adapter now fails hard (no mock fallback), RPC stub functions renamed, mock RPC export removed
   - Binary: Built with `cargo build -p x3-chain-node --release`
   - Feature: `native-real-vm-adapters` enabled (default in native runtime)

2. **Node.js 18+** with Polkadot dependencies installed:
   ```bash
   cd scripts/testnet && npm install
   ```

3. **Root account (Alice) available** on chain for authorization via sudo

### Run Benchmark

#### Single-process test (dev/debug):
```bash
cd scripts/testnet
node load-x3-comit-v2-tps.js \
  --wsEndpoint ws://127.0.0.1:9944 \
  --numSenders 12 \
  --concurrency 48 \
  --durationSec 60 \
  --finalityWaitSec 20 \
  --verbose
```

#### Multi-process benchmark (production/testnet):
```bash
cd scripts/testnet
python3 run-multiprocess-load.py \
  --rpc-ws ws://127.0.0.1:9944 \
  --workers 8 \
  --senders 240 \
  --duration-sec 180 \
  --finality-wait-sec 45 \
  --concurrency-total 1024 \
  --output benchmarks/x3_chain_submit_comit_v2_tps_multiprocess.json
```

### Output

Benchmark produces JSON with metrics:

```json
{
  "timestamp_utc": "2026-04-04T08:50:30Z",
  "benchmark": "x3_chain_submit_comit_v2_tps_multiprocess",
  "rpc_ws": "ws://127.0.0.1:9944",
  "workers": 8,
  "senders_total": 240,
  "successful_workers": 8,
  "finalized_total": 12480,
  "finalized_tps_submit_window": 69.3,
  "avg_block_time_ms": 200.5,
  "avg_signed_extrinsics_per_block": 50.2,
  "in_block_tps": 251.0,
  "avg_txpool_depth": 1204,
  "failure_reasons": { "rate_limit": 0, "nonce_conflict": 0 }
}
```

**Key metrics:**
- `finalized_tps_submit_window`: Real finalized TPS (via nonce delta) — **primary metric**
- `in_block_tps`: TPS during submission phase (may include pending)
- `avg_block_time_ms`: Chain's actual block production time
- `failure_reasons`: Per-account rate limits, nonce conflicts, other errors

## Detailed Setup

### 1. Deploy X3 Chain Node

Build the patched native runtime:
```bash
cargo build -p x3-chain-node --release
```

Start validator:
```bash
./target/release/x3-chain-node \
  --chain local \
  --validator \
  --tmp \
  --rpc-port 9944 \
  --ws-external
```

Verify it's accepting RPC:
```bash
curl -X POST http://127.0.0.1:9944 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_properties","params":[],"id":1}'
```

### 2. Authorize Benchmark Signers

Before running the load test, authorize all signers with sudo:

```bash
# Single command to authorize N signers (requires Alice/root on chain)
node authorize-accounts.js \
  --wsEndpoint ws://127.0.0.1:9944 \
  --baseDerivation //Alice//load \
  --count 240 \
  --sudoSeed //Alice
```

Or manually via chain UI (Polkadot.js Apps):
1. Connect to `ws://127.0.0.1:9944`
2. Extrinsics → x3Kernel → authorizeAccount
3. Select account → sign with sudo

### 3. Run Benchmark

**Dev mode** (small scale, 1–2 minutes):
```bash
node load-x3-comit-v2-tps.js \
  --wsEndpoint ws://127.0.0.1:9944 \
  --numSenders 16 \
  --concurrency 64 \
  --durationSec 120 \
  --finalityWaitSec 30 \
  --verbose
```

**Production mode** (full scale, 3–20 minutes):
```bash
python3 run-multiprocess-load.py \
  --rpc-ws ws://127.0.0.1:9944 \
  --workers 8 \
  --senders 240 \
  --duration-sec 600 \
  --finality-wait-sec 45 \
  --concurrency-total 1024 \
  --output benchmarks/x3_chain_baseline_tps.json
```

## Metrics Explanation

### Finalized TPS (Primary)
Measures **ground-truth finalization** via nonce delta:
- All benchmark signers start at nonce `N`
- After benchmark, measure final nonce `N'` per account
- Finalized extrinsics = Σ (N' - N) across all signers
- Finalized TPS = finalized extrinsics / (submit_time + finality_wait_time)

This avoids race conditions with in-flight submissions and pending blocks.

### In-Block TPS
Measures average signed extrinsics per block during submission phase:
- Average = total_submitted / blocks_during_submission
- TPS = average * blocks_per_sec (5 blocks/sec at 200ms/block)

This can exceed finalized TPS if many submissions are pending.

### Block Time
Average time between block production during benchmark:
- Should stabilize at 200ms (1 MILLISECS_PER_BLOCK = 200)
- Indicates chain stability and load impact

### Txpool Depth
Average size of pending transaction pool during benchmark:
- Reflects queue buildup under high concurrency
- Useful for tuning `concurrency-total` parameter

## Troubleshooting

### "Authorization required" errors
**Solution:** Signers not authorized. Run authorization script or sudo each signer before benchmark.

### "Rate limit exceeded" errors
**Expected behavior.** X3 allows 10 submissions/block per account. Script auto-distributes across signers.

### "Nonce conflict" errors
**Cause:** Concurrent submissions from same signer. Increase `--numSenders` or reduce `--concurrency`.

### Low finalized TPS (< 10/sec)
**Possible causes:**
1. Chain block time is higher than 200ms (check logs)
2. Rate limits are tight; increase senders
3. Finality time is long; adjust `--finalityWaitSec`
4. Some adapters (EVM/SVM) are slow; check runtime logs

### Script hangs on "waiting for finality"
**Cause:** Chain not producing blocks or not finalizing submissions.
1. Check chain logs: `grep -i "error\|panic" chain.log`
2. Verify RPC endpoint is reachable: `curl ws://127.0.0.1:9944`
3. Increase `--finalityWaitSec` or reduce `--durationSec`

## File Locations

- **Load script:** `scripts/testnet/load-x3-comit-v2-tps.js`
- **Multiprocess orchestrator:** `scripts/testnet/run-multiprocess-load.py`
- **Authorization helper:** `scripts/testnet/authorize-accounts.js` (create if needed)
- **Benchmark output:** `benchmarks/x3_chain_submit_comit_v2_tps_multiprocess.json`
- **Runtime patches:** `runtime/src/lib.rs` (EVM adapter, RPC wiring)
- **Node binary:** `target/release/x3-chain-node`

## Expected Performance

With the current conservative block weights:
- **Block weight budget:** ~150ms ref time per block
- **Block length limit:** 5 MB
- **Expected range:** 50–300 finalized TPS (depends on payload complexity, adapters, network)

If throughput is bottlenecked:
1. Check block weight utilization in chain logs
2. Enable perf mode (increase `MILLISECS_PER_BLOCK`, reduce block weight limits)
3. Re-run benchmark to measure peak throughput

## Next Steps

1. Deploy patched node on testnet or local chain
2. Authorize 100–1000 benchmark signers
3. Run full-scale benchmark for 10+ minutes
4. Compare finalized X3 TPS against Solana non-vote TPS and other chains
5. Identify bottlenecks (adapters, finality, block weights) and tune

---

**Last Updated:** 2026-04-04
**Benchmark Version:** x3_kernel_submit_comit_v2_real_payload
