"""GPU batch verifier for Ed25519 signatures.

This is used by SVM (Solana) and Substrate (Polkadot/Kusama) validators.

The GPU kernel is implemented in crates/gpu-swarm/src/cu_kernels/ed25519_batch.cu
and is built into libed25519_batch.so.
"""

from __future__ import annotations

import ctypes
import logging
import os
from dataclasses import dataclass
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
        elif not self.allow_failover:
            raise RuntimeError("CUDA runtime unavailable for required GPU mode")

        if self.parity_check and self._lib is not None:
            try:
                self._run_gpu_parity_check()
            except Exception as exc:
                self._disable_gpu("parity_check_failed", exc=exc)

    def _disable_gpu(self, reason: str, exc: Exception | None = None) -> None:
        """Disable GPU execution and optionally report through the callback."""
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
        if not self.allow_failover:
            raise RuntimeError(msg) from exc

    def _run_gpu_parity_check(self) -> None:
        """Run a short parity check and disable GPU execution on mismatch."""
        from cryptography.hazmat.primitives.asymmetric.ed25519 import (
            Ed25519PrivateKey,
        )

        seed = b"x3-chain-ed25519-selftest-seed-0000000000"[:32]
        message = b"x3-chain-ed25519-selftest".ljust(32, b"\x00")[:32]
        # A fixed, public test vector, not a secret.
        selftest_signer = Ed25519PrivateKey.from_private_bytes(seed)
        signature = selftest_signer.sign(message)
        pubkey = selftest_signer.public_key().public_bytes_raw()

        cpu_ok = self._verify_cpu([signature], [message], [pubkey])[0]
        try:
            gpu_ok = self._verify_gpu([signature], [message], [pubkey])[0]
        except Exception as exc:
            gpu_ok = False
            logger.warning("GPU ed25519 parity check failed: %s", exc)

        if not (cpu_ok and gpu_ok):
            raise RuntimeError("GPU ed25519 self-test did not match the CPU reference")

    def verify_batch(
        self,
        signatures: Iterable[bytes],
        messages: Iterable[bytes],
        pubkeys: Iterable[bytes],
    ) -> list[bool]:
        """Verify a batch of Ed25519 signatures with GPU preference."""
        signatures = list(signatures)
        messages = list(messages)
        pubkeys = list(pubkeys)

        if not (len(signatures) == len(messages) == len(pubkeys)):
            raise ValueError("signature, message, and pubkey batch sizes must match")
        if not signatures:
            return []

        # The CUDA kernel accepts one 32-byte message per signature. Protocols
        # such as Solana sign the raw transaction message, which is variable
        # length, so those batches use the CPU verifier rather than changing
        # the signed preimage.
        if any(len(message) != 32 for message in messages):
            if not self.allow_failover:
                raise RuntimeError("GPU Ed25519 verification requires 32-byte messages")
            return self._verify_cpu(signatures, messages, pubkeys)

        if self.runtime.available and self._lib is not None and self._gpu_ok:
            try:
                return self._verify_gpu(signatures, messages, pubkeys)
            except Exception as exc:
                self._disable_gpu("runtime_failure", exc=exc)
                if self.allow_failover:
                    return self._verify_cpu(signatures, messages, pubkeys)
                raise

        if not self.allow_failover:
            raise RuntimeError("GPU execution unavailable and CPU failover is disabled")
        return self._verify_cpu(signatures, messages, pubkeys)

    def _verify_gpu(
        self,
        signatures: Iterable[bytes],
        messages: Iterable[bytes],
        pubkeys: Iterable[bytes],
    ) -> list[bool]:
        lib = self._lib
        if lib is None:
            raise RuntimeError("GPU Ed25519 library is not loaded")
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
        status = lib.ed25519_verify_batch_host(
            ctypes.c_char_p(entries),
            ctypes.c_int(count),
            ctypes.byref(out),
        )
        if status != 0:
            raise RuntimeError("GPU ed25519 batch verification failed")

        results = [bool(out[index]) for index in range(count)]
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
        """Pack values into the kernel's 128-byte per-entry layout."""
        packed = bytearray()
        for signature, message, pubkey in zip(signatures, messages, pubkeys):
            if len(signature) != 64:
                raise ValueError("signature must be 64 bytes")
            if len(pubkey) != 32:
                raise ValueError("pubkey must be 32 bytes")
            if len(message) != 32:
                raise ValueError("message must be 32 bytes")
            packed.extend(signature)
            packed.extend(pubkey)
            packed.extend(message)
        if not packed:
            raise ValueError("ed25519 batch is empty")
        return bytes(packed)

    @staticmethod
    def _verify_cpu(
        signatures: Iterable[bytes], messages: Iterable[bytes], pubkeys: Iterable[bytes]
    ) -> list[bool]:
        """Verify with the required cryptography dependency."""
        signature_list = list(signatures)
        message_list = list(messages)
        pubkey_list = list(pubkeys)
        if not (len(signature_list) == len(message_list) == len(pubkey_list)):
            raise ValueError("signature, message, and pubkey batch sizes must match")

        try:
            from cryptography.exceptions import InvalidSignature
            from cryptography.hazmat.primitives.asymmetric.ed25519 import (
                Ed25519PublicKey,
            )
        except ImportError as exc:
            # cryptography is a declared dependency. Without it every signature
            # would read as invalid, which looks like an attack, not a broken
            # install; fail loudly instead.
            raise RuntimeError(
                "CPU signature verification needs the 'cryptography' package"
            ) from exc

        results: list[bool] = []
        for signature, message, pubkey in zip(
            signature_list, message_list, pubkey_list
        ):
            if len(signature) != 64 or len(pubkey) != 32:
                results.append(False)
                continue
            try:
                Ed25519PublicKey.from_public_bytes(pubkey).verify(signature, message)
                results.append(True)
            except (InvalidSignature, ValueError):
                results.append(False)
        return results
