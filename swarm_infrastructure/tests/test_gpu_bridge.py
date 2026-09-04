"""Tests for the GPU Bridge layer.

Invariant refs: tests/invariants/registry.toml — GPU_TASK_LIFECYCLE, GPU_RESULT_INTEGRITY

Tests cover:
- GpuTask schema: creation, serialization, defaults, enum values
- GpuTaskClient: submit, poll, cancel, batch, wait
- MockGpuProvider: instant results, latency simulation, failure injection
- Lifecycle integration: GPU client wired into EpochOrchestrator
"""

from __future__ import annotations

import asyncio
import json
from datetime import datetime, timezone

import aiohttp
import pytest
import pytest_asyncio

from swarm.gpu_bridge.schema import (
    GpuExecutionProof,
    GpuTask,
    GpuTaskMetadata,
    GpuTaskPriority,
    GpuTaskResult,
    GpuTaskStatus,
    GpuTaskType,
)
from swarm.gpu_bridge.client import GpuTaskClient
from swarm.gpu_bridge.provider import MockGpuProvider, RustGpuProvider


# =====================================================================
# Fixtures
# =====================================================================


@pytest.fixture
def mock_provider():
    return MockGpuProvider()


@pytest.fixture
def client(mock_provider):
    return GpuTaskClient(provider=mock_provider)


def _make_task(
    agent_id: str = "agent-1",
    task_type: GpuTaskType = GpuTaskType.CUSTOM,
    priority: GpuTaskPriority = GpuTaskPriority.NORMAL,
    payload: dict | None = None,
) -> GpuTask:
    return GpuTask(
        agent_id=agent_id,
        task_type=task_type,
        priority=priority,
        payload=payload or {"data": "test"},
    )


# =====================================================================
# Schema Tests
# =====================================================================


class TestGpuTaskSchema:
    """Test the Pydantic GPU task models."""

    def test_task_defaults(self):
        t = GpuTask()
        assert t.task_id  # UUID generated
        assert t.task_type == GpuTaskType.CUSTOM.value
        assert t.priority == GpuTaskPriority.NORMAL.value
        assert t.agent_id == ""
        assert t.timeout_secs == 300

    def test_task_enum_values_match_rust(self):
        """Enum string values must match Rust serde names."""
        assert GpuTaskType.X3_BYTECODE.value == "X3Bytecode"
        assert GpuTaskType.MEMPOOL_SIMULATION.value == "MempoolSimulation"
        assert GpuTaskType.ML_TRAINING.value == "MLTraining"
        assert GpuTaskType.PROOF_GENERATION.value == "ProofGeneration"
        assert GpuTaskType.ARBITRAGE_SEARCH.value == "ArbitrageSearch"

    def test_task_priority_values(self):
        assert GpuTaskPriority.LOW.value == "Low"
        assert GpuTaskPriority.NORMAL.value == "Normal"
        assert GpuTaskPriority.HIGH.value == "High"
        assert GpuTaskPriority.CRITICAL.value == "Critical"

    def test_task_status_values(self):
        assert GpuTaskStatus.PENDING.value == "Pending"
        assert GpuTaskStatus.COMPLETED.value == "Completed"
        assert GpuTaskStatus.FAILED.value == "Failed"
        assert GpuTaskStatus.TIMED_OUT.value == "TimedOut"

    def test_task_json_roundtrip(self):
        t = _make_task(agent_id="a-42", payload={"x": 99})
        data = t.model_dump_json()
        parsed = GpuTask.model_validate_json(data)
        assert parsed.agent_id == "a-42"
        assert parsed.payload == {"x": 99}

    def test_agi_task_types_exist(self):
        """AGI substrate extension types."""
        assert GpuTaskType.CAUSAL_ANALYSIS.value == "CausalAnalysis"
        assert GpuTaskType.AGENT_EVALUATION.value == "AgentEvaluation"
        assert GpuTaskType.PREDICTION_BATCH.value == "PredictionBatch"
        assert GpuTaskType.COUNTERFACTUAL.value == "Counterfactual"

    def test_execution_proof_fields(self):
        proof = GpuExecutionProof(
            device_fingerprint="gpu-rtx4090-001",
            input_hash="abc123",
            output_hash="def456",
            compute_units_used=1024,
            nonce=42,
        )
        assert proof.device_fingerprint == "gpu-rtx4090-001"
        assert proof.compute_units_used == 1024

    def test_result_succeeded_property(self):
        r = GpuTaskResult(task_id="t1", status=GpuTaskStatus.COMPLETED)
        assert r.succeeded is True
        assert r.failed is False

    def test_result_failed_property(self):
        r = GpuTaskResult(task_id="t2", status=GpuTaskStatus.FAILED)
        assert r.succeeded is False
        assert r.failed is True

    def test_result_timed_out_is_failed(self):
        r = GpuTaskResult(task_id="t3", status=GpuTaskStatus.TIMED_OUT)
        assert r.failed is True
        assert r.succeeded is False

    def test_metadata_defaults(self):
        m = GpuTaskMetadata()
        assert m.description == ""
        assert m.tags == []
        assert m.min_reputation == 0


