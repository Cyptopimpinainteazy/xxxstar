"""Tests for Phase 7 — Event Bus Wire-up.

Tests cover:
- SubstrateWiring creates all subsystems
- Event subscriptions established correctly
- Cross-layer event routing (death → anomaly, tripwire → log)
- Epoch events trigger anomaly scans
- Full epoch loop with all subsystems wired
- Event counting diagnostics

Invariant ref: tests/invariants/registry.toml — event_bus_wiring
"""

from __future__ import annotations

import asyncio
import uuid

import pytest
import pytest_asyncio

from swarm.causal.graph import CausalGraph
from swarm.causal.schema import CausalNode, NodeType
from swarm.core.agent import Agent, AgentConfig, Consequence
from swarm.core.enums import Domain
from swarm.core.lifecycle import EpochOrchestrator
from swarm.core.wiring import SubstrateWiring
from swarm.event_bus.bus import AsyncEventBus
from swarm.event_bus.events import BusEvent, EventType
from swarm.storage.backend import SqliteStorage
from swarm.tripwire.anomaly import BehavioralAnomalyDetector


# ---------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------


def _storage() -> SqliteStorage:
    return SqliteStorage(":memory:")


def _bus() -> AsyncEventBus:
    return AsyncEventBus(log_dir=None)


# ---------------------------------------------------------------
# Wiring construction
# ---------------------------------------------------------------


class TestWiringConstruction:
    """Test SubstrateWiring.build() creates and connects all subsystems."""

    def test_build_returns_orchestrator(self):
        wiring = SubstrateWiring(storage=_storage(), event_bus=_bus())
        orch = wiring.build()
        assert isinstance(orch, EpochOrchestrator)

    def test_all_subsystems_created(self):
        wiring = SubstrateWiring(storage=_storage(), event_bus=_bus())
        wiring.build()
        assert wiring.world_state is not None
        assert wiring.prediction_market is not None
        assert wiring.scoreboard is not None
        assert wiring.reaper is not None
        assert wiring.postmortem is not None
        assert wiring.scar_propagator is not None
        assert wiring.tripwire is not None
        assert wiring.causal_graph is not None
        assert wiring.anomaly_detector is not None

    def test_orchestrator_has_causal_graph(self):
        wiring = SubstrateWiring(storage=_storage(), event_bus=_bus())
        orch = wiring.build()
        assert orch.causal_graph is not None

    def test_anomaly_threshold_forwarded(self):
        wiring = SubstrateWiring(
            storage=_storage(),
            event_bus=_bus(),
            anomaly_threshold=0.42,
        )
        wiring.build()
        assert wiring.anomaly_detector._anomaly_threshold == pytest.approx(0.42)


# ---------------------------------------------------------------
# Event routing
# ---------------------------------------------------------------


class TestEventRouting:
    """Test cross-layer event routing through the bus."""

    @pytest.mark.asyncio
    async def test_event_counted(self):
        """Any published event gets counted."""
        bus = _bus()
        wiring = SubstrateWiring(storage=_storage(), event_bus=bus)
        wiring.build()

        event = BusEvent(
            event_type=EventType.AGENT_DEATH,
            agent_id="a1",
            layer="TEST",
            payload={"reason": "test"},
        )
        await bus.publish(event)
        assert wiring.event_counts.get(EventType.AGENT_DEATH.value, 0) >= 1

    @pytest.mark.asyncio
    async def test_total_events_increments(self):
        bus = _bus()
        wiring = SubstrateWiring(storage=_storage(), event_bus=bus)
        wiring.build()

        for _ in range(3):
            await bus.publish(
                BusEvent(
                    event_type=EventType.SCAR_RECORDED,
                    agent_id="a1",
                    layer="TEST",
                )
            )
        assert wiring.total_events_routed >= 3

    @pytest.mark.asyncio
    async def test_death_rebuilds_fingerprint(self):
        """AGENT_DEATH → anomaly detector rebuilds fingerprint."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        wiring.build()

        # Add some causal data for the agent
        cg = wiring.causal_graph
        cg.add_node(CausalNode(
            agent_id="a1", epoch=1,
            node_type=NodeType.ACTION, action_type="trade", value=1.0,
        ))

        # Publish death event
        await bus.publish(BusEvent(
            event_type=EventType.AGENT_DEATH,
            agent_id="a1",
            layer="REAPER",
            payload={"reason": "budget_exhaustion"},
        ))

        # Fingerprint should now exist
        assert "a1" in wiring.anomaly_detector._baselines

    @pytest.mark.asyncio
    async def test_tripwire_event_handled(self):
        """TRIPWIRE_TRIGGERED is counted."""
        bus = _bus()
        wiring = SubstrateWiring(storage=_storage(), event_bus=bus)
        wiring.build()

        await bus.publish(BusEvent(
            event_type=EventType.TRIPWIRE_TRIGGERED,
            agent_id="a1",
            layer="TRIPWIRE",
            severity="CRITICAL",
            payload={"signal_type": "REFUSAL"},
        ))
        assert wiring.event_counts.get(EventType.TRIPWIRE_TRIGGERED.value, 0) >= 1


# ---------------------------------------------------------------
# Anomaly scan integration
# ---------------------------------------------------------------


class TestAnomalyScanIntegration:
    """Test that epoch events trigger anomaly scans."""

    @pytest.mark.asyncio
    async def test_epoch_advance_triggers_scan(self):
        """EPOCH_ADVANCED event triggers anomaly scan when agents exist."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus, anomaly_threshold=0.3)
        orch = wiring.build()

        # Register a minimal agent
        config = AgentConfig(
            agent_id="a1",
            domain=Domain.MARKET,
            initial_mandates=["trade"],
            initial_budget=100.0,
        )
        agent = orch.spawn_agent(config)

        # Add baseline causal data
        cg = wiring.causal_graph
        for ep in range(1, 4):
            cg.add_node(CausalNode(
                agent_id="a1", epoch=ep,
                node_type=NodeType.ACTION, action_type="trade", value=1.0,
            ))
        wiring.anomaly_detector.build_fingerprint("a1")

        # Publish epoch advance
        await bus.publish(BusEvent(
            event_type=EventType.EPOCH_ADVANCED,
            agent_id="system",
            layer="WORLD_SIM",
            payload={"epoch": 4},
        ))

        # A scan report should have been persisted
        report = s.load("anomaly_detector", "swarm_report:4")
        assert report is not None
        assert report["epoch"] == 4


