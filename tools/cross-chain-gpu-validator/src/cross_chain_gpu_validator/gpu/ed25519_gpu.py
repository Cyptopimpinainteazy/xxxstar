"""GPU batch verifier for Ed25519 signatures.

This is used by SVM (Solana) and Substrate (Polkadot/Kusama) validators.

The GPU kernel is implemented in crates/gpu-swarm/src/cu_kernels/ed25519_batch.cu
and is built into libed25519_batch.so.
"""

from __future__ import annotations

from dataclasses import dataclass
import logging
import os
import ctypes
from typing import Callable, Iterable, Optional, Sequence

from .cuda_loader import CudaRuntime

logger = logging.getLogger(__name__)


@dataclass
class Ed25519BatchVerifier:
    """Batch verifier with GPU-first execution and CPU failover."""

    runtime: CudaRuntime
    kernel_dir: str
    parity_check: bool
    allow_failover: bool
    on_gpu_disabled: Optional[Callable[[str], None]]

    def __init__(
        self,
        runtime: CudaRuntime,
        kernel_dir: str,
        parity_check: bool = True,
        allow_failover: bool = True,
        on_gpu_disabled: Optional[Callable[[str], None]] = None,
    ) -> None:
        self.runtime = runtime
        self.kernel_dir = kernel_dir
        self.parity_check = parity_check
        self.allow_failover = allow_failover
        self.on_gpu_disabled = on_gpu_disabled
        self._lib = None
        self._gpu_ok = True

        if self.runtime.available:
            lib_path = os.path.join(self.kernel_dir, "build", "libed25519_batch.so")
            if os.path.exists(lib_path):
                self._lib = ctypes.CDLL(lib_path)
                self._lib.ed25519_verify_batch_host.argtypes = [
                    ctypes.c_void_p,
                    ctypes.c_int,
                    ctypes.c_void_p,
                ]
                self._lib.ed25519_verify_batch_host.restype = ctypes.c_int
            elif not self.allow_failover:
                raise RuntimeError("Missing libed25519_batch.so for required GPU mode")

        if self.parity_check and self._lib is not None:
            # Perform a quick self-test to ensure the GPU kernel produces correct results.
            try:
                self._run_gpu_parity_check()
            except Exception as e:
                self._disable_gpu("parity_check_failed", exc=e)

    def _disable_gpu(self, reason: str, exc: Exception | None = None) -> None:
        """Disable GPU execution and optionally report via callback/logging."""
        if not self._gpu_ok:
            return
        self._gpu_ok = False
        msg = f"GPU ed25519 disabled (reason={reason}) -> using CPU fallback"
        if exc is not None:
            logger.warning(msg + ": %s", exc)
        else:
            logger.warning(msg)
        if self.on_gpu_disabled:
            try:
                self.on_gpu_disabled("gpu_ed25519_disabled")
            except Exception:
                logger.exception("failed to report GPU disabled metric")

    def _run_gpu_parity_check(self) -> None:
        """Run a short parity check (GPU vs CPU) and disable GPU if mismatch."""
        try:
            import nacl.signing
        except ImportError:
            # Cannot verify GPU correctness without a known-good CPU reference.
            return

        # Deterministic test vector
        seed = b"x3-chain-ed25519-selftest-seed-0000000000"[:32]
        signing_key = nacl.signing.SigningKey(seed)
        message = b"x3-chain-ed25519-selftest".ljust(32, b"\x00")[:32]
        signature = signing_key.sign(message).signature
        pubkey = signing_key.verify_key.encode()

        cpu_ok = self._verify_cpu([signature], [message], [pubkey])[0]
        gpu_ok = True
        try:
            gpu_ok = self._verify_gpu([signature], [message], [pubkey])[0]
        except Exception as e:
            gpu_ok = False
            logger.warning("GPU ed25519 parity check failed: %s", e)

        if not (cpu_ok and gpu_ok):
            self._disable_gpu("parity_check_failed")

    def verify_batch(
        self, signatures: Iterable[bytes], messages: Iterable[bytes], pubkeys: Iterable[bytes]
    ) -> list[bool]:
        """Verify a batch of Ed25519 signatures with GPU preference."""

        # Only use GPU if the self-test passed.
        if self.runtime.available and self._lib is not None and self._gpu_ok:
            try:
                return self._verify_gpu(signatures, messages, pubkeys)
            except Exception as e:
                # GPU failed at runtime; fall back to CPU and mark GPU as disabled.
                self._disable_gpu("runtime_failure", exc=e)
                if self.allow_failover:
                    return self._verify_cpu(signatures, messages, pubkeys)
                raise

        return self._verify_cpu(signatures, messages, pubkeys)

    def _verify_gpu(
        self, signatures: Iterable[bytes], messages: Iterable[bytes], pubkeys: Iterable[bytes]
    ) -> list[bool]:
        signatures_list = list(signatures)
        messages_list = list(messages)
        pubkeys_list = list(pubkeys)

        count = len(signatures_list)
        if count == 0:
            raise ValueError("signatures batch is empty")
        if len(messages_list) != count or len(pubkeys_list) != count:
            raise ValueError("signature, message, and pubkey batch sizes must match")

        entries = self._pack_entries(signatures_list, messages_list, pubkeys_list)

        out = (ctypes.c_ubyte * count)()
        status = self._lib.ed25519_verify_batch_host(
            ctypes.c_char_p(entries),
            ctypes.c_int(count),
            ctypes.byref(out),
        )
        if status != 0:
            raise RuntimeError("GPU ed25519 batch verification failed")

        results = [bool(out[i]) for i in range(count)]

        if self.parity_check:
            cpu_results = self._verify_cpu(signatures_list, messages_list, pubkeys_list)
            if results != cpu_results:
                self._disable_gpu("parity_mismatch")
                raise RuntimeError("GPU ed25519 results diverged from CPU")

        return results

    @staticmethod
    def _pack_entries(
        signatures: Sequence[bytes], messages: Sequence[bytes], pubkeys: Sequence[bytes]
    ) -> bytes:
        """Pack per-signature data into the layout expected by the GPU kernel.

        Layout per entry (128 bytes):
          [sig R (32) | sig S (32) | pubkey (32) | message (32)]
        """
        packed = bytearray()
        for sig, msg, key in zip(signatures, messages, pubkeys):
            if len(sig) != 64:
                raise ValueError("signature must be 64 bytes")
            if len(key) != 32:
                raise ValueError("pubkey must be 32 bytes")
            if len(msg) != 32:
                raise ValueError("message must be 32 bytes")
            packed.extend(sig)
            packed.extend(key)
            packed.extend(msg)
        if not packed:
            raise ValueError("ed25519 batch is empty")
        return bytes(packed)

    @staticmethod
    def _verify_cpu(
        signatures: Iterable[bytes], messages: Iterable[bytes], pubkeys: Iterable[bytes]
    ) -> list[bool]:
        """CPU fallback verification.

        Uses PyNaCl if available (fast). If no CPU verifier is available, assumes all
        signatures are invalid (to avoid silently accepting invalid data).
        """
        try:
            import nacl.signing
            import nacl.exceptions
        except ImportError:
            # No CPU verifier available.
            return [False] * len(list(signatures))

        results: list[bool] = []
        for sig, msg, pk in zip(signatures, messages, pubkeys):
            try:
                verify_key = nacl.signing.VerifyKey(pk)
                verify_key.verify(msg, sig)
                results.append(True)
            except (nacl.exceptions.BadSignatureError, ValueError):
                results.append(False)
        return results
