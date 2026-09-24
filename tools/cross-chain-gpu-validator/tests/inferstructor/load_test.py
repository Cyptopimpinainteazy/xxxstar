#!/usr/bin/env python3
"""
GPU Acceleration Load Test v2 - Batch Mode
Targets 500K+ TPS using batch GPU endpoint

Sends transactions in large batches that get split across 3 GPUs
and processed via CUDA vectorized kernels.
"""

import asyncio
import aiohttp
import time
import statistics
from datetime import datetime
from typing import List, Dict

# Configuration
BRIDGE_URL = "http://localhost:9999/accelerate/gpu-batch"
API_KEY = "infra_3JDGhaxUOfLfyuFk-roJiR3FzgdgcipAH3vG5wpMzDo"
CHAIN = "solana"
TX_DATA_HEX = "48656c6c6f"

# Load progression: 1K, 5K, 10K, then +5K until failure
LOAD_LEVELS = [500000, 1000000, 2000000, 3000000, 4000000, 5000000,
               7500000, 10000000]

# Batch sizes sent per HTTP request to bridge
BATCH_CHUNK_SIZE = 500000  # Send 500K txns per HTTP call

MAX_FAILURE_RATE = 0.10
MAX_LATENCY_MS = 60000  # 60 seconds max (for huge batches)


class LevelResult:
    def __init__(self, level: int):
        self.level = level
        self.total = 0
        self.successful = 0
        self.failed = 0
        self.batch_latencies: List[float] = []
        self.per_tx_latencies: List[float] = []
        self.lane_dist: Dict[str, int] = {}
        self.start_time = time.time()
        self.end_time = None

    def finalize(self):
        self.end_time = time.time()

    @property
    def duration(self) -> float:
        return (self.end_time or time.time()) - self.start_time

    @property
    def tps(self) -> float:
        return self.successful / self.duration if self.duration > 0 else 0

    @property
    def success_rate(self) -> float:
        return self.successful / max(self.total, 1)

    @property
    def failure_rate(self) -> float:
        return 1.0 - self.success_rate

    @property
    def avg_per_tx_ms(self) -> float:
        return statistics.mean(self.per_tx_latencies) if self.per_tx_latencies else 0

    @property
    def avg_batch_ms(self) -> float:
        return statistics.mean(self.batch_latencies) if self.batch_latencies else 0


def build_batch(start_id: int, count: int) -> list:
    """Build a batch of transaction payloads"""
    return [
        {"tx_hash": f"tx_{start_id + i}", "tx_data": TX_DATA_HEX}
        for i in range(count)
    ]


async def send_batch(session: aiohttp.ClientSession, batch: list) -> dict:
    """Send a batch to the bridge gpu-batch endpoint"""
    headers = {"X-API-Key": API_KEY, "Content-Type": "application/json"}
    payload = {"transactions": batch, "chain": CHAIN}

    start = time.time()
    try:
        async with session.post(
            BRIDGE_URL, json=payload, headers=headers,
            timeout=aiohttp.ClientTimeout(total=120)
        ) as resp:
            latency = (time.time() - start) * 1000
            if resp.status == 200:
                data = await resp.json()
                data['_latency_ms'] = latency
                return data
            else:
                text = await resp.text()
                return {"successful": 0, "total": len(batch), "_latency_ms": latency, "_error": text}
    except Exception as e:
        latency = (time.time() - start) * 1000
        return {"successful": 0, "total": len(batch), "_latency_ms": latency, "_error": str(e)}


async def warmup(session: aiohttp.ClientSession):
    """Warm up all GPU lanes"""
    print(f"  Warming up GPUs...", end="", flush=True)
    batch = build_batch(0, 30)
    await send_batch(session, batch)
    print(" Done!")


async def run_level(level: int) -> LevelResult:
    print(f"\n{'='*70}")
    print(f"  LOAD LEVEL: {level:,} TRANSACTIONS")
    print(f"{'='*70}")

    result = LevelResult(level)

    connector = aiohttp.TCPConnector(limit=200, limit_per_host=100)
    async with aiohttp.ClientSession(connector=connector) as session:
        await warmup(session)

        # Split into chunks and send in parallel bursts
        chunks = []
        for i in range(0, level, BATCH_CHUNK_SIZE):
            chunk_size = min(BATCH_CHUNK_SIZE, level - i)
            chunks.append(build_batch(i, chunk_size))

        print(f"  Sending {level:,} txns in {len(chunks)} batch(es) of up to {BATCH_CHUNK_SIZE:,}...")

        result.start_time = time.time()

        # Send all chunks in parallel
        tasks = [send_batch(session, chunk) for chunk in chunks]
        responses = await asyncio.gather(*tasks)

        result.finalize()

        # Process responses
        for resp in responses:
            batch_latency = resp.get('_latency_ms', 0)
            successful = resp.get('successful', 0)
            total_in_batch = resp.get('total', len(resp.get('results', [])))
            if total_in_batch == 0:
                total_in_batch = successful

            result.total += total_in_batch
            result.successful += successful
            result.failed += (total_in_batch - successful)
            result.batch_latencies.append(batch_latency)

            if successful > 0:
                result.per_tx_latencies.append(batch_latency / successful)

            # Lane distribution
            lane_dist = resp.get('lane_distribution', {})
            for lane, count in lane_dist.items():
                result.lane_dist[lane] = result.lane_dist.get(lane, 0) + count

    # Print results
    print(f"\n  RESULTS:")
    print(f"     Total Txns:    {result.total:,}")
    print(f"     Successful:    {result.successful:,} ({result.success_rate*100:.2f}%)")
    print(f"     Failed:        {result.failed:,}")
    print(f"     Duration:      {result.duration:.3f}s")
    print(f"     THROUGHPUT:    {result.tps:,.0f} TPS")
    print(f"     Avg Batch:     {result.avg_batch_ms:.2f}ms")
    print(f"     Avg Per-Tx:    {result.avg_per_tx_ms:.4f}ms")

    if result.lane_dist:
        print(f"\n  GPU DISTRIBUTION:")
        for lane, count in sorted(result.lane_dist.items()):
            pct = count / max(result.successful, 1) * 100
            print(f"     {lane:12s}: {count:,} ({pct:.1f}%)")

    return result


