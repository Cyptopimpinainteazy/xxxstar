"""GPU kernel dispatcher for cross-chain validation.

This module is intentionally thin: concrete GPU work is delegated to the
ctypes-backed kernel bindings that load the same CUDA shared libraries used by
the Rust X3 VM GPU hostcalls. Unsupported operations fail explicitly instead of
silently running CPU simulations under a GPU label.
"""

from __future__ import annotations

from time import perf_counter
from typing import Iterable

from .cuda_loader import CudaRuntime
from .keccak_gpu import KeccakBatchHasher
from .secp256k1_gpu import Secp256k1BatchVerifier


class GPUKernelError(Exception):
    """GPU kernel execution error."""


class GPUKernels:
    """Manager for GPU-backed cross-chain cryptographic kernels."""

    def __init__(
        self,
        device_id: int = 0,
        kernel_dir: str = "infra-structure/validator/kernels",
        parity_check: bool = True,
        allow_failover: bool = True,
    ) -> None:
        self.device_id = device_id
        self.runtime = CudaRuntime.detect()
        self.kernel_dir = kernel_dir
        self.parity_check = parity_check
        self.allow_failover = allow_failover
        self._keccak = KeccakBatchHasher(
            self.runtime,
            kernel_dir,
            parity_check=parity_check,
            allow_failover=allow_failover,
        )
        self._secp256k1 = Secp256k1BatchVerifier(
            self.runtime,
            kernel_dir,
            parity_check=parity_check,
            allow_failover=allow_failover,
        )

    def sha256_batch(self, data: Iterable[bytes]) -> list[bytes]:
        """Batch SHA-256 hashing.

        The Rust X3 VM has a CUDA SHA-256 hostcall, but this Python validator
        package does not yet expose a direct X3 VM execution bridge. Avoid
        presenting CPU hashing as GPU execution here.
        """
        raise GPUKernelError("SHA-256 GPU execution must go through x3-vm GPU hostcalls")

    def ed25519_verify_batch(
        self,
        messages: Iterable[bytes],
        signatures: Iterable[bytes],
        pubkeys: Iterable[bytes],
    ) -> list[bool]:
        """Batch Ed25519 signature verification."""
        raise GPUKernelError("Ed25519 GPU execution must go through x3-vm GPU hostcalls")

    def poh_verify(self, hashes: list[bytes], count: int) -> bool:
        """Verify Proof-of-History sequence."""
        raise GPUKernelError("PoH GPU execution must go through x3-vm GPU hostcalls")

    def keccak256_batch(self, data: Iterable[bytes]) -> list[bytes]:
        """Batch Keccak-256 hashing using the real CUDA binding when loaded."""
        try:
            return self._keccak.hash_batch(data)
        except Exception as exc:
            raise GPUKernelError(str(exc)) from exc

    def secp256k1_verify_batch(
        self,
        messages: Iterable[bytes],
        signatures: Iterable[bytes],
        pubkeys: Iterable[bytes],
    ) -> list[bool]:
        """Batch secp256k1 signature verification using the real CUDA binding."""
        try:
            return self._secp256k1.verify_batch(signatures, messages, pubkeys)
        except Exception as exc:
            raise GPUKernelError(str(exc)) from exc

    def benchmark(self, payloads: Iterable[bytes]) -> dict[str, float | int | bool]:
        """Benchmark the currently wired Python GPU path."""
        payloads_list = list(payloads)
        if not payloads_list:
            raise GPUKernelError("benchmark payloads must not be empty")

        start = perf_counter()
        self.keccak256_batch(payloads_list)
        elapsed = perf_counter() - start

        return {
            "gpu_available": self.runtime.available,
            "device_id": self.device_id,
            "batch_size": len(payloads_list),
            "keccak256_ops_per_sec": len(payloads_list) / elapsed if elapsed else 0.0,
        }


if __name__ == "__main__":
    kernels = GPUKernels()
    payloads = [bytes([idx % 256]) * 32 for idx in range(1024)]
    print(kernels.benchmark(payloads))
