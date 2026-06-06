"""GPU task client — submits tasks and polls results.

The client is backend-agnostic: it delegates to a GpuProvider
(mock for tests, HTTP/FFI for real Rust coordinator).
"""

from __future__ import annotations

import asyncio
from typing import Dict, List, Optional, Protocol, runtime_checkable

from swarm.gpu_bridge.schema import (
    GpuTask,
    GpuTaskResult,
    GpuTaskStatus,
)


@runtime_checkable
class GpuProvider(Protocol):
    """Abstract backend for GPU task execution."""

    async def submit(self, task: GpuTask) -> str:
        """Submit a task, return task_id."""
        ...

    async def poll(self, task_id: str) -> Optional[GpuTaskResult]:
        """Get result if available, else None."""
        ...

    async def cancel(self, task_id: str) -> bool:
        """Cancel a pending/executing task. Returns True if cancelled."""
        ...

    async def list_pending(self) -> List[str]:
        """Return task_ids still in progress."""
        ...


class GpuTaskClient:
    """High-level GPU task client for agents.

    Usage::

        client = GpuTaskClient(provider=mock)
        tid = await client.submit_task(task)
        result = await client.wait_for_result(tid, timeout=10.0)
    """

    def __init__(self, provider: GpuProvider) -> None:
        self._provider = provider
        self._submitted: Dict[str, GpuTask] = {}

    # ------------------------------------------------------------------
    # Core API
    # ------------------------------------------------------------------

    async def submit_task(self, task: GpuTask) -> str:
        """Submit a GPU task, return its task_id."""
        task_id = await self._provider.submit(task)
        self._submitted[task_id] = task
        return task_id

    async def poll_result(self, task_id: str) -> Optional[GpuTaskResult]:
        """Non-blocking poll for a result."""
        return await self._provider.poll(task_id)

    async def cancel_task(self, task_id: str) -> bool:
        """Attempt to cancel a pending task."""
        ok = await self._provider.cancel(task_id)
        if ok:
            self._submitted.pop(task_id, None)
        return ok

    async def list_pending(self) -> List[str]:
        """Return all pending task ids."""
        return await self._provider.list_pending()

    async def wait_for_result(
        self,
        task_id: str,
        timeout: float = 60.0,
        poll_interval: float = 0.05,
    ) -> Optional[GpuTaskResult]:
        """Block until result is ready or timeout expires."""
        elapsed = 0.0
        while elapsed < timeout:
            result = await self._provider.poll(task_id)
            if result is not None:
                return result
            await asyncio.sleep(poll_interval)
            elapsed += poll_interval
        return None

    async def submit_and_wait(
        self,
        task: GpuTask,
        timeout: float = 60.0,
    ) -> Optional[GpuTaskResult]:
        """Convenience: submit + wait_for_result."""
        tid = await self.submit_task(task)
        return await self.wait_for_result(tid, timeout=timeout)

    # ------------------------------------------------------------------
    # Batch API
    # ------------------------------------------------------------------

    async def submit_batch(self, tasks: List[GpuTask]) -> List[str]:
        """Submit multiple tasks, return list of task_ids."""
        return [await self.submit_task(t) for t in tasks]

    async def wait_all(
        self,
        task_ids: List[str],
        timeout: float = 60.0,
    ) -> Dict[str, Optional[GpuTaskResult]]:
        """Wait for all tasks, return {task_id: result_or_None}."""
        results: Dict[str, Optional[GpuTaskResult]] = {}
        coros = [self.wait_for_result(tid, timeout=timeout) for tid in task_ids]
        settled = await asyncio.gather(*coros)
        for tid, res in zip(task_ids, settled):
            results[tid] = res
        return results

    # ------------------------------------------------------------------
    # Introspection
    # ------------------------------------------------------------------

    @property
    def submitted_tasks(self) -> Dict[str, GpuTask]:
        """All tasks this client has submitted (including completed)."""
        return dict(self._submitted)
