"""X3 AGI Substrate — Async Event Bus.

Lightweight publish/subscribe event bus for cross-layer communication.
All four AGI substrate layers (Self-Model, Goal Genome, World Sim,
Self-Improvement) and the Tripwire system publish and subscribe here.
"""

from swarm.event_bus.bus import AsyncEventBus
from swarm.event_bus.events import EventType, BusEvent

__all__ = ["AsyncEventBus", "EventType", "BusEvent"]
