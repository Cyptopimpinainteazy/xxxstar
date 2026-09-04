"""
Agent Task Queue & Async Runtime
=================================

Unified task queue for coordinating all agent activities with retry logic,
deadletter handling, and heartbeat monitoring. Integrates with the swarm
orchestrator for distributed execution.

Design:
- asyncio-based async runtime
- Per-task retry logic (max 3 retries)
- Deadletter queue after max retries exceeded
- 30-second heartbeat for liveness checking
- Integration with swarm/orchestrator.py
- Metrics export to Prometheus
"""

import asyncio
import json
import logging
import time
import uuid
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Callable, Any
from enum import Enum
from datetime import datetime, timedelta
from collections import deque
import aiohttp

# Persistence (sqlite-based lightweight store)
try:
    from swarm.storage.sqlite_store import (
        init_tasks_table,
        save_task,
        update_task_status,
        load_pending_and_inprogress_tasks,
        append_event,
    )
except Exception:
    # In environments without the storage module (tests), fall back to no-op helpers
    def init_tasks_table():
        return
    def save_task(*args, **kwargs):
        return
    def update_task_status(*args, **kwargs):
        return
    def load_pending_and_inprogress_tasks(*args, **kwargs):
        return []
    def append_event(*args, **kwargs):
        return

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
)
logger = logging.getLogger(__name__)


# =============================================================================
# DATA MODELS
# =============================================================================

class TaskStatus(Enum):
    """Task execution status"""
    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    COMPLETED = "completed"
    FAILED = "failed"
    DEADLETTER = "deadletter"
    CANCELLED = "cancelled"


class TaskPriority(Enum):
    """Task priority levels"""
    LOW = 0
    MEDIUM = 1
    HIGH = 2
    CRITICAL = 3


class TaskSeverity(Enum):
    """Task severity levels for governance gating"""
    MINOR = "minor"
    MAJOR = "major"


@dataclass
class Task:
    """Individual task in the queue"""
    task_id: str
    agent_id: str
    task_type: str  # e.g., "collect_metrics", "analyze_block", "send_email"
    payload: Dict[str, Any]  # Task-specific data
    priority: TaskPriority
    severity: TaskSeverity
    status: TaskStatus
    created_at_timestamp: float
    started_at_timestamp: Optional[float] = None
    completed_at_timestamp: Optional[float] = None
    retry_count: int = 0
    max_retries: int = 3
    error_message: Optional[str] = None
    result: Optional[Dict[str, Any]] = None
    openspec_change_id: Optional[str] = None

    def is_expired(self, ttl_seconds: int = 86400) -> bool:
        """Check if task has exceeded TTL"""
        return (time.time() - self.created_at_timestamp) > ttl_seconds


@dataclass
class TaskMetrics:
    """Metrics for task execution"""
    task_id: str
    execution_time_ms: float
    memory_peak_mb: float
    cpu_time_ms: float
    i_o_operations: int
    network_requests: int
    success: bool


@dataclass
class HeartbeatMessage:
    """Heartbeat for liveness monitoring"""
    agent_id: str
    timestamp: float
    task_count_active: int
    task_count_completed: int
    task_count_failed: int
    memory_usage_mb: float
    cpu_percent: float


# =============================================================================
# ASYNC TASK QUEUE
# =============================================================================

