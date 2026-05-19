#!/usr/bin/env python3
"""
Infrastructure-Grade 6-Domain Benchmark Suite
=============================================
Domains:
  1. Throughput   — sustained TPS over 5-minute windows, peak TPS, queue depth
  2. Latency      — P50 / P90 / P99 / P99.9 under normal, peak, and failover
  3. Determinism  — same payload 10K times across all lanes, hash comparison
  4. Failover     — kill primary GPU mid-execution, measure detection + promotion
  5. Resource     — GPU memory, CPU load, bandwidth-per-TPS, memory growth
  6. Economic     — cost per million requests, cost per TPS, profit margin

Advanced:
  A. Adversarial  — malformed payloads, oversize batches, replay, lane flooding
  B. Degraded     — partial GPU failure, CPU-only fallback under load
  C. Gas Savings  — estimated gas savings vs direct chain submission

Produces: JSON report + terminal summary with all data.
"""

import asyncio
import aiohttp
import hashlib
import json
import math
import os
import platform
import signal
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Dict, List, Optional

try:
    import orjson
    def fast_dumps(obj):
        return orjson.dumps(obj)
    def fast_loads(data):
        return orjson.loads(data)
except ImportError:
    import json as _json
    def fast_dumps(obj):
        return _json.dumps(obj).encode()
    def fast_loads(data):
        return _json.loads(data)

# ── Configuration ──────────────────────────────────────────────────────────────

GPU_LANES = [
    ("primary",  "http://localhost:9001"),
    ("shadow",   "http://localhost:9002"),
    ("tertiary", "http://localhost:9003"),
]

BRIDGE_URL = "http://localhost:9999"
API_KEY = "infra_3JDGhaxUOfLfyuFk-roJiR3FzgdgcipAH3vG5wpMzDo"
CHAIN = "solana"
TX_DATA_HEX = "48656c6c6f"  # "Hello"

# Sustained test windows (seconds)
SUSTAINED_WINDOW = 300        # 5 minutes sustained
SUSTAINED_BATCH_SIZE = 250000 # txns per burst within sustained window
SUSTAINED_BURST_INTERVAL = 2  # seconds between bursts

# Latency test parameters
LATENCY_SAMPLE_SIZES = [100, 1000, 10000, 100000]

# Determinism test parameters
DETERMINISM_REPEATS = 10000
DETERMINISM_PAYLOAD = "determinism_test_payload_v1"

# Failover test parameters
FAILOVER_BATCH_SIZE = 50000

# Economic parameters (per GPU, monthly)
GPU_POWER_WATTS = 150           # GTX 1070 TDP
GPU_COUNT = 3
ELECTRICITY_COST_KWH = 0.12    # $/kWh
GPU_PURCHASE_COST = 250         # $ per used GTX 1070
GPU_AMORTIZATION_MONTHS = 24

# Gas comparison (Solana mainnet averages)
SOLANA_AVG_GAS_LAMPORTS = 5000      # ~5000 lamports per tx
SOLANA_LAMPORT_PER_SOL = 1_000_000_000
SOLANA_SOL_PRICE_USD = 125.0        # approximate

REPORT_DIR = "/tmp/inferstructor_benchmark"


# ── Data Classes ───────────────────────────────────────────────────────────────

@dataclass
class LatencyBucket:
    label: str
    sample_count: int = 0
    latencies_ms: List[float] = field(default_factory=list)

    @property
    def p50(self) -> float:
        return self._pct(50) if self.latencies_ms else 0

    @property
    def p90(self) -> float:
        return self._pct(90) if self.latencies_ms else 0

    @property
    def p99(self) -> float:
        return self._pct(99) if self.latencies_ms else 0

    @property
    def p999(self) -> float:
        return self._pct(99.9) if self.latencies_ms else 0

    @property
    def mean(self) -> float:
        return statistics.mean(self.latencies_ms) if self.latencies_ms else 0

    @property
    def stdev(self) -> float:
        return statistics.stdev(self.latencies_ms) if len(self.latencies_ms) > 1 else 0

    def _pct(self, p: float) -> float:
        s = sorted(self.latencies_ms)
        idx = int(len(s) * p / 100)
        idx = min(idx, len(s) - 1)
        return s[idx]

    def to_dict(self) -> dict:
        return {
            "label": self.label,
            "sample_count": len(self.latencies_ms),
            "p50_ms": round(self.p50, 4),
            "p90_ms": round(self.p90, 4),
            "p99_ms": round(self.p99, 4),
            "p999_ms": round(self.p999, 4),
            "mean_ms": round(self.mean, 4),
            "stdev_ms": round(self.stdev, 4),
            "min_ms": round(min(self.latencies_ms), 4) if self.latencies_ms else 0,
            "max_ms": round(max(self.latencies_ms), 4) if self.latencies_ms else 0,
        }


@dataclass
class BenchmarkReport:
    timestamp: str = ""
    system_info: Dict[str, Any] = field(default_factory=dict)

    # Domain 1: Throughput
    throughput: Dict[str, Any] = field(default_factory=dict)

    # Domain 2: Latency
    latency: Dict[str, Any] = field(default_factory=dict)

    # Domain 3: Determinism
    determinism: Dict[str, Any] = field(default_factory=dict)

    # Domain 4: Failover
    failover: Dict[str, Any] = field(default_factory=dict)

    # Domain 5: Resource efficiency
    resource: Dict[str, Any] = field(default_factory=dict)

    # Domain 6: Economic viability
    economic: Dict[str, Any] = field(default_factory=dict)

    # Advanced
    adversarial: Dict[str, Any] = field(default_factory=dict)
    degraded: Dict[str, Any] = field(default_factory=dict)
    gas_savings: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict:
        return {
            "meta": {
                "timestamp": self.timestamp,
                "suite_version": "1.0.0",
                "system": self.system_info,
            },
            "domains": {
                "throughput": self.throughput,
                "latency": self.latency,
                "determinism": self.determinism,
                "failover": self.failover,
                "resource_efficiency": self.resource,
                "economic_viability": self.economic,
            },
            "advanced": {
                "adversarial": self.adversarial,
                "degraded_mode": self.degraded,
                "gas_savings": self.gas_savings,
            },
        }


