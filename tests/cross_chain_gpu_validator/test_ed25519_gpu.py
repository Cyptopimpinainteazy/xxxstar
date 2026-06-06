"""Validate the GPU Ed25519 batch verification kernel.

This test is intended to run on a machine with CUDA available and the
`libed25519_batch.so` kernel built. It exercises the GPU path and ensures
results match a trusted CPU reference (PyNaCl).

If CUDA isn't available or the kernel isn't built, this test is skipped.
"""

from __future__ import annotations

import os
import sys

# Ensure we import the local source tree rather than an installed package.
ROOT_DIR = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", ".."))
SRC_DIR = os.path.join(ROOT_DIR, "cross-chain-gpu-validator", "src")
if SRC_DIR not in sys.path:
    sys.path.insert(0, SRC_DIR)

import pytest

try:
    import nacl.signing
except ImportError:  # pragma: no cover
    nacl = None  # type: ignore

from cross_chain_gpu_validator.gpu.cuda_loader import CudaRuntime
from cross_chain_gpu_validator.gpu.ed25519_gpu import Ed25519BatchVerifier


def _kernel_dir() -> str:
    # Relative to tests/ directory
    return os.path.normpath(
        os.path.join(os.path.dirname(__file__), "..", "..", "cross-chain-gpu-validator", "kernels")
    )


@pytest.mark.skipif(nacl is None, reason="PyNaCl is required for Ed25519 reference verification")
def test_ed25519_gpu_matches_cpu() -> None:
    runtime = CudaRuntime.detect()
    if not runtime.available:
        pytest.skip("CUDA runtime not available")

    kernel_dir = _kernel_dir()
    lib_path = os.path.join(kernel_dir, "build", "libed25519_batch.so")
    if not os.path.exists(lib_path):
        pytest.skip("GPU ed25519 kernel not built")

    verifier = Ed25519BatchVerifier(
        runtime,
        kernel_dir,
        parity_check=False,
        allow_failover=False,
    )

    signing_key = nacl.signing.SigningKey.generate()
    pubkey = signing_key.verify_key.encode()

    # A handful of random messages ensures we exercise the shader beyond the
    # single deterministic self-test vector.
    messages = [os.urandom(32) for _ in range(32)]
    signatures = [signing_key.sign(m).signature for m in messages]
    results = verifier.verify_batch(signatures, messages, [pubkey] * len(messages))
    assert all(results)

    # A single invalid signature should be rejected.
    bad_sig = bytearray(signatures[0])
    bad_sig[0] ^= 0x01
    bad_results = verifier.verify_batch([bytes(bad_sig)], [messages[0]], [pubkey])
    assert bad_results == [False]
