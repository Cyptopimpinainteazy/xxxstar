"""
Ed25519 GPU Batch Verifier — Python ctypes wrapper

Loads libed25519_batch.so and provides a high-level API for batch
signature verification on CUDA GPUs.

Falls back to pure-Python (nacl/ed25519) verification when GPU is unavailable.
"""

import ctypes
import os
import struct
import time
from pathlib import Path
from typing import List, Optional, Tuple

# Locate the shared library
_LIB_SEARCH_PATHS = [
    Path(__file__).parent / "cu_kernels" / "build" / "libed25519_batch.so",
    Path(__file__).parent.parent / "src" / "cu_kernels" / "build" / "libed25519_batch.so",
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
                # Set function signatures
                _lib.ed25519_verify_batch_host.restype = ctypes.c_int
                _lib.ed25519_verify_batch_host.argtypes = [
                    ctypes.c_char_p,  # entries
                    ctypes.c_int,     # count
                    ctypes.c_char_p,  # results
                ]
                _lib.ed25519_verify_batch_multi_gpu.restype = ctypes.c_int
                _lib.ed25519_verify_batch_multi_gpu.argtypes = [
                    ctypes.c_char_p,
                    ctypes.c_int,
                    ctypes.c_char_p,
                ]
                _lib.ed25519_print_gpu_info.restype = None
                _lib.ed25519_print_gpu_info.argtypes = []
                return _lib
            except OSError as e:
                print(f"Warning: Failed to load {path}: {e}")
    return None


class Ed25519GPUVerifier:
    """GPU-accelerated Ed25519 batch signature verifier."""

    SIG_ENTRY_SIZE = 128  # R(32) + s(32) + pubkey(32) + msg(32)

    def __init__(self, multi_gpu: bool = True):
        self.lib = _load_lib()
        self.multi_gpu = multi_gpu
        self.gpu_available = self.lib is not None

    def print_info(self):
        if self.lib:
            self.lib.ed25519_print_gpu_info()
        else:
            print("GPU not available — using CPU fallback")

    def verify_batch(
        self,
        signatures: List[Tuple[bytes, bytes, bytes, bytes]],
    ) -> List[bool]:
        """
        Verify a batch of Ed25519 signatures on GPU.

        Args:
            signatures: List of (R, s, public_key, message) tuples
                        Each element is 32 bytes.

        Returns:
            List of booleans: True if valid, False if invalid.
        """
        count = len(signatures)
        if count == 0:
            return []

        if not self.gpu_available:
            return self._verify_cpu_fallback(signatures)

        # Pack entries: each is 128 bytes = R(32) + s(32) + pubkey(32) + msg(32)
        entries = bytearray(count * self.SIG_ENTRY_SIZE)
        for i, (R, s, pubkey, msg) in enumerate(signatures):
            offset = i * self.SIG_ENTRY_SIZE
            entries[offset:offset+32] = R[:32]
            entries[offset+32:offset+64] = s[:32]
            entries[offset+64:offset+96] = pubkey[:32]
            entries[offset+96:offset+128] = msg[:32]

        results = bytearray(count)

        entries_buf = (ctypes.c_char * len(entries)).from_buffer(entries)
        results_buf = (ctypes.c_char * count).from_buffer(results)

        if self.multi_gpu:
            ret = self.lib.ed25519_verify_batch_multi_gpu(entries_buf, count, results_buf)
        else:
            ret = self.lib.ed25519_verify_batch_host(entries_buf, count, results_buf)

        if ret != 0:
            print(f"Warning: GPU verification returned error {ret}, falling back to CPU")
            return self._verify_cpu_fallback(signatures)

        return [bool(results[i]) for i in range(count)]

    def benchmark(self, batch_size: int = 1024, iterations: int = 10) -> dict:
        """Run a benchmark with random data (checking throughput, not correctness)."""
        import os as _os

        # Generate random entries (signatures will be invalid, but we measure throughput)
        entries = _os.urandom(batch_size * self.SIG_ENTRY_SIZE)

        results = bytearray(batch_size)
        entries_buf = ctypes.c_char_p(entries)
        results_buf = (ctypes.c_char * batch_size).from_buffer(results)

        # Warmup
        if self.lib:
            self.lib.ed25519_verify_batch_host(entries_buf, batch_size, results_buf)

        times = []
        for _ in range(iterations):
            start = time.perf_counter()
            if self.lib and self.multi_gpu:
                self.lib.ed25519_verify_batch_multi_gpu(entries_buf, batch_size, results_buf)
            elif self.lib:
                self.lib.ed25519_verify_batch_host(entries_buf, batch_size, results_buf)
            else:
                time.sleep(0.001)  # Simulate for CPU path
            elapsed = time.perf_counter() - start
            times.append(elapsed)

        avg_time = sum(times) / len(times)
        throughput = batch_size / avg_time if avg_time > 0 else 0

        return {
            "batch_size": batch_size,
            "iterations": iterations,
            "avg_time_ms": avg_time * 1000,
            "throughput_sigs_per_sec": throughput,
            "gpu_available": self.gpu_available,
            "multi_gpu": self.multi_gpu,
        }

    @staticmethod
    def _verify_cpu_fallback(
        signatures: List[Tuple[bytes, bytes, bytes, bytes]],
    ) -> List[bool]:
        """Pure-Python Ed25519 verification (slow, for correctness reference)."""
        try:
            from nacl.signing import VerifyKey
            from nacl.exceptions import BadSignatureError
            results = []
            for R, s, pubkey, msg in signatures:
                try:
                    vk = VerifyKey(pubkey)
                    vk.verify(msg, R + s)
                    results.append(True)
                except (BadSignatureError, Exception):
                    results.append(False)
            return results
        except ImportError:
            # No nacl available, return all False
            return [False] * len(signatures)
