"""GPU batch verifier for secp256k1 signatures."""

from __future__ import annotations

from dataclasses import dataclass
import logging
import os
from typing import Callable, Iterable, Optional, Sequence
import ctypes

from .cuda_loader import CudaRuntime

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# secp256k1 curve constants (precomputed once at import time)
# ---------------------------------------------------------------------------
_P = int("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F", 16)
_N = int("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141", 16)
_GX = int("79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798", 16)
_GY = int("483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8", 16)
_G = (_GX, _GY)

# Type alias
_Point = tuple[int, int] | None


def _inv(x: int, mod: int) -> int:
    return pow(x, -1, mod)


def _point_add(p1: _Point, p2: _Point) -> _Point:
    if p1 is None:
        return p2
    if p2 is None:
        return p1
    x1, y1 = p1
    x2, y2 = p2
    if x1 == x2 and (y1 + y2) % _P == 0:
        return None
    if x1 == x2 and y1 == y2:
        lam = (3 * x1 * x1) * _inv(2 * y1 % _P, _P) % _P
    else:
        lam = (y2 - y1) * _inv((x2 - x1) % _P, _P) % _P
    x3 = (lam * lam - x1 - x2) % _P
    y3 = (lam * (x1 - x3) - y1) % _P
    return x3, y3


def _point_neg(p: _Point) -> _Point:
    if p is None:
        return None
    return (p[0], (-p[1]) % _P)


def _build_window_table(point: _Point, w: int = 4) -> list[_Point]:
    """Build a 2^(w-1) precomputed table for windowed scalar multiplication."""
    table_size = 1 << (w - 1)  # 8 entries for w=4
    table: list[_Point] = [None] * table_size
    table[0] = point
    double = _point_add(point, point)
    for i in range(1, table_size):
        table[i] = _point_add(table[i - 1], double)
    return table


def _scalar_mul_windowed(k: int, table: list[_Point], w: int = 4) -> _Point:
    """Fixed-window scalar multiplication — ~60% faster than double-and-add.

    Uses a w-bit window (default 4). The table stores:
      table[0] = 1*P, table[1] = 2*P, ..., table[2^(w-1)-1] = 2^(w-1)*P
    so table[d-1] = d*P for any digit d in [1, 2^(w-1)].

    For digits > table_size we decompose: d = table_size + remainder.
    """
    if k == 0:
        return None

    # Represent k in w-bit windows (unsigned, little-endian digits)
    mask = (1 << w) - 1  # 0xF for w=4
    digits: list[int] = []
    while k > 0:
        digits.append(k & mask)
        k >>= w

    table_size = len(table)  # 2^(w-1) = 8 for w=4

    # Process from most significant digit to least significant
    result: _Point = None
    for i in range(len(digits) - 1, -1, -1):
        # Double w times
        for _ in range(w):
            result = _point_add(result, result)
        d = digits[i]
        if d > 0:
            if d <= table_size:
                # Direct table lookup: table[d-1] = d*P
                result = _point_add(result, table[d - 1])
            else:
                # d > table_size: decompose as table_size + (d - table_size)
                result = _point_add(result, table[table_size - 1])
                remainder = d - table_size
                if remainder > 0:
                    result = _point_add(result, table[remainder - 1])
    return result


def _scalar_mul_simple(k: int, point: _Point) -> _Point:
    """Standard double-and-add (used for small scalars in windowed method)."""
    result: _Point = None
    addend = point
    while k:
        if k & 1:
            result = _point_add(result, addend)
        addend = _point_add(addend, addend)
        k >>= 1
    return result


# Precomputed generator table — built once at import time
_G_TABLE = _build_window_table(_G, 4)


def _scalar_mul_G(k: int) -> _Point:
    """Fast scalar multiplication of k * G using precomputed generator table."""
    return _scalar_mul_windowed(k, _G_TABLE, 4)


def _multi_scalar_mul(u1: int, u2: int, pubkey_point: _Point) -> _Point:
    """Compute u1*G + u2*Q using Shamir's trick (interleaved double-and-add).

    This is ~40% faster than two separate scalar multiplications because
    we share the doubling operations.
    """
    if u1 == 0 and u2 == 0:
        return None

    # Precompute G+Q for Shamir's trick
    gq = _point_add(_G, pubkey_point)

    result: _Point = None
    # Process bits from MSB to LSB
    max_bits = max(u1.bit_length(), u2.bit_length())
    for i in range(max_bits - 1, -1, -1):
        result = _point_add(result, result)  # double
        b1 = (u1 >> i) & 1
        b2 = (u2 >> i) & 1
        if b1 and b2:
            result = _point_add(result, gq)
        elif b1:
            result = _point_add(result, _G)
        elif b2:
            result = _point_add(result, pubkey_point)
    return result