async def check_health():
    print("Checking system health...")
    endpoints = [
        ("Primary",  "http://localhost:9001/health"),
        ("Shadow",   "http://localhost:9002/health"),
        ("Tertiary", "http://localhost:9003/health"),
        ("Bridge",   "http://localhost:9999/health"),
    ]
    async with aiohttp.ClientSession() as session:
        for name, url in endpoints:
            try:
                async with session.get(url, timeout=aiohttp.ClientTimeout(total=3)) as r:
                    if r.status == 200:
                        data = await r.json()
                        gpu_info = ""
                        if 'gpu' in data:
                            gpu_info = f" | GPU available: {data['gpu'].get('available', '?')}"
                        print(f"  OK  {name:12s}{gpu_info}")
                    else:
                        print(f"  ERR {name:12s}: HTTP {r.status}")
                        return False
            except Exception as e:
                print(f"  ERR {name:12s}: {e}")
                return False
    return True


def should_continue(result: LevelResult) -> bool:
    if result.failure_rate > MAX_FAILURE_RATE:
        print(f"\n  STOP: Failure rate {result.failure_rate*100:.1f}% > {MAX_FAILURE_RATE*100:.0f}%")
        return False
    if result.avg_batch_ms > MAX_LATENCY_MS:
        print(f"\n  STOP: Batch latency {result.avg_batch_ms:.0f}ms > {MAX_LATENCY_MS}ms")
        return False
    return True


async def main():
    print(f"""
{'='*70}
  GPU ACCELERATION LOAD TEST v2 - BATCH MODE
  Target: 500K+ TPS across 3x GTX 1070 GPUs
{'='*70}

  Levels: {', '.join(f'{x:,}' for x in LOAD_LEVELS[:8])} ...
  Batch chunk: {BATCH_CHUNK_SIZE:,} txns per HTTP call
  Started: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
""")

    if not await check_health():
        print("\nSystem health check failed!")
        return

    print("\nAll systems healthy. Starting load test...\n")
    await asyncio.sleep(1)

    results: List[LevelResult] = []

    for level in LOAD_LEVELS:
        result = await run_level(level)
        results.append(result)

        if not should_continue(result):
            print(f"\n  PEAK REACHED at {level:,} transactions")
            break

        print(f"\n  Cooling down 2s...")
        await asyncio.sleep(2)

    # Final summary
    print(f"\n\n{'='*70}")
    print(f"  FINAL RESULTS")
    print(f"{'='*70}\n")
    print(f"{'Level':>10}  {'Success':>10}  {'Rate':>8}  {'Duration':>10}  {'TPS':>12}  {'Per-Tx':>10}")
    print(f"{'-'*70}")

    for r in results:
        print(f"{r.level:>10,}  {r.successful:>10,}  {r.success_rate*100:>7.1f}%  {r.duration:>9.3f}s  {r.tps:>11,.0f}  {r.avg_per_tx_ms:>9.4f}ms")

    if results:
        best = max(results, key=lambda r: r.tps)
        print(f"\n  PEAK PERFORMANCE:")
        print(f"    Level:      {best.level:,}")
        print(f"    Throughput: {best.tps:,.0f} TPS")
        print(f"    Success:    {best.success_rate*100:.2f}%")
        print(f"    Duration:   {best.duration:.3f}s")
        print(f"    Per-Tx:     {best.avg_per_tx_ms:.4f}ms")

        if best.lane_dist:
            print(f"\n  GPU DISTRIBUTION at peak:")
            for lane, count in sorted(best.lane_dist.items()):
                pct = count / max(best.successful, 1) * 100
                print(f"    {lane:12s}: {count:>8,} ({pct:.1f}%)")

    print(f"\n  Completed: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"{'='*70}\n")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n\nLoad test interrupted.")
    except Exception as e:
        print(f"\n\nLoad test failed: {e}")
        import traceback
        traceback.print_exc()
