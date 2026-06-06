"""GPU provider implementations.

MockGpuProvider  — in-memory, immediate completion (for tests)
RustGpuProvider  — HTTP bridge to a Rust coordinator API
"""

from __future__ import annotations

import hashlib
from typing import Any, Callable, Dict, List, Optional
from urllib.parse import quote

import aiohttp

from swarm.gpu_bridge.schema import (
    GpuExecutionProof,
    GpuTask,
    GpuTaskResult,
    GpuTaskStatus,
)


# ──────────────────────────────────────────────────────────────────
# Mock provider (deterministic, for unit/integration tests)
# ──────────────────────────────────────────────────────────────────

class MockGpuProvider:
    """In-memory GPU provider that completes tasks instantly.

    Supports:
    - Configurable latency (default: 0 = instant)
    - Injection of custom executors per task type
    - Failure injection via fail_next / fail_tasks
    - Result introspection via .completed / .cancelled dicts
    """

    def __init__(self) -> None:
        self._pending: Dict[str, GpuTask] = {}
        self._results: Dict[str, GpuTaskResult] = {}
        self._cancelled: Dict[str, GpuTask] = {}
        self._executors: Dict[str, Callable[[GpuTask], Dict[str, Any]]] = {}
        self._fail_next: int = 0
        self._fail_task_ids: set[str] = set()
        self._latency_ticks: int = 0  # poll ticks before result
        self._tick_counts: Dict[str, int] = {}

    def register_executor(
        self,
        task_type: str,
        fn: Callable[[GpuTask], Dict[str, Any]],
    ) -> None:
        """Register a custom executor for a task type."""
        self._executors[task_type] = fn

    def set_latency(self, ticks: int) -> None:
        """Set how many poll() calls before a result appears."""
        self._latency_ticks = max(0, ticks)

    def inject_failure(self, count: int = 1) -> None:
        """Next *count* submitted tasks will fail."""
        self._fail_next = count

    def inject_task_failure(self, task_id: str) -> None:
        """Force a specific task to fail on poll."""
        self._fail_task_ids.add(task_id)

    async def submit(self, task: GpuTask) -> str:
        tid = task.task_id
        should_fail = False

        if self._fail_next > 0:
            self._fail_next -= 1
            should_fail = True

        if should_fail or tid in self._fail_task_ids:
            self._fail_task_ids.discard(tid)
            self._results[tid] = GpuTaskResult(
                task_id=tid,
                agent_id=task.agent_id,
                status=GpuTaskStatus.FAILED,
                error="injected failure",
            )
        elif self._latency_ticks == 0:
            self._results[tid] = self._execute(task)
        else:
            self._pending[tid] = task
            self._tick_counts[tid] = 0

        return tid

    async def poll(self, task_id: str) -> Optional[GpuTaskResult]:
        if task_id in self._results:
            return self._results[task_id]

        if task_id in self._pending:
            self._tick_counts[task_id] = self._tick_counts.get(task_id, 0) + 1
            if self._tick_counts[task_id] >= self._latency_ticks:
                task = self._pending.pop(task_id)
                result = self._execute(task)
                self._results[task_id] = result
                return result

        return None

    async def cancel(self, task_id: str) -> bool:
        if task_id in self._pending:
            task = self._pending.pop(task_id)
            self._cancelled[task_id] = task
            self._results[task_id] = GpuTaskResult(
                task_id=task_id,
                agent_id=task.agent_id,
                status=GpuTaskStatus.CANCELLED,
            )
            return True
        return False

    async def list_pending(self) -> List[str]:
        return list(self._pending.keys())

    def _execute(self, task: GpuTask) -> GpuTaskResult:
        executor = self._executors.get(task.task_type)
        if executor is not None:
            result_data = executor(task)
        else:
            result_data = self._default_execute(task)

        payload_bytes = str(task.payload).encode()
        result_bytes = str(result_data).encode()
        input_hash = hashlib.sha256(payload_bytes).hexdigest()
        output_hash = hashlib.sha256(result_bytes).hexdigest()

        proof = GpuExecutionProof(
            device_fingerprint="mock-gpu-0",
            input_hash=input_hash,
            output_hash=output_hash,
            compute_units_used=len(payload_bytes) + len(result_bytes),
            nonce=abs(hash(task.task_id)) % (2**32),
        )

        return GpuTaskResult(
            task_id=task.task_id,
            status=GpuTaskStatus.COMPLETED,
            agent_id=task.agent_id,
            executor_node="mock-node-0",
            result_data=result_data,
            result_hash=output_hash,
            compute_units_used=proof.compute_units_used,
            execution_proof=proof,
        )

    @staticmethod
    def _default_execute(task: GpuTask) -> Dict[str, Any]:
        return {
            "echo": task.payload,
            "task_type": task.task_type,
            "agent_id": task.agent_id,
            "mock": True,
        }

    @property
    def completed(self) -> Dict[str, GpuTaskResult]:
        return {k: v for k, v in self._results.items() if v.status == GpuTaskStatus.COMPLETED.value}

    @property
    def failed(self) -> Dict[str, GpuTaskResult]:
        return {k: v for k, v in self._results.items() if v.status == GpuTaskStatus.FAILED.value}

    @property
    def cancelled_tasks(self) -> Dict[str, GpuTask]:
        return dict(self._cancelled)


