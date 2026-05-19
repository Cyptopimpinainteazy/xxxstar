"""Multi-GPU batch scheduler for x3-chain validator.

Dynamically assigns chains to GPUs based on TPS demand, VRAM usage,
and kernel profiles.  Supports preemptible swarm tasks on idle cycles.
"""

from __future__ import annotations

import ctypes
import os
import time
import threading
import json
from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional, Callable


class GpuStatus(Enum):
    IDLE = "idle"
    VALIDATING = "validating"
    SWARM = "swarm"
    ERROR = "error"


@dataclass
class GpuDevice:
    device_id: int
    name: str = "GTX 1070"
    vram_total_mb: int = 8192
    vram_used_mb: int = 0
    status: GpuStatus = GpuStatus.IDLE
    assigned_chains: List[str] = field(default_factory=list)
    current_tps: float = 0.0
    utilization_pct: float = 0.0
    last_heartbeat: float = field(default_factory=time.time)

    @property
    def vram_free_mb(self) -> int:
        return self.vram_total_mb - self.vram_used_mb

    @property
    def is_available(self) -> bool:
        return self.status in (GpuStatus.IDLE, GpuStatus.SWARM)


@dataclass
class ChainWorkload:
    chain_id: str
    chain_name: str
    target_tps: float
    kernel_type: str  # "evm" or "svm"
    vram_estimate_mb: int = 256  # per-chain VRAM footprint
    priority: int = 1
    assigned_gpu: Optional[int] = None


@dataclass
class SchedulerMetrics:
    total_gpus: int = 0
    active_gpus: int = 0
    total_tps: float = 0.0
    chains_served: int = 0
    swarm_gpus: int = 0
    gpu_utilization_avg: float = 0.0
    vram_utilization_avg: float = 0.0


