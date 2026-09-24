"""
SHA-256 GPU Batch Hasher + PoH Chain — Python ctypes wrapper

Loads libsha256_batch.so and provides APIs for:
  1. Batch SHA-256 hashing (independent 32-byte inputs)
  2. PoH chain computation (sequential SHA-256 chains in parallel)

Falls back to hashlib.sha256 when GPU is unavailable.
"""

import ctypes
import hashlib
import os
import time
from pathlib import Path
from typing import List, Optional

_LIB_SEARCH_PATHS = [
    Path(__file__).parent / "cu_kernels" / "build" / "libsha256_batch.so",
    Path(__file__).parent.parent / "src" / "cu_kernels" / "build" / "libsha256_batch.so",
]

_lib: Optional[ctypes.CDLL] = None


def _load_lib() -> Optional[ctypes.CDLL]:
    global _lib
    if _lib is not None:
        return _lib
    for path in _LIB_SEARCH_PATHS:
        if path.exists():
            try:
                _lib = ctypes.CDLL(str(path))
                _lib.sha256_batch_host.restype = ctypes.c_int
                _lib.sha256_batch_host.argtypes = [
                    ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p
                ]
                _lib.sha256_poh_chain_host.restype = ctypes.c_int
                _lib.sha256_poh_chain_host.argtypes = [
                    ctypes.c_char_p, ctypes.c_int, ctypes.c_int, ctypes.c_char_p
                ]
                _lib.sha256_batch_multi_gpu.restype = ctypes.c_int
                _lib.sha256_batch_multi_gpu.argtypes = [
                    ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p
                ]
                return _lib
            except OSError as e:
                print(f"Warning: Failed to load {path}: {e}")
    return None


class SHA256GPUHasher:
    """GPU-accelerated SHA-256 batch hasher."""

    def __init__(self, multi_gpu: bool = True):
        self.lib = _load_lib()
        self.multi_gpu = multi_gpu
        self.gpu_available = self.lib is not None

    def hash_batch(self, inputs: List[bytes]) -> List[bytes]:
        """
        Compute SHA-256 of each 32-byte input independently.

        Args:
            inputs: List of 32-byte inputs.

        Returns:
            List of 32-byte SHA-256 digests.
        """
        count = len(inputs)
        if count == 0:
            return []

        if not self.gpu_available:
            return self._hash_cpu(inputs)

        flat_in = bytearray(count * 32)
        for i, inp in enumerate(inputs):
            flat_in[i*32:(i+1)*32] = inp[:32]

        flat_out = bytearray(count * 32)
        in_buf = ctypes.c_char_p(bytes(flat_in))
        out_buf = (ctypes.c_char * len(flat_out)).from_buffer(flat_out)

        if self.multi_gpu:
            ret = self.lib.sha256_batch_multi_gpu(in_buf, count, out_buf)
        else:
            ret = self.lib.sha256_batch_host(in_buf, count, out_buf)

        if ret != 0:
            return self._hash_cpu(inputs)

        return [bytes(flat_out[i*32:(i+1)*32]) for i in range(count)]

    def poh_chain(self, seeds: List[bytes], chain_length: int) -> List[bytes]:
        """
        Compute parallel PoH chains on GPU.

        Each chain starts with a 32-byte seed and applies SHA-256
        `chain_length` times sequentially. Multiple chains are computed
        in parallel on different GPU threads.

        Args:
            seeds: List of 32-byte seed values.
            chain_length: Number of SHA-256 iterations per chain.

        Returns:
            List of 32-byte final hash values.
        """
        num_chains = len(seeds)
        if num_chains == 0:
            return []

        if not self.gpu_available:
            return self._poh_cpu(seeds, chain_length)

        flat_seeds = bytearray(num_chains * 32)
        for i, seed in enumerate(seeds):
            flat_seeds[i*32:(i+1)*32] = seed[:32]

        flat_results = bytearray(num_chains * 32)
        seed_buf = ctypes.c_char_p(bytes(flat_seeds))
        result_buf = (ctypes.c_char * len(flat_results)).from_buffer(flat_results)

        ret = self.lib.sha256_poh_chain_host(seed_buf, num_chains, chain_length, result_buf)
        if ret != 0:
            return self._poh_cpu(seeds, chain_length)

        return [bytes(flat_results[i*32:(i+1)*32]) for i in range(num_chains)]

    def benchmark_batch(self, count: int = 100000, iterations: int = 5) -> dict:
        """Benchmark batch SHA-256 throughput."""
        data = os.urandom(count * 32)
        out = bytearray(count * 32)
        in_buf = ctypes.c_char_p(data)
        out_buf = (ctypes.c_char * len(out)).from_buffer(out)

        # Warmup
        if self.lib:
            self.lib.sha256_batch_host(in_buf, count, out_buf)

        times = []
        for _ in range(iterations):
            t0 = time.perf_counter()
            if self.lib and self.multi_gpu:
                self.lib.sha256_batch_multi_gpu(in_buf, count, out_buf)
            elif self.lib:
                self.lib.sha256_batch_host(in_buf, count, out_buf)
            else:
                for i in range(count):
                    hashlib.sha256(data[i*32:(i+1)*32]).digest()
            elapsed = time.perf_counter() - t0
            times.append(elapsed)

        avg = sum(times) / len(times)
        throughput = count / avg if avg > 0 else 0

        return {
            "count": count,
            "iterations": iterations,
            "avg_time_ms": avg * 1000,
            "throughput_hashes_per_sec": throughput,
            "gpu_available": self.gpu_available,
        }

    def benchmark_poh(self, num_chains: int = 1024, chain_length: int = 1000, iterations: int = 5) -> dict:
        """Benchmark PoH chain throughput."""
        seeds = os.urandom(num_chains * 32)
        results = bytearray(num_chains * 32)
        seed_buf = ctypes.c_char_p(seeds)
        result_buf = (ctypes.c_char * len(results)).from_buffer(results)

        # Warmup
        if self.lib:
            self.lib.sha256_poh_chain_host(seed_buf, num_chains, chain_length, result_buf)

        times = []
        for _ in range(iterations):
            t0 = time.perf_counter()
            if self.lib:
                self.lib.sha256_poh_chain_host(seed_buf, num_chains, chain_length, result_buf)
            elapsed = time.perf_counter() - t0
            times.append(elapsed)

        avg = sum(times) / len(times)
        total_hashes = num_chains * chain_length
        throughput = total_hashes / avg if avg > 0 else 0

        return {
            "num_chains": num_chains,
            "chain_length": chain_length,
            "total_hashes": total_hashes,
            "iterations": iterations,
            "avg_time_ms": avg * 1000,
            "throughput_hashes_per_sec": throughput,
            "gpu_available": self.gpu_available,
        }

    @staticmethod
    def _hash_cpu(inputs: List[bytes]) -> List[bytes]:
        return [hashlib.sha256(inp).digest() for inp in inputs]

    @staticmethod
    def _poh_cpu(seeds: List[bytes], chain_length: int) -> List[bytes]:
        results = []
        for seed in seeds:
            h = seed
            for _ in range(chain_length):
                h = hashlib.sha256(h).digest()
            results.append(h)
        return results