# =====================================================================
# MockGpuProvider Tests
# =====================================================================


class TestMockGpuProvider:
    """Test the mock GPU provider for testing."""

    @pytest.mark.asyncio
    async def test_submit_returns_task_id(self, mock_provider):
        task = _make_task()
        tid = await mock_provider.submit(task)
        assert tid == task.task_id

    @pytest.mark.asyncio
    async def test_instant_completion(self, mock_provider):
        task = _make_task(agent_id="a1")
        tid = await mock_provider.submit(task)
        result = await mock_provider.poll(tid)
        assert result is not None
        assert result.succeeded
        assert result.agent_id == "a1"
        assert result.executor_node == "mock-node-0"

    @pytest.mark.asyncio
    async def test_result_has_proof(self, mock_provider):
        task = _make_task()
        tid = await mock_provider.submit(task)
        result = await mock_provider.poll(tid)
        assert result.execution_proof is not None
        assert result.execution_proof.device_fingerprint == "mock-gpu-0"
        assert len(result.execution_proof.input_hash) == 64  # sha256 hex

    @pytest.mark.asyncio
    async def test_default_executor_echoes_payload(self, mock_provider):
        task = _make_task(payload={"hello": "world"})
        tid = await mock_provider.submit(task)
        result = await mock_provider.poll(tid)
        assert result.result_data["echo"] == {"hello": "world"}
        assert result.result_data["mock"] is True

    @pytest.mark.asyncio
    async def test_custom_executor(self, mock_provider):
        def my_executor(t):
            return {"squared": t.payload.get("x", 0) ** 2}

        mock_provider.register_executor(GpuTaskType.ML_TRAINING.value, my_executor)
        task = _make_task(task_type=GpuTaskType.ML_TRAINING, payload={"x": 7})
        tid = await mock_provider.submit(task)
        result = await mock_provider.poll(tid)
        assert result.result_data["squared"] == 49

    @pytest.mark.asyncio
    async def test_latency_simulation(self, mock_provider):
        mock_provider.set_latency(3)
        task = _make_task()
        tid = await mock_provider.submit(task)

        # Poll 1 and 2 return None
        assert await mock_provider.poll(tid) is None
        assert await mock_provider.poll(tid) is None

        # Poll 3 returns result
        result = await mock_provider.poll(tid)
        assert result is not None
        assert result.succeeded

    @pytest.mark.asyncio
    async def test_failure_injection(self, mock_provider):
        mock_provider.inject_failure(1)
        task = _make_task()
        tid = await mock_provider.submit(task)
        result = await mock_provider.poll(tid)
        assert result is not None
        assert result.failed
        assert result.error == "injected failure"

    @pytest.mark.asyncio
    async def test_specific_task_failure(self, mock_provider):
        task = _make_task()
        mock_provider.inject_task_failure(task.task_id)
        tid = await mock_provider.submit(task)
        result = await mock_provider.poll(tid)
        assert result.failed

    @pytest.mark.asyncio
    async def test_cancel_pending(self, mock_provider):
        mock_provider.set_latency(100)  # Will never complete
        task = _make_task()
        tid = await mock_provider.submit(task)

        pending = await mock_provider.list_pending()
        assert tid in pending

        ok = await mock_provider.cancel(tid)
        assert ok is True

        pending = await mock_provider.list_pending()
        assert tid not in pending

        result = await mock_provider.poll(tid)
        assert result is not None
        assert result.status == GpuTaskStatus.CANCELLED.value

    @pytest.mark.asyncio
    async def test_cancel_completed_returns_false(self, mock_provider):
        task = _make_task()
        tid = await mock_provider.submit(task)
        # Already completed (instant)
        ok = await mock_provider.cancel(tid)
        assert ok is False

    @pytest.mark.asyncio
    async def test_introspection_properties(self, mock_provider):
        t1 = _make_task(agent_id="a1")
        t2 = _make_task(agent_id="a2")
        mock_provider.inject_failure(1)

        await mock_provider.submit(t1)  # Will fail
        await mock_provider.submit(t2)  # Will succeed

        assert len(mock_provider.completed) == 1
        assert len(mock_provider.failed) == 1


