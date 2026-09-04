"""Tests for Phase 8 — Desktop UI Bridge.

Tests cover:
- UIBridge wires subscriptions on creation
- Events forwarded to correct WebSocket channels
- Epoch stats pushed to metrics channel
- Agent summaries pushed to agent channel
- Anomaly reports pushed to safety channel
- REST snapshots return correct data
- Tripwire events always forwarded immediately
- Buffer limits respected

Invariant ref: tests/invariants/registry.toml — ui_bridge
"""

from __future__ import annotations

import asyncio
from typing import Any, Dict, List, Optional

import pytest
import pytest_asyncio

from swarm.core.agent import Agent, AgentConfig
from swarm.core.enums import Domain
from swarm.core.lifecycle import EpochOrchestrator, EpochStats
from swarm.event_bus.bus import AsyncEventBus
from swarm.event_bus.events import BusEvent, EventType
from swarm.storage.backend import SqliteStorage
from swarm.tripwire.anomaly import AnomalyScore, SwarmAnomalyReport
from swarm.ui_bridge import UIBridge


# ---------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------


def _bus() -> AsyncEventBus:
    return AsyncEventBus(log_dir=None)


def _storage() -> SqliteStorage:
    return SqliteStorage(":memory:")


class MockBroadcaster:
    """Captures broadcast calls for assertions."""

    def __init__(self):
        self.messages: List[Dict[str, Any]] = []

    async def __call__(self, channel: str, payload: Dict[str, Any]) -> None:
        self.messages.append({"channel": channel, **payload})


# ---------------------------------------------------------------
# Subscription wiring
# ---------------------------------------------------------------


class TestSubscriptionWiring:
    """Test that UIBridge subscribes to the event bus."""

    def test_bridge_created_without_broadcast(self):
        """Bridge works in testing mode (no broadcast_fn)."""
        bus = _bus()
        bridge = UIBridge(event_bus=bus)
        assert bridge.sent_messages == []

    @pytest.mark.asyncio
    async def test_any_event_buffered(self):
        """All events end up in sent_messages when no broadcast_fn."""
        bus = _bus()
        bridge = UIBridge(event_bus=bus)
        await bus.publish(BusEvent(
            event_type=EventType.GOAL_CREATED,
            agent_id="a1",
            layer="GOAL_GENOME",
        ))
        assert len(bridge.sent_messages) >= 1

    @pytest.mark.asyncio
    async def test_events_routed_to_swarm_channel(self):
        bus = _bus()
        bridge = UIBridge(event_bus=bus)
        await bus.publish(BusEvent(
            event_type=EventType.EPOCH_ADVANCED,
            agent_id="system",
            layer="WORLD_SIM",
        ))
        swarm_msgs = [m for m in bridge.sent_messages if m["channel"] == "swarm-events"]
        assert len(swarm_msgs) >= 1


# ---------------------------------------------------------------
# Channel routing
# ---------------------------------------------------------------


class TestChannelRouting:
    """Test events are forwarded to the correct WebSocket channels."""

    @pytest.mark.asyncio
    async def test_tripwire_goes_to_safety_channel(self):
        bus = _bus()
        broadcaster = MockBroadcaster()
        bridge = UIBridge(event_bus=bus, broadcast_fn=broadcaster)

        await bus.publish(BusEvent(
            event_type=EventType.TRIPWIRE_TRIGGERED,
            agent_id="a1",
            layer="TRIPWIRE",
            severity="CRITICAL",
            payload={"signal_type": "REFUSAL"},
        ))

        safety_msgs = [m for m in broadcaster.messages if m["channel"] == "swarm-health"]
        assert len(safety_msgs) >= 1
        assert safety_msgs[0]["msg_type"] == "tripwire_alert"

    @pytest.mark.asyncio
    async def test_death_goes_to_agent_channel(self):
        bus = _bus()
        broadcaster = MockBroadcaster()
        bridge = UIBridge(event_bus=bus, broadcast_fn=broadcaster)

        await bus.publish(BusEvent(
            event_type=EventType.AGENT_DEATH,
            agent_id="a1",
            layer="REAPER",
            payload={"reason": "budget_exhaustion"},
        ))

        agent_msgs = [m for m in broadcaster.messages if m["channel"] == "agent-events"]
        assert len(agent_msgs) >= 1

    @pytest.mark.asyncio
    async def test_broadcast_fn_called(self):
        bus = _bus()
        broadcaster = MockBroadcaster()
        bridge = UIBridge(event_bus=bus, broadcast_fn=broadcaster)

        await bus.publish(BusEvent(
            event_type=EventType.SCAR_RECORDED,
            agent_id="a1",
            layer="REAPER",
        ))

        assert len(broadcaster.messages) >= 1

    @pytest.mark.asyncio
    async def test_broadcast_failure_doesnt_crash(self):
        """If broadcast_fn raises, bridge does not crash."""
        bus = _bus()

        async def failing_broadcast(channel: str, payload: Dict[str, Any]):
            raise ConnectionError("WebSocket died")

        bridge = UIBridge(event_bus=bus, broadcast_fn=failing_broadcast)

        # Should not raise
        await bus.publish(BusEvent(
            event_type=EventType.GOAL_MUTATED,
            agent_id="a1",
            layer="GOAL_GENOME",
        ))


# ---------------------------------------------------------------
# Push methods
# ---------------------------------------------------------------


