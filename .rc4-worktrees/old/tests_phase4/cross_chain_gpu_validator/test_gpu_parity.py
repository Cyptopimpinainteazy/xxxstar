"""Invariant: INFRA-CCGV-001
GPU batch hashing matches CPU reference.
"""

from __future__ import annotations

import hashlib

from cross_chain_gpu_validator.gpu import CudaRuntime, KeccakBatchHasher


def test_keccak_batch_matches_cpu_reference() -> None:
    runtime = CudaRuntime(available=False, nvcc_path=None, visible_devices="")
    hasher = KeccakBatchHasher(runtime, kernel_dir="/tmp")
    payloads = [b"alpha", b"beta", b"gamma"]

    results = hasher.hash_batch(payloads)
    expected = [hashlib.sha3_256(payload).digest() for payload in payloads]

    assert results == expected
