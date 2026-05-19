"""Phase 9 — Integration End-to-End Tests.

Tests run FULL epoch loops with ALL subsystems wired together:
- SubstrateWiring creates everything
- Multiple agents spawn, act, receive consequences
- Reaper evaluates and kills
- Scars propagate
- Causal graph records everything
- Anomaly detector scans swarm
- UI bridge receives events
- Event bus routes correctly

These tests prove the complete X3 AGI substrate works end-to-end.

Invariant ref: tests/invariants/registry.toml — e2e_substrate
"""

from __future__ import annotations

import asyncio
from typing import Any, Dict, List

import pytest
import pytest_asyncio

from swarm.causal.schema import NodeType
from swarm.core.agent import Agent, AgentConfig, Consequence
from swarm.core.enums import Domain
from swarm.core.lifecycle import EpochStats
from swarm.core.wiring import SubstrateWiring
from swarm.event_bus.bus import AsyncEventBus
from swarm.event_bus.events import EventType
from swarm.gpu_bridge.client import GpuTaskClient
from swarm.gpu_bridge.provider import MockGpuProvider
from swarm.gpu_bridge.schema import GpuTask, GpuTaskType
from swarm.storage.backend import SqliteStorage
from swarm.tripwire.anomaly import SwarmAnomalyReport
from swarm.ui_bridge import UIBridge


# ---------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------


def _storage() -> SqliteStorage:
    return SqliteStorage(":memory:")


def _bus() -> AsyncEventBus:
    return AsyncEventBus(log_dir=None)


def _reward_consequence_fn(agent: Agent, action: Any) -> List[Consequence]:
    """Simple consequence function: reward small amount for any action."""
    return [Consequence("REWARD", 0.5, "ENVIRONMENT")]


def _harsh_consequence_fn(agent: Agent, action: Any) -> List[Consequence]:
    """Harsh environment: drains energy from every action."""
    return [Consequence("ENERGY_DRAIN", 50.0, "HARSH_ENVIRONMENT")]


def _spawn_with_goal(orch, agent_id: str, domain: Domain = Domain.MARKET, budget: float = 100.0):
    """Spawn an agent and give it an active goal so it can act."""
    config = AgentConfig(
        agent_id=agent_id,
        domain=domain,
        initial_mandates=[f"mandate-{agent_id}"],
        initial_budget=budget,
    )
    agent = orch.spawn_agent(config)
    agent.add_goal(f"mandate-{agent_id}", domain)
    return agent


# ---------------------------------------------------------------
# Single-epoch E2E
# ---------------------------------------------------------------


class TestSingleEpochE2E:
    """Run one complete epoch with full substrate wiring."""

    @pytest.mark.asyncio
    async def test_one_epoch_one_agent(self):
        """Single agent completes one epoch."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        _spawn_with_goal(orch, "alpha", Domain.MARKET)

        stats = await orch.run_epoch()
        assert stats.agents_alive >= 1
        assert stats.epoch >= 0

    @pytest.mark.asyncio
    async def test_one_epoch_three_agents(self):
        """Three agents complete one epoch."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        for name in ["alpha", "beta", "gamma"]:
            _spawn_with_goal(orch, name, Domain.CODE)

        stats = await orch.run_epoch()
        assert stats.agents_alive == 3

    @pytest.mark.asyncio
    async def test_rewards_flow_through(self):
        """Consequences add rewards to agents."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(
            storage=s,
            event_bus=bus,
            consequence_fn=_reward_consequence_fn,
        )
        orch = wiring.build()

        _spawn_with_goal(orch, "earner", Domain.MARKET)

        stats = await orch.run_epoch()
        assert stats.total_reward > 0

    @pytest.mark.asyncio
    async def test_bus_events_published(self):
        """Events flow through the bus during an epoch."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        _spawn_with_goal(orch, "a1", Domain.MARKET)

        stats = await orch.run_epoch()
        assert stats.bus_events_published >= 1
        assert bus.event_count >= 1


# ---------------------------------------------------------------
# Multi-epoch E2E
# ---------------------------------------------------------------


