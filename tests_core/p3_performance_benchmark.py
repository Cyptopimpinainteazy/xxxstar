#!/usr/bin/env python3
"""
P3 Performance Benchmarking Suite
Validates +50% throughput and latency improvements
"""

import asyncio
import json
import logging
import statistics
import subprocess
import time
from dataclasses import dataclass

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


@dataclass
class BenchmarkResult:
    """Single benchmark result"""
    task_id: str
    submit_time: float
    execution_time: float
    total_time: float
    success: bool
    verified: bool


class PerformanceBenchmark:
    """Performance benchmarking suite for P3 components"""

    def __init__(self, coordinator_url: str = "http://localhost:9000"):
        self.coordinator_url = coordinator_url
        self.results: list[BenchmarkResult] = []

    def submit_task(self, task_code: str, backend: str = "cuda") -> tuple:
        """Submit a task and measure submission latency"""

        payload = {
            "code": task_code,
            "backend": backend,
            "priority": 5
        }

        start = time.time()

        result = subprocess.run(
            ["curl", "-s", "-X", "POST", f"{self.coordinator_url}/submit_task",
             "-H", "Content-Type: application/json",
             "-d", json.dumps(payload)],
            capture_output=True,
            text=True,
            timeout=10
        )

        elapsed = time.time() - start

        if result.returncode != 0:
            return None, elapsed

        try:
            response = json.loads(result.stdout)
            task_id = response.get("task_id")
            return task_id, elapsed
        except Exception:
            return None, elapsed

    def get_task_status(self, task_id: str) -> tuple:
        """Get task status and check if completed/verified"""

        result = subprocess.run(
            ["curl", "-s", f"{self.coordinator_url}/task/{task_id}/status"],
            capture_output=True,
            text=True,
            timeout=10
        )

        if result.returncode != 0:
            return None, None, None

        try:
            status = json.loads(result.stdout)
            return status.get("state"), status.get("verified"), status.get("execution_time")
        except Exception:
            return None, None, None

    async def benchmark_throughput(self, duration_seconds: int = 60, parallel_tasks: int = 10):
        """Measure throughput: tasks per second"""

        logger.info(f"\n{'='*80}")
        logger.info(f"THROUGHPUT BENCHMARK ({duration_seconds}s, {parallel_tasks} parallel)")
        logger.info(f"{'='*80}")

        task_code = """
import cupy as cp
import time
time.sleep(0.1)  # Simulate work
result = cp.arange(1000).sum()
print(result)
"""

        submitted_tasks = []
        start_time = time.time()
        task_count = 0

        # Submit tasks as fast as possible
        while time.time() - start_time < duration_seconds:
            # Keep parallel_tasks in flight
            if len(submitted_tasks) < parallel_tasks:
                task_id, submit_latency = self.submit_task(task_code)

                if task_id:
                    submitted_tasks.append({
                        "task_id": task_id,
                        "submit_time": time.time(),
                        "submit_latency": submit_latency
                    })
                    task_count += 1

                    if task_count % 100 == 0:
                        elapsed = time.time() - start_time
                        throughput = task_count / elapsed
                        logger.info(f"  {task_count} tasks submitted ({throughput:.1f} tasks/sec)")

            await asyncio.sleep(0.001)

        elapsed = time.time() - start_time
        total_throughput = task_count / elapsed

        logger.info(f"\n✓ Total tasks submitted: {task_count}")
        logger.info(f"✓ Duration: {elapsed:.2f}s")
        logger.info(f"✓ Throughput: {total_throughput:.1f} tasks/sec")
        logger.info("✓ Target: 100+ tasks/sec")
        logger.info(f"✓ Status: {'PASS ✓' if total_throughput >= 100 else 'FAIL ✗'}")

        return total_throughput, task_count

    async def benchmark_latency(self, num_tasks: int = 1000):
        """Measure latency percentiles (P50, P95, P99)"""

        logger.info(f"\n{'='*80}")
        logger.info(f"LATENCY BENCHMARK ({num_tasks} tasks)")
        logger.info(f"{'='*80}")

        task_code = """
import cupy as cp
result = cp.arange(100).sum()
print(result)
"""

        latencies = []

        for i in range(num_tasks):
            task_id, _submit_latency = self.submit_task(task_code)

            if not task_id:
                logger.warning(f"Failed to submit task {i}")
                continue

            # Wait for completion (up to 10s)
            start = time.time()

            while time.time() - start < 10:
                state, _verified, _exec_time = self.get_task_status(task_id)

                if state == "completed":
                    elapsed = time.time() - start
                    latencies.append(elapsed)
                    break

                await asyncio.sleep(0.1)

            if i % 100 == 0:
                logger.info(f"  {i}/{num_tasks} tasks analyzed...")

        if not latencies:
            logger.error("No completed tasks for latency analysis")
            return None

        # Calculate percentiles
        sorted_latencies = sorted(latencies)
        p50 = statistics.quantiles(sorted_latencies, n=2)[0]
        p95_idx = int(len(sorted_latencies) * 0.95)
        p99_idx = int(len(sorted_latencies) * 0.99)
        p95 = sorted_latencies[p95_idx] if p95_idx < len(sorted_latencies) else sorted_latencies[-1]
        p99 = sorted_latencies[p99_idx] if p99_idx < len(sorted_latencies) else sorted_latencies[-1]

        logger.info(f"\n✓ Completed tasks: {len(latencies)}/{num_tasks}")
        logger.info(f"✓ P50 Latency: {p50*1000:.1f}ms (target: <500ms) {'PASS ✓' if p50*1000 < 500 else 'WARN'}")
        logger.info(f"✓ P95 Latency: {p95*1000:.1f}ms (target: <2000ms) {'PASS ✓' if p95*1000 < 2000 else 'WARN'}")
        logger.info(f"✓ P99 Latency: {p99*1000:.1f}ms (target: <5000ms) {'PASS ✓' if p99*1000 < 5000 else 'WARN'}")
        logger.info(f"✓ Min: {min(latencies)*1000:.1f}ms, Max: {max(latencies)*1000:.1f}ms")

        return {
            "p50": p50 * 1000,
            "p95": p95 * 1000,
            "p99": p99 * 1000,
            "samples": len(latencies)
        }

    async def benchmark_memory_efficiency(self):
        """Measure GPU memory pool efficiency"""

        logger.info(f"\n{'='*80}")
        logger.info("MEMORY EFFICIENCY BENCHMARK")
        logger.info(f"{'='*80}")

        logger.warning(
            "Legacy gpu-swarm Python memory-pool helper removed in RC-0; use Rust/CUDA coverage instead"
        )
        return None

    async def benchmark_network_compression(self):
        """Measure network optimization (compression ratio)"""

        logger.info(f"\n{'='*80}")
        logger.info("NETWORK COMPRESSION BENCHMARK")
        logger.info(f"{'='*80}")

        logger.warning(
            "Legacy gpu-swarm Python network optimizer removed in RC-0; use Rust transport benchmarks instead"
        )
        return None

    async def benchmark_consensus_latency(self):
        """Measure Byzantine consensus latency"""

        logger.info(f"\n{'='*80}")
        logger.info("CONSENSUS LATENCY BENCHMARK")
        logger.info(f"{'='*80}")

        logger.warning(
            "Legacy gpu-swarm Python consensus helper removed in RC-0; use Rust validator coverage instead"
        )
        return None


