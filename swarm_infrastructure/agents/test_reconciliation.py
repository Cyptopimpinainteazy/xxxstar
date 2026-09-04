import asyncio
import pytest
import time
from swarm.agents.task_queue import AsyncTaskQueue, TaskPriority

pytestmark = pytest.mark.asyncio

async def test_reconciliation_requeues_and_executes(tmp_path, monkeypatch):
    # Use a small in-memory DB path for isolation
    db_path = str(tmp_path / "agents.db")
    # Monkeypatch sqlite default path
    import os
    os.environ['AGENT_DB_PATH'] = db_path

    queue = AsyncTaskQueue(max_concurrent_tasks=1)

    # Handler will simulate a short running task
    async def short_handler(payload):
        await asyncio.sleep(0.2)
        return {'ok': True, 'value': payload.get('value')}

    queue.register_handler('short_task', short_handler)

    # Run queue start in background
    start_task = asyncio.create_task(queue.start())

    # Submit a task
    task_id = await queue.submit_task('agent-x', 'short_task', {'value': 42}, priority=TaskPriority.MEDIUM)

    # Wait until task is started
    start_time = time.time()
    while True:
        # If the task has started (persisted as in_progress) break
        from swarm.storage.sqlite_store import load_pending_and_inprogress_tasks
        rows = load_pending_and_inprogress_tasks()
        if any(r['task_id'] == task_id for r in rows):
            break
        if time.time() - start_time > 5:
            pytest.fail("Task did not appear in pending/inprogress store")
        await asyncio.sleep(0.05)

    # Simulate crash: cancel the running queue
    await queue.stop()
    start_task.cancel()
    # don't await start_task to avoid propagating CancelledError in test harness
    await asyncio.sleep(0.01)  # allow cancellation to propagate

    # Create a new queue instance and register handler
    new_queue = AsyncTaskQueue(max_concurrent_tasks=1)
    new_queue.register_handler('short_task', short_handler)

    # Reconcile pending tasks and run them
    await new_queue.reconcile_pending_tasks()

    # Start the new queue in background to process requeued tasks
    start_task2 = asyncio.create_task(new_queue.start())

    # Wait for completion
    timeout = 10
    end = time.time() + timeout
    completed = False
    while time.time() < end:
        metrics = new_queue.get_metrics()
        if metrics['completed_tasks'] > 0:
            completed = True
            break
        await asyncio.sleep(0.05)

    # Cleanup
    await new_queue.stop()
    start_task2.cancel()
    try:
        await start_task2
    except Exception:
        pass

    assert completed, "Reconciled task did not complete after restart"