class MultiGpuScheduler:
    """Assigns chain validation workloads across GPUs dynamically.

    Scheduling policy:
    1. High-priority chains get dedicated GPUs.
    2. Remaining chains share GPUs, grouped by kernel profile (EVM/SVM).
    3. Leftover GPU capacity is offered to the swarm scheduler.
    4. VRAM limits (8 GB per GTX 1070) are respected.
    """

    def __init__(self, gpu_count: Optional[int] = None):
        self.gpu_count = gpu_count or self._detect_gpu_count()
        self.gpus: Dict[int, GpuDevice] = {
            i: GpuDevice(device_id=i) for i in range(self.gpu_count)
        }
        self.workloads: Dict[str, ChainWorkload] = {}
        self._lock = threading.Lock()
        self._running = False
        self._swarm_callback: Optional[Callable[[int], None]] = None

    @staticmethod
    def _detect_gpu_count() -> int:
        """Detect CUDA device count via runtime or env var."""
        env_count = os.environ.get("CUDA_DEVICE_COUNT")
        if env_count:
            return int(env_count)
        try:
            libcudart = ctypes.CDLL("libcudart.so")
            count = ctypes.c_int(0)
            libcudart.cudaGetDeviceCount(ctypes.byref(count))
            return max(count.value, 1)
        except OSError:
            return 1

    def register_chain(self, workload: ChainWorkload) -> None:
        with self._lock:
            self.workloads[workload.chain_id] = workload

    def unregister_chain(self, chain_id: str) -> None:
        with self._lock:
            self.workloads.pop(chain_id, None)

    def set_swarm_callback(self, callback: Callable[[int], None]) -> None:
        """Register callback invoked with GPU device_id when cycles are free."""
        self._swarm_callback = callback

    def schedule(self) -> Dict[int, List[str]]:
        """Run one scheduling pass. Returns gpu_id → [chain_ids]."""
        with self._lock:
            return self._do_schedule()

    def _do_schedule(self) -> Dict[int, List[str]]:
        # Reset assignments
        for gpu in self.gpus.values():
            gpu.assigned_chains = []
            gpu.vram_used_mb = 0
            gpu.status = GpuStatus.IDLE

        # Sort workloads: high-priority first, then by VRAM descending (best-fit-decreasing)
        sorted_chains = sorted(
            self.workloads.values(),
            key=lambda w: (-w.priority, -w.vram_estimate_mb, -w.target_tps),
        )

        assignments: Dict[int, List[str]] = {i: [] for i in range(self.gpu_count)}

        # Phase 1: Group chains by kernel type for cache/context affinity
        kernel_groups: Dict[str, List[ChainWorkload]] = {}
        for chain in sorted_chains:
            kernel_groups.setdefault(chain.kernel_type, []).append(chain)

        # Phase 2: Assign groups, trying to co-locate same-kernel chains
        for kernel_type, chains in kernel_groups.items():
            for chain in chains:
                best_gpu = self._find_best_gpu_affinity(chain, assignments)
                if best_gpu is not None:
                    self.gpus[best_gpu].assigned_chains.append(chain.chain_id)
                    self.gpus[best_gpu].vram_used_mb += chain.vram_estimate_mb
                    self.gpus[best_gpu].status = GpuStatus.VALIDATING
                    chain.assigned_gpu = best_gpu
                    assignments[best_gpu].append(chain.chain_id)
                else:
                    chain.assigned_gpu = None

        # Offer idle GPUs to swarm
        for gpu in self.gpus.values():
            if gpu.status == GpuStatus.IDLE:
                gpu.status = GpuStatus.SWARM
                if self._swarm_callback:
                    try:
                        self._swarm_callback(gpu.device_id)
                    except Exception:
                        pass

        return assignments

    def _find_best_gpu_affinity(
        self, chain: ChainWorkload, assignments: Dict[int, List[str]]
    ) -> Optional[int]:
        """Find GPU with kernel affinity preference and sufficient VRAM.

        Prefers GPUs already running the same kernel type (avoids context
        switches), then falls back to most-free-VRAM (best-fit-decreasing).
        """
        best_affinity_id: Optional[int] = None
        best_affinity_free = -1
        best_any_id: Optional[int] = None
        best_any_free = -1

        for gpu in self.gpus.values():
            if gpu.vram_free_mb < chain.vram_estimate_mb:
                continue

            free = gpu.vram_free_mb

            # Check if this GPU already has same kernel type
            has_affinity = False
            for existing_cid in gpu.assigned_chains:
                existing = self.workloads.get(existing_cid)
                if existing and existing.kernel_type == chain.kernel_type:
                    has_affinity = True
                    break

            if has_affinity:
                # Among affinity matches, pick the one with least free VRAM
                # (tightest fit = less fragmentation)
                if best_affinity_id is None or free < best_affinity_free:
                    best_affinity_free = free
                    best_affinity_id = gpu.device_id
            else:
                # General fallback: most free VRAM
                if free > best_any_free:
                    best_any_free = free
                    best_any_id = gpu.device_id

        # Prefer affinity match, fall back to any GPU with space
        return best_affinity_id if best_affinity_id is not None else best_any_id

    def get_metrics(self) -> SchedulerMetrics:
        with self._lock:
            active = sum(1 for g in self.gpus.values() if g.status == GpuStatus.VALIDATING)
            swarm = sum(1 for g in self.gpus.values() if g.status == GpuStatus.SWARM)
            total_tps = sum(g.current_tps for g in self.gpus.values())
            util_avg = (
                sum(g.utilization_pct for g in self.gpus.values()) / max(len(self.gpus), 1)
            )
            vram_avg = (
                sum(g.vram_used_mb / g.vram_total_mb * 100 for g in self.gpus.values())
                / max(len(self.gpus), 1)
            )
            return SchedulerMetrics(
                total_gpus=self.gpu_count,
                active_gpus=active,
                total_tps=total_tps,
                chains_served=sum(len(g.assigned_chains) for g in self.gpus.values()),
                swarm_gpus=swarm,
                gpu_utilization_avg=util_avg,
                vram_utilization_avg=vram_avg,
            )

    def update_gpu_stats(self, device_id: int, tps: float, utilization: float, vram_mb: int) -> None:
        with self._lock:
            if device_id in self.gpus:
                gpu = self.gpus[device_id]
                gpu.current_tps = tps
                gpu.utilization_pct = utilization
                gpu.vram_used_mb = vram_mb
                gpu.last_heartbeat = time.time()

    def to_json(self) -> str:
        metrics = self.get_metrics()
        gpus = []
        for gpu in self.gpus.values():
            gpus.append({
                "device_id": gpu.device_id,
                "name": gpu.name,
                "status": gpu.status.value,
                "vram_total_mb": gpu.vram_total_mb,
                "vram_used_mb": gpu.vram_used_mb,
                "tps": gpu.current_tps,
                "utilization_pct": gpu.utilization_pct,
                "chains": gpu.assigned_chains,
            })
        return json.dumps({
            "scheduler": {
                "total_gpus": metrics.total_gpus,
                "active_gpus": metrics.active_gpus,
                "swarm_gpus": metrics.swarm_gpus,
                "total_tps": metrics.total_tps,
                "chains_served": metrics.chains_served,
                "gpu_utilization_avg": metrics.gpu_utilization_avg,
                "vram_utilization_avg": metrics.vram_utilization_avg,
            },
            "gpus": gpus,
        }, indent=2)
