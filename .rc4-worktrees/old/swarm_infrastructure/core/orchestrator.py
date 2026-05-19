"""Minimal orchestrator & job distribution classes for local testing."""

from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple
import time
import uuid


@dataclass
class GPUCapabilities:
    vendor: str
    device_name: str
    vram_mb: int
    cuda: bool = False
    compute_score: float = 0.0


@dataclass
class Contributor:
    contributor_id: str
    wallet: Optional[str]
    capabilities: GPUCapabilities
    online: bool = True
    utilization: float = 0.0
    temperature_c: float = 0.0
    tasks_completed: int = 0
    tasks_failed: int = 0
    active_task_id: Optional[str] = None
    last_heartbeat_at: float = field(default_factory=time.time)


@dataclass
class Task:
    task_id: str
    workload_type: str
    payload: dict
    required_vram_mb: int = 0
    min_compute_score: float = 0.0
    max_runtime_s: Optional[int] = None
    status: str = "queued"
    created_at: float = field(default_factory=time.time)
    assigned_to: Optional[str] = None
    assigned_at: Optional[float] = None
    started_at: Optional[float] = None
    finished_at: Optional[float] = None
    result: Optional[dict] = None
    error: Optional[str] = None
    priority: str = "normal"


@dataclass
class RequestResult:
    task: Optional[Task]
    reason: Optional[str] = None


class GPUOrchestrator:
    def __init__(self, gpu_manager):
        self.gpu_manager = gpu_manager

    def register_contributor(self, contributor_id: str, wallet: Optional[str], capabilities: GPUCapabilities):
        self.gpu_manager.register(contributor_id, wallet, capabilities)

    def heartbeat(self, contributor_id: str, utilization=None, temperature_c=None, power_w=None, uptime_s=None):
        self.gpu_manager.heartbeat(contributor_id, utilization or 0.0, temperature_c or 0.0)

    def enqueue_task(self, workload_type: str, payload: dict, required_vram_mb: int = 0, min_compute_score: float = 0.0, max_runtime_s: Optional[int] = None, priority=None) -> str:
        normalized_priority = str(priority).lower() if priority is not None else "normal"
        return self.gpu_manager.enqueue_task(
            workload_type,
            payload,
            required_vram_mb,
            min_compute_score,
            max_runtime_s,
            normalized_priority,
        )

    def request_task(self, contributor_id: str) -> RequestResult:
        return self.gpu_manager.assign_task_to(contributor_id)

    def submit_result(self, contributor_id: str, task_id: str, success: bool, result=None, error=None) -> bool:
        return self.gpu_manager.submit_result(contributor_id, task_id, success, result, error)

    def cancel_task(self, task_id: str) -> bool:
        return self.gpu_manager.cancel_task(task_id)

    def snapshot(self) -> dict:
        return self.gpu_manager.snapshot()


class AgentJobDistributionManager:
    def __init__(self, total_gpus: int, gpu_orchestrator: GPUOrchestrator):
        self.total_gpus = total_gpus
        self.orch = gpu_orchestrator
        self._distribution_targets: Dict[str, float] = {}

    def update_distribution_targets(self, new_dist: dict):
        if not isinstance(new_dist, dict):
            raise ValueError("distribution targets must be a dictionary")

        cleaned: Dict[str, float] = {}
        total = 0.0
        for workload_type, share in new_dist.items():
            if share is None:
                continue
            value = float(share)
            if value < 0:
                raise ValueError("distribution targets must be non-negative")
            cleaned[str(workload_type)] = value
            total += value

        if total > 1.0 and total != 0.0:
            cleaned = {
                workload_type: value / total
                for workload_type, value in cleaned.items()
            }

        self._distribution_targets = cleaned
        return True

    def get_distribution_stats(self) -> dict:
        tasks = self.orch.gpu_manager.list_tasks(limit=10_000)
        queued_by_type: Dict[str, int] = {}
        assigned_by_type: Dict[str, int] = {}

        for task in tasks:
            workload_type = str(task.workload_type)
            if task.status == "queued":
                queued_by_type[workload_type] = queued_by_type.get(workload_type, 0) + 1
            elif task.status == "assigned":
                assigned_by_type[workload_type] = assigned_by_type.get(workload_type, 0) + 1

        return {
            "total_gpus": self.total_gpus,
            "queue_depth": self.orch.gpu_manager.queue_depth(),
            "targets": dict(self._distribution_targets),
            "queued_by_type": queued_by_type,
            "assigned_by_type": assigned_by_type,
            "contributors_online": sum(
                1
                for contributor in self.orch.gpu_manager.list_contributors()
                if contributor.online
            ),
        }

    def reallocate_based_on_performance(self) -> dict:
        stats = self.get_distribution_stats()
        queued_by_type = stats["queued_by_type"]
        total_queued = sum(queued_by_type.values())

        if total_queued == 0:
            return {
                "success": True,
                "changed": False,
                "reason": "no queued work to rebalance",
                "distribution": dict(self._distribution_targets),
            }

        recommended = {
            workload_type: count / total_queued
            for workload_type, count in queued_by_type.items()
        }
        changed = recommended != self._distribution_targets
        self._distribution_targets = recommended

        return {
            "success": True,
            "changed": changed,
            "reason": "rebalanced from current queued workload mix",
            "distribution": recommended,
        }
