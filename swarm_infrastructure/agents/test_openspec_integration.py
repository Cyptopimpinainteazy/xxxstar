import asyncio
import pytest

from swarm.openspec_integration import OpenSpecValidator
from swarm.agents.task_queue import AsyncTaskQueue, TaskPriority


class RunnerStub:
    def __init__(self, code=0, output="ok"):
        self.code = code
        self.output = output
        self.calls = 0

    def __call__(self, command, cwd):
        self.calls += 1
        return self.code, self.output


def test_validator_caches_results():
    runner = RunnerStub(code=0, output="valid")
    validator = OpenSpecValidator(
        openspec_bin="/usr/bin/openspec",
        workspace_root="/tmp",
        cache_ttl_s=300,
        runner=runner,
    )

    first = validator.validate_change("change-1")
    second = validator.validate_change("change-1")

    assert first.ok is True
    assert second.ok is True
    assert runner.calls == 1


@pytest.mark.asyncio
async def test_major_task_requires_change_id():
    queue = AsyncTaskQueue(openspec_validator=lambda _: (True, "ok"))

    with pytest.raises(ValueError):
        await queue.submit_task(
            agent_id="agent-1",
            task_type="test",
            payload={"severity": "major"},
            priority=TaskPriority.MEDIUM,
        )


@pytest.mark.asyncio
async def test_major_task_blocks_on_validation_failure():
    queue = AsyncTaskQueue(openspec_validator=lambda _: (False, "fail"))

    with pytest.raises(ValueError):
        await queue.submit_task(
            agent_id="agent-1",
            task_type="test",
            payload={"severity": "major", "openspec_change_id": "change-1"},
            priority=TaskPriority.MEDIUM,
        )


@pytest.mark.asyncio
async def test_major_task_accepts_validation_success():
    queue = AsyncTaskQueue(openspec_validator=lambda _: (True, "ok"))

    task_id = await queue.submit_task(
        agent_id="agent-1",
        task_type="test",
        payload={"severity": "major", "openspec_change_id": "change-1"},
        priority=TaskPriority.MEDIUM,
    )

    assert isinstance(task_id, str)