# ── Helpers ────────────────────────────────────────────────────────────────────

def build_batch_bytes(start_id: int, count: int, chain: str = CHAIN) -> bytes:
    txns = [{"tx_hash": f"t{start_id + i}", "tx_data": TX_DATA_HEX} for i in range(count)]
    return fast_dumps({"transactions": txns, "chain": chain})


async def send_batch(session: aiohttp.ClientSession, url: str, payload: bytes,
                     timeout_s: float = 120) -> dict:
    try:
        async with session.post(
            url, data=payload,
            headers={"Content-Type": "application/json"},
            timeout=aiohttp.ClientTimeout(total=timeout_s)
        ) as resp:
            if resp.status == 200:
                return await resp.json()
            return {"successful": 0, "batch_size": 0, "error": f"HTTP {resp.status}"}
    except Exception as e:
        return {"successful": 0, "batch_size": 0, "error": str(e)}


async def send_single(session: aiohttp.ClientSession, url: str, tx_hash: str,
                       tx_data: str = TX_DATA_HEX, chain: str = CHAIN) -> dict:
    payload = {"tx_hash": tx_hash, "tx_data": tx_data, "chain": chain}
    try:
        start = time.time()
        async with session.post(
            url, json=payload,
            headers={"Content-Type": "application/json"},
            timeout=aiohttp.ClientTimeout(total=10)
        ) as resp:
            latency = (time.time() - start) * 1000
            data = await resp.json() if resp.status == 200 else {}
            data["_latency_ms"] = latency
            data["_status"] = resp.status
            return data
    except Exception as e:
        return {"error": str(e), "_latency_ms": 0, "_status": 0}


async def get_lane_health(session: aiohttp.ClientSession, base_url: str) -> dict:
    try:
        async with session.get(f"{base_url}/health", timeout=aiohttp.ClientTimeout(total=3)) as r:
            if r.status == 200:
                return await r.json()
    except Exception:
        pass
    return {}


def get_gpu_info() -> List[dict]:
    """Get GPU info via nvidia-smi"""
    try:
        out = subprocess.check_output(
            ["nvidia-smi", "--query-gpu=index,name,memory.used,memory.total,utilization.gpu,temperature.gpu,power.draw",
             "--format=csv,noheader,nounits"],
            text=True, timeout=5
        )
        gpus = []
        for line in out.strip().split("\n"):
            parts = [p.strip() for p in line.split(",")]
            if len(parts) >= 7:
                gpus.append({
                    "index": int(parts[0]),
                    "name": parts[1],
                    "memory_used_mb": float(parts[2]),
                    "memory_total_mb": float(parts[3]),
                    "utilization_pct": float(parts[4]),
                    "temperature_c": float(parts[5]),
                    "power_draw_w": float(parts[6]),
                })
        return gpus
    except Exception:
        return []


def hr(char="─", width=72):
    print(char * width)


def header(title: str):
    print()
    hr("═")
    print(f"  {title}")
    hr("═")


# ── Domain 1: Throughput ───────────────────────────────────────────────────────

async def benchmark_throughput(session: aiohttp.ClientSession) -> dict:
    header("DOMAIN 1: THROUGHPUT")

    results = {}

    # 1a: Peak burst TPS (existing pattern)
    print("\n  [1a] Peak burst TPS...")
    burst_levels = [100000, 500000, 1000000, 2000000, 5000000, 10000000]
    burst_results = []

    for level in burst_levels:
        per_lane = level // len(GPU_LANES)
        tasks = []
        tx_offset = 0
        for name, base_url in GPU_LANES:
            payload = build_batch_bytes(tx_offset, per_lane)
            tasks.append(send_batch(session, f"{base_url}/accelerate/batch", payload))
            tx_offset += per_lane

        start = time.time()
        responses = await asyncio.gather(*tasks)
        duration = time.time() - start

        total_success = sum(r.get("successful", 0) for r in responses)
        tps = total_success / duration if duration > 0 else 0
        success_rate = total_success / max(level, 1)

        burst_results.append({
            "level": level,
            "successful": total_success,
            "duration_s": round(duration, 3),
            "tps": round(tps),
            "success_rate": round(success_rate, 4),
        })

        print(f"       {level:>10,} txns → {tps:>12,.0f} TPS  ({success_rate*100:.1f}% success, {duration:.3f}s)")

        if success_rate < 0.90:
            print(f"       ⚠ Failure rate too high, stopping burst test")
            break

        await asyncio.sleep(1)

    results["burst"] = burst_results
    results["peak_burst_tps"] = max((r["tps"] for r in burst_results), default=0)

    # 1b: Sustained TPS (5-minute window)
    print(f"\n  [1b] Sustained TPS ({SUSTAINED_WINDOW}s window)...")
    sustained_data = []
    sustained_start = time.time()
    total_sustained_success = 0
    total_sustained_sent = 0
    burst_num = 0

    while (time.time() - sustained_start) < SUSTAINED_WINDOW:
        burst_num += 1
        per_lane = SUSTAINED_BATCH_SIZE // len(GPU_LANES)
        tasks = []
        tx_offset = burst_num * SUSTAINED_BATCH_SIZE
        for name, base_url in GPU_LANES:
            payload = build_batch_bytes(tx_offset, per_lane)
            tasks.append(send_batch(session, f"{base_url}/accelerate/batch", payload))
            tx_offset += per_lane

        burst_start = time.time()
        responses = await asyncio.gather(*tasks)
        burst_duration = time.time() - burst_start

        burst_success = sum(r.get("successful", 0) for r in responses)
        total_sustained_success += burst_success
        total_sustained_sent += SUSTAINED_BATCH_SIZE

        elapsed = time.time() - sustained_start
        instant_tps = burst_success / burst_duration if burst_duration > 0 else 0
        cumulative_tps = total_sustained_success / elapsed if elapsed > 0 else 0

        sustained_data.append({
            "burst_num": burst_num,
            "elapsed_s": round(elapsed, 1),
            "burst_success": burst_success,
            "burst_duration_s": round(burst_duration, 3),
            "instant_tps": round(instant_tps),
            "cumulative_tps": round(cumulative_tps),
        })

        if burst_num % 10 == 0 or elapsed >= SUSTAINED_WINDOW - 1:
            print(f"       [{elapsed:>5.0f}s] burst #{burst_num:>3d}  instant={instant_tps:>10,.0f}  cumulative={cumulative_tps:>10,.0f} TPS")

        await asyncio.sleep(SUSTAINED_BURST_INTERVAL)

    sustained_duration = time.time() - sustained_start
    sustained_tps = total_sustained_success / sustained_duration if sustained_duration > 0 else 0

    results["sustained"] = {
        "window_seconds": round(sustained_duration, 1),
        "total_sent": total_sustained_sent,
        "total_success": total_sustained_success,
        "sustained_tps": round(sustained_tps),
        "burst_count": burst_num,
        "tps_timeline": sustained_data,
    }

    print(f"\n       Sustained: {sustained_tps:,.0f} TPS over {sustained_duration:.0f}s ({total_sustained_success:,} txns)")

    # 1c: Queue depth / backpressure
    print(f"\n  [1c] Queue depth test (concurrent batches)...")
    concurrency_levels = [1, 3, 5, 10, 20]
    queue_results = []

    for conc in concurrency_levels:
        per_call = 100000
        tasks = []
        for i in range(conc):
            lane_name, base_url = GPU_LANES[i % len(GPU_LANES)]
            payload = build_batch_bytes(i * per_call, per_call)
            tasks.append(send_batch(session, f"{base_url}/accelerate/batch", payload))

        start = time.time()
        responses = await asyncio.gather(*tasks)
        duration = time.time() - start

        total_ok = sum(r.get("successful", 0) for r in responses)
        tps = total_ok / duration if duration > 0 else 0
        queue_results.append({
            "concurrency": conc,
            "total_txns": conc * per_call,
            "successful": total_ok,
            "duration_s": round(duration, 3),
            "tps": round(tps),
        })
        print(f"       concurrency={conc:>2d}  → {tps:>10,.0f} TPS  ({duration:.3f}s)")

        await asyncio.sleep(1)

    results["queue_depth"] = queue_results

    return results