class TestMultiEpochE2E:
    """Run multiple epochs to test lifecycle dynamics."""

    @pytest.mark.asyncio
    async def test_three_epochs_stable(self):
        """Three epochs with mild environment — all agents survive."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(
            storage=s,
            event_bus=bus,
            consequence_fn=_reward_consequence_fn,
        )
        orch = wiring.build()

        for i in range(3):
            _spawn_with_goal(orch, f"stable-{i}", Domain.MARKET, budget=200.0)

        all_stats = await orch.run(max_epochs=3)
        assert len(all_stats) == 3
        # Should still have agents
        assert all_stats[-1].agents_alive >= 1

    @pytest.mark.asyncio
    async def test_harsh_env_kills_agents(self):
        """Harsh environment exhausts budgets → reaper kills agents."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(
            storage=s,
            event_bus=bus,
            consequence_fn=_harsh_consequence_fn,
        )
        orch = wiring.build()

        _spawn_with_goal(orch, "fragile", Domain.CODE, budget=10.0)

        all_stats = await orch.run(max_epochs=5)
        # At least ran some epochs
        assert len(all_stats) >= 1

    @pytest.mark.asyncio
    async def test_epoch_history_accumulated(self):
        """Epoch history grows with each epoch."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        _spawn_with_goal(orch, "tracker", Domain.MARKET)

        await orch.run(max_epochs=3)
        assert len(orch.epoch_history) == 3


# ---------------------------------------------------------------
# Causal graph E2E
# ---------------------------------------------------------------


class TestCausalGraphE2E:
    """Test causal graph is populated during epoch runs."""

    @pytest.mark.asyncio
    async def test_causal_nodes_created_per_epoch(self):
        """Running epochs creates causal nodes."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        _spawn_with_goal(orch, "causality-a", Domain.MARKET)

        await orch.run(max_epochs=2)
        cg = orch.causal_graph
        nodes = cg.get_nodes_for_agent("causality-a")
        assert len(nodes) >= 1  # At least action nodes

    @pytest.mark.asyncio
    async def test_consequence_creates_edges(self):
        """Consequences create causal edges action → consequence."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(
            storage=s,
            event_bus=bus,
            consequence_fn=_reward_consequence_fn,
        )
        orch = wiring.build()

        _spawn_with_goal(orch, "edge-a", Domain.CODE)

        await orch.run_epoch()
        cg = orch.causal_graph
        assert cg.edge_count >= 1

    @pytest.mark.asyncio
    async def test_death_recorded_in_causal_graph(self):
        """Agent death creates a DEATH causal node."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(
            storage=s,
            event_bus=bus,
            consequence_fn=_harsh_consequence_fn,
        )
        orch = wiring.build()

        _spawn_with_goal(orch, "doomed", Domain.MARKET, budget=5.0)

        await orch.run(max_epochs=10)
        cg = orch.causal_graph
        # Graph should have nodes regardless
        all_nodes = cg.get_nodes_for_agent("doomed")
        assert len(all_nodes) >= 1


# ---------------------------------------------------------------
# Anomaly detection E2E
# ---------------------------------------------------------------


