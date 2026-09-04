"""Invariant: INFRA-CCGV-004
Metrics snapshot includes required fields.
"""

from __future__ import annotations

from cross_chain_gpu_validator.metrics import MetricsStore


def test_metrics_snapshot_fields() -> None:
    metrics = MetricsStore()
    snapshot = metrics.snapshot_dict()

    for key in [
        "svm_tps",
        "evm_tps",
        "atomic_success_rate",
        "atomic_rollbacks",
        "pending_swaps",
        "gpu_health",
        "svm_rpc_latency_ms",
        "evm_rpc_latency_ms",
    ]:
        assert key in snapshot
