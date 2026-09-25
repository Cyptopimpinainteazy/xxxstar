#!/usr/bin/env python3
"""
GPU Acceleration Load Test v3 - Direct-to-GPU Mode
Sends batches directly to GPU lanes (bypasses bridge) for max throughput.
Uses orjson for fast serialization.
"""

import asyncio
import aiohttp
import time
import statistics
import orjson
from datetime import datetime

# GPU Lane endpoints (direct, no bridge)
GPU_LANES = [
    ("primary",  "http://localhost:9001/accelerate/batch"),
    ("shadow",   "http://localhost:9002/accelerate/batch"),
    ("tertiary", "http://localhost:9003/accelerate/batch"),
]

CHAIN = "solana"
TX_DATA_HEX = "48656c6c6f"

LOAD_LEVELS = [100000, 250000, 500000, 750000, 1000000,
               1500000, 2000000, 3000000, 5000000, 10000000]

# Per-lane batch size (each GPU gets this many per HTTP call)
LANE_BATCH_SIZE = 500000

MAX_FAILURE_RATE = 0.10


def build_batch_bytes(start_id: int, count: int) -> bytes:
    """Build batch payload as raw bytes using orjson"""
    txns = [{"tx_hash": f"t{start_id + i}", "tx_data": TX_DATA_HEX} for i in range(count)]
    return orjson.dumps({"transactions": txns, "chain": CHAIN})


async def send_batch_raw(session: aiohttp.ClientSession, url: str, payload: bytes) -> dict:
    """Send pre-serialized batch to a GPU lane"""
    try:
        async with session.post(
            url, data=payload,
            headers={"Content-Type": "application/json"},
            timeout=aiohttp.ClientTimeout(total=120)
        ) as resp:
            if resp.status == 200:
                return await resp.json()
            return {"successful": 0, "batch_size": 0, "error": f"HTTP {resp.status}"}
    except Exception as e:
        return {"successful": 0, "batch_size": 0, "error": str(e)}


async def warmup(session: aiohttp.ClientSession):
    """Warm up all GPU lanes"""
    print("  Warming up GPUs...", end="", flush=True)
    payload = orjson.dumps({"transactions": [{"tx_hash": "w", "tx_data": TX_DATA_HEX}], "chain": CHAIN})
    tasks = [send_batch_raw(session, url, payload) for _, url in GPU_LANES]
    await asyncio.gather(*tasks)
    print(" Done!")


async def run_level(level: int) -> dict:
    print(f"\n{'='*70}")
    print(f"  LOAD LEVEL: {level:,} TRANSACTIONS")
    print(f"{'='*70}")

    num_lanes = len(GPU_LANES)
    per_lane = level // num_lanes
    remainder = level % num_lanes

    connector = aiohttp.TCPConnector(limit=200, limit_per_host=100)
    async with aiohttp.ClientSession(connector=connector) as session:
        await warmup(session)

        # Build payloads for each lane (pre-serialize with orjson)
        tasks = []
        lane_counts = {}
        tx_offset = 0

        for i, (name, url) in enumerate(GPU_LANES):
            count = per_lane + (1 if i < remainder else 0)
            lane_counts[name] = count

            # Split into sub-batches if needed
            for chunk_start in range(0, count, LANE_BATCH_SIZE):
                chunk_size = min(LANE_BATCH_SIZE, count - chunk_start)
                payload = build_batch_bytes(tx_offset, chunk_size)
                tasks.append((name, url, payload, chunk_size))
                tx_offset += chunk_size

        print(f"  Sending {level:,} txns across {num_lanes} GPUs ({len(tasks)} HTTP calls)...")

        start = time.time()
        coros = [send_batch_raw(session, url, payload) for _, url, payload, _ in tasks]
        results = await asyncio.gather(*coros)
        duration = time.time() - start

        # Tally results
        total_success = 0
        total_failed = 0
        lane_dist = {}

        for (name, _, _, chunk_size), result in zip(tasks, results):
            success = result.get('successful', 0)
            total_success += success
            total_failed += (chunk_size - success)
            lane_dist[name] = lane_dist.get(name, 0) + success

        tps = total_success / duration if duration > 0 else 0
        success_rate = total_success / max(level, 1)

        print(f"\n  RESULTS:")
        print(f"     Total Txns:    {level:,}")
        print(f"     Successful:    {total_success:,} ({success_rate*100:.2f}%)")
        print(f"     Failed:        {total_failed:,}")
        print(f"     Duration:      {duration:.3f}s")
        print(f"     THROUGHPUT:    {tps:,.0f} TPS")
        print(f"     Per-Tx:        {duration/max(total_success,1)*1000:.4f}ms")

        print(f"\n  GPU DISTRIBUTION:")
        for name, count in sorted(lane_dist.items()):
            pct = count / max(total_success, 1) * 100
            print(f"     {name:12s}: {count:>10,} ({pct:.1f}%)")

        return {
            "level": level, "successful": total_success, "failed": total_failed,
            "duration": duration, "tps": tps, "success_rate": success_rate,
            "lane_dist": lane_dist,
        }


async def main():
    print(f"""
{'='*70}
  GPU LOAD TEST v3 - DIRECT TO GPU (bypass bridge)
  Target: 500K+ TPS across 3x GTX 1070 GPUs
{'='*70}

  Levels: {', '.join(f'{x:,}' for x in LOAD_LEVELS[:6])} ...
  Per-lane batch: {LANE_BATCH_SIZE:,} | orjson serialization
  Started: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
""")

    # Health check
    async with aiohttp.ClientSession() as session:
        for name, url in GPU_LANES:
            base = url.rsplit('/', 2)[0]
            try:
                async with session.get(f"{base}/health", timeout=aiohttp.ClientTimeout(total=3)) as r:
                    if r.status == 200:
                        print(f"  OK  {name}")
                    else:
                        print(f"  ERR {name}: HTTP {r.status}")
                        return
            except Exception as e:
                print(f"  ERR {name}: {e}")
                return

    print("\nAll GPUs healthy. Starting...\n")
    await asyncio.sleep(1)

    results = []
    for level in LOAD_LEVELS:
        result = await run_level(level)
        results.append(result)

        if result['success_rate'] < (1 - MAX_FAILURE_RATE):
            print(f"\n  STOP: Failure rate too high")
            break

        print(f"\n  Cooling down 2s...")
        await asyncio.sleep(2)

    # Summary
    print(f"\n\n{'='*70}")
    print(f"  FINAL RESULTS - DIRECT GPU MODE")
    print(f"{'='*70}\n")
    print(f"{'Level':>12}  {'Success':>12}  {'Rate':>8}  {'Duration':>10}  {'TPS':>14}")
    print(f"{'-'*70}")

    for r in results:
        print(f"{r['level']:>12,}  {r['successful']:>12,}  {r['success_rate']*100:>7.1f}%  {r['duration']:>9.3f}s  {r['tps']:>13,.0f}")

    if results:
        best = max(results, key=lambda r: r['tps'])
        print(f"\n  PEAK: {best['tps']:,.0f} TPS at {best['level']:,} txns ({best['duration']:.3f}s)")
        print(f"  GPU: {' | '.join(f'{k}: {v:,}' for k, v in sorted(best['lane_dist'].items()))}")

    print(f"\n  Completed: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"{'='*70}\n")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nInterrupted.")
