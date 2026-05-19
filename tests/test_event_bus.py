"""Tests for the async event bus."""

import json

import pytest

from swarm.event_bus.bus import AsyncEventBus
from swarm.event_bus.events import BusEvent, EventType


@pytest.fixture
def bus(tmp_path):
    return AsyncEventBus(log_dir=str(tmp_path))


class TestAsyncEventBus:
    """INV-EVENTBUS-001: Event bus delivers typed events to subscribers."""

    @pytest.mark.asyncio
    async def test_typed_subscription(self, bus):
        received = []

        async def handler(event: BusEvent):
            received.append(event)

        bus.subscribe(EventType.AGENT_DEATH, handler)

        event = BusEvent(
            event_type=EventType.AGENT_DEATH,
            agent_id="agent-1",
            layer="TEST",
            payload={"reason": "killed"},
        )
        await bus.publish(event)

        assert len(received) == 1
        assert received[0].agent_id == "agent-1"

    @pytest.mark.asyncio
    async def test_wildcard_subscription(self, bus):
        received = []

        async def handler(event: BusEvent):
            received.append(event)

        bus.subscribe_all(handler)

        await bus.publish(
            BusEvent(event_type=EventType.AGENT_DEATH, agent_id="a1", layer="T")
        )
        await bus.publish(
            BusEvent(event_type=EventType.GOAL_DIED, agent_id="a2", layer="T")
        )

        assert len(received) == 2

    @pytest.mark.asyncio
    async def test_journal_persistence(self, bus, tmp_path):
        event = BusEvent(
            event_type=EventType.EPOCH_ADVANCED,
            agent_id="world",
            layer="WORLD_SIM",
            payload={"epoch": 42},
        )
        await bus.publish(event)

        # Check that a JSONL file was written in the log_dir
        import glob
        jsonl_files = glob.glob(str(tmp_path / "*.jsonl"))
        assert len(jsonl_files) >= 1
        with open(jsonl_files[0]) as f:
            lines = f.readlines()
        assert len(lines) >= 1
        data = json.loads(lines[0])
        assert data["event_type"] == "EPOCH_ADVANCED"

    @pytest.mark.asyncio
    async def test_handler_error_isolation(self, bus):
        """A failing handler must not prevent other handlers from running."""
        good_received = []

        async def bad_handler(event: BusEvent):
            raise RuntimeError("boom")

        async def good_handler(event: BusEvent):
            good_received.append(event)

        bus.subscribe(EventType.GOAL_MUTATED, bad_handler)
        bus.subscribe(EventType.GOAL_MUTATED, good_handler)

        await bus.publish(
            BusEvent(event_type=EventType.GOAL_MUTATED, agent_id="a", layer="T")
        )

        assert len(good_received) == 1

    @pytest.mark.asyncio
    async def test_unsubscribed_events_not_delivered(self, bus):
        received = []

        async def handler(event: BusEvent):
            received.append(event)

        bus.subscribe(EventType.SCAR_RECORDED, handler)

        await bus.publish(
            BusEvent(event_type=EventType.GOAL_DIED, agent_id="a", layer="T")
        )

        assert len(received) == 0