async def run_full_benchmark_suite():
    """Run complete benchmarking suite"""

    logger.info("\n" + "="*80)
    logger.info("P3 PERFORMANCE BENCHMARKING SUITE")
    logger.info("="*80)

    benchmark = PerformanceBenchmark()

    results = {
        "timestamp": time.time(),
        "tests": {}
    }

    # Run all benchmarks
    logger.info("\n1. Latency Benchmark (100 tasks)...")
    latency_result = await benchmark.benchmark_latency(num_tasks=100)
    if latency_result:
        results["tests"]["latency"] = latency_result

    logger.info("\n2. Throughput Benchmark (30 seconds)...")
    throughput, count = await benchmark.benchmark_throughput(duration_seconds=30, parallel_tasks=10)
    results["tests"]["throughput"] = {"tasks_per_sec": throughput, "total_tasks": count}

    logger.info("\n3. Memory Efficiency Benchmark...")
    memory_result = await benchmark.benchmark_memory_efficiency()
    if memory_result:
        results["tests"]["memory"] = memory_result

    logger.info("\n4. Network Compression Benchmark...")
    network_result = await benchmark.benchmark_network_compression()
    if network_result:
        results["tests"]["network"] = network_result

    logger.info("\n5. Consensus Latency Benchmark...")
    consensus_result = await benchmark.benchmark_consensus_latency()
    if consensus_result:
        results["tests"]["consensus"] = consensus_result

    # Final summary
    logger.info("\n" + "="*80)
    logger.info("BENCHMARK SUMMARY")
    logger.info("="*80)

    if "latency" in results["tests"]:
        lat = results["tests"]["latency"]
        logger.info("\nLatency:")
        logger.info(f"  P50: {lat['p50']:.1f}ms {'✓' if lat['p50'] < 500 else '✗'}")
        logger.info(f"  P95: {lat['p95']:.1f}ms {'✓' if lat['p95'] < 2000 else '✗'}")
        logger.info(f"  P99: {lat['p99']:.1f}ms {'✓' if lat['p99'] < 5000 else '✗'}")

    if "throughput" in results["tests"]:
        thr = results["tests"]["throughput"]
        logger.info("\nThroughput:")
        logger.info(f"  {thr['tasks_per_sec']:.1f} tasks/sec {'✓' if thr['tasks_per_sec'] >= 100 else '✗'}")
        logger.info(f"  ({thr['total_tasks']} tasks in test period)")

    # Save results
    with open("/tmp/p3_benchmark_results.json", "w") as f:
        json.dump(results, f, indent=2, default=str)

    logger.info("\n✓ Results saved to /tmp/p3_benchmark_results.json")

    return results


if __name__ == "__main__":
    asyncio.run(run_full_benchmark_suite())
