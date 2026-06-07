"""In-memory metrics store for the validator service."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import threading
import time
from typing import Any


@dataclass
class MetricsSnapshot:
    """Snapshot of current metrics values."""

    timestamp: float
    svm_tps: float
    evm_tps: float
    atomic_success_rate: float
    atomic_rollbacks: int
    pending_swaps: int
    gpu_health: str
    svm_rpc_latency_ms: float
    evm_rpc_latency_ms: float


class MetricsStore:
    """Thread-safe metrics aggregator."""

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._svm_tps = 0.0
        self._evm_tps = 0.0
        self._atomic_success_rate = 1.0
        self._atomic_rollbacks = 0
        self._pending_swaps = 0
        self._gpu_health = "unknown"
        self._svm_rpc_latency_ms = 0.0
        self._evm_rpc_latency_ms = 0.0

    def update_throughput(self, svm_tps: float, evm_tps: float) -> None:
        with self._lock:
            self._svm_tps = svm_tps
            self._evm_tps = evm_tps

    def update_atomic(self, success_rate: float, rollbacks: int) -> None:
        with self._lock:
            self._atomic_success_rate = success_rate
            self._atomic_rollbacks = rollbacks

    def update_pending(self, pending: int) -> None:
        with self._lock:
            self._pending_swaps = pending

    def update_gpu_health(self, status: str) -> None:
        with self._lock:
            self._gpu_health = status

    def update_rpc_latency(self, svm_ms: float, evm_ms: float) -> None:
        with self._lock:
            self._svm_rpc_latency_ms = svm_ms
            self._evm_rpc_latency_ms = evm_ms

    def snapshot(self) -> MetricsSnapshot:
        with self._lock:
            return MetricsSnapshot(
                timestamp=time.time(),
                svm_tps=self._svm_tps,
                evm_tps=self._evm_tps,
                atomic_success_rate=self._atomic_success_rate,
                atomic_rollbacks=self._atomic_rollbacks,
                pending_swaps=self._pending_swaps,
                gpu_health=self._gpu_health,
                svm_rpc_latency_ms=self._svm_rpc_latency_ms,
                evm_rpc_latency_ms=self._evm_rpc_latency_ms,
            )

    def snapshot_dict(self) -> dict[str, Any]:
        return asdict(self.snapshot())