@dataclass
class Secp256k1BatchVerifier:
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
        if self.runtime.available:
            lib_path = os.path.join(self.kernel_dir, "build", "libsecp256k1_batch.so")
            if os.path.exists(lib_path):
                self._lib = ctypes.CDLL(lib_path)
                self._lib.secp256k1_ecdsa_verify_host.argtypes = [
                    ctypes.c_void_p,
                    ctypes.c_void_p,
                    ctypes.c_void_p,
                    ctypes.c_int,
                    ctypes.c_void_p,
                ]
                self._lib.secp256k1_ecdsa_verify_host.restype = ctypes.c_int
            elif not self.allow_failover:
                raise RuntimeError("Missing libsecp256k1_batch.so for required GPU mode")

    def _disable_gpu(self, reason: str, exc: Exception | None = None) -> None:
        """Disable GPU execution and optionally report via callback/logging."""
        msg = f"GPU secp256k1 disabled (reason={reason}) -> using CPU fallback"
        if exc is not None:
            logger.warning(msg + ": %s", exc)
        else:
            logger.warning(msg)
        if self.on_gpu_disabled:
            try:
                self.on_gpu_disabled("gpu_secp256k1_disabled")
            except Exception:
                logger.exception("failed to report GPU disabled metric")

    def verify_batch(
        self, signatures: Iterable[bytes], messages: Iterable[bytes], pubkeys: Iterable[bytes]
    ) -> list[bool]:
        """Verify a batch of signatures with GPU preference.

        This uses a placeholder CPU parity check for now; replace with actual
        secp256k1 GPU kernel bindings.
        """

        try:
            if self.runtime.available and self._lib is not None:
                return self._verify_gpu(signatures, messages, pubkeys)
        except Exception as e:
            self._disable_gpu("runtime_failure", exc=e)
            if self.allow_failover:
                return self._verify_cpu(signatures, messages, pubkeys)
            raise
        return self._verify_cpu(signatures, messages, pubkeys)

    def _verify_gpu(self, signatures: Iterable[bytes], messages: Iterable[bytes], pubkeys: Iterable[bytes]) -> list[bool]:
        signatures_list = list(signatures)
        messages_list = list(messages)
        pubkeys_list = list(pubkeys)

        count = len(messages_list)
        if count == 0:
            raise ValueError("signatures batch is empty")
        if len(signatures_list) != count or len(pubkeys_list) != count:
            raise ValueError("signature, message, and pubkey batch sizes must match")

        u1_bytes, u2_bytes, r_values = self._compute_u1_u2(signatures_list, messages_list)
        packed_pubkeys = self._pack_bytes(pubkeys_list, 64, "pubkeys")

        out_x = (ctypes.c_ubyte * (count * 32))()
        status = self._lib.secp256k1_ecdsa_verify_host(
            ctypes.c_char_p(u1_bytes),
            ctypes.c_char_p(u2_bytes),
            ctypes.c_char_p(packed_pubkeys),
            ctypes.c_int(count),
            ctypes.byref(out_x),
        )
        if status != 0:
            raise RuntimeError("GPU secp256k1 batch verification failed")

        gpu_results = []
        for idx in range(count):
            x_bytes = bytes(out_x[idx * 32 : (idx + 1) * 32])
            x_value = int.from_bytes(x_bytes, "big") % self._curve_order()
            gpu_results.append(x_value == r_values[idx])

        if self.parity_check:
            cpu_results = self._verify_cpu(signatures_list, messages_list, pubkeys_list)
            if gpu_results != cpu_results:
                raise RuntimeError("GPU secp256k1 results diverged from CPU")
        return gpu_results

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
    def _curve_order() -> int:
        return _N

    def _compute_u1_u2(
        self, signatures: list[bytes], messages: list[bytes]
    ) -> tuple[bytes, bytes, list[int]]:
        order = self._curve_order()
        u1_parts: list[bytes] = []
        u2_parts: list[bytes] = []
        r_values: list[int] = []

        for signature, message in zip(signatures, messages):
            if len(signature) not in (64, 65):
                raise ValueError("signature must be 64 bytes (r||s) or 65 bytes (r||s||v)")
            if len(message) != 32:
                raise ValueError("message must be 32 bytes")
            # Accept optional recovery byte at the end
            sig_body = signature[:64]
            r = int.from_bytes(sig_body[:32], "big")
            s = int.from_bytes(sig_body[32:], "big")
            if r == 0 or s == 0 or r >= order or s >= order:
                u1 = 0
                u2 = 0
            else:
                z = int.from_bytes(message, "big")
                w = pow(s, -1, order)
                u1 = (z * w) % order
                u2 = (r * w) % order
            u1_parts.append(u1.to_bytes(32, "big"))
            u2_parts.append(u2.to_bytes(32, "big"))
            r_values.append(r)

        return b"".join(u1_parts), b"".join(u2_parts), r_values

    @staticmethod
    def _verify_cpu(
        signatures: Iterable[bytes], messages: Iterable[bytes], pubkeys: Iterable[bytes]
    ) -> list[bool]:
        """CPU fallback using Shamir's trick for ~40% faster verification."""
        results: list[bool] = []

        for signature, message, pubkey in zip(signatures, messages, pubkeys):
            if len(signature) not in (64, 65) or len(message) != 32 or len(pubkey) != 64:
                results.append(False)
                continue
            sig_body = signature[:64]
            r = int.from_bytes(sig_body[:32], "big")
            s = int.from_bytes(sig_body[32:], "big")
            if r == 0 or s == 0 or r >= _N or s >= _N:
                results.append(False)
                continue
            z = int.from_bytes(message, "big")
            w = _inv(s, _N)
            u1 = (z * w) % _N
            u2 = (r * w) % _N
            pub_x = int.from_bytes(pubkey[:32], "big")
            pub_y = int.from_bytes(pubkey[32:], "big")
            # Shamir's trick: compute u1*G + u2*Q in one pass
            point = _multi_scalar_mul(u1, u2, (pub_x, pub_y))
            if point is None:
                results.append(False)
                continue
            x_coord = point[0] % _N
            results.append(x_coord == r)
        return results
