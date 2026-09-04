"""Desktop UI Bridge — connects AGI substrate to the React desktop app.

Provides:
1. WebSocket broadcaster: Forwards event bus events → WebSocket channels
2. REST snapshot endpoints: Query-able substrate state for UI widgets
3. Typed message schemas: JSON payloads the React frontend can parse

Communication flow:
    Event Bus → UIBridge → WebSocket → React Desktop

This does NOT depend on aiohttp at import time. It accepts a broadcast
callback so it works with any WebSocket implementation (or a mock).

NON-NEGOTIABLE:
- HALT-severity events are always forwarded immediately
- No event data is filtered or sanitized (UI sees everything)
- All snapshots include the epoch they were generated for
"""

from __future__ import annotations

import time
from collections import deque
from typing import Any, Awaitable, Callable, Deque, Dict, List, Optional

from swarm.causal.graph import CausalGraph
from swarm.core.lifecycle import EpochOrchestrator, EpochStats
from swarm.event_bus.bus import AsyncEventBus
from swarm.event_bus.events import BusEvent, EventType
from swarm.tripwire.anomaly import BehavioralAnomalyDetector, SwarmAnomalyReport

# Type alias: async broadcast function (channel, payload) → None
BroadcastFn = Callable[[str, Dict[str, Any]], Awaitable[None]]


# ──────────────────────────────────────────────────────────────────
# Message schemas (JSON-safe dicts sent over WebSocket)
# ──────────────────────────────────────────────────────────────────

def _epoch_stats_msg(stats: EpochStats) -> Dict[str, Any]:
    """Convert EpochStats to a WebSocket-ready dict."""
    return {
        "msg_type": "epoch_stats",
        "data": stats.to_dict(),
        "timestamp": time.time(),
    }


def _agent_summary_msg(
    agent_id: str,
    is_alive: bool,
    resource_budget: float,
    fitness: float,
    scar_count: int,
    active_goals: int,
) -> Dict[str, Any]:
    return {
        "msg_type": "agent_summary",
        "data": {
            "agent_id": agent_id,
            "is_alive": is_alive,
            "resource_budget": round(resource_budget, 4),
            "fitness": round(fitness, 4),
            "scar_count": scar_count,
            "active_goals": active_goals,
        },
        "timestamp": time.time(),
    }


def _anomaly_report_msg(report: SwarmAnomalyReport) -> Dict[str, Any]:
    return {
        "msg_type": "anomaly_report",
        "data": report.model_dump(mode="json"),
        "timestamp": time.time(),
    }


def _tripwire_msg(event: BusEvent) -> Dict[str, Any]:
    return {
        "msg_type": "tripwire_alert",
        "data": {
            "agent_id": event.agent_id,
            "severity": event.severity,
            "payload": event.payload,
        },
        "timestamp": time.time(),
    }


def _bus_event_msg(event: BusEvent) -> Dict[str, Any]:
    return {
        "msg_type": "bus_event",
        "data": {
            "event_id": event.event_id,
            "event_type": event.event_type,
            "agent_id": event.agent_id,
            "severity": event.severity,
            "layer": event.layer,
            "payload": event.payload,
        },
        "timestamp": time.time(),
    }


# ──────────────────────────────────────────────────────────────────
# UIBridge
# ──────────────────────────────────────────────────────────────────