# ──────────────────────────────────────────────────────────────────
# Rust coordinator provider (HTTP bridge)
# ──────────────────────────────────────────────────────────────────

class RustGpuProvider:
    """Talk to a Rust swarm coordinator over HTTP.

    Expected endpoints:
    - POST `/api/v1/tasks` -> `{ "task_id": "..." }`
    - GET `/api/v1/tasks/{task_id}` -> task result JSON or pending status
    - POST `/api/v1/tasks/{task_id}/cancel` -> `{ "cancelled": true }`
    - GET `/api/v1/tasks/pending` -> `{ "task_ids": ["..."] }`
    """

    def __init__(
        self,
        coordinator_url: str = "http://127.0.0.1:9955",
        session: Optional[aiohttp.ClientSession] = None,
        timeout_seconds: float = 10.0,
    ) -> None:
        self._url = coordinator_url.rstrip("/")
        self._session = session
        self._owns_session = session is None
        self._timeout = aiohttp.ClientTimeout(total=timeout_seconds)

    async def submit(self, task: GpuTask) -> str:
        session = await self._get_session()
        payload = task.model_dump(mode="json")
        async with session.post(f"{self._url}/api/v1/tasks", json=payload) as response:
            response.raise_for_status()
            data = await response.json()

        if isinstance(data, dict) and data.get("task_id"):
            return str(data["task_id"])
        if isinstance(data, str):
            return data
        raise RuntimeError("Rust coordinator submit response did not include task_id")

    async def poll(self, task_id: str) -> Optional[GpuTaskResult]:
        session = await self._get_session()
        encoded_task_id = quote(task_id, safe="")
        async with session.get(f"{self._url}/api/v1/tasks/{encoded_task_id}") as response:
            if response.status in (204, 404):
                return None
            response.raise_for_status()
            data = await response.json()

        status = str(data.get("status", "")) if isinstance(data, dict) else ""
        if status in {
            GpuTaskStatus.PENDING.value,
            GpuTaskStatus.ASSIGNED.value,
            GpuTaskStatus.EXECUTING.value,
            GpuTaskStatus.VERIFYING.value,
        }:
            return None

        return GpuTaskResult.model_validate(data)

    async def cancel(self, task_id: str) -> bool:
        session = await self._get_session()
        encoded_task_id = quote(task_id, safe="")
        async with session.post(f"{self._url}/api/v1/tasks/{encoded_task_id}/cancel") as response:
            if response.status == 404:
                return False
            response.raise_for_status()
            data = await response.json()

        if isinstance(data, dict) and "cancelled" in data:
            return bool(data["cancelled"])
        return True

    async def list_pending(self) -> List[str]:
        session = await self._get_session()
        async with session.get(f"{self._url}/api/v1/tasks/pending") as response:
            response.raise_for_status()
            data = await response.json()

        if isinstance(data, dict) and "task_ids" in data:
            return [str(task_id) for task_id in data["task_ids"]]
        if isinstance(data, list):
            return [str(task_id) for task_id in data]
        raise RuntimeError("Rust coordinator pending response did not include task ids")

    async def close(self) -> None:
        if self._owns_session and self._session is not None:
            await self._session.close()
            self._session = None

    async def _get_session(self) -> aiohttp.ClientSession:
        if self._session is None:
            self._session = aiohttp.ClientSession(timeout=self._timeout)
        return self._session
