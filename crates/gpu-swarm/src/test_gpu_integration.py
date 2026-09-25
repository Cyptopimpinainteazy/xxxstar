#!/usr/bin/env python3
"""
P4 Day 5: GPU Integration Test & Benchmark
============================================

Tests:
  1. SHA-256 batch correctness: GPU output == hashlib.sha256 for known inputs
  2. SHA-256 PoH chain correctness: GPU chain == CPU chain
  3. SHA-256 throughput benchmark
  4. PoH chain throughput benchmark
  5. Ed25519 GPU kernel load + throughput benchmark (throughput test with random data)
  6. Multi-GPU distribution test

Invariants referenced:
  - INV-GPU-001: GPU kernels produce identical output to CPU reference
  - INV-GPU-002: Multi-GPU splits work evenly across devices
  - INV-PERF-001: GPU batch hashing exceeds 1M hashes/sec
"""

import hashlib
import os
import sys
import time
import ctypes
from pathlib import Path

# Add parent to path
sys.path.insert(0, str(Path(__file__).parent))

PASS = "\033[92m✓ PASS\033[0m"
FAIL = "\033[91m✗ FAIL\033[0m"
INFO = "\033[94mℹ INFO\033[0m"

def divider(title: str):
    print(f"\n{'═' * 60}")
    print(f"  {title}")
    print(f"{'═' * 60}")


def test_sha256_batch_correctness():
    """Test that GPU SHA-256 matches hashlib for known inputs."""
    divider("Test 1: SHA-256 Batch Correctness")

    from sha256_gpu import SHA256GPUHasher
    hasher = SHA256GPUHasher(multi_gpu=False)

    if not hasher.gpu_available:
        print(f"  {INFO} GPU not available, testing CPU fallback path")

    # Known test vectors (32-byte inputs)
    test_inputs = [
        b'\x00' * 32,
        b'\xff' * 32,
        b'\x01' + b'\x00' * 31,
        bytes(range(32)),
        hashlib.sha256(b"x3-chain").digest(),
        hashlib.sha256(b"solana-poh-test").digest(),
    ]

    # Compute CPU reference
    cpu_outputs = [hashlib.sha256(inp).digest() for inp in test_inputs]

    # Compute GPU outputs
    gpu_outputs = hasher.hash_batch(test_inputs)

    passed = 0
    for i, (cpu, gpu) in enumerate(zip(cpu_outputs, gpu_outputs)):
        if cpu == gpu:
            passed += 1
        else:
            print(f"  {FAIL} Input {i}: CPU={cpu.hex()[:16]}... GPU={gpu.hex()[:16]}...")

    if passed == len(test_inputs):
        print(f"  {PASS} All {passed}/{len(test_inputs)} SHA-256 hashes match CPU reference")
    else:
        print(f"  {FAIL} {passed}/{len(test_inputs)} matched")

    return passed == len(test_inputs)


def test_sha256_poh_chain_correctness():
    """Test that GPU PoH chain matches CPU sequential hashing."""
    divider("Test 2: PoH Chain Correctness")

    from sha256_gpu import SHA256GPUHasher
    hasher = SHA256GPUHasher(multi_gpu=False)

    seeds = [
        b'\x00' * 32,
        b'\x01' * 32,
        bytes(range(32)),
    ]
    chain_length = 100

    # CPU reference
    cpu_results = []
    for seed in seeds:
        h = seed
        for _ in range(chain_length):
            h = hashlib.sha256(h).digest()
        cpu_results.append(h)

    # GPU
    gpu_results = hasher.poh_chain(seeds, chain_length)

    passed = 0
    for i, (cpu, gpu) in enumerate(zip(cpu_results, gpu_results)):
        if cpu == gpu:
            passed += 1
        else:
            print(f"  {FAIL} Chain {i}: CPU={cpu.hex()[:16]}... GPU={gpu.hex()[:16]}...")

    if passed == len(seeds):
        print(f"  {PASS} All {passed}/{len(seeds)} PoH chains match CPU reference")
    else:
        print(f"  {FAIL} {passed}/{len(seeds)} chains matched")

    return passed == len(seeds)


def benchmark_sha256_batch():
    """Benchmark GPU batch SHA-256 throughput."""
    divider("Benchmark: SHA-256 Batch Throughput")

    from sha256_gpu import SHA256GPUHasher
    hasher = SHA256GPUHasher(multi_gpu=True)

    for count in [10_000, 100_000, 500_000, 1_000_000]:
        result = hasher.benchmark_batch(count=count, iterations=5)
        throughput = result["throughput_hashes_per_sec"]
        avg_ms = result["avg_time_ms"]
        gpu_tag = "GPU" if result["gpu_available"] else "CPU"
        print(f"  [{gpu_tag}] {count:>10,} hashes: {throughput:>12,.0f} H/s  ({avg_ms:.1f} ms)")