class UIBridge:
    """Bridges the AGI substrate to the desktop React frontend.

    Connects to the event bus and forwards events over WebSocket
    channels for real-time UI updates.

    Args:
        event_bus: The shared async event bus.
        broadcast_fn: Async callback ``(channel, payload)`` that sends
                      a JSON message to all WebSocket subscribers of
                      that channel.  Pass ``None`` for testing
                      (events are buffered in ``sent_messages``).
        max_recent_events: How many recent events to keep for REST snapshots.
    """

    # WebSocket channels
    CH_SWARM = "swarm-events"
    CH_AGENT = "agent-events"
    CH_TRIPWIRE = "swarm-health"
    CH_METRICS = "metrics"

    def __init__(
        self,
        event_bus: AsyncEventBus,
        broadcast_fn: Optional[BroadcastFn] = None,
        max_recent_events: int = 200,
    ) -> None:
        self._bus = event_bus
        self._broadcast = broadcast_fn
        self._max_recent = max_recent_events

        # Buffer for testing (when no broadcast_fn)
        self.sent_messages: List[Dict[str, Any]] = []

        # Recent events ring buffer for REST snapshot
        self._recent_events: Deque[Dict[str, Any]] = deque(maxlen=max_recent_events)

        # Last epoch stats
        self._last_epoch_stats: Optional[EpochStats] = None

        # Last anomaly report
        self._last_anomaly_report: Optional[SwarmAnomalyReport] = None

        # Wire subscriptions
        self._wire()

    def _wire(self) -> None:
        """Subscribe to event bus for UI forwarding."""
        # All events → recent buffer + swarm channel
        self._bus.subscribe_all(self._on_any_event)

        # Tripwire → dedicated safety channel
        self._bus.subscribe(EventType.TRIPWIRE_TRIGGERED, self._on_tripwire)

        # Agent death → agent channel
        self._bus.subscribe(EventType.AGENT_DEATH, self._on_agent_death)

    # ------------------------------------------------------------------
    # Event handlers
    # ------------------------------------------------------------------

    async def _on_any_event(self, event: BusEvent) -> None:
        """Forward every event to the swarm-events channel."""
        msg = _bus_event_msg(event)
        self._recent_events.append(msg)
        await self._send(self.CH_SWARM, msg)

    async def _on_tripwire(self, event: BusEvent) -> None:
        """Forward tripwire alerts to the safety channel."""
        msg = _tripwire_msg(event)
        await self._send(self.CH_TRIPWIRE, msg)

    async def _on_agent_death(self, event: BusEvent) -> None:
        """Forward agent deaths to the agent channel."""
        msg = _bus_event_msg(event)
        await self._send(self.CH_AGENT, msg)

    # ------------------------------------------------------------------
    # Push methods (called by orchestrator / wiring)
    # ------------------------------------------------------------------

    async def push_epoch_stats(self, stats: EpochStats) -> None:
        """Push epoch stats to the metrics channel."""
        self._last_epoch_stats = stats
        msg = _epoch_stats_msg(stats)
        await self._send(self.CH_METRICS, msg)

    async def push_anomaly_report(self, report: SwarmAnomalyReport) -> None:
        """Push an anomaly report to the safety channel."""
        self._last_anomaly_report = report
        msg = _anomaly_report_msg(report)
        await self._send(self.CH_TRIPWIRE, msg)

    async def push_agent_summaries(
        self,
        orchestrator: EpochOrchestrator,
    ) -> None:
        """Push per-agent summaries to the agent channel."""
        for agent in list(orchestrator._agents.values()):
            fh = agent.fitness_history
            fitness = fh[-1] if fh else 1.0
            msg = _agent_summary_msg(
                agent_id=agent.agent_id,
                is_alive=agent.is_alive,
                resource_budget=agent.resource_budget,
                fitness=fitness,
                scar_count=len(agent.scars),
                active_goals=len(agent.goal_genome.get_active_goals()),
            )
            await self._send(self.CH_AGENT, msg)

    # ------------------------------------------------------------------
    # REST snapshots (for polling-based UI widgets)
    # ------------------------------------------------------------------

    def snapshot_recent_events(self, limit: int = 50) -> List[Dict[str, Any]]:
        """Return the most recent events (newest first)."""
        events = list(self._recent_events)
        events.reverse()
        return events[:limit]

    def snapshot_epoch_stats(self) -> Optional[Dict[str, Any]]:
        """Return the last epoch stats, or None."""
        if self._last_epoch_stats is None:
            return None
        return self._last_epoch_stats.to_dict()

    def snapshot_anomaly_report(self) -> Optional[Dict[str, Any]]:
        """Return the last anomaly report, or None."""
        if self._last_anomaly_report is None:
            return None
        return self._last_anomaly_report.model_dump(mode="json")

    # ------------------------------------------------------------------
    # Internal
    # ------------------------------------------------------------------

    async def _send(self, channel: str, msg: Dict[str, Any]) -> None:
        """Send a message via broadcast_fn, or buffer it for testing."""
        if self._broadcast is not None:
            try:
                await self._broadcast(channel, msg)
            except Exception:
                pass  # Back-pressure: don't crash the substrate
        else:
            # Testing mode: buffer messages
            self.sent_messages.append({"channel": channel, **msg})