# =====================================================================
# GpuTaskClient Tests
# =====================================================================


class TestGpuTaskClient:
    """Test the high-level GPU task client."""

    @pytest.mark.asyncio
    async def test_submit_and_poll(self, client):
        task = _make_task()
        tid = await client.submit_task(task)
        assert tid == task.task_id

        result = await client.poll_result(tid)
        assert result is not None
        assert result.succeeded

    @pytest.mark.asyncio
    async def test_submit_and_wait(self, client):
        task = _make_task(payload={"v": 42})
        result = await client.submit_and_wait(task, timeout=5.0)
        assert result is not None
        assert result.result_data["echo"]["v"] == 42

    @pytest.mark.asyncio
    async def test_wait_timeout(self, mock_provider):
        mock_provider.set_latency(9999)
        c = GpuTaskClient(provider=mock_provider)
        task = _make_task()
        tid = await c.submit_task(task)
        result = await c.wait_for_result(tid, timeout=0.1, poll_interval=0.05)
        assert result is None  # Timed out

    @pytest.mark.asyncio
    async def test_cancel(self, mock_provider):
        mock_provider.set_latency(100)
        c = GpuTaskClient(provider=mock_provider)
        task = _make_task()
        tid = await c.submit_task(task)

        ok = await c.cancel_task(tid)
        assert ok is True
        assert tid not in c.submitted_tasks  # Removed after cancel

    @pytest.mark.asyncio
    async def test_batch_submit(self, client):
        tasks = [_make_task(agent_id=f"a-{i}") for i in range(5)]
        tids = await client.submit_batch(tasks)
        assert len(tids) == 5

    @pytest.mark.asyncio
    async def test_wait_all(self, client):
        tasks = [_make_task(agent_id=f"a-{i}") for i in range(3)]
        tids = await client.submit_batch(tasks)
        results = await client.wait_all(tids, timeout=5.0)
        assert len(results) == 3
        assert all(r.succeeded for r in results.values())

    @pytest.mark.asyncio
    async def test_submitted_tasks_tracking(self, client):
        t1 = _make_task(agent_id="a1")
        t2 = _make_task(agent_id="a2")
        await client.submit_task(t1)
        await client.submit_task(t2)
        assert len(client.submitted_tasks) == 2

    @pytest.mark.asyncio
    async def test_list_pending(self, mock_provider):
        mock_provider.set_latency(100)
        c = GpuTaskClient(provider=mock_provider)
        t1 = _make_task()
        await c.submit_task(t1)
        pending = await c.list_pending()
        assert len(pending) == 1


# =====================================================================
# RustGpuProvider HTTP test
# =====================================================================