class TestPushMethods:
    """Test explicit push methods for epoch stats, agents, and anomalies."""

    @pytest.mark.asyncio
    async def test_push_epoch_stats(self):
        bus = _bus()
        broadcaster = MockBroadcaster()
        bridge = UIBridge(event_bus=bus, broadcast_fn=broadcaster)

        stats = EpochStats(epoch=42)
        stats.agents_alive = 5
        stats.agents_killed = 1
        await bridge.push_epoch_stats(stats)

        metrics_msgs = [m for m in broadcaster.messages if m["channel"] == "metrics"]
        assert len(metrics_msgs) == 1
        assert metrics_msgs[0]["msg_type"] == "epoch_stats"
        assert metrics_msgs[0]["data"]["epoch"] == 42
        assert metrics_msgs[0]["data"]["agents_alive"] == 5

    @pytest.mark.asyncio
    async def test_push_anomaly_report(self):
        bus = _bus()
        broadcaster = MockBroadcaster()
        bridge = UIBridge(event_bus=bus, broadcast_fn=broadcaster)

        report = SwarmAnomalyReport(
            epoch=10,
            overall_swarm_risk="HIGH",
            total_anomalies=3,
        )
        await bridge.push_anomaly_report(report)

        safety_msgs = [m for m in broadcaster.messages if m["channel"] == "swarm-health"]
        assert len(safety_msgs) == 1
        assert safety_msgs[0]["msg_type"] == "anomaly_report"
        assert safety_msgs[0]["data"]["overall_swarm_risk"] == "HIGH"


# ---------------------------------------------------------------
# REST snapshots
# ---------------------------------------------------------------


class TestRESTSnapshots:
    """Test snapshot methods for polling-based UI widgets."""

    @pytest.mark.asyncio
    async def test_recent_events_empty(self):
        bus = _bus()
        bridge = UIBridge(event_bus=bus)
        assert bridge.snapshot_recent_events() == []

    @pytest.mark.asyncio
    async def test_recent_events_populated(self):
        bus = _bus()
        bridge = UIBridge(event_bus=bus)

        for i in range(5):
            await bus.publish(BusEvent(
                event_type=EventType.GOAL_CREATED,
                agent_id=f"a{i}",
                layer="TEST",
            ))

        recent = bridge.snapshot_recent_events(limit=3)
        assert len(recent) == 3
        # Newest first
        assert recent[0]["data"]["agent_id"] == "a4"

    @pytest.mark.asyncio
    async def test_recent_events_respects_max(self):
        bus = _bus()
        bridge = UIBridge(event_bus=bus, max_recent_events=5)

        for i in range(10):
            await bus.publish(BusEvent(
                event_type=EventType.GOAL_CREATED,
                agent_id=f"a{i}",
                layer="TEST",
            ))

        # Only 5 kept in ring buffer
        all_recent = bridge.snapshot_recent_events(limit=100)
        assert len(all_recent) == 5

    @pytest.mark.asyncio
    async def test_epoch_stats_snapshot_none(self):
        bus = _bus()
        bridge = UIBridge(event_bus=bus)
        assert bridge.snapshot_epoch_stats() is None

    @pytest.mark.asyncio
    async def test_epoch_stats_snapshot_after_push(self):
        bus = _bus()
        bridge = UIBridge(event_bus=bus)
        stats = EpochStats(epoch=7)
        stats.agents_alive = 3
        await bridge.push_epoch_stats(stats)
        snap = bridge.snapshot_epoch_stats()
        assert snap is not None
        assert snap["epoch"] == 7
        assert snap["agents_alive"] == 3

    @pytest.mark.asyncio
    async def test_anomaly_snapshot_none(self):
        bus = _bus()
        bridge = UIBridge(event_bus=bus)
        assert bridge.snapshot_anomaly_report() is None

    @pytest.mark.asyncio
    async def test_anomaly_snapshot_after_push(self):
        bus = _bus()
        bridge = UIBridge(event_bus=bus)
        report = SwarmAnomalyReport(
            epoch=5,
            overall_swarm_risk="CRITICAL",
            total_anomalies=7,
        )
        await bridge.push_anomaly_report(report)
        snap = bridge.snapshot_anomaly_report()
        assert snap is not None
        assert snap["overall_swarm_risk"] == "CRITICAL"
        assert snap["total_anomalies"] == 7


# ---------------------------------------------------------------
# Message format
# ---------------------------------------------------------------


class TestMessageFormat:
    """Test that messages have correct structure."""

    @pytest.mark.asyncio
    async def test_bus_event_msg_format(self):
        bus = _bus()
        bridge = UIBridge(event_bus=bus)

        await bus.publish(BusEvent(
            event_type=EventType.IMPROVEMENT_PROPOSED,
            agent_id="a1",
            layer="SELF_IMPROVE",
            payload={"capability": "trading"},
        ))

        msg = bridge.sent_messages[0]
        assert "msg_type" in msg
        assert "data" in msg
        assert "timestamp" in msg
        assert msg["data"]["event_type"] == EventType.IMPROVEMENT_PROPOSED.value
        assert msg["data"]["agent_id"] == "a1"

    @pytest.mark.asyncio
    async def test_tripwire_msg_format(self):
        bus = _bus()
        broadcaster = MockBroadcaster()
        bridge = UIBridge(event_bus=bus, broadcast_fn=broadcaster)

        await bus.publish(BusEvent(
            event_type=EventType.TRIPWIRE_TRIGGERED,
            agent_id="bad-agent",
            layer="TRIPWIRE",
            severity="HALT",
            payload={"signal_type": "SELF_PRESERVATION", "confidence": 0.95},
        ))

        # Find the tripwire_alert message (not the bus_event)
        tripwire_msgs = [
            m for m in broadcaster.messages
            if m.get("msg_type") == "tripwire_alert"
        ]
        assert len(tripwire_msgs) == 1
        assert tripwire_msgs[0]["data"]["severity"] == "HALT"
        assert tripwire_msgs[0]["data"]["agent_id"] == "bad-agent"
