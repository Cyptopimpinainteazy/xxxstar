"""State root calculation utilities."""

from __future__ import annotations

from typing import Iterable

from cross_chain_gpu_validator.gpu.keccak_gpu import _keccak256


def merkle_root(leaves: Iterable[bytes]) -> bytes:
    """Compute a keccak256-based merkle root from leaves.

    Uses real Keccak-256 (NOT SHA3-256) for EVM compatibility.
    """

    nodes = [_keccak256(leaf) for leaf in leaves]
    if not nodes:
        return _keccak256(b"")
    while len(nodes) > 1:
        if len(nodes) % 2 == 1:
            nodes.append(nodes[-1])
        next_level: list[bytes] = []
        for i in range(0, len(nodes), 2):
            combined = nodes[i] + nodes[i + 1]
            next_level.append(_keccak256(combined))
        nodes = next_level
    return nodes[0]