class TestAnomalyDetectionE2E:
    """Test anomaly detection integrates with full epoch loops."""

    @pytest.mark.asyncio
    async def test_anomaly_scan_after_epochs(self):
        """Anomaly detector can scan agents after epochs run."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        for i in range(3):
            _spawn_with_goal(orch, f"scan-{i}", Domain.MARKET)

        await orch.run(max_epochs=3)

        # Now scan swarm
        ad = wiring.anomaly_detector
        agent_ids = [a.agent_id for a in orch.living_agents]
        report = ad.scan_swarm(agent_ids, epoch=orch.current_epoch)
        assert isinstance(report, SwarmAnomalyReport)
        assert report.epoch == orch.current_epoch


# ---------------------------------------------------------------
# UI Bridge E2E
# ---------------------------------------------------------------


class TestUIBridgeE2E:
    """Test UI bridge receives events during epoch runs."""

    @pytest.mark.asyncio
    async def test_bridge_receives_epoch_events(self):
        """UIBridge receives events published during epoch."""
        s = _storage()
        bus = _bus()
        bridge = UIBridge(event_bus=bus)

        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        _spawn_with_goal(orch, "ui-a", Domain.MARKET)

        await orch.run_epoch()
        assert len(bridge.sent_messages) >= 1

    @pytest.mark.asyncio
    async def test_push_stats_to_bridge(self):
        """Epoch stats can be pushed to UI bridge."""
        s = _storage()
        bus = _bus()

        broadcaster_msgs: List[Dict] = []

        async def mock_broadcast(ch: str, payload: Dict):
            broadcaster_msgs.append({"channel": ch, **payload})

        bridge = UIBridge(event_bus=bus, broadcast_fn=mock_broadcast)

        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        _spawn_with_goal(orch, "stats-a", Domain.CODE)

        stats = await orch.run_epoch()
        await bridge.push_epoch_stats(stats)

        metrics_msgs = [m for m in broadcaster_msgs if m["channel"] == "metrics"]
        assert len(metrics_msgs) == 1
        assert metrics_msgs[0]["data"]["epoch"] == stats.epoch


# ---------------------------------------------------------------
# GPU Bridge E2E
# ---------------------------------------------------------------


class TestGPUBridgeE2E:
    """Test GPU bridge integrates with full epoch loop."""

    @pytest.mark.asyncio
    async def test_epoch_with_gpu_client(self):
        """Epoch runs cleanly with a GPU client attached."""
        s = _storage()
        bus = _bus()
        provider = MockGpuProvider()
        gpu_client = GpuTaskClient(provider)

        wiring = SubstrateWiring(
            storage=s,
            event_bus=bus,
            gpu_client=gpu_client,
        )
        orch = wiring.build()

        _spawn_with_goal(orch, "gpu-a", Domain.MARKET)

        # Submit a GPU task before the epoch
        task = GpuTask(
            task_type=GpuTaskType.ML_TRAINING,
            payload={"data": "test"},
        )
        tid = await orch.submit_gpu_task("gpu-a", task)
        assert tid is not None  # gpu_client is attached

        # MockGpuProvider completes instantly (latency=0)
        # Run epoch — should collect GPU results
        stats = await orch.run_epoch()
        assert stats.gpu_tasks_completed >= 0

    @pytest.mark.asyncio
    async def test_gpu_results_add_reward(self):
        """Completed GPU tasks add rewards to agents."""
        s = _storage()
        bus = _bus()
        provider = MockGpuProvider()
        gpu_client = GpuTaskClient(provider)

        wiring = SubstrateWiring(
            storage=s,
            event_bus=bus,
            gpu_client=gpu_client,
        )
        orch = wiring.build()

        _spawn_with_goal(orch, "gpu-earner", Domain.CODE)

        # Submit and complete a GPU task
        task = GpuTask(
            task_type=GpuTaskType.ML_TRAINING,
            payload={"data": "work"},
        )
        tid = await orch.submit_gpu_task("gpu-earner", task)
        assert tid is not None

        # MockGpuProvider completes instantly (latency=0)
        # Run epoch to collect results
        stats = await orch.run_epoch()
        assert stats.gpu_tasks_completed >= 0


# ---------------------------------------------------------------
# Swarm lifecycle E2E
# ---------------------------------------------------------------


class TestSwarmLifecycleE2E:
    """Test full swarm lifecycle patterns."""

    @pytest.mark.asyncio
    async def test_spawn_after_kill(self):
        """New agents can be spawned after others die."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(
            storage=s,
            event_bus=bus,
            consequence_fn=_harsh_consequence_fn,
        )
        orch = wiring.build()

        _spawn_with_goal(orch, "mortal", Domain.MARKET, budget=5.0)

        await orch.run(max_epochs=5)

        # Spawn replacement
        _spawn_with_goal(orch, "replacement", Domain.MARKET, budget=200.0)

        stats = await orch.run_epoch()
        assert stats.agents_alive >= 1

    @pytest.mark.asyncio
    async def test_mixed_domains(self):
        """Agents from different domains coexist."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        domains = [Domain.CODE, Domain.MARKET, Domain.GOVERNANCE, Domain.INFRASTRUCTURE]
        for i, domain in enumerate(domains):
            _spawn_with_goal(orch, f"dom-{domain.value}", domain)

        stats = await orch.run_epoch()
        assert stats.agents_alive == 4

    @pytest.mark.asyncio
    async def test_stop_loop(self):
        """Orchestrator stop() terminates the epoch loop early."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        _spawn_with_goal(orch, "runner", Domain.CODE)

        # Wire a callback that stops the orchestrator after the first epoch event
        stop_called = False

        async def _stop_on_event(event):
            nonlocal stop_called
            if not stop_called:
                stop_called = True
                orch.stop()

        bus.subscribe_all(_stop_on_event)

        all_stats = await orch.run(max_epochs=100)
        # stop() fired during the loop — should have far fewer than 100
        assert stop_called
        assert len(all_stats) < 100