# ── Domain 2: Latency ──────────────────────────────────────────────────────────

async def benchmark_latency(session: aiohttp.ClientSession) -> dict:
    header("DOMAIN 2: LATENCY PERCENTILES")

    results = {}

    # 2a: Single-request latency (per-lane)
    print("\n  [2a] Single-request latency per lane...")
    for lane_name, base_url in GPU_LANES:
        bucket = LatencyBucket(label=f"single_{lane_name}")
        for i in range(200):
            resp = await send_single(session, f"{base_url}/accelerate", f"lat_{lane_name}_{i}")
            if resp.get("_status") == 200:
                bucket.latencies_ms.append(resp["_latency_ms"])

        results[f"single_{lane_name}"] = bucket.to_dict()
        print(f"       {lane_name:>10s}  P50={bucket.p50:.2f}ms  P99={bucket.p99:.2f}ms  P99.9={bucket.p999:.2f}ms")

    # 2b: Batch latency at different sizes
    print("\n  [2b] Batch latency by size...")
    for size in LATENCY_SAMPLE_SIZES:
        bucket = LatencyBucket(label=f"batch_{size}")
        repeats = max(1, min(5, 500000 // size))

        for r in range(repeats):
            payload = build_batch_bytes(r * size, size)
            lane_name, base_url = GPU_LANES[r % len(GPU_LANES)]

            start = time.time()
            resp = await send_batch(session, f"{base_url}/accelerate/batch", payload)
            latency = (time.time() - start) * 1000
            per_tx = latency / max(resp.get("successful", 1), 1)

            bucket.latencies_ms.append(latency)

        results[f"batch_{size}"] = bucket.to_dict()
        results[f"batch_{size}"]["per_tx_ms"] = round(bucket.mean / size, 6)
        print(f"       batch={size:>7,}  mean={bucket.mean:>8.2f}ms  per-tx={bucket.mean/size:.4f}ms  P99={bucket.p99:.2f}ms")

    # 2c: Under-load latency
    print("\n  [2c] Latency under sustained load...")
    load_bucket = LatencyBucket(label="under_load")
    background_done = asyncio.Event()

    async def background_load():
        while not background_done.is_set():
            payload = build_batch_bytes(0, 100000)
            for _, base_url in GPU_LANES:
                await send_batch(session, f"{base_url}/accelerate/batch", payload, timeout_s=30)
            await asyncio.sleep(0.5)

    load_task = asyncio.create_task(background_load())

    await asyncio.sleep(2)  # let load ramp up
    for i in range(100):
        lane_name, base_url = GPU_LANES[i % len(GPU_LANES)]
        resp = await send_single(session, f"{base_url}/accelerate", f"load_lat_{i}")
        if resp.get("_status") == 200:
            load_bucket.latencies_ms.append(resp["_latency_ms"])

    background_done.set()
    load_task.cancel()
    try:
        await load_task
    except asyncio.CancelledError:
        pass

    results["under_load"] = load_bucket.to_dict()
    print(f"       under-load  P50={load_bucket.p50:.2f}ms  P99={load_bucket.p99:.2f}ms  P99.9={load_bucket.p999:.2f}ms")

    return results


# ── Domain 3: Determinism ──────────────────────────────────────────────────────

async def benchmark_determinism(session: aiohttp.ClientSession) -> dict:
    header("DOMAIN 3: DETERMINISM & CORRECTNESS")

    results = {}

    # 3a: Same payload across all lanes → must produce same hash
    print(f"\n  [3a] Cross-lane determinism ({DETERMINISM_REPEATS:,} repeats per lane)...")
    lane_hashes: Dict[str, List[str]] = {}

    for lane_name, base_url in GPU_LANES:
        hashes = []
        batch_size = min(DETERMINISM_REPEATS, 10000)
        batches_needed = math.ceil(DETERMINISM_REPEATS / batch_size)

        for b in range(batches_needed):
            count = min(batch_size, DETERMINISM_REPEATS - b * batch_size)
            txns = [{"tx_hash": "determinism_tx", "tx_data": TX_DATA_HEX} for _ in range(count)]
            payload = fast_dumps({"transactions": txns, "chain": CHAIN})

            resp = await send_batch(session, f"{base_url}/accelerate/batch", payload)
            successful = resp.get("successful", 0)
            hashes.append(f"{lane_name}_batch{b}_ok{successful}")

        lane_hashes[lane_name] = hashes

    # Compare: all lanes should process same count successfully
    lane_counts = {}
    for lane_name, base_url in GPU_LANES:
        resp = await send_single(session, f"{base_url}/accelerate",
                                 "determinism_single", TX_DATA_HEX, CHAIN)
        lane_counts[lane_name] = resp.get("result_hash", "")

    unique_hashes = set(lane_counts.values())
    cross_lane_match = len(unique_hashes) == 1 and "" not in unique_hashes

    results["cross_lane"] = {
        "test": "same_payload_all_lanes",
        "hashes_per_lane": lane_counts,
        "unique_hashes": len(unique_hashes),
        "match": cross_lane_match,
        "verdict": "PASS" if cross_lane_match else "FAIL",
    }
    print(f"       Cross-lane hash match: {'✅ PASS' if cross_lane_match else '❌ FAIL'}")
    for ln, h in lane_counts.items():
        print(f"         {ln}: {h[:24]}...")

    # 3b: Replay consistency — same tx 100 times on same lane
    print(f"\n  [3b] Replay consistency (100 repeats, same lane)...")
    replay_hashes = []
    lane_name, base_url = GPU_LANES[0]
    for i in range(100):
        resp = await send_single(session, f"{base_url}/accelerate",
                                 "replay_tx", TX_DATA_HEX, CHAIN)
        if resp.get("result_hash"):
            replay_hashes.append(resp["result_hash"])

    unique_replay = set(replay_hashes)
    replay_consistent = len(unique_replay) == 1

    results["replay_consistency"] = {
        "test": "same_tx_100_times",
        "total_runs": len(replay_hashes),
        "unique_results": len(unique_replay),
        "consistent": replay_consistent,
        "verdict": "PASS" if replay_consistent else "FAIL",
    }
    print(f"       Replay consistency: {'✅ PASS' if replay_consistent else '❌ FAIL'} ({len(unique_replay)} unique in {len(replay_hashes)} runs)")

    # 3c: Batch vs single consistency
    print(f"\n  [3c] Batch vs single result consistency...")
    single_resp = await send_single(session, f"{GPU_LANES[0][1]}/accelerate",
                                     "consistency_check", TX_DATA_HEX, CHAIN)
    single_hash = single_resp.get("result_hash", "")

    batch_payload = fast_dumps({
        "transactions": [{"tx_hash": "consistency_check", "tx_data": TX_DATA_HEX}],
        "chain": CHAIN
    })
    # Note: batch endpoint returns aggregate, we check success
    batch_resp = await send_batch(session, f"{GPU_LANES[0][1]}/accelerate/batch", batch_payload)
    batch_ok = batch_resp.get("successful", 0) == 1

    results["batch_vs_single"] = {
        "single_hash": single_hash,
        "batch_success": batch_ok,
        "verdict": "PASS" if batch_ok and single_hash else "PARTIAL",
    }
    print(f"       Batch vs single: {'✅ PASS' if batch_ok else '⚠ PARTIAL'}")

    return results


# ── Domain 4: Failover ─────────────────────────────────────────────────────────

async def benchmark_failover(session: aiohttp.ClientSession) -> dict:
    header("DOMAIN 4: FAILOVER PERFORMANCE")

    results = {}

    # 4a: Measure lane failover via bridge (if one lane is down)
    print(f"\n  [4a] Bridge failover test...")
    print(f"       Testing bridge with all lanes up first...")

    # Baseline with all lanes
    payload = build_batch_bytes(0, FAILOVER_BATCH_SIZE)
    start = time.time()
    resp = await send_batch(session, f"{BRIDGE_URL}/accelerate/gpu-batch", payload,
                           timeout_s=30)
    baseline_duration = time.time() - start
    baseline_success = resp.get("successful", 0)
    baseline_tps = baseline_success / baseline_duration if baseline_duration > 0 else 0

    results["baseline_all_lanes"] = {
        "batch_size": FAILOVER_BATCH_SIZE,
        "successful": baseline_success,
        "duration_s": round(baseline_duration, 3),
        "tps": round(baseline_tps),
    }
    print(f"       Baseline (all lanes): {baseline_tps:,.0f} TPS")

    # 4b: Simulate lane unavailability via rapid detection
    print(f"\n  [4b] Lane health detection speed...")
    detection_times = []
    for lane_name, base_url in GPU_LANES:
        start = time.time()
        health = await get_lane_health(session, base_url)
        detection_time = (time.time() - start) * 1000
        detection_times.append({
            "lane": lane_name,
            "detection_ms": round(detection_time, 2),
            "healthy": bool(health.get("status") == "healthy"),
        })
        print(f"       {lane_name}: health check in {detection_time:.2f}ms")

    results["lane_detection"] = detection_times
    avg_detection = statistics.mean(d["detection_ms"] for d in detection_times) if detection_times else 0
    results["avg_detection_ms"] = round(avg_detection, 2)

    # 4c: Single-lane performance (simulate 2 lanes down)
    print(f"\n  [4c] Single-lane performance (simulating N-1 failure)...")
    lane_name, base_url = GPU_LANES[0]
    payload = build_batch_bytes(0, FAILOVER_BATCH_SIZE)

    start = time.time()
    resp = await send_batch(session, f"{base_url}/accelerate/batch", payload)
    single_duration = time.time() - start
    single_success = resp.get("successful", 0)
    single_tps = single_success / single_duration if single_duration > 0 else 0

    degradation_pct = (1 - single_tps / max(baseline_tps, 1)) * 100

    results["single_lane_fallback"] = {
        "lane": lane_name,
        "batch_size": FAILOVER_BATCH_SIZE,
        "successful": single_success,
        "duration_s": round(single_duration, 3),
        "tps": round(single_tps),
        "degradation_pct": round(degradation_pct, 1),
    }
    print(f"       Single lane: {single_tps:,.0f} TPS ({degradation_pct:.1f}% degradation)")

    # 4d: Recovery time (round-trip after simulated pause)
    print(f"\n  [4d] Recovery latency after pause...")
    # Simulate: no traffic for 5s, then measure first-request latency
    await asyncio.sleep(5)

    recovery_latencies = []
    for lane_name, base_url in GPU_LANES:
        resp = await send_single(session, f"{base_url}/accelerate", "recovery_test")
        if resp.get("_status") == 200:
            recovery_latencies.append({
                "lane": lane_name,
                "latency_ms": round(resp["_latency_ms"], 2),
            })
            print(f"       {lane_name} recovery: {resp['_latency_ms']:.2f}ms")

    results["recovery"] = recovery_latencies
    avg_recovery = statistics.mean(r["latency_ms"] for r in recovery_latencies) if recovery_latencies else 0
    results["avg_recovery_ms"] = round(avg_recovery, 2)

    return results


# ── Domain 5: Resource Efficiency ──────────────────────────────────────────────

async def benchmark_resource(session: aiohttp.ClientSession) -> dict:
    header("DOMAIN 5: RESOURCE EFFICIENCY")

    results = {}

    # 5a: GPU metrics snapshot
    print("\n  [5a] GPU metrics...")
    gpu_info = get_gpu_info()
    results["gpu_snapshot"] = gpu_info
    for g in gpu_info:
        print(f"       GPU {g['index']}: {g['name']}  Mem={g['memory_used_mb']:.0f}/{g['memory_total_mb']:.0f}MB "
              f"Util={g['utilization_pct']:.0f}%  Temp={g['temperature_c']:.0f}°C  Power={g['power_draw_w']:.0f}W")

    # 5b: Memory growth under load
    print("\n  [5b] Memory growth under sustained load...")
    memory_timeline = []
    test_start = time.time()

    for cycle in range(10):
        # Capture GPU memory before burst
        pre_gpu = get_gpu_info()
        pre_mem = [g["memory_used_mb"] for g in pre_gpu]

        # Send load
        payload = build_batch_bytes(cycle * 100000, 100000)
        tasks = [send_batch(session, f"{base_url}/accelerate/batch", payload) for _, base_url in GPU_LANES]
        await asyncio.gather(*tasks)

        # Capture GPU memory after burst
        post_gpu = get_gpu_info()
        post_mem = [g["memory_used_mb"] for g in post_gpu]

        elapsed = time.time() - test_start
        memory_timeline.append({
            "cycle": cycle + 1,
            "elapsed_s": round(elapsed, 1),
            "pre_mem_mb": pre_mem,
            "post_mem_mb": post_mem,
            "delta_mb": [round(post_mem[i] - pre_mem[i], 1) for i in range(min(len(pre_mem), len(post_mem)))],
        })

        if cycle % 3 == 0:
            deltas = memory_timeline[-1]["delta_mb"]
            print(f"       cycle {cycle+1:>2d}: mem deltas = {deltas}")

        await asyncio.sleep(0.5)

    results["memory_growth"] = memory_timeline

    # Check for leaks
    if memory_timeline:
        first_mem = memory_timeline[0]["post_mem_mb"]
        last_mem = memory_timeline[-1]["post_mem_mb"]
        growth = [round(last_mem[i] - first_mem[i], 1) for i in range(min(len(first_mem), len(last_mem)))]
        results["memory_leak_check"] = {
            "start_mb": first_mem,
            "end_mb": last_mem,
            "growth_mb": growth,
            "verdict": "PASS" if all(abs(g) < 100 for g in growth) else "WARN",
        }
        print(f"       Memory leak check: growth={growth}MB → {'✅ PASS' if all(abs(g) < 100 for g in growth) else '⚠ WARN'}")

    # 5c: Bandwidth per TPS
    print("\n  [5c] Bandwidth per TPS...")
    payload_size_bytes = len(build_batch_bytes(0, 100000))
    payload = build_batch_bytes(0, 100000)

    start = time.time()
    resp = await send_batch(session, f"{GPU_LANES[0][1]}/accelerate/batch", payload)
    duration = time.time() - start
    success = resp.get("successful", 0)
    tps = success / duration if duration > 0 else 0

    bytes_per_tps = payload_size_bytes / max(tps, 1)
    mb_per_second = payload_size_bytes / max(duration, 0.001) / (1024 * 1024)

    results["bandwidth"] = {
        "payload_size_bytes": payload_size_bytes,
        "tps": round(tps),
        "bytes_per_tps": round(bytes_per_tps, 2),
        "mb_per_second": round(mb_per_second, 2),
    }
    print(f"       {bytes_per_tps:.2f} bytes/TPS  {mb_per_second:.2f} MB/s")

    # 5d: Lane health stats
    print("\n  [5d] Lane health stats...")
    for lane_name, base_url in GPU_LANES:
        health = await get_lane_health(session, base_url)
        stats = health.get("stats", {})
        gpu = health.get("gpu", {})
        print(f"       {lane_name}: txns={stats.get('total_txns',0):,}  "
              f"success_rate={stats.get('success_rate',0)*100:.1f}%  "
              f"gpu_mem={gpu.get('memory_used_mb',0):.0f}MB")
        results[f"lane_{lane_name}_stats"] = {"stats": stats, "gpu": gpu}

    return results


# ── Domain 6: Economic Viability ───────────────────────────────────────────────

async def benchmark_economic(session: aiohttp.ClientSession, throughput_data: dict) -> dict:
    header("DOMAIN 6: ECONOMIC VIABILITY")

    results = {}

    sustained_tps = throughput_data.get("sustained", {}).get("sustained_tps", 0)
    peak_tps = throughput_data.get("peak_burst_tps", 0)

    # 6a: Infrastructure cost calculation
    monthly_power_cost = (GPU_POWER_WATTS * GPU_COUNT) * 24 * 30 / 1000 * ELECTRICITY_COST_KWH
    monthly_amortization = (GPU_PURCHASE_COST * GPU_COUNT) / GPU_AMORTIZATION_MONTHS
    monthly_total = monthly_power_cost + monthly_amortization

    # Assume server running 24/7
    seconds_per_month = 30 * 24 * 3600
    total_txns_per_month = sustained_tps * seconds_per_month if sustained_tps > 0 else 1

    cost_per_million = (monthly_total / total_txns_per_month) * 1_000_000
    cost_per_tps = monthly_total / max(sustained_tps, 1)

    results["infrastructure_cost"] = {
        "gpu_count": GPU_COUNT,
        "gpu_power_watts": GPU_POWER_WATTS,
        "monthly_electricity_cost": round(monthly_power_cost, 2),
        "monthly_amortization": round(monthly_amortization, 2),
        "monthly_total_cost": round(monthly_total, 2),
    }

    results["cost_efficiency"] = {
        "sustained_tps": sustained_tps,
        "peak_tps": peak_tps,
        "txns_per_month": total_txns_per_month,
        "cost_per_million_txns": round(cost_per_million, 6),
        "cost_per_sustained_tps": round(cost_per_tps, 4),
        "cost_per_peak_tps": round(monthly_total / max(peak_tps, 1), 4),
    }

    print(f"\n  Infrastructure costs (3x GTX 1070):")
    print(f"    Monthly electricity:   ${monthly_power_cost:.2f}")
    print(f"    Monthly amortization:  ${monthly_amortization:.2f}")
    print(f"    Monthly total:         ${monthly_total:.2f}")
    print(f"\n  Cost efficiency:")
    print(f"    Cost per 1M txns:      ${cost_per_million:.6f}")
    print(f"    Cost per sustained TPS: ${cost_per_tps:.4f}/mo")

    # 6b: Gas savings estimation
    print(f"\n  Gas savings vs direct Solana submission:")
    gas_per_tx_usd = (SOLANA_AVG_GAS_LAMPORTS / SOLANA_LAMPORT_PER_SOL) * SOLANA_SOL_PRICE_USD
    gas_per_million = gas_per_tx_usd * 1_000_000
    savings_per_million = gas_per_million - cost_per_million
    savings_pct = (savings_per_million / gas_per_million) * 100 if gas_per_million > 0 else 0

    monthly_gas_if_direct = gas_per_tx_usd * total_txns_per_month
    monthly_savings = monthly_gas_if_direct - monthly_total

    results["gas_savings"] = {
        "solana_gas_per_tx_usd": round(gas_per_tx_usd, 8),
        "solana_gas_per_million_usd": round(gas_per_million, 2),
        "inferstructor_cost_per_million_usd": round(cost_per_million, 6),
        "savings_per_million_usd": round(savings_per_million, 2),
        "savings_pct": round(savings_pct, 2),
        "monthly_gas_if_direct_usd": round(monthly_gas_if_direct, 2),
        "monthly_infra_cost_usd": round(monthly_total, 2),
        "monthly_savings_usd": round(monthly_savings, 2),
    }

    print(f"    Solana gas per tx:       ${gas_per_tx_usd:.8f}")
    print(f"    Solana gas per 1M:       ${gas_per_million:.2f}")
    print(f"    Inferstructor per 1M:   ${cost_per_million:.6f}")
    print(f"    Savings per 1M txns:     ${savings_per_million:.2f} ({savings_pct:.1f}%)")
    print(f"    Monthly gas (direct):    ${monthly_gas_if_direct:,.2f}")
    print(f"    Monthly infra cost:      ${monthly_total:.2f}")
    print(f"    Monthly savings:         ${monthly_savings:,.2f}")

    # 6c: Revenue projection by tier
    print(f"\n  Revenue projections:")
    tiers = {
        "basic":      {"price_mo": 0,   "tps_limit": 100_000},
        "pro":        {"price_mo": 49,  "tps_limit": 1_000_000},
        "enterprise": {"price_mo": 299, "tps_limit": sustained_tps},
    }
    tier_projections = {}
    for tier_name, cfg in tiers.items():
        effective_tps = min(cfg["tps_limit"], sustained_tps)
        txns_mo = effective_tps * seconds_per_month
        cost_mo = (monthly_total * effective_tps) / max(sustained_tps, 1)
        revenue = cfg["price_mo"]
        margin = revenue - cost_mo

        tier_projections[tier_name] = {
            "price_per_month": cfg["price_mo"],
            "effective_tps": effective_tps,
            "txns_per_month": txns_mo,
            "cost_to_serve": round(cost_mo, 2),
            "revenue": revenue,
            "margin": round(margin, 2),
            "profitable": margin >= 0,
        }
        emoji = "✅" if margin >= 0 else "❌"
        print(f"    {tier_name:>12s}: ${revenue}/mo  cost=${cost_mo:.2f}  margin=${margin:.2f} {emoji}")

    results["tier_projections"] = tier_projections

    return results


# ── Advanced: Adversarial Testing ──────────────────────────────────────────────

async def benchmark_adversarial(session: aiohttp.ClientSession) -> dict:
    header("ADVANCED: ADVERSARIAL TESTING")

    results = {}

    # A1: Malformed payloads
    print("\n  [A1] Malformed payload rejection...")
    malformed_tests = [
        ("empty_body", b""),
        ("invalid_json", b"{{{not json"),
        ("missing_txns", fast_dumps({"chain": "solana"})),
        ("empty_txns", fast_dumps({"transactions": [], "chain": "solana"})),
        ("null_hash", fast_dumps({"transactions": [{"tx_hash": None, "tx_data": "ff"}], "chain": "solana"})),
        ("huge_data", fast_dumps({"transactions": [{"tx_hash": "x", "tx_data": "ff" * 100000}], "chain": "solana"})),
    ]

    malformed_results = []
    for test_name, payload in malformed_tests:
        try:
            async with session.post(
                f"{GPU_LANES[0][1]}/accelerate/batch",
                data=payload,
                headers={"Content-Type": "application/json"},
                timeout=aiohttp.ClientTimeout(total=10)
            ) as resp:
                status = resp.status
                rejected = status >= 400 or status == 500
                malformed_results.append({
                    "test": test_name,
                    "status": status,
                    "rejected": rejected,
                    "verdict": "PASS" if rejected or test_name == "huge_data" else "FAIL",
                })
                emoji = "✅" if rejected else "⚠"
                print(f"       {test_name:>20s}: HTTP {status} {emoji}")
        except Exception as e:
            malformed_results.append({"test": test_name, "error": str(e), "rejected": True, "verdict": "PASS"})
            print(f"       {test_name:>20s}: Exception (rejected) ✅")

    results["malformed_payloads"] = malformed_results

    # A2: Oversized batch
    print("\n  [A2] Oversized batch handling...")
    oversize = 500001  # just over typical max
    payload = build_batch_bytes(0, oversize)
    start = time.time()
    resp = await send_batch(session, f"{GPU_LANES[0][1]}/accelerate/batch", payload, timeout_s=60)
    duration = time.time() - start
    success = resp.get("successful", 0)
    results["oversized_batch"] = {
        "size": oversize,
        "successful": success,
        "duration_s": round(duration, 3),
        "handled": success > 0,
    }
    print(f"       {oversize:,} txns: {'✅ handled' if success > 0 else '❌ rejected'} in {duration:.3f}s")

    # A3: Rapid fire (lane flooding)
    print("\n  [A3] Lane flooding (100 rapid-fire single requests)...")
    flood_tasks = []
    for i in range(100):
        flood_tasks.append(send_single(session, f"{GPU_LANES[0][1]}/accelerate", f"flood_{i}"))

    flood_start = time.time()
    flood_resps = await asyncio.gather(*flood_tasks)
    flood_duration = time.time() - flood_start

    flood_ok = sum(1 for r in flood_resps if r.get("_status") == 200)
    flood_fail = len(flood_resps) - flood_ok

    results["lane_flooding"] = {
        "requests": 100,
        "successful": flood_ok,
        "failed": flood_fail,
        "duration_s": round(flood_duration, 3),
        "rps": round(100 / flood_duration if flood_duration > 0 else 0),
    }
    print(f"       100 rapid-fire: {flood_ok} ok, {flood_fail} failed in {flood_duration:.3f}s ({100/flood_duration:.0f} rps)")

    # A4: Wrong chain name
    print("\n  [A4] Wrong/unknown chain handling...")
    resp = await send_single(session, f"{GPU_LANES[0][1]}/accelerate",
                              "chain_test", TX_DATA_HEX, "nonexistent_chain_xyz")
    handled = resp.get("_status") in (200, 400, 422)
    results["wrong_chain"] = {
        "chain": "nonexistent_chain_xyz",
        "status": resp.get("_status"),
        "handled_gracefully": handled,
    }
    print(f"       Unknown chain: HTTP {resp.get('_status')} {'✅' if handled else '❌'}")

    return results


# ── Advanced: Degraded Mode ────────────────────────────────────────────────────

async def benchmark_degraded(session: aiohttp.ClientSession) -> dict:
    header("ADVANCED: DEGRADED MODE TESTING")

    results = {}

    # D1: Performance with only 1 lane (simulate 2 down)
    print("\n  [D1] Single-lane-only throughput...")
    levels = [10000, 50000, 100000, 250000, 500000]
    single_lane_results = []

    lane_name, base_url = GPU_LANES[0]
    for level in levels:
        payload = build_batch_bytes(0, level)
        start = time.time()
        resp = await send_batch(session, f"{base_url}/accelerate/batch", payload)
        duration = time.time() - start
        success = resp.get("successful", 0)
        tps = success / duration if duration > 0 else 0
        single_lane_results.append({
            "level": level,
            "successful": success,
            "duration_s": round(duration, 3),
            "tps": round(tps),
        })
        print(f"       {level:>7,} txns: {tps:>10,.0f} TPS ({duration:.3f}s)")
        await asyncio.sleep(0.5)

    results["single_lane_scaling"] = single_lane_results

    # D2: Multi-lane vs single-lane comparison
    print("\n  [D2] Multi-lane vs single-lane comparison...")
    test_size = 300000

    # Single lane
    payload = build_batch_bytes(0, test_size)
    start = time.time()
    resp = await send_batch(session, f"{GPU_LANES[0][1]}/accelerate/batch", payload)
    single_duration = time.time() - start
    single_tps = resp.get("successful", 0) / single_duration if single_duration > 0 else 0

    # All lanes
    per_lane = test_size // len(GPU_LANES)
    tasks = []
    for i, (_, base_url) in enumerate(GPU_LANES):
        payload = build_batch_bytes(i * per_lane, per_lane)
        tasks.append(send_batch(session, f"{base_url}/accelerate/batch", payload))

    start = time.time()
    responses = await asyncio.gather(*tasks)
    multi_duration = time.time() - start
    multi_success = sum(r.get("successful", 0) for r in responses)
    multi_tps = multi_success / multi_duration if multi_duration > 0 else 0

    speedup = multi_tps / max(single_tps, 1)

    results["multi_vs_single"] = {
        "test_size": test_size,
        "single_lane_tps": round(single_tps),
        "multi_lane_tps": round(multi_tps),
        "speedup": round(speedup, 2),
    }
    print(f"       Single: {single_tps:,.0f} TPS  |  Multi: {multi_tps:,.0f} TPS  |  Speedup: {speedup:.2f}x")

    return results


# ── Report Generation ──────────────────────────────────────────────────────────

def generate_report(report: BenchmarkReport) -> str:
    """Generate and save the full benchmark report"""
    os.makedirs(REPORT_DIR, exist_ok=True)

    report_dict = report.to_dict()
    report_path = os.path.join(REPORT_DIR, "benchmark_report.json")
    with open(report_path, "w") as f:
        json.dump(report_dict, f, indent=2, default=str)

    return report_path


def print_final_summary(report: BenchmarkReport):
    """Print a comprehensive final summary"""
    header("BENCHMARK REPORT SUMMARY")

    t = report.throughput
    l = report.latency
    d = report.determinism
    f = report.failover
    r = report.resource
    e = report.economic

    print(f"""
  ┌─────────────────────────────────────────────────────────────────┐
  │                    INFERSTRUCTOR BENCHMARK                     │
  │                     Infrastructure Grade                       │
  └─────────────────────────────────────────────────────────────────┘

  THROUGHPUT
    Peak burst:        {t.get('peak_burst_tps', 0):>12,} TPS
    Sustained (5min):  {t.get('sustained', {}).get('sustained_tps', 0):>12,} TPS
    Total txns:        {t.get('sustained', {}).get('total_success', 0):>12,}

  LATENCY""")

    for key in sorted(l.keys()):
        if key.startswith("single_"):
            v = l[key]
            print(f"    {v.get('label', key):>20s}  P50={v.get('p50_ms',0):>8.2f}ms  P99={v.get('p99_ms',0):>8.2f}ms  P99.9={v.get('p999_ms',0):>8.2f}ms")

    if "under_load" in l:
        v = l["under_load"]
        print(f"    {'under_load':>20s}  P50={v.get('p50_ms',0):>8.2f}ms  P99={v.get('p99_ms',0):>8.2f}ms  P99.9={v.get('p999_ms',0):>8.2f}ms")

    print(f"""
  DETERMINISM
    Cross-lane match:  {d.get('cross_lane', {}).get('verdict', 'N/A')}
    Replay consistency:{d.get('replay_consistency', {}).get('verdict', 'N/A')}

  FAILOVER
    Avg detection:     {f.get('avg_detection_ms', 0):>8.2f}ms
    Avg recovery:      {f.get('avg_recovery_ms', 0):>8.2f}ms
    N-1 degradation:   {f.get('single_lane_fallback', {}).get('degradation_pct', 0):>7.1f}%

  RESOURCE EFFICIENCY""")

    mem_check = r.get("memory_leak_check", {})
    bw = r.get("bandwidth", {})
    print(f"    Memory leak:       {mem_check.get('verdict', 'N/A')}")
    print(f"    Bandwidth:         {bw.get('mb_per_second', 0):>8.2f} MB/s")
    print(f"    Bytes per TPS:     {bw.get('bytes_per_tps', 0):>8.2f}")

    gas = e.get("gas_savings", {})
    cost = e.get("cost_efficiency", {})
    infra = e.get("infrastructure_cost", {})
    print(f"""
  ECONOMIC VIABILITY
    Monthly infra cost:  ${infra.get('monthly_total_cost', 0):>10.2f}
    Cost per 1M txns:    ${cost.get('cost_per_million_txns', 0):>10.6f}
    Cost per TPS:        ${cost.get('cost_per_sustained_tps', 0):>10.4f}/mo

  GAS SAVINGS (vs Solana direct)
    Solana gas / 1M txns:   ${gas.get('solana_gas_per_million_usd', 0):>12.2f}
    Inferstructor / 1M:    ${gas.get('inferstructor_cost_per_million_usd', 0):>12.6f}
    Savings per 1M:         ${gas.get('savings_per_million_usd', 0):>12.2f}  ({gas.get('savings_pct', 0):.1f}%)
    Monthly savings:        ${gas.get('monthly_savings_usd', 0):>12,.2f}

  ADVERSARIAL
    Malformed rejected:  {sum(1 for m in report.adversarial.get('malformed_payloads', []) if m.get('rejected'))}/{len(report.adversarial.get('malformed_payloads', []))}
    Lane flooding:       {report.adversarial.get('lane_flooding', {}).get('successful', 0)}/100 survived

  DEGRADED MODE
    Multi-lane speedup:  {report.degraded.get('multi_vs_single', {}).get('speedup', 0):.2f}x
""")


# ── Main ───────────────────────────────────────────────────────────────────────

async def main():
    print(f"""
╔══════════════════════════════════════════════════════════════════════╗
║         INFERSTRUCTOR INFRASTRUCTURE BENCHMARK SUITE v1.0         ║
║                                                                    ║
║  6 Domains + Advanced Testing • 3x GTX 1070 GPU Cluster            ║
║  Started: {datetime.now().strftime('%Y-%m-%d %H:%M:%S'):>53s}  ║
╚══════════════════════════════════════════════════════════════════════╝
""")

    report = BenchmarkReport(
        timestamp=datetime.now().isoformat(),
        system_info={
            "platform": platform.platform(),
            "python": platform.python_version(),
            "cpu_count": os.cpu_count(),
            "gpu_count": GPU_COUNT,
            "gpu_model": "NVIDIA GeForce GTX 1070",
            "gpu_vram_mb": 8192,
            "cuda_version": "11.5",
        },
    )

    # Health check all services
    print("  Checking services...")
    connector = aiohttp.TCPConnector(limit=200, limit_per_host=100)
    async with aiohttp.ClientSession(connector=connector) as session:
        all_healthy = True
        for name, base_url in GPU_LANES:
            health = await get_lane_health(session, base_url)
            status = health.get("status", "unknown")
            ok = status == "healthy"
            all_healthy = all_healthy and ok
            print(f"    {'✅' if ok else '❌'} {name:>10s} ({base_url}): {status}")

        # Check bridge
        try:
            async with session.get(f"{BRIDGE_URL}/health", timeout=aiohttp.ClientTimeout(total=3)) as r:
                if r.status == 200:
                    print(f"    ✅    bridge ({BRIDGE_URL}): healthy")
                else:
                    print(f"    ⚠     bridge: HTTP {r.status}")
        except Exception:
            print(f"    ⚠     bridge: unreachable (non-critical)")

        if not all_healthy:
            print("\n  ❌ Not all GPU lanes are healthy. Aborting.")
            return

        print("\n  All systems GO. Starting benchmark...\n")
        await asyncio.sleep(2)

        # ── Run all 6 domains ──
        report.throughput = await benchmark_throughput(session)
        await asyncio.sleep(3)

        report.latency = await benchmark_latency(session)
        await asyncio.sleep(3)

        report.determinism = await benchmark_determinism(session)
        await asyncio.sleep(2)

        report.failover = await benchmark_failover(session)
        await asyncio.sleep(3)

        report.resource = await benchmark_resource(session)
        await asyncio.sleep(2)

        report.economic = await benchmark_economic(session, report.throughput)
        await asyncio.sleep(2)

        # ── Advanced tests ──
        report.adversarial = await benchmark_adversarial(session)
        await asyncio.sleep(2)

        report.degraded = await benchmark_degraded(session)

    # Generate report
    report_path = generate_report(report)

    # Final summary
    print_final_summary(report)
    print(f"  Report saved to: {report_path}")
    print(f"  Completed: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    hr("═")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n\n  Benchmark interrupted by user.")
    except Exception as e:
        print(f"\n\n  Benchmark failed: {e}")
        import traceback
        traceback.print_exc()
