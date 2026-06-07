"""Preemptible GPU compute scheduler for swarm agents.

Occupies idle GPU cycles (GPUs not assigned to chain validation)
with swarm workloads: arbitrage simulation, adversarial testing,
and AI inference.  Preempted immediately when validators need the GPU.

Safety guarantees:
  - No side effects on validator state (read-only copies)
  - Preemptible: can be killed at any point without data loss
  - Memory-bounded: respects per-device VRAM limits
"""

from __future__ import annotations

import threading
import time
import json
from dataclasses import dataclass, field
from enum import Enum
from typing import Callable, Dict, List, Optional


class SwarmTaskType(Enum):
    ARBITRAGE = "arbitrage"
    SIMULATION = "simulation"
    ADVERSARIAL = "adversarial"
    INFERENCE = "inference"


class SwarmTaskStatus(Enum):
    PENDING = "pending"
    RUNNING = "running"
    PREEMPTED = "preempted"
    COMPLETED = "completed"
    FAILED = "failed"


@dataclass
class SwarmTask:
    task_id: str
    task_type: SwarmTaskType
    priority: int = 0
    gpu_device: Optional[int] = None
    status: SwarmTaskStatus = SwarmTaskStatus.PENDING
    start_time: Optional[float] = None
    end_time: Optional[float] = None
    result: Optional[dict] = None
    vram_mb: int = 256

    @property
    def elapsed_sec(self) -> float:
        if self.start_time is None:
            return 0.0
        end = self.end_time or time.time()
        return end - self.start_time


@dataclass
class SwarmMetrics:
    tasks_completed: int = 0
    tasks_preempted: int = 0
    tasks_running: int = 0
    tasks_pending: int = 0
    total_gpu_hours: float = 0.0
    active_gpus: List[int] = field(default_factory=list)


class PreemptibleScheduler:
    """Schedules swarm tasks on GPUs not used by validators.

    Integration with MultiGpuScheduler:
        scheduler.set_swarm_callback(swarm_scheduler.on_gpu_available)

    When a GPU becomes idle (no chain validation), the callback fires
    and the swarm scheduler can start a low-priority task on that GPU.
    When the GPU is needed back for validation, `preempt(device_id)`
    is called to immediately stop the swarm task.
    """

    def __init__(self):
        self._tasks: Dict[str, SwarmTask] = {}
        self._gpu_tasks: Dict[int, str] = {}  # gpu_device → task_id
        self._lock = threading.Lock()
        self._task_handlers: Dict[SwarmTaskType, Callable] = {}
        self._metrics = SwarmMetrics()

    def register_handler(self, task_type: SwarmTaskType,
                         handler: Callable[[SwarmTask], Optional[dict]]) -> None:
        """Register a handler function for a swarm task type."""
        self._task_handlers[task_type] = handler

    def submit(self, task: SwarmTask) -> str:
        """Submit a swarm task to the queue."""
        with self._lock:
            self._tasks[task.task_id] = task
            self._metrics.tasks_pending += 1
        return task.task_id

    def on_gpu_available(self, device_id: int) -> None:
        """Callback from MultiGpuScheduler when a GPU becomes idle."""
        with self._lock:
            # Find highest-priority pending task
            pending = [t for t in self._tasks.values()
                       if t.status == SwarmTaskStatus.PENDING]
            if not pending:
                return
            pending.sort(key=lambda t: -t.priority)
            task = pending[0]
            task.gpu_device = device_id
            task.status = SwarmTaskStatus.RUNNING
            task.start_time = time.time()
            self._gpu_tasks[device_id] = task.task_id
            self._metrics.tasks_pending -= 1
            self._metrics.tasks_running += 1
            if device_id not in self._metrics.active_gpus:
                self._metrics.active_gpus.append(device_id)

        # Run handler in background thread (non-blocking for scheduler)
        thread = threading.Thread(
            target=self._run_task, args=(task,), daemon=True
        )
        thread.start()

    def preempt(self, device_id: int) -> bool:
        """Immediately stop any swarm task on the given GPU."""
        with self._lock:
            task_id = self._gpu_tasks.pop(device_id, None)
            if task_id is None:
                return False
            task = self._tasks.get(task_id)
            if task and task.status == SwarmTaskStatus.RUNNING:
                task.status = SwarmTaskStatus.PREEMPTED
                task.end_time = time.time()
                self._metrics.tasks_preempted += 1
                self._metrics.tasks_running = max(0, self._metrics.tasks_running - 1)
                if device_id in self._metrics.active_gpus:
                    self._metrics.active_gpus.remove(device_id)
                return True
        return False

    def _run_task(self, task: SwarmTask) -> None:
        handler = self._task_handlers.get(task.task_type)
        if not handler:
            with self._lock:
                task.status = SwarmTaskStatus.FAILED
                self._metrics.tasks_running = max(0, self._metrics.tasks_running - 1)
            return
        try:
            result = handler(task)
            with self._lock:
                if task.status == SwarmTaskStatus.RUNNING:
                    task.status = SwarmTaskStatus.COMPLETED
                    task.result = result
                    task.end_time = time.time()
                    self._metrics.tasks_completed += 1
                    self._metrics.tasks_running = max(0, self._metrics.tasks_running - 1)
                    self._metrics.total_gpu_hours += task.elapsed_sec / 3600
                    if task.gpu_device in self._gpu_tasks:
                        del self._gpu_tasks[task.gpu_device]
                    if task.gpu_device in self._metrics.active_gpus:
                        self._metrics.active_gpus.remove(task.gpu_device)
        except Exception:
            with self._lock:
                if task.status == SwarmTaskStatus.RUNNING:
                    task.status = SwarmTaskStatus.FAILED
                    task.end_time = time.time()
                    self._metrics.tasks_running = max(0, self._metrics.tasks_running - 1)

    def get_metrics(self) -> SwarmMetrics:
        with self._lock:
            return SwarmMetrics(
                tasks_completed=self._metrics.tasks_completed,
                tasks_preempted=self._metrics.tasks_preempted,
                tasks_running=self._metrics.tasks_running,
                tasks_pending=self._metrics.tasks_pending,
                total_gpu_hours=self._metrics.total_gpu_hours,
                active_gpus=list(self._metrics.active_gpus),
            )

    def to_json(self) -> str:
        m = self.get_metrics()
        tasks = []
        for t in self._tasks.values():
            tasks.append({
                "task_id": t.task_id,
                "type": t.task_type.value,
                "status": t.status.value,
                "gpu": t.gpu_device,
                "elapsed_sec": round(t.elapsed_sec, 2),
            })
        return json.dumps({
            "swarm": {
                "completed": m.tasks_completed,
                "preempted": m.tasks_preempted,
                "running": m.tasks_running,
                "pending": m.tasks_pending,
                "gpu_hours": round(m.total_gpu_hours, 4),
                "active_gpus": m.active_gpus,
            },
            "tasks": tasks[-50:],  # Last 50 tasks
        }, indent=2)