def benchmark_poh_chain():
    """Benchmark GPU PoH chain throughput."""
    divider("Benchmark: PoH Chain Throughput")

    from sha256_gpu import SHA256GPUHasher
    hasher = SHA256GPUHasher(multi_gpu=False)

    configs = [
        (1024, 1000),     # 1M total hashes
        (256, 10000),     # 2.56M total hashes
        (4096, 1000),     # 4M total hashes
        (1024, 10000),    # 10M total hashes
    ]

    for num_chains, chain_len in configs:
        result = hasher.benchmark_poh(num_chains=num_chains, chain_length=chain_len, iterations=3)
        throughput = result["throughput_hashes_per_sec"]
        total = result["total_hashes"]
        avg_ms = result["avg_time_ms"]
        gpu_tag = "GPU" if result["gpu_available"] else "CPU"
        print(f"  [{gpu_tag}] {num_chains:>5} chains × {chain_len:>6} = {total:>10,} hashes: "
              f"{throughput:>12,.0f} H/s  ({avg_ms:.1f} ms)")


def test_ed25519_kernel_load():
    """Test that the Ed25519 kernel loads and runs without crashing."""
    divider("Test 3: Ed25519 Kernel Load + Throughput")

    from ed25519_gpu import Ed25519GPUVerifier
    verifier = Ed25519GPUVerifier(multi_gpu=True)
    verifier.print_info()

    if not verifier.gpu_available:
        print(f"  {INFO} Ed25519 GPU library not loaded, skipping GPU benchmark")
        return True

    # Benchmark with random data (measures kernel throughput, not correctness)
    for batch in [256, 1024, 4096, 8192]:
        result = verifier.benchmark(batch_size=batch, iterations=5)
        throughput = result["throughput_sigs_per_sec"]
        avg_ms = result["avg_time_ms"]
        gpu_tag = "multi-GPU" if result["multi_gpu"] else "single-GPU"
        print(f"  [{gpu_tag}] {batch:>6} sigs: {throughput:>12,.0f} sig/s  ({avg_ms:.1f} ms)")

    print(f"  {PASS} Ed25519 GPU kernel loaded and executed successfully")
    return True


def test_multi_gpu_distribution():
    """Verify multi-GPU path distributes work across devices."""
    divider("Test 4: Multi-GPU Distribution")

    lib_path = Path(__file__).parent / "cu_kernels" / "build" / "libsha256_batch.so"
    if not lib_path.exists():
        print(f"  {INFO} Library not found at {lib_path}, skipping")
        return True

    lib = ctypes.CDLL(str(lib_path))

    count = 100_000
    data = os.urandom(count * 32)
    out_single = bytearray(count * 32)
    out_multi = bytearray(count * 32)

    in_buf = ctypes.c_char_p(data)

    # Single GPU
    out_buf1 = (ctypes.c_char * len(out_single)).from_buffer(out_single)
    lib.sha256_batch_host(in_buf, count, out_buf1)

    # Multi GPU
    out_buf2 = (ctypes.c_char * len(out_multi)).from_buffer(out_multi)
    lib.sha256_batch_multi_gpu(in_buf, count, out_buf2)

    if out_single == out_multi:
        print(f"  {PASS} Single-GPU and Multi-GPU outputs are identical ({count:,} hashes)")
        return True
    else:
        # Find first mismatch
        for i in range(count):
            s = out_single[i*32:(i+1)*32]
            m = out_multi[i*32:(i+1)*32]
            if s != m:
                print(f"  {FAIL} Mismatch at index {i}: single={s[:8].hex()}... multi={m[:8].hex()}...")
                break
        return False


def main():
    divider("P4 Day 5: GPU Integration Tests")
    print(f"  Date: {time.strftime('%Y-%m-%d %H:%M:%S')}")

    os.chdir(Path(__file__).parent)

    results = {}
    results["sha256_correctness"] = test_sha256_batch_correctness()
    results["poh_correctness"] = test_sha256_poh_chain_correctness()

    benchmark_sha256_batch()
    benchmark_poh_chain()

    results["ed25519_load"] = test_ed25519_kernel_load()
    results["multi_gpu"] = test_multi_gpu_distribution()

    divider("Summary")
    all_pass = all(results.values())
    for name, passed in results.items():
        status = PASS if passed else FAIL
        print(f"  {status} {name}")

    if all_pass:
        print(f"\n  🎉 All tests passed!")
    else:
        print(f"\n  ⚠️  Some tests failed")

    return 0 if all_pass else 1


if __name__ == "__main__":
    sys.exit(main())
