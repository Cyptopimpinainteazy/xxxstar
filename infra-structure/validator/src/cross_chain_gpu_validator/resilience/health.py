"""GPU + Node Health Scoring Engine.

Polls GPU state, RPC connectivity, consensus participation, and cross-chain
sync to compute a composite health score every ``interval`` seconds.

Score < threshold triggers lane failover via the LaneOrchestrator.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import threading
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Callable


# ─── Health Score ────────────────────────────────────────────


class HealthComponent(Enum):
    GPU_AVAILABLE = "gpu_available"
    GPU_TEMPERATURE = "gpu_temperature"
    GPU_MEMORY = "gpu_memory"
    GPU_UTILIZATION = "gpu_utilization"
    GPU_KERNEL_LATENCY = "gpu_kernel_latency"
    BLOCK_PROGRESS = "block_progress"
    PEER_COUNT = "peer_count"
    CONSENSUS_VOTES = "consensus_votes"
    RPC_LATENCY = "rpc_latency"
    CROSS_CHAIN_SYNC = "cross_chain_sync"


@dataclass
class HealthScore:
    """Composite health score (0.0–1.0) with per-component breakdown."""

    overall: float = 1.0
    components: dict[str, float] = field(default_factory=dict)
    timestamp: float = 0.0
    degraded: bool = False

    @property
    def healthy(self) -> bool:
        return self.overall >= 0.7 and not self.degraded

    @property
    def critical(self) -> bool:
        return self.overall < 0.3

    def to_dict(self) -> dict:
        return {
            "overall": round(self.overall, 4),
            "healthy": self.healthy,
            "critical": self.critical,
            "degraded": self.degraded,
            "components": {k: round(v, 4) for k, v in self.components.items()},
            "timestamp": self.timestamp,
        }


# ─── GPU Probe ───────────────────────────────────────────────


@dataclass
class GpuStats:
    """Raw GPU statistics from nvidia-smi."""

    available: bool = False
    temperature_c: int = 0
    memory_used_mb: int = 0
    memory_total_mb: int = 0
    utilization_pct: int = 0
    power_draw_w: float = 0.0
    device_name: str = ""
    driver_version: str = ""
    error: str | None = None

    @property
    def memory_pct(self) -> float:
        if self.memory_total_mb == 0:
            return 0.0
        return (self.memory_used_mb / self.memory_total_mb) * 100.0


def probe_gpu() -> GpuStats:
    """Query nvidia-smi for GPU health.  Returns GpuStats with available=False
    on any error — never raises."""
    nvidia_smi = shutil.which("nvidia-smi")
    if nvidia_smi is None:
        return GpuStats(available=False, error="nvidia-smi not found")

    try:
        result = subprocess.run(
            [
                nvidia_smi,
                "--query-gpu=temperature.gpu,memory.used,memory.total,"
                "utilization.gpu,power.draw,name,driver_version",
                "--format=csv,noheader,nounits",
            ],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode != 0:
            return GpuStats(available=False, error=result.stderr.strip())

        parts = [p.strip() for p in result.stdout.strip().split(",")]
        if len(parts) < 7:
            return GpuStats(available=False, error="unexpected nvidia-smi output")

        return GpuStats(
            available=True,
            temperature_c=int(parts[0]),
            memory_used_mb=int(parts[1]),
            memory_total_mb=int(parts[2]),
            utilization_pct=int(parts[3]),
            power_draw_w=float(parts[4]),
            device_name=parts[5],
            driver_version=parts[6],
        )
    except (subprocess.TimeoutExpired, FileNotFoundError, ValueError) as exc:
        return GpuStats(available=False, error=str(exc))


# ─── Node Health ─────────────────────────────────────────────


@dataclass
class NodeHealth:
    """Aggregated health state for the local node."""

    gpu: GpuStats = field(default_factory=GpuStats)
    score: HealthScore = field(default_factory=HealthScore)
    block_height: int = 0
    peer_count: int = 0
    rpc_latency_ms: float = 0.0
    consensus_votes_recent: int = 0
    cross_chain_lag_blocks: int = 0
    uptime_seconds: float = 0.0
    last_check: float = 0.0

    def to_dict(self) -> dict:
        return {
            "gpu_available": self.gpu.available,
            "gpu_temp_c": self.gpu.temperature_c,
            "gpu_memory_pct": round(self.gpu.memory_pct, 1),
            "gpu_utilization_pct": self.gpu.utilization_pct,
            "score": self.score.to_dict(),
            "block_height": self.block_height,
            "peer_count": self.peer_count,
            "rpc_latency_ms": round(self.rpc_latency_ms, 2),
            "consensus_votes_recent": self.consensus_votes_recent,
            "cross_chain_lag_blocks": self.cross_chain_lag_blocks,
            "uptime_seconds": round(self.uptime_seconds, 1),
        }


# ─── Weights ─────────────────────────────────────────────────

# Default scoring weights — component importance for overall score
DEFAULT_WEIGHTS: dict[str, float] = {
    HealthComponent.GPU_AVAILABLE.value: 0.20,
    HealthComponent.GPU_TEMPERATURE.value: 0.05,
    HealthComponent.GPU_MEMORY.value: 0.10,
    HealthComponent.GPU_UTILIZATION.value: 0.05,
    HealthComponent.GPU_KERNEL_LATENCY.value: 0.10,
    HealthComponent.BLOCK_PROGRESS.value: 0.15,
    HealthComponent.PEER_COUNT.value: 0.10,
    HealthComponent.CONSENSUS_VOTES.value: 0.10,
    HealthComponent.RPC_LATENCY.value: 0.05,
    HealthComponent.CROSS_CHAIN_SYNC.value: 0.10,
}


def _score_gpu_temp(temp_c: int) -> float:
    """Score GPU temperature: 1.0 = cool, 0.0 = thermal throttle."""
    if temp_c <= 60:
        return 1.0
    if temp_c >= 90:
        return 0.0
    return 1.0 - (temp_c - 60) / 30.0


def _score_gpu_memory(pct: float) -> float:
    if pct <= 80:
        return 1.0
    if pct >= 98:
        return 0.0
    return 1.0 - (pct - 80) / 18.0


def _score_rpc_latency(ms: float) -> float:
    if ms <= 20:
        return 1.0
    if ms >= 500:
        return 0.0
    return 1.0 - (ms - 20) / 480.0


def _score_peer_count(peers: int) -> float:
    if peers >= 10:
        return 1.0
    if peers <= 0:
        return 0.0
    return peers / 10.0


# ─── Health Daemon ───────────────────────────────────────────


class GpuHealthDaemon:
    """Background daemon that continuously monitors GPU + node health.

    Call ``start()`` to spawn a background thread.  The daemon updates
    ``self.health`` every ``interval`` seconds and invokes ``on_critical``
    when the score drops below ``threshold``.

    Parameters
    ----------
    interval : float
        Seconds between health probes (default 5).
    threshold : float
        Score below which ``on_critical`` fires (default 0.5).
    on_critical : callable
        ``fn(NodeHealth)`` invoked when score < threshold.
    on_recovery : callable
        ``fn(NodeHealth)`` invoked when score returns above threshold.
    weights : dict
        Per-component scoring weights (see ``DEFAULT_WEIGHTS``).
    """

    def __init__(
        self,
        interval: float = 5.0,
        threshold: float = 0.5,
        on_critical: Callable[[NodeHealth], None] | None = None,
        on_recovery: Callable[[NodeHealth], None] | None = None,
        weights: dict[str, float] | None = None,
    ) -> None:
        self._interval = max(interval, 0.5)
        self._threshold = threshold
        self._on_critical = on_critical
        self._on_recovery = on_recovery
        self._weights = weights or dict(DEFAULT_WEIGHTS)
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None
        self._start_time = time.monotonic()

        # External hooks — set by LaneOrchestrator
        self._block_height_fn: Callable[[], int] | None = None
        self._peer_count_fn: Callable[[], int] | None = None
        self._rpc_latency_fn: Callable[[], float] | None = None
        self._consensus_votes_fn: Callable[[], int] | None = None
        self._cross_chain_lag_fn: Callable[[], int] | None = None
        self._kernel_latency_fn: Callable[[], float] | None = None

        self.health = NodeHealth()
        self._was_critical = False
        self._lock = threading.Lock()

    # ── External data hooks ──────────────────────────────────

    def set_block_height_fn(self, fn: Callable[[], int]) -> None:
        self._block_height_fn = fn

    def set_peer_count_fn(self, fn: Callable[[], int]) -> None:
        self._peer_count_fn = fn

    def set_rpc_latency_fn(self, fn: Callable[[], float]) -> None:
        self._rpc_latency_fn = fn

    def set_consensus_votes_fn(self, fn: Callable[[], int]) -> None:
        self._consensus_votes_fn = fn

    def set_cross_chain_lag_fn(self, fn: Callable[[], int]) -> None:
        self._cross_chain_lag_fn = fn

    def set_kernel_latency_fn(self, fn: Callable[[], float]) -> None:
        self._kernel_latency_fn = fn

    # ── Lifecycle ────────────────────────────────────────────

    def start(self) -> None:
        if self._thread is not None and self._thread.is_alive():
            return
        self._stop.clear()
        self._start_time = time.monotonic()
        self._thread = threading.Thread(
            target=self._run, daemon=True, name="gpu-health-daemon"
        )
        self._thread.start()

    def stop(self) -> None:
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout=self._interval + 1)
            self._thread = None

    @property
    def running(self) -> bool:
        return self._thread is not None and self._thread.is_alive()

    # ── Core loop ────────────────────────────────────────────

    def _run(self) -> None:
        while not self._stop.is_set():
            try:
                self._probe()
            except Exception:
                # Daemon must never crash
                pass
            self._stop.wait(self._interval)

    def _probe(self) -> None:
        """Run a single health probe cycle."""
        now = time.time()
        gpu = probe_gpu()

        components: dict[str, float] = {}

        # GPU components
        components[HealthComponent.GPU_AVAILABLE.value] = 1.0 if gpu.available else 0.0
        components[HealthComponent.GPU_TEMPERATURE.value] = (
            _score_gpu_temp(gpu.temperature_c) if gpu.available else 0.0
        )
        components[HealthComponent.GPU_MEMORY.value] = (
            _score_gpu_memory(gpu.memory_pct) if gpu.available else 0.0
        )
        components[HealthComponent.GPU_UTILIZATION.value] = (
            1.0 if gpu.available else 0.0
        )

        # Kernel latency
        kernel_lat = 0.0
        if self._kernel_latency_fn:
            try:
                kernel_lat = self._kernel_latency_fn()
            except Exception:
                pass
        components[HealthComponent.GPU_KERNEL_LATENCY.value] = (
            max(0.0, 1.0 - kernel_lat / 100.0)  # score 0 at 100ms+
        )

        # Block progress
        block_h = 0
        if self._block_height_fn:
            try:
                block_h = self._block_height_fn()
            except Exception:
                pass
        components[HealthComponent.BLOCK_PROGRESS.value] = 1.0 if block_h > 0 else 0.0

        # Peers
        peers = 0
        if self._peer_count_fn:
            try:
                peers = self._peer_count_fn()
            except Exception:
                pass
        components[HealthComponent.PEER_COUNT.value] = _score_peer_count(peers)

        # Consensus votes
        votes = 0
        if self._consensus_votes_fn:
            try:
                votes = self._consensus_votes_fn()
            except Exception:
                pass
        components[HealthComponent.CONSENSUS_VOTES.value] = min(votes / 10.0, 1.0)

        # RPC latency
        rpc_lat = 0.0
        if self._rpc_latency_fn:
            try:
                rpc_lat = self._rpc_latency_fn()
            except Exception:
                pass
        components[HealthComponent.RPC_LATENCY.value] = _score_rpc_latency(rpc_lat)

        # Cross-chain sync
        cc_lag = 0
        if self._cross_chain_lag_fn:
            try:
                cc_lag = self._cross_chain_lag_fn()
            except Exception:
                pass
        components[HealthComponent.CROSS_CHAIN_SYNC.value] = (
            max(0.0, 1.0 - cc_lag / 50.0)
        )

        # Weighted overall
        overall = 0.0
        for comp, value in components.items():
            overall += value * self._weights.get(comp, 0.0)
        overall = min(max(overall, 0.0), 1.0)

        degraded = not gpu.available
        score = HealthScore(
            overall=overall,
            components=components,
            timestamp=now,
            degraded=degraded,
        )

        with self._lock:
            self.health = NodeHealth(
                gpu=gpu,
                score=score,
                block_height=block_h,
                peer_count=peers,
                rpc_latency_ms=rpc_lat,
                consensus_votes_recent=votes,
                cross_chain_lag_blocks=cc_lag,
                uptime_seconds=time.monotonic() - self._start_time,
                last_check=now,
            )

        # Critical / recovery callbacks
        is_critical = overall < self._threshold
        if is_critical and not self._was_critical:
            self._was_critical = True
            if self._on_critical:
                try:
                    self._on_critical(self.health)
                except Exception:
                    pass
        elif not is_critical and self._was_critical:
            self._was_critical = False
            if self._on_recovery:
                try:
                    self._on_recovery(self.health)
                except Exception:
                    pass

    def force_check(self) -> NodeHealth:
        """Run a probe immediately and return the result."""
        self._probe()
        with self._lock:
            return self.health