# ---------------------------------------------------------------
# Full epoch with wiring
# ---------------------------------------------------------------


class TestFullEpochWithWiring:
    """Test running a full epoch through wired subsystems."""

    @pytest.mark.asyncio
    async def test_single_epoch_completes(self):
        """One epoch with wired subsystems should complete."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        config = AgentConfig(
            agent_id="a1",
            domain=Domain.CODE,
            initial_mandates=["code"],
            initial_budget=50.0,
        )
        orch.spawn_agent(config)

        stats = await orch.run_epoch()
        assert stats.epoch >= 0
        assert stats.agents_alive >= 1

    @pytest.mark.asyncio
    async def test_events_published_through_bus(self):
        """After an epoch, the bus should have received events."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        config = AgentConfig(
            agent_id="a1",
            domain=Domain.MARKET,
            initial_mandates=["trade"],
            initial_budget=50.0,
        )
        orch.spawn_agent(config)

        await orch.run_epoch()
        # Bus should have recorded at least the epoch advance event
        assert bus.event_count >= 1

    @pytest.mark.asyncio
    async def test_multiple_agents_epoch(self):
        """Multiple agents run through a wired epoch."""
        s = _storage()
        bus = _bus()
        wiring = SubstrateWiring(storage=s, event_bus=bus)
        orch = wiring.build()

        for i in range(3):
            config = AgentConfig(
                agent_id=f"agent-{i}",
                domain=Domain.MARKET,
                initial_mandates=["trade"],
                initial_budget=50.0,
            )
            orch.spawn_agent(config)

        stats = await orch.run_epoch()
        assert stats.agents_alive == 3


# ---------------------------------------------------------------
# Diagnostics
# ---------------------------------------------------------------


class TestDiagnostics:
    """Test wiring diagnostic capabilities."""

    @pytest.mark.asyncio
    async def test_event_counts_by_type(self):
        bus = _bus()
        wiring = SubstrateWiring(storage=_storage(), event_bus=bus)
        wiring.build()

        await bus.publish(BusEvent(
            event_type=EventType.GOAL_CREATED,
            agent_id="a1",
            layer="GOAL_GENOME",
        ))
        await bus.publish(BusEvent(
            event_type=EventType.GOAL_CREATED,
            agent_id="a2",
            layer="GOAL_GENOME",
        ))
        await bus.publish(BusEvent(
            event_type=EventType.AGENT_DEATH,
            agent_id="a3",
            layer="REAPER",
        ))

        counts = wiring.event_counts
        assert counts.get(EventType.GOAL_CREATED.value, 0) == 2
        assert counts.get(EventType.AGENT_DEATH.value, 0) == 1

    def test_initial_counts_empty(self):
        wiring = SubstrateWiring(storage=_storage(), event_bus=_bus())
        wiring.build()
        assert wiring.total_events_routed == 0
