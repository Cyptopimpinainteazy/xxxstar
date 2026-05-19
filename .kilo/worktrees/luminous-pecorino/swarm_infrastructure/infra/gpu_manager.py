"""In-memory GPU manager for local testing."""

from dataclasses import dataclass, field
from typing import Dict, List, Optional
import time
import uuid
from swarm.core.orchestrator import GPUCapabilities, Contributor, Task


class GPUManager:
    def __init__(self, total_gpus: int = 0):
        self.total_gpus = total_gpus
        self.contributors: Dict[str, Contributor] = {}
        self.tasks: Dict[str, Task] = {}
        self.queue: List[Task] = []

    # Contributor management
    def register(self, contributor_id: str, wallet: Optional[str], capabilities: GPUCapabilities):
        c = Contributor(contributor_id=contributor_id, wallet=wallet, capabilities=capabilities)
        self.contributors[contributor_id] = c
        return c

    def heartbeat(self, contributor_id: str, utilization: float, temperature_c: float):
        c = self.contributors.get(contributor_id)
        if c:
            c.utilization = utilization
            c.temperature_c = temperature_c
            c.last_heartbeat_at = time.time()
            c.online = True

    def mark_offline(self, contributor_id: str):
        c = self.contributors.get(contributor_id)
        if c:
            c.online = False

    def list_contributors(self) -> List[Contributor]:
        return list(self.contributors.values())

    # Task management
    def enqueue_task(self, workload_type: str, payload: dict, required_vram_mb: int = 0, min_compute_score: float = 0.0, max_runtime_s: Optional[int] = None, priority: str = "normal") -> str:
        tid = str(uuid.uuid4())
        t = Task(
            task_id=tid,
            workload_type=str(workload_type),
            payload=payload,
            required_vram_mb=required_vram_mb,
            min_compute_score=min_compute_score,
            max_runtime_s=max_runtime_s,
            priority=priority,
        )
        self.tasks[tid] = t
        self.queue.append(t)
        return tid

    def assign_task_to(self, contributor_id: str):
        c = self.contributors.get(contributor_id)
        if not c or not c.online:
            return type('R', (), {'task': None, 'reason': 'contributor offline or unknown'})()
        if c.active_task_id:
            return type('R', (), {'task': None, 'reason': 'contributor already has an active task'})()
        if not self.queue:
            return type('R', (), {'task': None, 'reason': 'no tasks queued'})()

        task_index = None
        for index, task in enumerate(self.queue):
            if self._can_accept_task(c, task):
                task_index = index
                break

        if task_index is None:
            return type('R', (), {'task': None, 'reason': 'no queued tasks match contributor capabilities'})()

        t = self.queue.pop(task_index)
        t.status = 'assigned'
        t.assigned_to = contributor_id
        t.assigned_at = time.time()
        t.started_at = t.assigned_at
        c.active_task_id = t.task_id
        return type('R', (), {'task': t, 'reason': None})()

    def sweep_timeouts(self, heartbeat_timeout_s: int = 120, task_timeout_s: int = 3600):
        """Sweep stale contributors and tasks."""
        now = time.time()
        # Contributors: mark offline if no heartbeat recently
        for c in list(self.contributors.values()):
            last = getattr(c, 'last_heartbeat_at', None)
            if last is None or (now - last) > heartbeat_timeout_s:
                c.online = False
        # Tasks: mark assigned tasks as failed if exceeded timeout
        for t in list(self.tasks.values()):
            if t.status == 'assigned':
                assigned_at = getattr(t, 'assigned_at', None)
                max_runtime = getattr(t, 'max_runtime_s', None) or task_timeout_s
                if assigned_at and (now - assigned_at) > max_runtime:
                    t.status = 'failed'
                    # clear contributor active task
                    if t.assigned_to and t.assigned_to in self.contributors:
                        self.contributors[t.assigned_to].tasks_failed += 1
                        self.contributors[t.assigned_to].active_task_id = None
                    t.assigned_to = None

    def submit_result(self, contributor_id: str, task_id: str, success: bool, result=None, error=None) -> bool:
        t = self.tasks.get(task_id)
        c = self.contributors.get(contributor_id)
        if not t or not c:
            return False
        if t.assigned_to != contributor_id or t.status != 'assigned':
            return False
        t.status = 'completed' if success else 'failed'
        t.result = result if success else None
        t.error = None if success else (error or 'task execution failed')
        t.finished_at = time.time()
        c.active_task_id = None
        if success:
            c.tasks_completed += 1
        else:
            c.tasks_failed += 1
        return True

    def cancel_task(self, task_id: str) -> bool:
        t = self.tasks.get(task_id)
        if not t:
            return False
        t.status = 'cancelled'
        t.finished_at = time.time()
        self.queue = [q for q in self.queue if q.task_id != task_id]
        if t.assigned_to and t.assigned_to in self.contributors:
            self.contributors[t.assigned_to].active_task_id = None
        t.assigned_to = None
        return True

    def list_tasks(self, limit: int = 100):
        return list(self.tasks.values())[:limit]

    def queue_depth(self) -> int:
        return len(self.queue)

    def get_queue_stats(self):
        return {
            'queued': len(self.queue),
            'assigned': sum(1 for task in self.tasks.values() if task.status == 'assigned'),
            'completed': sum(1 for task in self.tasks.values() if task.status == 'completed'),
            'failed': sum(1 for task in self.tasks.values() if task.status == 'failed'),
            'cancelled': sum(1 for task in self.tasks.values() if task.status == 'cancelled'),
        }

    def get_task(self, task_id: str) -> Optional[Task]:
        return self.tasks.get(task_id)

    def snapshot(self) -> dict:
        contributors = self.list_contributors()
        return {
            'total_gpus': self.total_gpus,
            'contributors_total': len(contributors),
            'contributors_online': sum(1 for contributor in contributors if contributor.online),
            'queue_depth': self.queue_depth(),
            'queue_stats': self.get_queue_stats(),
        }

    @staticmethod
    def _can_accept_task(contributor: Contributor, task: Task) -> bool:
        capabilities = contributor.capabilities
        if capabilities.vram_mb < task.required_vram_mb:
            return False
        if capabilities.compute_score < task.min_compute_score:
            return False
        return True