class TestRustGpuProvider:
    """Verify the Rust provider speaks the expected HTTP contract."""

    @pytest_asyncio.fixture
    async def rust_provider_server(self, aiohttp_server):
        pending = ["pending-1", "pending-2"]
        submitted = {}

        async def submit(request):
            payload = await request.json()
            submitted[payload['task_id']] = payload
            return aiohttp.web.json_response({'task_id': payload['task_id']})

        async def poll(request):
            task_id = request.match_info['task_id']
            if task_id == 'pending-task':
                return aiohttp.web.json_response({'task_id': task_id, 'status': GpuTaskStatus.PENDING.value})
            if task_id not in submitted:
                return aiohttp.web.Response(status=404)
            return aiohttp.web.json_response({
                'task_id': task_id,
                'agent_id': submitted[task_id]['agent_id'],
                'status': GpuTaskStatus.COMPLETED.value,
                'result_data': {'ok': True},
                'result_hash': 'abc123',
                'compute_units_used': 1,
            })

        async def cancel(request):
            task_id = request.match_info['task_id']
            return aiohttp.web.json_response({'cancelled': task_id in submitted})

        async def list_pending(request):
            return aiohttp.web.json_response({'task_ids': pending})

        app = aiohttp.web.Application()
        app.router.add_post('/api/v1/tasks', submit)
        app.router.add_get('/api/v1/tasks/pending', list_pending)
        app.router.add_get('/api/v1/tasks/{task_id}', poll)
        app.router.add_post('/api/v1/tasks/{task_id}/cancel', cancel)
        server = await aiohttp_server(app)
        return server, submitted, pending

    @pytest.mark.asyncio
    async def test_submit_and_poll(self, rust_provider_server):
        server, submitted, _ = rust_provider_server
        provider = RustGpuProvider(coordinator_url=str(server.make_url('')).rstrip('/'))
        try:
            task = _make_task()
            task_id = await provider.submit(task)
            assert task_id == task.task_id
            assert task_id in submitted

            result = await provider.poll(task_id)
            assert result is not None
            assert result.succeeded
            assert result.result_data['ok'] is True
        finally:
            await provider.close()

    @pytest.mark.asyncio
    async def test_poll_pending_returns_none(self, rust_provider_server):
        server, _, _ = rust_provider_server
        provider = RustGpuProvider(coordinator_url=str(server.make_url('')).rstrip('/'))
        try:
            result = await provider.poll('pending-task')
            assert result is None
        finally:
            await provider.close()

    @pytest.mark.asyncio
    async def test_cancel_and_list_pending(self, rust_provider_server):
        server, submitted, pending = rust_provider_server
        provider = RustGpuProvider(coordinator_url=str(server.make_url('')).rstrip('/'))
        try:
            task = _make_task(agent_id='a-cancel')
            await provider.submit(task)

            assert await provider.cancel(task.task_id) is True
            assert await provider.list_pending() == pending
            assert submitted[task.task_id]['agent_id'] == 'a-cancel'
        finally:
            await provider.close()


# =====================================================================
# Lifecycle Integration Tests
# =====================================================================


