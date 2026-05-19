#!/usr/bin/env python3
"""
GPU Acceleration Load Testing
Ramps from 1K → 5K → 10K → 15K... until peak performance is found
"""

import asyncio
import aiohttp
import time
import statistics
from datetime import datetime
from typing import List, Dict, Tuple
import json

# Configuration
BRIDGE_URL = "http://localhost:9999/accelerate"
API_KEY = "infra_3JDGhaxUOfLfyuFk-roJiR3FzgdgcipAH3vG5wpMzDo"
CHAIN = "solana"

# Load test progression
LOAD_LEVELS = [1000, 5000, 10000, 15000, 20000, 25000, 30000, 35000, 40000, 45000, 50000]
WARMUP_REQUESTS = 20  # Warm up GPUs before each level
ACCEPTABLE_FAILURE_RATE = 0.05  # 5% max failure rate
ACCEPTABLE_LATENCY_MS = 10000  # 10 seconds max average latency (concurrent burst model)


class LoadTestResult:
    def __init__(self, level: int):
        self.level = level
        self.successful = 0
        self.failed = 0
        self.latencies: List[float] = []
        self.start_time = time.time()
        self.end_time = None
        self.lane_distribution: Dict[str, int] = {}
        
    def add_result(self, success: bool, latency_ms: float, lane_id: str):
        if success:
            self.successful += 1
            self.latencies.append(latency_ms)
            self.lane_distribution[lane_id] = self.lane_distribution.get(lane_id, 0) + 1
        else:
            self.failed += 1
    
    def finalize(self):
        self.end_time = time.time()
        
    @property
    def total_requests(self) -> int:
        return self.successful + self.failed
    
    @property
    def success_rate(self) -> float:
        if self.total_requests == 0:
            return 0.0
        return self.successful / self.total_requests
    
    @property
    def failure_rate(self) -> float:
        return 1.0 - self.success_rate
    
    @property
    def avg_latency_ms(self) -> float:
        return statistics.mean(self.latencies) if self.latencies else 0.0
    
    @property
    def median_latency_ms(self) -> float:
        return statistics.median(self.latencies) if self.latencies else 0.0
    
    @property
    def p95_latency_ms(self) -> float:
        if not self.latencies:
            return 0.0
        sorted_latencies = sorted(self.latencies)
        p95_idx = int(len(sorted_latencies) * 0.95)
        return sorted_latencies[p95_idx]
    
    @property
    def p99_latency_ms(self) -> float:
        if not self.latencies:
            return 0.0
        sorted_latencies = sorted(self.latencies)
        p99_idx = int(len(sorted_latencies) * 0.99)
        return sorted_latencies[p99_idx]
    
    @property
    def duration_seconds(self) -> float:
        if self.end_time is None:
            return time.time() - self.start_time
        return self.end_time - self.start_time
    
    @property
    def throughput_tps(self) -> float:
        if self.duration_seconds == 0:
            return 0.0
        return self.successful / self.duration_seconds


async def send_acceleration_request(session: aiohttp.ClientSession, tx_id: int) -> Tuple[bool, float, str]:
    """Send a single acceleration request"""
    tx_hash = f"load_test_tx_{tx_id}_{int(time.time() * 1000000)}"
    tx_data = "48656c6c6f476f6c64656e476174657761794161626263646566"  # "HelloGoldenGatewayAabcdef" in hex
    
    payload = {
        "tx_hash": tx_hash,
        "tx_data": tx_data,
        "chain": CHAIN
    }
    
    headers = {
        "X-API-Key": API_KEY,
        "Content-Type": "application/json"
    }
    
    start_time = time.time()
    
    try:
        async with session.post(BRIDGE_URL, json=payload, headers=headers, timeout=aiohttp.ClientTimeout(total=10)) as response:
            data = await response.json()
            latency_ms = (time.time() - start_time) * 1000
            
            success = data.get("success", False)
            lane_id = data.get("lane_id", "unknown")
            
            return (success, latency_ms, lane_id)
    
    except asyncio.TimeoutError:
        latency_ms = (time.time() - start_time) * 1000
        return (False, latency_ms, "timeout")
    except Exception as e:
        latency_ms = (time.time() - start_time) * 1000
        return (False, latency_ms, "error")


async def run_warmup(session: aiohttp.ClientSession):
    """Warm up GPUs before load test"""
    print(f"  🔥 Warming up GPUs ({WARMUP_REQUESTS} requests)...", end="", flush=True)
    tasks = [send_acceleration_request(session, i) for i in range(WARMUP_REQUESTS)]
    await asyncio.gather(*tasks)
    print(" Done!")


async def run_load_level(level: int) -> LoadTestResult:
    """Run a single load test level"""
    print(f"\n{'='*70}")
    print(f"📊 LOAD TEST LEVEL: {level:,} CONCURRENT REQUESTS")
    print(f"{'='*70}")
    
    result = LoadTestResult(level)
    
    async with aiohttp.ClientSession() as session:
        # Warm up GPUs
        await run_warmup(session)
        
        # Run actual load test
        print(f"  ⚡ Sending {level:,} concurrent requests...")
        start_time = time.time()
        
        tasks = [send_acceleration_request(session, i) for i in range(level)]
        responses = await asyncio.gather(*tasks)
        
        # Process results
        for success, latency_ms, lane_id in responses:
            result.add_result(success, latency_ms, lane_id)
        
        result.finalize()
    
    # Print results
    print(f"\n  ✅ RESULTS:")
    print(f"     Total Requests:   {result.total_requests:,}")
    print(f"     Successful:       {result.successful:,} ({result.success_rate*100:.2f}%)")
    print(f"     Failed:           {result.failed:,} ({result.failure_rate*100:.2f}%)")
    print(f"     Duration:         {result.duration_seconds:.2f}s")
    print(f"     Throughput:       {result.throughput_tps:.2f} TPS")
    print(f"\n  ⏱️  LATENCY:")
    print(f"     Average:          {result.avg_latency_ms:.2f}ms")
    print(f"     Median:           {result.median_latency_ms:.2f}ms")
    print(f"     P95:              {result.p95_latency_ms:.2f}ms")
    print(f"     P99:              {result.p99_latency_ms:.2f}ms")
    print(f"\n  🔀 LANE DISTRIBUTION:")
    for lane, count in sorted(result.lane_distribution.items()):
        percentage = (count / result.successful * 100) if result.successful > 0 else 0
        print(f"     {lane:12s}: {count:,} ({percentage:.1f}%)")
    
    return result


