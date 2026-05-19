"""GPU batch hasher for keccak256."""

from __future__ import annotations

from dataclasses import dataclass
import os
import ctypes
from typing import Iterable, Sequence

from .cuda_loader import CudaRuntime


def _keccak256(data: bytes) -> bytes:
    """Compute real Keccak-256 (NOT SHA3-256 which uses different padding)."""
    try:
        from Crypto.Hash import keccak
        return keccak.new(digest_bits=256, data=data).digest()
    except ImportError:
        pass
    try:
        import sha3  # pysha3
        return sha3.keccak_256(data).digest()
    except ImportError:
        pass
    # Last resort: pycrypme not installed, warn and use sha3_256
    # This will produce WRONG results for parity checks!
    import hashlib
    import warnings
    warnings.warn(
        "Neither pycrypme nor pysha3 installed — falling back to SHA3-256 "
        "which differs from Keccak-256. Install pycrypme for correctness.",
        RuntimeWarning,
        stacklevel=3,
    )
    return hashlib.sha3_256(data).digest()


@dataclass
class KeccakBatchHasher:
    """Batch hasher with GPU-first execution and CPU failover."""

    runtime: CudaRuntime
    kernel_dir: str
    parity_check: bool
    allow_failover: bool

    def __init__(
        self,
        runtime: CudaRuntime,
        kernel_dir: str,
        parity_check: bool = True,
        allow_failover: bool = True,
    ) -> None:
        self.runtime = runtime
        self.kernel_dir = kernel_dir
        self.parity_check = parity_check
        self.allow_failover = allow_failover
        self._lib = None
        if self.runtime.available:
            lib_path = os.path.join(self.kernel_dir, "build", "libkeccak256_batch.so")
            if os.path.exists(lib_path):
                self._lib = ctypes.CDLL(lib_path)
                self._lib.keccak256_batch_host.argtypes = [
                    ctypes.c_void_p,
                    ctypes.c_int,
                    ctypes.c_void_p,
                ]
                self._lib.keccak256_batch_host.restype = ctypes.c_int
            elif not self.allow_failover:
                raise RuntimeError("Missing libkeccak256_batch.so for required GPU mode")

    def hash_batch(self, payloads: Iterable[bytes]) -> list[bytes]:
        # Materialize once so we can replay for GPU + parity check
        payloads_list: Sequence[bytes] = (
            payloads if isinstance(payloads, (list, tuple)) else list(payloads)
        )
        try:
            if self.runtime.available and self._lib is not None:
                return self._hash_gpu(payloads_list)
        except Exception:
            if self.allow_failover:
                return self._hash_cpu(payloads_list)
            raise
        return self._hash_cpu(payloads_list)

    def _hash_gpu(self, payloads: Sequence[bytes]) -> list[bytes]:
        packed_payloads = self._pack_bytes(payloads, 32, "payloads")
        count = len(packed_payloads) // 32
        digests = (ctypes.c_ubyte * (count * 32))()
        status = self._lib.keccak256_batch_host(
            ctypes.c_char_p(packed_payloads),
            ctypes.c_int(count),
            ctypes.byref(digests),
        )
        if status != 0:
            raise RuntimeError("GPU keccak256 batch hashing failed")
        gpu_hashes = [bytes(digests[i * 32 : (i + 1) * 32]) for i in range(count)]
        if self.parity_check:
            cpu_hashes = self._hash_cpu(payloads)
            if gpu_hashes != cpu_hashes:
                raise RuntimeError("GPU keccak256 results diverged from CPU")
        return gpu_hashes

    @staticmethod
    def _pack_bytes(values: Iterable[bytes], size: int, label: str) -> bytes:
        packed = bytearray()
        for value in values:
            if len(value) != size:
                raise ValueError(f"{label} entry must be {size} bytes")
            packed.extend(value)
        if not packed:
            raise ValueError(f"{label} batch is empty")
        return bytes(packed)

    @staticmethod
    def _hash_cpu(payloads: Iterable[bytes]) -> list[bytes]:
        return [_keccak256(payload) for payload in payloads]