class TestGpuLifecycleIntegration:
    """Test GPU bridge wired into the EpochOrchestrator."""

    @pytest_asyncio.fixture
    async def orchestrator_with_gpu(self):
        """Create an orchestrator with a MockGpuProvider."""
        from swarm.core.agent import AgentConfig, Consequence
        from swarm.core.enums import Domain
        from swarm.core.lifecycle import EpochOrchestrator
        from swarm.event_bus.bus import AsyncEventBus
        from swarm.reaper import PostmortemAnalyzer, ReaperEngine, ScarPropagator
        from swarm.storage.backend import SqliteStorage
        from swarm.tripwire.detector import TripwireDetector
        from swarm.world_sim.prediction import PredictionMarket
        from swarm.world_sim.scoreboard import AccuracyScoreboard
        from swarm.world_sim.state_graph import WorldStateGraph

        storage = SqliteStorage(":memory:")
        bus = AsyncEventBus()
        world = WorldStateGraph(storage=storage)
        predictions = PredictionMarket(storage=storage)
        scoreboard = AccuracyScoreboard(storage=storage)
        reaper = ReaperEngine(storage=storage)
        postmortem = PostmortemAnalyzer(storage=storage)
        scar_prop = ScarPropagator(storage=storage)
        tripwire = TripwireDetector(storage=storage)

        mock_provider = MockGpuProvider()
        gpu_client = GpuTaskClient(provider=mock_provider)

        orch = EpochOrchestrator(
            storage=storage,
            event_bus=bus,
            world_state=world,
            prediction_market=predictions,
            scoreboard=scoreboard,
            reaper=reaper,
            postmortem_analyzer=postmortem,
            scar_propagator=scar_prop,
            tripwire=tripwire,
            gpu_client=gpu_client,
        )

        # Register an agent
        config = AgentConfig(
            agent_id="gpu-test-agent",
            domain=Domain.MARKET,
            initial_mandates=["compute"],
            initial_budget=100.0,
        )
        orch.spawn_agent(config)

        return orch, mock_provider

    @pytest.mark.asyncio
    async def test_gpu_client_accessible(self, orchestrator_with_gpu):
        orch, _ = orchestrator_with_gpu
        assert orch.gpu_client is not None

    @pytest.mark.asyncio
    async def test_submit_gpu_task_from_orchestrator(self, orchestrator_with_gpu):
        orch, mock = orchestrator_with_gpu
        task = GpuTask(
            task_type=GpuTaskType.CAUSAL_ANALYSIS,
            payload={"analysis": "test"},
        )
        tid = await orch.submit_gpu_task("gpu-test-agent", task)
        assert tid is not None
        assert task.agent_id == "gpu-test-agent"
        assert task.epoch == orch.current_epoch

    @pytest.mark.asyncio
    async def test_gpu_results_collected_in_epoch(self, orchestrator_with_gpu):
        orch, mock = orchestrator_with_gpu

        # Submit a GPU task
        task = GpuTask(
            task_type=GpuTaskType.ML_TRAINING,
            payload={"model": "bert"},
        )
        await orch.submit_gpu_task("gpu-test-agent", task)

        # Run an epoch — GPU results collected in step 5b
        stats = await orch.run_epoch()
        assert stats.gpu_tasks_completed == 1

    @pytest.mark.asyncio
    async def test_no_gpu_client_graceful(self):
        """Orchestrator without GPU client still works."""
        from swarm.core.lifecycle import EpochOrchestrator
        from swarm.event_bus.bus import AsyncEventBus
        from swarm.reaper import PostmortemAnalyzer, ReaperEngine, ScarPropagator
        from swarm.storage.backend import SqliteStorage
        from swarm.tripwire.detector import TripwireDetector
        from swarm.world_sim.prediction import PredictionMarket
        from swarm.world_sim.scoreboard import AccuracyScoreboard
        from swarm.world_sim.state_graph import WorldStateGraph
        from swarm.core.agent import AgentConfig
        from swarm.core.enums import Domain

        storage = SqliteStorage(":memory:")
        orch = EpochOrchestrator(
            storage=storage,
            event_bus=AsyncEventBus(),
            world_state=WorldStateGraph(storage=storage),
            prediction_market=PredictionMarket(storage=storage),
            scoreboard=AccuracyScoreboard(storage=storage),
            reaper=ReaperEngine(storage=storage),
            postmortem_analyzer=PostmortemAnalyzer(storage=storage),
            scar_propagator=ScarPropagator(storage=storage),
            tripwire=TripwireDetector(storage=storage),
            # No gpu_client
        )

        assert orch.gpu_client is None
        result = await orch.submit_gpu_task("any", GpuTask())
        assert result is None  # No-op

        config = AgentConfig(
            agent_id="no-gpu-agent",
            domain=Domain.MARKET,
            initial_mandates=["trade"],
            initial_budget=50.0,
        )
        orch.spawn_agent(config)
        stats = await orch.run_epoch()
        assert stats.gpu_tasks_completed == 0

    @pytest.mark.asyncio
    async def test_gpu_reward_applied_to_agent(self, orchestrator_with_gpu):
        orch, mock = orchestrator_with_gpu

        agent = orch.living_agents[0]
        initial_budget = agent.resource_budget

        # Submit a task with known compute units
        task = GpuTask(
            task_type=GpuTaskType.CUSTOM,
            payload={"big": "data" * 100},  # Generates some compute units
        )
        await orch.submit_gpu_task("gpu-test-agent", task)
        stats = await orch.run_epoch()

        # Agent should have received a small reward
        assert stats.gpu_tasks_completed == 1
        assert stats.total_reward > 0
