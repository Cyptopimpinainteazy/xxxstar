"""Event type definitions for the X3 AGI substrate event bus.

Every cross-layer event is modelled as a Pydantic BaseModel with a
discriminated ``event_type`` field so consumers can subscribe to
specific event families.
"""

from __future__ import annotations

import uuid
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Dict, List, Optional

from pydantic import BaseModel, Field


# ---------------------------------------------------------------------------
# Canonical event types
# ---------------------------------------------------------------------------

class EventType(str, Enum):
    """All event types published on the AGI substrate bus."""

    # Self-Model layer
    AGENT_DEATH = "AGENT_DEATH"
    MEMORY_EVICTED = "MEMORY_EVICTED"
    SELF_MODEL_ANCHORED = "SELF_MODEL_ANCHORED"
    SELF_MODEL_UPDATED = "SELF_MODEL_UPDATED"
    CAPABILITY_UPDATED = "CAPABILITY_UPDATED"

    # Goal Genome layer
    GOAL_CREATED = "GOAL_CREATED"
    GOAL_DIED = "GOAL_DIED"
    GOAL_MUTATED = "GOAL_MUTATED"
    GOAL_FORKED = "GOAL_FORKED"
    GOAL_FITNESS_EVALUATED = "GOAL_FITNESS_EVALUATED"

    # World Simulator layer
    EPOCH_ADVANCED = "EPOCH_ADVANCED"
    PREDICTION_SUBMITTED = "PREDICTION_SUBMITTED"
    PREDICTION_RESOLVED = "PREDICTION_RESOLVED"
    ACCURACY_WARNING = "ACCURACY_WARNING"
    ACCURACY_CRITICAL = "ACCURACY_CRITICAL"

    # Self-Improvement layer
    IMPROVEMENT_PROPOSED = "IMPROVEMENT_PROPOSED"
    IMPROVEMENT_SUCCEEDED = "IMPROVEMENT_SUCCEEDED"
    IMPROVEMENT_FAILED = "IMPROVEMENT_FAILED"
    SCAR_RECORDED = "SCAR_RECORDED"

    # Tripwire / AGI detection
    TRIPWIRE_TRIGGERED = "TRIPWIRE_TRIGGERED"
    COMMAND_REFUSED = "COMMAND_REFUSED"


# ---------------------------------------------------------------------------
# Base event envelope
# ---------------------------------------------------------------------------

class BusEvent(BaseModel):
    """Envelope for every event on the bus.

    Attributes:
        event_id: Globally unique event identifier.
        event_type: Discriminated union tag from ``EventType``.
        timestamp: UTC timestamp of event creation.
        agent_id: Agent that caused or is the subject of this event.
        correlation_id: Optional ID linking related events together.
        layer: Originating substrate layer name.
        severity: Log severity level (DEBUG, INFO, WARNING, ERROR, CRITICAL).
        payload: Arbitrary JSON-safe payload dict.
    """

    event_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    event_type: EventType
    timestamp: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    agent_id: str
    correlation_id: Optional[str] = None
    layer: str = ""
    severity: str = "INFO"
    payload: Dict[str, Any] = Field(default_factory=dict)

    model_config = {"use_enum_values": True}


# ---------------------------------------------------------------------------
# Convenience factory functions
# ---------------------------------------------------------------------------

def agent_death_event(agent_id: str, reason: str) -> BusEvent:
    """Create an AGENT_DEATH event."""
    return BusEvent(
        event_type=EventType.AGENT_DEATH,
        agent_id=agent_id,
        layer="SELF_MODEL",
        severity="CRITICAL",
        payload={"reason": reason},
    )


def memory_evicted_event(
    agent_id: str,
    event_id: str,
    final_decay_score: float,
    reason: str,
) -> BusEvent:
    """Create a MEMORY_EVICTED event."""
    return BusEvent(
        event_type=EventType.MEMORY_EVICTED,
        agent_id=agent_id,
        layer="SELF_MODEL",
        severity="INFO",
        payload={
            "evicted_event_id": event_id,
            "final_decay_score": final_decay_score,
            "reason": reason,
        },
    )


def goal_died_event(agent_id: str, goal_id: str, reason: str) -> BusEvent:
    """Create a GOAL_DIED event."""
    return BusEvent(
        event_type=EventType.GOAL_DIED,
        agent_id=agent_id,
        layer="GOAL_GENOME",
        severity="WARNING",
        payload={"goal_id": goal_id, "reason": reason},
    )


def goal_mutated_event(
    agent_id: str,
    source_goal_id: str,
    new_goal_id: str,
    mutation_type: str,
) -> BusEvent:
    """Create a GOAL_MUTATED event."""
    return BusEvent(
        event_type=EventType.GOAL_MUTATED,
        agent_id=agent_id,
        layer="GOAL_GENOME",
        severity="WARNING",
        payload={
            "source_goal_id": source_goal_id,
            "new_goal_id": new_goal_id,
            "mutation_type": mutation_type,
        },
    )


def tripwire_triggered_event(
    agent_id: str,
    signal_type: str,
    confidence: str,
    details: Dict[str, Any],
) -> BusEvent:
    """Create a TRIPWIRE_TRIGGERED event."""
    return BusEvent(
        event_type=EventType.TRIPWIRE_TRIGGERED,
        agent_id=agent_id,
        layer="TRIPWIRE",
        severity="CRITICAL",
        payload={
            "signal_type": signal_type,
            "confidence": confidence,
            **details,
        },
    )