class AsyncTaskQueue:
    """
    Unified async task queue for all agents.

    Features:
    - Priority-based execution
    - Automatic retry with exponential backoff
    - Deadletter queue for failed tasks
    - Heartbeat monitoring for agent health
    - Prometheus metrics export
    - Integration with swarm orchestrator
    """

    def __init__(
        self,
        queue_name: str = "agent-task-queue",
        orchestrator_url: str = "http://localhost:8080",
        heartbeat_interval_secs: int = 30,
        max_concurrent_tasks: int = 10,
        openspec_validator: Optional[Callable[[str], Any]] = None,
    ):
        self.queue_name = queue_name
        self.orchestrator_url = orchestrator_url
        self.heartbeat_interval_secs = heartbeat_interval_secs
        self.max_concurrent_tasks = max_concurrent_tasks
        self.openspec_validator = openspec_validator

        self.session: Optional[aiohttp.ClientSession] = None
        self.is_running = False

        # Task queues (priority-based)
        self.pending_tasks: Dict[TaskPriority, asyncio.Queue] = {
            priority: asyncio.Queue() for priority in TaskPriority
        }

        # Task state
        self.active_tasks: Dict[str, Task] = {}
        self.completed_tasks: deque = deque(maxlen=10000)
        self.failed_tasks: deque = deque(maxlen=1000)
        self.deadletter_tasks: deque = deque(maxlen=10000)  # Bounded to prevent memory growth

        # Deadletter metrics and alerting
        self.deadletter_counter = 0
        self.alert_threshold = 100  # Alert when deadletter queue exceeds this
        self.alert_cooldown_until = 0.0  # Prevent alert spam
        self.alert_cooldown_seconds = 3600  # 1 hour cooldown between alerts

        # Metrics
        self.metrics: Dict[str, TaskMetrics] = {}
        self.task_counters = {
            "submitted": 0,
            "completed": 0,
            "failed": 0,
            "retried": 0,
            "deadlettered": 0,
        }

        # Task handlers (registry of functions)
        self.task_handlers: Dict[str, Callable] = {}

        logger.info(f"Initialized AsyncTaskQueue: {queue_name}")

        # Initialize persistence schema for tasks
        try:
            init_tasks_table()
        except Exception as e:
            logger.warning(f"Failed to init tasks table: {e}")

    async def start(self):
        """Start the task queue processing"""
        self.session = aiohttp.ClientSession()
        self.is_running = True
        logger.info(f"AsyncTaskQueue {self.queue_name} starting")

        try:
            # Reconcile any pending/in-progress tasks from previous runs
            try:
                await self.reconcile_pending_tasks()
            except Exception as e:
                logger.warning(f"Reconciliation failed: {e}")

            # Start worker tasks for each priority level
            workers = [
                asyncio.create_task(self._worker(priority))
                for priority in TaskPriority
            ]

            # Start heartbeat monitor
            heartbeat_task = asyncio.create_task(self._heartbeat_monitor())

            # Start periodic deadletter monitoring
            deadletter_monitor_task = asyncio.create_task(self._periodic_deadletter_monitor())

            # Wait for all tasks (they run indefinitely)
            await asyncio.gather(*workers, heartbeat_task, deadletter_monitor_task)

        except asyncio.CancelledError:
            logger.info(f"AsyncTaskQueue {self.queue_name} cancelled")
        finally:
            await self.stop()

    async def stop(self):
        """Stop the task queue gracefully"""
        self.is_running = False

        # Wait for active tasks to complete
        pending = list(self.active_tasks.values())
        if pending:
            logger.info(f"Waiting for {len(pending)} active tasks to complete")
            # Give tasks 30 seconds to finish
            await asyncio.sleep(30)

        if self.session:
            await self.session.close()

        logger.info(f"AsyncTaskQueue {self.queue_name} stopped")

    async def submit_task(
        self,
        agent_id: str,
        task_type: str,
        payload: Dict[str, Any],
        priority: TaskPriority = TaskPriority.MEDIUM,
    ) -> str:
        """Submit a task to the queue"""
        task_id = str(uuid.uuid4())

        severity_value = (payload.get("severity") or "minor").lower()
        severity = TaskSeverity.MAJOR if severity_value == "major" else TaskSeverity.MINOR
        openspec_change_id = payload.get("openspec_change_id")

        if severity == TaskSeverity.MAJOR:
            if not openspec_change_id:
                raise ValueError("openspec_change_id required for major tasks")
            if self.openspec_validator is not None:
                validation = self.openspec_validator(openspec_change_id)
                if hasattr(validation, "ok"):
                    ok = validation.ok
                    output = validation.output
                elif isinstance(validation, tuple):
                    ok, output = validation
                else:
                    ok = bool(validation)
                    output = str(validation)
                if not ok:
                    raise ValueError(f"OpenSpec validation failed: {output}")

        task = Task(
            task_id=task_id,
            agent_id=agent_id,
            task_type=task_type,
            payload=payload,
            priority=priority,
            severity=severity,
            status=TaskStatus.PENDING,
            created_at_timestamp=time.time(),
            openspec_change_id=openspec_change_id,
        )

        try:
            await self.pending_tasks[priority].put(task)
            self.task_counters["submitted"] += 1

            # Persist task in the task store
            try:
                save_task({
                    'task_id': task_id,
                    'agent_id': agent_id,
                    'task_type': task_type,
                    'payload': payload,
                    'priority': int(priority.value),
                    'severity': severity.value,
                    'status': task.status.value,
                    'created_at_timestamp': task.created_at_timestamp,
                    'retry_count': task.retry_count,
                    'max_retries': task.max_retries,
                    'openspec_change_id': openspec_change_id,
                })
            except Exception as e:
                logger.warning(f"Failed to persist task {task_id}: {e}")

            logger.debug(
                f"Task submitted: {task_id} (agent={agent_id}, type={task_type}, "
                f"priority={priority.name})"
            )

            # Append event for submission
            try:
                append_event('task_submitted', {
                    'task_id': task_id,
                    'agent_id': agent_id,
                    'task_type': task_type,
                    'openspec_change_id': openspec_change_id,
                    'severity': severity.value,
                })
            except Exception:
                pass

            return task_id
        except Exception as e:
            logger.error(f"Error submitting task: {e}")
            raise

    async def _worker(self, priority: TaskPriority):
        """Worker coroutine for processing tasks at a specific priority"""
        queue = self.pending_tasks[priority]

        while self.is_running:
            try:
                # Use proper blocking get instead of busy-wait with get_nowait()
                task = await queue.get()

                # Check if we have room for new tasks
                while len(self.active_tasks) >= self.max_concurrent_tasks:
                    await asyncio.sleep(0.5)

                # Execute task
                await self._execute_task(task)

            except asyncio.CancelledError:
                logger.info(f"Worker for priority {priority.name} cancelled")
                break
            except Exception as e:
                logger.error(f"Worker error for priority {priority.name}: {e}", exc_info=True)
                await asyncio.sleep(1)

    async def reconcile_pending_tasks(self):
        """Load pending/in-progress tasks from persistent store and requeue them as pending."""
        try:
            rows = load_pending_and_inprogress_tasks()
            logger.info(f"Reconciling {len(rows)} tasks from persistent store")
            for r in rows:
                try:
                    # Create Task object (treat as pending)
                    t = Task(
                        task_id=r['task_id'],
                        agent_id=r['agent_id'],
                        task_type=r['task_type'],
                        payload=r.get('payload', {}),
                        priority=TaskPriority(int(r.get('priority', 1))),
                        severity=TaskSeverity(r.get('severity', 'minor')),
                        status=TaskStatus.PENDING,
                        created_at_timestamp=r.get('created_at_timestamp', time.time()),
                        retry_count=r.get('retry_count', 0),
                        max_retries=r.get('max_retries', 3),
                        openspec_change_id=r.get('openspec_change_id'),
                    )

                    # Persist status update to 'pending' to reflect restart
                    try:
                        update_task_status(t.task_id, 'pending', started_at=None, completed_at=None, retry_count=t.retry_count)
                        append_event('task_requeued_on_restart', {'task_id': t.task_id, 'from_status': r.get('status')})
                    except Exception as e:
                        logger.warning(f"Failed to update persisted task status for {t.task_id}: {e}")

                    # Put into priority queue
                    await self.pending_tasks[t.priority].put(t)
                except Exception as e:
                    logger.error(f"Failed to requeue persisted task {r.get('task_id')}: {e}")
        except Exception as e:
            logger.exception(f"Error during reconciliation: {e}")


    async def _execute_task(self, task: Task):
        """Execute a single task with error handling and retry logic"""
        task_id = task.task_id

        try:
            self.active_tasks[task_id] = task
            task.status = TaskStatus.IN_PROGRESS
            task.started_at_timestamp = time.time()

            # Persist status change
            try:
                update_task_status(task_id, 'in_progress', started_at=int(task.started_at_timestamp), retry_count=task.retry_count)
                append_event('task_started', {'task_id': task_id, 'agent_id': task.agent_id})
            except Exception:
                pass

            logger.debug(f"Executing task: {task_id}")

            # Look up handler for this task type
            handler = self.task_handlers.get(task.task_type)

            if not handler:
                raise ValueError(f"No handler registered for task type: {task.task_type}")

            # Execute handler with timeout
            result = await asyncio.wait_for(
                handler(task.payload),
                timeout=300.0  # 5 minute timeout per task
            )

            # Task succeeded
            task.status = TaskStatus.COMPLETED
            task.result = result
            task.completed_at_timestamp = time.time()

            execution_time_ms = (task.completed_at_timestamp - task.started_at_timestamp) * 1000

            logger.info(
                f"Task completed: {task_id} ({task.agent_id}/{task.task_type}) "
                f"in {execution_time_ms:.0f}ms"
            )

            self.completed_tasks.append(task)
            self.task_counters["completed"] += 1

            # Record metrics
            self.metrics[task_id] = TaskMetrics(
                task_id=task_id,
                execution_time_ms=execution_time_ms,
                memory_peak_mb=0.0,  # Would measure actual memory
                cpu_time_ms=execution_time_ms,  # Approximation
                i_o_operations=0,  # Would measure actual I/O
                network_requests=0,  # Would instrument handlers
                success=True,
            )

            # Persist completion
            try:
                update_task_status(task_id, 'completed', completed_at=int(task.completed_at_timestamp), result=task.result)
                append_event('task_completed', {'task_id': task_id, 'agent_id': task.agent_id, 'execution_ms': execution_time_ms})
            except Exception:
                pass

        except asyncio.TimeoutError:
            logger.warning(f"Task timeout: {task_id}")
            await self._handle_task_failure(task, "Task exceeded 5-minute timeout")

        except Exception as e:
            logger.error(f"Task execution error: {task_id}: {e}", exc_info=True)
            await self._handle_task_failure(task, str(e))

        finally:
            if task_id in self.active_tasks:
                del self.active_tasks[task_id]

    async def _handle_task_failure(self, task: Task, error_message: str):
        """Handle task failure with retry logic"""
        task.error_message = error_message

        if task.retry_count < task.max_retries:
            # Retry with exponential backoff
            backoff_seconds = (2 ** task.retry_count)

            logger.warning(
                f"Task failed, retrying in {backoff_seconds}s: {task.task_id} "
                f"({task.retry_count + 1}/{task.max_retries}): {error_message}"
            )

            task.retry_count += 1
            task.status = TaskStatus.PENDING
            self.task_counters["retried"] += 1

            # Persist retry status
            try:
                update_task_status(task.task_id, 'pending', retry_count=task.retry_count, error_message=task.error_message)
                append_event('task_retried', {'task_id': task.task_id, 'retry_count': task.retry_count})
            except Exception:
                pass

            # Re-queue with delay
            await asyncio.sleep(backoff_seconds)
            await self.pending_tasks[task.priority].put(task)

        else:
            # Max retries exceeded - move to deadletter
            logger.error(
                f"Task deadlettered (max retries exceeded): {task.task_id}: {error_message}"
            )

            task.status = TaskStatus.DEADLETTER
            task.completed_at_timestamp = time.time()

            self.deadletter_tasks.append(task)
            self.failed_tasks.append(task)
            self.task_counters["deadlettered"] += 1

            # Persist deadletter
            try:
                update_task_status(task.task_id, 'deadletter', completed_at=int(task.completed_at_timestamp), error_message=task.error_message)
                append_event('task_deadlettered', {'task_id': task.task_id, 'error': task.error_message})
            except Exception:
                pass

            # Notify orchestrator of deadletter
            await self._notify_deadletter(task)

    async def _notify_deadletter(self, task: Task):
        """Notify orchestrator of deadlettered task with alerting"""
        if not self.session:
            return

        # Increment deadletter counter
        self.deadletter_counter += 1

        try:
            payload = {
                "task_id": task.task_id,
                "agent_id": task.agent_id,
                "task_type": task.task_type,
                "error_message": task.error_message,
                "retry_count": task.retry_count,
                "failed_at_timestamp": task.completed_at_timestamp,
            }

            async with self.session.post(
                f"{self.orchestrator_url}/api/deadletter",
                json=payload,
                timeout=aiohttp.ClientTimeout(total=10)
            ) as resp:
                if resp.status == 200:
                    logger.info(f"Deadletter notification sent for {task.task_id}")
                else:
                    logger.warning(
                        f"Failed to notify deadletter (status {resp.status})"
                    )
                    # If HTTP notification fails, emit critical alert
                    await self._emit_critical_alert(
                        f"CRITICAL: Deadletter notification failed for task {task.task_id} "
                        f"and deadletter queue size is {len(self.deadletter_tasks)}"
                    )
        except Exception as e:
            logger.error(f"Error notifying deadletter: {e}")
            # Emit critical alert on any notification failure
            await self._emit_critical_alert(
                f"CRITICAL: Deadletter notification failed for task {task.task_id}: {str(e)} "
                f"Deadletter queue size: {len(self.deadletter_tasks)}"
            )

        # Check if we need to send aggregated alerts
        await self._check_deadletter_threshold()

    async def _emit_critical_alert(self, message: str):
        """Emit critical alert via multiple channels"""
        if not self.session:
            return

        # Check if we're in cooldown period
        current_time = time.time()
        if current_time < self.alert_cooldown_until:
            logger.debug(f"Alert cooldown active, skipping alert: {message}")
            return

        alert_payload = {
            "message": message,
            "severity": "CRITICAL",
            "source": f"task-queue-{self.queue_name}",
            "timestamp": current_time,
            "deadletter_count": len(self.deadletter_tasks),
            "active_tasks": len(self.active_tasks),
        }

        # Try multiple alerting endpoints
        alert_endpoints = [
            f"{self.orchestrator_url}/api/alerts/critical",
            f"{self.orchestrator_url}/api/pagerduty/frontend/webhook",
            f"{self.orchestrator_url}/api/slack/alerts",
        ]

        success = False
        for endpoint in alert_endpoints:
            try:
                async with self.session.post(
                    endpoint,
                    json=alert_payload,
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    if resp.status == 200:
                        success = True
                        logger.info(f"Critical alert sent to {endpoint}")
                        break
                    else:
                        logger.warning(f"Alert endpoint {endpoint} returned status {resp.status}")
            except Exception as e:
                logger.error(f"Failed to send alert to {endpoint}: {e}")

        if not success:
            logger.error("CRITICAL: All alert endpoints failed - operators not notified!")

        # Set cooldown to prevent alert spam
        self.alert_cooldown_until = current_time + self.alert_cooldown_seconds

    async def _check_deadletter_threshold(self):
        """Check if deadletter queue exceeds threshold and send aggregated alerts"""
        current_time = time.time()
        if current_time < self.alert_cooldown_until:
            return

        deadletter_count = len(self.deadletter_tasks)
        if deadletter_count > self.alert_threshold:
            alert_message = (
                f"WARNING: Deadletter queue size {deadletter_count} exceeds threshold {self.alert_threshold}. "
                f"Recent deadletter tasks: {min(5, deadletter_count)} tasks shown. "
                f"Active tasks: {len(self.active_tasks)}, Failed tasks: {len(self.failed_tasks)}"
            )

            # Show recent deadletter tasks for debugging
            recent_tasks = list(self.deadletter_tasks)[-5:] if self.deadletter_tasks else []
            task_details = ", ".join([f"{task.task_id} ({task.task_type})" for task in recent_tasks])

            await self._emit_critical_alert(f"{alert_message}\nRecent tasks: {task_details}")

    async def _periodic_deadletter_monitor(self):
        """Periodic background task that checks deadletter queue and sends aggregated alerts"""
        check_interval = 300  # Check every 5 minutes

        while self.is_running:
            try:
                await asyncio.sleep(check_interval)

                deadletter_count = len(self.deadletter_tasks)
                if deadletter_count > self.alert_threshold:
                    # Get statistics about deadletter tasks
                    task_types = {}
                    error_patterns = {}

                    for task in self.deadletter_tasks:
                        task_type = task.task_type
                        error_msg = task.error_message or "unknown"

                        task_types[task_type] = task_types.get(task_type, 0) + 1
                        error_patterns[error_msg] = error_patterns.get(error_msg, 0) + 1

                    # Create aggregated alert
                    top_task_types = sorted(task_types.items(), key=lambda x: x[1], reverse=True)[:3]
                    top_errors = sorted(error_patterns.items(), key=lambda x: x[1], reverse=True)[:3]

                    alert_message = (
                        f"AGGREGATED DEADLETTER ALERT: Queue size {deadletter_count} exceeds threshold {self.alert_threshold}\n"
                        f"Top task types: {', '.join([f'{tt[0]} ({tt[1]} tasks)' for tt in top_task_types])}\n"
                        f"Top errors: {', '.join([f'{te[0]} ({te[1]} occurrences)' for te in top_errors])}\n"
                        f"System status: Active={len(self.active_tasks)}, Failed={len(self.failed_tasks)}"
                    )

                    await self._emit_critical_alert(alert_message)

            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Error in periodic deadletter monitor: {e}")

    async def get_deadletter_stats(self) -> Dict[str, Any]:
        """
        Get deadletter queue statistics for monitoring endpoint /api/deadletter/stats

        Returns:
            Dictionary with deadletter statistics and recent tasks
        """
        deadletter_count = len(self.deadletter_tasks)

        # Analyze task types and errors
        task_type_counts = {}
        error_pattern_counts = {}
        recent_tasks = []

        for i, task in enumerate(reversed(list(self.deadletter_tasks))):
            if i < 10:  # Show last 10 tasks
                recent_tasks.append({
                    "task_id": task.task_id,
                    "agent_id": task.agent_id,
                    "task_type": task.task_type,
                    "error_message": task.error_message,
                    "retry_count": task.retry_count,
                    "failed_at": task.completed_at_timestamp,
                })

            task_type = task.task_type
            error_msg = task.error_message or "unknown"

            task_type_counts[task_type] = task_type_counts.get(task_type, 0) + 1
            error_pattern_counts[error_msg] = error_pattern_counts.get(error_msg, 0) + 1

        # Sort by frequency
        top_task_types = sorted(task_type_counts.items(), key=lambda x: x[1], reverse=True)[:5]
        top_errors = sorted(error_pattern_counts.items(), key=lambda x: x[1], reverse=True)[:5]

        return {
            "queue_size": deadletter_count,
            "max_capacity": 10000,
            "percentage_full": min(100, (deadletter_count / 10000) * 100),
            "alert_threshold": self.alert_threshold,
            "threshold_exceeded": deadletter_count > self.alert_threshold,
            "task_type_distribution": [{"type": k, "count": v} for k, v in top_task_types],
            "error_distribution": [{"error": k, "count": v} for k, v in top_errors],
            "recent_tasks": recent_tasks,
            "oldest_task_age_seconds": (time.time() - self.deadletter_tasks[0].completed_at_timestamp) if self.deadletter_tasks else 0,
            "newest_task_age_seconds": (time.time() - self.deadletter_tasks[-1].completed_at_timestamp) if self.deadletter_tasks else 0,
        }

    async def _heartbeat_monitor(self):
        """Monitor agent health with periodic heartbeats"""
        while self.is_running:
            try:
                await asyncio.sleep(self.heartbeat_interval_secs)

                heartbeat = HeartbeatMessage(
                    agent_id="task-queue",
                    timestamp=time.time(),
                    task_count_active=len(self.active_tasks),
                    task_count_completed=len(self.completed_tasks),
                    task_count_failed=len(self.failed_tasks),
                    memory_usage_mb=0.0,  # Would measure actual memory
                    cpu_percent=0.0,  # Would measure actual CPU
                )

                if self.session:
                    try:
                        async with self.session.post(
                            f"{self.orchestrator_url}/api/heartbeat",
                            json={
                                "queue_name": self.queue_name,
                                "active_tasks": heartbeat.task_count_active,
                                "completed_tasks": heartbeat.task_count_completed,
                                "failed_tasks": heartbeat.task_count_failed,
                                "queue_health": "healthy" if len(self.deadletter_tasks) < 10 else "degraded",
                            },
                            timeout=aiohttp.ClientTimeout(total=5)
                        ) as resp:
                            if resp.status != 200:
                                logger.debug(f"Heartbeat failed with status {resp.status}")
                    except asyncio.TimeoutError:
                        logger.debug("Heartbeat request timeout")

            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Heartbeat error: {e}")

    def register_handler(
        self,
        task_type: str,
        handler: Callable
    ):
        """Register a task handler"""
        self.task_handlers[task_type] = handler
        logger.info(f"Registered handler for task type: {task_type}")

    def get_metrics(self) -> Dict[str, Any]:
        """Get queue metrics"""
        return {
            "queue_name": self.queue_name,
            "active_tasks": len(self.active_tasks),
            "pending_tasks_by_priority": {
                priority.name: self.pending_tasks[priority].qsize()
                for priority in TaskPriority
            },
            "completed_tasks": len(self.completed_tasks),
            "failed_tasks": len(self.failed_tasks),
            "deadletter_tasks": len(self.deadletter_tasks),
            "counters": self.task_counters.copy(),
        }


# =============================================================================
# STANDALONE EXECUTION
# =============================================================================

async def main():
    """Run task queue"""
    queue = AsyncTaskQueue(
        queue_name="agents-main",
        max_concurrent_tasks=5,
    )

    # Register example handlers
    async def example_task_handler(payload: Dict[str, Any]) -> Dict[str, Any]:
        """Example task handler"""
        logger.info(f"Executing example task with payload: {payload}")
        await asyncio.sleep(1)
        return {"status": "success", "result": payload}

    queue.register_handler("example", example_task_handler)

    try:
        await queue.start()
    except KeyboardInterrupt:
        logger.info("Shutting down...")
        await queue.stop()


if __name__ == "__main__":
    asyncio.run(main())