async def check_system_health():
    """Check if all GPU lanes are healthy before starting"""
    print("🏥 Checking system health...")
    
    lane_endpoints = [
        ("Primary", "http://localhost:9001/health"),
        ("Shadow", "http://localhost:9002/health"),
        ("Tertiary", "http://localhost:9003/health"),
        ("Bridge", "http://localhost:9999/stats"),
    ]
    
    async with aiohttp.ClientSession() as session:
        for name, endpoint in lane_endpoints:
            try:
                async with session.get(endpoint, timeout=aiohttp.ClientTimeout(total=3)) as response:
                    if response.status == 200:
                        print(f"  ✅ {name:12s}: Healthy")
                    else:
                        print(f"  ⚠️  {name:12s}: Status {response.status}")
                        return False
            except Exception as e:
                print(f"  ❌ {name:12s}: Unreachable ({e})")
                return False
    
    return True


def should_continue(result: LoadTestResult) -> bool:
    """Determine if we should continue to next load level"""
    if result.failure_rate > ACCEPTABLE_FAILURE_RATE:
        print(f"\n  ⛔ STOPPING: Failure rate {result.failure_rate*100:.2f}% exceeds threshold {ACCEPTABLE_FAILURE_RATE*100:.0f}%")
        return False
    
    if result.avg_latency_ms > ACCEPTABLE_LATENCY_MS:
        print(f"\n  ⛔ STOPPING: Average latency {result.avg_latency_ms:.2f}ms exceeds threshold {ACCEPTABLE_LATENCY_MS}ms")
        return False
    
    return True


async def main():
    print(f"""
╔═══════════════════════════════════════════════════════════════════╗
║                 GPU ACCELERATION LOAD TEST                        ║
║                   Ramp-Up Performance Test                        ║
╚═══════════════════════════════════════════════════════════════════╝

Test Configuration:
  • Load Levels: {', '.join(f'{x:,}' for x in LOAD_LEVELS[:5])} ... (up to {LOAD_LEVELS[-1]:,})
  • Max Failure Rate: {ACCEPTABLE_FAILURE_RATE*100:.0f}%
  • Max Avg Latency: {ACCEPTABLE_LATENCY_MS}ms
  • Warmup Requests: {WARMUP_REQUESTS} per level
  
Started at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
""")
    
    # Pre-flight checks
    if not await check_system_health():
        print("\n❌ System health check failed. Please ensure all services are running.")
        return
    
    print("\n✅ All systems healthy. Starting load test...\n")
    await asyncio.sleep(2)  # Brief pause before starting
    
    # Run load tests
    results: List[LoadTestResult] = []
    
    for level in LOAD_LEVELS:
        result = await run_load_level(level)
        results.append(result)
        
        if not should_continue(result):
            print(f"\n🏁 PEAK CAPACITY REACHED AT {level:,} REQUESTS")
            break
        
        # Brief pause between levels
        print(f"\n  ⏳ Cooling down for 3 seconds before next level...")
        await asyncio.sleep(3)
    
    # Final summary
    print(f"\n\n")
    print(f"{'='*70}")
    print(f"🏆 FINAL LOAD TEST SUMMARY")
    print(f"{'='*70}\n")
    
    print(f"{'Level':<10} {'Requests':<12} {'Success%':<12} {'Avg Latency':<15} {'TPS':<10}")
    print(f"{'-'*70}")
    
    for result in results:
        print(f"{result.level:<10,} {result.total_requests:<12,} {result.success_rate*100:<11.2f}% {result.avg_latency_ms:<14.2f}ms {result.throughput_tps:<10.2f}")
    
    if results:
        best_result = max(results, key=lambda r: r.throughput_tps)
        print(f"\n🥇 PEAK PERFORMANCE:")
        print(f"   Level: {best_result.level:,} requests")
        print(f"   Throughput: {best_result.throughput_tps:.2f} TPS")
        print(f"   Success Rate: {best_result.success_rate*100:.2f}%")
        print(f"   Avg Latency: {best_result.avg_latency_ms:.2f}ms")
        
        print(f"\n📈 GPU UTILIZATION:")
        if best_result.lane_distribution:
            total_reqs = sum(best_result.lane_distribution.values())
            for lane, count in sorted(best_result.lane_distribution.items()):
                percentage = (count / total_reqs * 100) if total_reqs > 0 else 0
                print(f"   {lane:12s}: {count:,} requests ({percentage:.1f}%)")
    
    print(f"\n✅ Load test completed at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"{'='*70}\n")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n\n⚠️  Load test interrupted by user.")
    except Exception as e:
        print(f"\n\n❌ Load test failed: {e}")
        raise
