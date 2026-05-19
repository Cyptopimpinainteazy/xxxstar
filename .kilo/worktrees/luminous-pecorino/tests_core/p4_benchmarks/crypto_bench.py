#!/usr/bin/env python3
"""Unified crypto benchmark: SHA-256, Ed25519, PoH, Keccak-256, secp256k1.

Measures CPU throughput for all five primitives and attempts GPU acceleration
where CUDA kernels are available. Outputs a JSON report with ops/sec for each.

Reference baselines (P4 Day-1):
  SHA-256 batch      : 1,660,473 hash/sec  (CPU)
  Ed25519 sig verify : 600,711   sig/sec   (CPU)
  PoH chain          : 1,660,473 hash/sec  (CPU, sequential)
  Keccak-256         : not measured
  secp256k1          : not measured
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
import time
from pathlib import Path


def _find_cuda_library(lib_name: str) -> Path | None:
    candidates: list[Path] = []

    env_dir = os.getenv("X3_CUDA_LIB_DIR")
    if env_dir:
        candidates.append(Path(env_dir) / lib_name)

    root = Path(__file__).resolve().parents[2]
    candidates.extend(
        [
            root / "crates" / "cross-chain-gpu-validator" / "kernels" / "build" / lib_name,
            root / "crates" / "x3-gpu-validator-swarm" / "kernels" / "build" / lib_name,
            root / "cross-chain-gpu-validator" / "kernels" / "build" / lib_name,
            Path("/usr/local/lib/x3-chain") / lib_name,
            Path("/usr/lib/x3-chain") / lib_name,
        ]
    )

    for candidate in candidates:
        if candidate.exists():
            return candidate

    return None

# ---------------------------------------------------------------------------
# SHA-256 CPU benchmark
# ---------------------------------------------------------------------------

def bench_sha256(count: int = 200_000, iterations: int = 5) -> dict:
    """Benchmark SHA-256 batch hashing (CPU)."""
    payloads = [os.urandom(32) for _ in range(count)]
    # Warmup
    for p in payloads[:1000]:
        hashlib.sha256(p).digest()

    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        for p in payloads:
            hashlib.sha256(p).digest()
        times.append(time.perf_counter() - t0)

    avg = sum(times) / len(times)
    throughput = count / avg if avg > 0 else 0
    return {
        "operation": "SHA-256",
        "mode": "CPU (hashlib)",
        "batch_size": count,
        "iterations": iterations,
        "avg_time_ms": round(avg * 1000, 2),
        "ops_per_sec": round(throughput),
    }


# ---------------------------------------------------------------------------
# PoH (Proof of History) — sequential SHA-256 chain
# ---------------------------------------------------------------------------

def bench_poh(num_chains: int = 64, chain_length: int = 50_000, iterations: int = 3) -> dict:
    """Benchmark PoH chain: sequential SHA-256 chains."""
    seeds = [os.urandom(32) for _ in range(num_chains)]
    total_hashes = num_chains * chain_length

    # Warmup (short chain)
    h = seeds[0]
    for _ in range(1000):
        h = hashlib.sha256(h).digest()

    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        for seed in seeds:
            h = seed
            for _ in range(chain_length):
                h = hashlib.sha256(h).digest()
        times.append(time.perf_counter() - t0)

    avg = sum(times) / len(times)
    throughput = total_hashes / avg if avg > 0 else 0
    return {
        "operation": "PoH (SHA-256 chain)",
        "mode": "CPU (hashlib)",
        "num_chains": num_chains,
        "chain_length": chain_length,
        "total_hashes": total_hashes,
        "iterations": iterations,
        "avg_time_ms": round(avg * 1000, 2),
        "ops_per_sec": round(throughput),
    }


# ---------------------------------------------------------------------------
# Ed25519 signature verification (CPU)
# ---------------------------------------------------------------------------

def bench_ed25519(count: int = 10_000, iterations: int = 5) -> dict:
    """Benchmark Ed25519 signature verification."""
    try:
        from nacl.signing import SigningKey
    except ImportError:
        # Fallback: use ed25519 from cryptography or hashlib-based stub
        return _bench_ed25519_hashlib_stub(count, iterations)

    # Generate key pair and sign random messages
    sk = SigningKey.generate()
    vk = sk.verify_key
    msgs = [os.urandom(64) for _ in range(count)]
    signed = [sk.sign(m) for m in msgs]

    # Warmup
    for s in signed[:100]:
        vk.verify(s)

    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        for s in signed:
            vk.verify(s)
        times.append(time.perf_counter() - t0)

    avg = sum(times) / len(times)
    throughput = count / avg if avg > 0 else 0
    return {
        "operation": "Ed25519 sig verify",
        "mode": "CPU (PyNaCl/libsodium)",
        "batch_size": count,
        "iterations": iterations,
        "avg_time_ms": round(avg * 1000, 2),
        "ops_per_sec": round(throughput),
    }


def _bench_ed25519_hashlib_stub(count: int, iterations: int) -> dict:
    """Fallback Ed25519 bench using cryptography library."""
    try:
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
        sk = Ed25519PrivateKey.generate()
        pk = sk.public_key()
        msgs = [os.urandom(64) for _ in range(count)]
        sigs = [sk.sign(m) for m in msgs]

        # Warmup
        for m, s in zip(msgs[:100], sigs[:100], strict=False):
            pk.verify(s, m)

        times = []
        for _ in range(iterations):
            t0 = time.perf_counter()
            for m, s in zip(msgs, sigs, strict=False):
                pk.verify(s, m)
            times.append(time.perf_counter() - t0)

        avg = sum(times) / len(times)
        throughput = count / avg if avg > 0 else 0
        return {
            "operation": "Ed25519 sig verify",
            "mode": "CPU (cryptography)",
            "batch_size": count,
            "iterations": iterations,
            "avg_time_ms": round(avg * 1000, 2),
            "ops_per_sec": round(throughput),
        }
    except ImportError:
        return {
            "operation": "Ed25519 sig verify",
            "mode": "SKIPPED (no nacl/cryptography)",
            "ops_per_sec": 0,
        }


# ---------------------------------------------------------------------------
# Keccak-256 batch hashing (CPU)
# ---------------------------------------------------------------------------

def bench_keccak(count: int = 200_000, iterations: int = 5) -> dict:
    """Benchmark Keccak-256 (SHA3-256) batch hashing."""
    payloads = [os.urandom(32) for _ in range(count)]

    # Warmup
    for p in payloads[:1000]:
        hashlib.sha3_256(p).digest()

    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        for p in payloads:
            hashlib.sha3_256(p).digest()
        times.append(time.perf_counter() - t0)

    avg = sum(times) / len(times)
    throughput = count / avg if avg > 0 else 0
    return {
        "operation": "Keccak-256",
        "mode": "CPU (hashlib.sha3_256)",
        "batch_size": count,
        "iterations": iterations,
        "avg_time_ms": round(avg * 1000, 2),
        "ops_per_sec": round(throughput),
    }


# ---------------------------------------------------------------------------
# secp256k1 ECDSA verification (CPU)
# ---------------------------------------------------------------------------

def bench_secp256k1(count: int = 5_000, iterations: int = 3) -> dict:
    """Benchmark secp256k1 ECDSA signature verification."""
    try:
        from cryptography.hazmat.primitives import hashes
        from cryptography.hazmat.primitives.asymmetric import ec

        sk = ec.generate_private_key(ec.SECP256K1())
        pk = sk.public_key()
        msgs = [os.urandom(32) for _ in range(count)]
        sigs = [sk.sign(m, ec.ECDSA(hashes.SHA256())) for m in msgs]

        # Warmup
        for m, s in zip(msgs[:100], sigs[:100], strict=False):
            pk.verify(s, m, ec.ECDSA(hashes.SHA256()))

        times = []
        for _ in range(iterations):
            t0 = time.perf_counter()
            for m, s in zip(msgs, sigs, strict=False):
                pk.verify(s, m, ec.ECDSA(hashes.SHA256()))
            times.append(time.perf_counter() - t0)

        avg = sum(times) / len(times)
        throughput = count / avg if avg > 0 else 0
        return {
            "operation": "secp256k1 ECDSA verify",
            "mode": "CPU (cryptography/OpenSSL)",
            "batch_size": count,
            "iterations": iterations,
            "avg_time_ms": round(avg * 1000, 2),
            "ops_per_sec": round(throughput),
        }
    except (ImportError, Exception) as e:
        return {
            "operation": "secp256k1 ECDSA verify",
            "mode": f"SKIPPED ({e})",
            "ops_per_sec": 0,
        }


# ---------------------------------------------------------------------------
# GPU benchmarks (attempt to load CUDA wrappers)
# ---------------------------------------------------------------------------

def bench_sha256_gpu() -> dict | None:
    """GPU SHA-256 batch hashing via libsha256_batch.so."""
    import ctypes as _ct

    lib_path = _find_cuda_library("libsha256_batch.so")
    if lib_path is None:
        return None

    try:
        lib = _ct.CDLL(str(lib_path))
        lib.sha256_batch_host.argtypes = [_ct.c_void_p, _ct.c_int, _ct.c_void_p]
        lib.sha256_batch_host.restype = _ct.c_int
    except OSError:
        return None

    count = 500_000
    iterations = 5
    data = os.urandom(count * 32)
    out = (_ct.c_ubyte * (count * 32))()

    lib.sha256_batch_host(_ct.c_char_p(data), _ct.c_int(count), _ct.byref(out))

    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        lib.sha256_batch_host(_ct.c_char_p(data), _ct.c_int(count), _ct.byref(out))
        times.append(time.perf_counter() - t0)

    avg = sum(times) / len(times)
    throughput = count / avg if avg > 0 else 0
    return {
        "operation": "SHA-256 GPU",
        "mode": "GPU (CUDA)",
        "batch_size": count,
        "iterations": iterations,
        "avg_time_ms": round(avg * 1000, 2),
        "ops_per_sec": round(throughput),
    }


def bench_poh_gpu() -> dict | None:
    """GPU PoH benchmark via libsha256_batch.so."""
    import ctypes as _ct

    lib_path = _find_cuda_library("libsha256_batch.so")
    if lib_path is None:
        return None

    try:
        lib = _ct.CDLL(str(lib_path))
        lib.sha256_poh_chain_host.argtypes = [_ct.c_void_p, _ct.c_int, _ct.c_int, _ct.c_void_p]
        lib.sha256_poh_chain_host.restype = _ct.c_int
    except OSError:
        return None

    num_chains = 2048
    chain_length = 2000
    iterations = 5
    seeds = os.urandom(num_chains * 32)
    out = (_ct.c_ubyte * (num_chains * 32))()

    lib.sha256_poh_chain_host(
        _ct.c_char_p(seeds), _ct.c_int(num_chains), _ct.c_int(chain_length), _ct.byref(out)
    )

    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        lib.sha256_poh_chain_host(
            _ct.c_char_p(seeds), _ct.c_int(num_chains), _ct.c_int(chain_length), _ct.byref(out)
        )
        times.append(time.perf_counter() - t0)

    avg = sum(times) / len(times)
    throughput = (num_chains * chain_length) / avg if avg > 0 else 0
    return {
        "operation": "PoH GPU",
        "mode": "GPU (CUDA)",
        "batch_size": num_chains,
        "iterations": iterations,
        "avg_time_ms": round(avg * 1000, 2),
        "ops_per_sec": round(throughput),
    }


def bench_ed25519_gpu() -> dict | None:
    """GPU Ed25519 batch verification via libed25519_batch.so."""
    import ctypes as _ct

    lib_path = _find_cuda_library("libed25519_batch.so")
    if lib_path is None:
        return None

    try:
        lib = _ct.CDLL(str(lib_path))
        lib.ed25519_verify_batch_multi_gpu.argtypes = [_ct.c_void_p, _ct.c_int, _ct.c_void_p]
        lib.ed25519_verify_batch_multi_gpu.restype = _ct.c_int
    except OSError:
        return None

    batch_size = 10_000
    iterations = 10
    entries = os.urandom(batch_size * 128)
    out = (_ct.c_ubyte * batch_size)()

    lib.ed25519_verify_batch_multi_gpu(_ct.c_char_p(entries), _ct.c_int(batch_size), _ct.byref(out))

    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        lib.ed25519_verify_batch_multi_gpu(
            _ct.c_char_p(entries), _ct.c_int(batch_size), _ct.byref(out)
        )
        times.append(time.perf_counter() - t0)

    avg = sum(times) / len(times)
    throughput = batch_size / avg if avg > 0 else 0
    return {
        "operation": "Ed25519 GPU",
        "mode": "GPU (CUDA)",
        "batch_size": batch_size,
        "iterations": iterations,
        "avg_time_ms": round(avg * 1000, 2),
        "ops_per_sec": round(throughput),
    }


def bench_keccak_gpu(count: int = 500_000, iterations: int = 5) -> dict | None:
    """GPU Keccak-256 batch hashing via libkeccak256_batch.so."""
    import ctypes as _ct

    lib_path = Path(__file__).resolve().parents[2] / "cross-chain-gpu-validator" / "kernels" / "build" / "libkeccak256_batch.so"
    if not lib_path.exists():
        return None
    try:
        lib = _ct.CDLL(str(lib_path))
        lib.keccak256_batch_host.argtypes = [_ct.c_void_p, _ct.c_int, _ct.c_void_p]
        lib.keccak256_batch_host.restype = _ct.c_int
    except OSError:
        return None

    data = os.urandom(count * 32)
    out = (_ct.c_ubyte * (count * 32))()

    # Warmup
    lib.keccak256_batch_host(_ct.c_char_p(data), _ct.c_int(count), _ct.byref(out))

    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        lib.keccak256_batch_host(_ct.c_char_p(data), _ct.c_int(count), _ct.byref(out))
        times.append(time.perf_counter() - t0)

    avg = sum(times) / len(times)
    throughput = count / avg if avg > 0 else 0
    return {
        "operation": "Keccak-256 GPU",
        "mode": "GPU (CUDA)",
        "batch_size": count,
        "iterations": iterations,
        "avg_time_ms": round(avg * 1000, 2),
        "ops_per_sec": round(throughput),
    }


def bench_secp256k1_gpu(count: int = 10_000, iterations: int = 5) -> dict | None:
    """GPU secp256k1 ECDSA Shamir-trick (u1*G + u2*Q) via libsecp256k1_batch.so."""
    import ctypes as _ct

    lib_path = Path(__file__).resolve().parents[2] / "cross-chain-gpu-validator" / "kernels" / "build" / "libsecp256k1_batch.so"
    if not lib_path.exists():
        return None
    try:
        lib = _ct.CDLL(str(lib_path))
        lib.secp256k1_ecdsa_verify_host.argtypes = [
            _ct.c_void_p, _ct.c_void_p, _ct.c_void_p, _ct.c_int, _ct.c_void_p
        ]
        lib.secp256k1_ecdsa_verify_host.restype = _ct.c_int
    except OSError:
        return None

    # Random u1, u2, pubkeys (throughput test, not correctness)
    u1_data = os.urandom(count * 32)
    u2_data = os.urandom(count * 32)
    pk_data = os.urandom(count * 64)
    out = (_ct.c_ubyte * (count * 32))()

    # Warmup
    lib.secp256k1_ecdsa_verify_host(
        _ct.c_char_p(u1_data), _ct.c_char_p(u2_data), _ct.c_char_p(pk_data),
        _ct.c_int(count), _ct.byref(out))

    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        lib.secp256k1_ecdsa_verify_host(
            _ct.c_char_p(u1_data), _ct.c_char_p(u2_data), _ct.c_char_p(pk_data),
            _ct.c_int(count), _ct.byref(out))
        times.append(time.perf_counter() - t0)

    avg = sum(times) / len(times)
    throughput = count / avg if avg > 0 else 0
    return {
        "operation": "secp256k1 GPU",
        "mode": "GPU (CUDA)",
        "batch_size": count,
        "iterations": iterations,
        "avg_time_ms": round(avg * 1000, 2),
        "ops_per_sec": round(throughput),
    }


# ---------------------------------------------------------------------------
# Main runner
# ---------------------------------------------------------------------------

def main() -> None:
    print("=" * 72)
    print("  X3 Chain — Unified Crypto Benchmark")
    print("  SHA-256 | Ed25519 | PoH | Keccak-256 | secp256k1")
    print("=" * 72)
    print()

    results: list[dict] = []

    benchmarks = [
        ("SHA-256 batch", bench_sha256),
        ("Ed25519 sig verify", bench_ed25519),
        ("PoH (SHA-256 chain)", bench_poh),
        ("Keccak-256 batch", bench_keccak),
        ("secp256k1 ECDSA verify", bench_secp256k1),
    ]

    for label, fn in benchmarks:
        print(f"  [{label}] running...")
        try:
            r = fn()
            results.append(r)
            ops = r.get("ops_per_sec", 0)
            mode = r.get("mode", "?")
            print(f"  [{label}] {ops:>12,} ops/sec  ({mode})")
        except Exception as e:
            print(f"  [{label}] ERROR: {e}")
            results.append({"operation": label, "mode": f"ERROR: {e}", "ops_per_sec": 0})

    # Attempt GPU benchmarks
    print()
    print("  --- GPU acceleration (if available) ---")
    gpu_benchmarks = [
        ("SHA-256 GPU", bench_sha256_gpu),
        ("PoH GPU", bench_poh_gpu),
        ("Ed25519 GPU", bench_ed25519_gpu),
        ("Keccak-256 GPU", bench_keccak_gpu),
        ("secp256k1 GPU", bench_secp256k1_gpu),
    ]
    for label, fn in gpu_benchmarks:
        try:
            r = fn()
            if r:
                r["operation"] = label
                r["mode"] = "GPU (CUDA)"
                results.append(r)
                ops = r.get("ops_per_sec",
                            r.get("throughput_hashes_per_sec",
                                   r.get("throughput_sigs_per_sec", 0)))
                if isinstance(ops, float):
                    ops = round(ops)
                print(f"  [{label}] {ops:>12,} ops/sec  (GPU)")
            else:
                print(f"  [{label}] GPU not available, skipped")
        except Exception as e:
            print(f"  [{label}] {e}")

    # Summary table
    print()
    print("=" * 72)
    print(f"  {'Operation':<30} {'ops/sec':>15} {'Mode':<30}")
    print("-" * 72)
    for r in results:
        op = r.get("operation", "?")
        ops = r.get("ops_per_sec",
                     r.get("throughput_hashes_per_sec",
                           r.get("throughput_sigs_per_sec", 0)))
        mode = r.get("mode", "?")
        if isinstance(ops, float):
            ops = round(ops)
        print(f"  {op:<30} {ops:>15,} {mode:<30}")
    print("=" * 72)

    # Write JSON report
    out_dir = Path(__file__).parent
    report_path = out_dir / "crypto_bench_report.json"
    with open(report_path, "w") as f:
        json.dump({"benchmarks": results, "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S")}, f, indent=2)
    print(f"\n  Report: {report_path}")


if __name__ == "__main__":
    main()
