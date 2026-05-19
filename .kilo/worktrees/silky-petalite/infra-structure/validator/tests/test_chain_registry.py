"""Tests for chain registry and default chain loading."""
from __future__ import annotations

from cross_chain_gpu_validator.chain_registry import load_default_chain_configs


def test_default_chain_count_at_least_102() -> None:
    configs = load_default_chain_configs()
    assert len(configs) >= 102, f"expected >=102 chains, got {len(configs)}"


def test_each_config_has_rpc_and_algorithms() -> None:
    configs = load_default_chain_configs()
    for cid, cfg in configs.items():
        assert cfg.rpc_url, f"chain {cid} missing rpc_url"
        assert cfg.sig_algorithm is not None
        assert cfg.hash_algorithm is not None
