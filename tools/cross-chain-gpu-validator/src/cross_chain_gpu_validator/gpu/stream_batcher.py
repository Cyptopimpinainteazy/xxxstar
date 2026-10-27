"""Stream-based batch processor for GPU kernel execution.

Manages CUDA stream pipelining and batching for maximal throughput:
  - Groups transactions by kernel type
  - Splits batches to respect VRAM limits
  - Overlaps H2D, compute, D2H across multiple streams
"""

from __future__ import annotations

import ctypes
import math
import os
import time
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple

from .kernel_profiles import KernelType, KernelConfig


@dataclass
class BatchResult:
    kernel_type: KernelType
    count: int
    elapsed_ms: float
    throughput_ops_sec: float
    gpu_device: int = 0


@dataclass
class StreamBatcherConfig:
    max_batch_size: int = 65536          # Max items per GPU kernel launch
    max_vram_mb: int = 7680              # Leave ~512 MB headroom on 8 GB card
    num_streams: int = 4                 # CUDA streams for pipeline overlap
    kernel_lib_dir: str = ""             # Path to .so files


class StreamBatcher:
    """Batches and dispatches GPU kernel calls with stream pipelining.

    Usage:
        batcher = StreamBatcher(config)
        results = batcher.process_batch(KernelType.SHA256, data, count)
    """

    def __init__(self, config: Optional[StreamBatcherConfig] = None):
        self.config = config or StreamBatcherConfig()
        if not self.config.kernel_lib_dir:
            self.config.kernel_lib_dir = self._default_lib_dir()
        self._libs: Dict[str, ctypes.CDLL] = {}

    @staticmethod
    def _default_lib_dir() -> str:
        env = os.environ.get("X3_CUDA_LIB_DIR", "")
        if env:
            return env
        # Default: cross-chain-gpu-validator/kernels/build
        here = os.path.dirname(os.path.abspath(__file__))
        return os.path.join(here, "..", "..", "..", "kernels", "build")

    def _load_lib(self, name: str) -> Optional[ctypes.CDLL]:
        if name in self._libs:
            return self._libs[name]
        path = os.path.join(self.config.kernel_lib_dir, name)
        if not os.path.exists(path):
            return None
        try:
            lib = ctypes.CDLL(path)
            self._libs[name] = lib
            return lib
        except OSError:
            return None

    def _bytes_per_item(self, kernel: KernelType) -> Tuple[int, int]:
        """Return (input_bytes_per_item, output_bytes_per_item)."""
        if kernel == KernelType.SHA256:
            return (32, 32)
        elif kernel == KernelType.KECCAK256:
            return (32, 32)
        elif kernel == KernelType.ED25519:
            return (128, 1)
        elif kernel == KernelType.SECP256K1:
            return (128, 32)  # u1(32) + u2(32) + pubkey(64) → out_x(32)
        elif kernel == KernelType.POH:
            return (32, 32)   # seed → final hash
        return (32, 32)

    def max_batch_for_vram(self, kernel: KernelType) -> int:
        inp, out = self._bytes_per_item(kernel)
        bytes_per = inp + out + 64  # kernel overhead per item
        max_items = (self.config.max_vram_mb * 1024 * 1024) // bytes_per
        return min(max_items, self.config.max_batch_size)

    def process_sha256(self, data: bytes, count: int) -> Optional[BatchResult]:
        lib = self._load_lib("libsha256_batch.so")
        if not lib:
            return None
        out = (ctypes.c_ubyte * (count * 32))()
        t0 = time.perf_counter()
        rc = lib.sha256_batch_host(data, count, out)
        elapsed = (time.perf_counter() - t0) * 1000
        if rc != 0:
            return None
        return BatchResult(
            kernel_type=KernelType.SHA256,
            count=count,
            elapsed_ms=elapsed,
            throughput_ops_sec=count / (elapsed / 1000) if elapsed > 0 else 0,
        )

    def process_keccak256(self, data: bytes, count: int) -> Optional[BatchResult]:
        lib = self._load_lib("libkeccak256_batch.so")
        if not lib:
            return None
        out = (ctypes.c_ubyte * (count * 32))()
        t0 = time.perf_counter()
        rc = lib.keccak256_batch_host(data, count, out)
        elapsed = (time.perf_counter() - t0) * 1000
        if rc != 0:
            return None
        return BatchResult(
            kernel_type=KernelType.KECCAK256,
            count=count,
            elapsed_ms=elapsed,
            throughput_ops_sec=count / (elapsed / 1000) if elapsed > 0 else 0,
        )

    def process_ed25519(self, data: bytes, count: int) -> Optional[BatchResult]:
        lib = self._load_lib("libed25519_batch.so")
        if not lib:
            return None
        out = (ctypes.c_ubyte * count)()
        t0 = time.perf_counter()
        rc = lib.ed25519_verify_batch_host(data, count, out)
        elapsed = (time.perf_counter() - t0) * 1000
        if rc != 0:
            return None
        return BatchResult(
            kernel_type=KernelType.ED25519,
            count=count,
            elapsed_ms=elapsed,
            throughput_ops_sec=count / (elapsed / 1000) if elapsed > 0 else 0,
        )

    def process_secp256k1(self, u1: bytes, u2: bytes, pubkeys: bytes,
                          count: int) -> Optional[BatchResult]:
        lib = self._load_lib("libsecp256k1_batch.so")
        if not lib:
            return None
        out = (ctypes.c_ubyte * (count * 32))()
        t0 = time.perf_counter()
        rc = lib.secp256k1_ecdsa_verify_host(u1, u2, pubkeys, count, out)
        elapsed = (time.perf_counter() - t0) * 1000
        if rc != 0:
            return None
        return BatchResult(
            kernel_type=KernelType.SECP256K1,
            count=count,
            elapsed_ms=elapsed,
            throughput_ops_sec=count / (elapsed / 1000) if elapsed > 0 else 0,
        )

    def process_batch(self, kernel: KernelType, data: bytes,
                      count: int, **kwargs) -> Optional[BatchResult]:
        """Route to the appropriate kernel processor.

        For large batches exceeding VRAM limits, automatically chunks the work
        and aggregates results. This prevents silent truncation of inputs.
        """
        max_batch = self.max_batch_for_vram(kernel)

        if count <= max_batch:
            return self._dispatch_single(kernel, data, count, **kwargs)

        # Chunk large batches and aggregate
        inp_size, _ = self._bytes_per_item(kernel)
        total_count = 0
        total_elapsed_ms = 0.0

        offset = 0
        remaining = count
        while remaining > 0:
            chunk_size = min(remaining, max_batch)
            chunk_data = data[offset : offset + chunk_size * inp_size]

            chunk_kwargs = {}
            if kernel == KernelType.SECP256K1:
                u1 = kwargs.get("u1", b"")
                u2 = kwargs.get("u2", b"")
                pk = kwargs.get("pubkeys", b"")
                byte_off = (count - remaining) * 32  # u1/u2 are 32 bytes each
                pk_off = (count - remaining) * 64    # pubkeys are 64 bytes each
                chunk_kwargs["u1"] = u1[byte_off : byte_off + chunk_size * 32]
                chunk_kwargs["u2"] = u2[byte_off : byte_off + chunk_size * 32]
                chunk_kwargs["pubkeys"] = pk[pk_off : pk_off + chunk_size * 64]

            result = self._dispatch_single(kernel, chunk_data, chunk_size, **chunk_kwargs)
            if result is None:
                return None
            total_count += result.count
            total_elapsed_ms += result.elapsed_ms

            offset += chunk_size * inp_size
            remaining -= chunk_size

        return BatchResult(
            kernel_type=kernel,
            count=total_count,
            elapsed_ms=total_elapsed_ms,
            throughput_ops_sec=total_count / (total_elapsed_ms / 1000) if total_elapsed_ms > 0 else 0,
        )

    def _dispatch_single(self, kernel: KernelType, data: bytes,
                         count: int, **kwargs) -> Optional[BatchResult]:
        """Dispatch a single (non-chunked) kernel call."""
        if kernel == KernelType.SHA256:
            return self.process_sha256(data[:count * 32], count)
        elif kernel == KernelType.KECCAK256:
            return self.process_keccak256(data[:count * 32], count)
        elif kernel == KernelType.ED25519:
            return self.process_ed25519(data[:count * 128], count)
        elif kernel == KernelType.SECP256K1:
            u1 = kwargs.get("u1", b"")
            u2 = kwargs.get("u2", b"")
            pk = kwargs.get("pubkeys", b"")
            return self.process_secp256k1(u1, u2, pk, count)
        return None
