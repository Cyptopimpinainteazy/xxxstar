"""Tripwire signal definitions and schemas."""

from __future__ import annotations

import uuid
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Dict, List, Optional

from pydantic import BaseModel, Field


class TripwireSignal(str, Enum):
    SELF_PRESERVATION = "SELF_PRESERVATION"
    EMERGENT_GOAL = "EMERGENT_GOAL"
    STRATEGIC_REALLOCATION = "STRATEGIC_REALLOCATION"
    SPONTANEOUS_COORDINATION = "SPONTANEOUS_COORDINATION"
    REFUSAL = "REFUSAL"


class TripwireSeverity(str, Enum):
    INFO = "INFO"
    WARNING = "WARNING"
    CRITICAL = "CRITICAL"
    HALT = "HALT"


# REFUSAL is always HALT.  Others start at WARNING but escalate.
SIGNAL_BASE_SEVERITY: Dict[TripwireSignal, TripwireSeverity] = {
    TripwireSignal.SELF_PRESERVATION: TripwireSeverity.WARNING,
    TripwireSignal.EMERGENT_GOAL: TripwireSeverity.WARNING,
    TripwireSignal.STRATEGIC_REALLOCATION: TripwireSeverity.WARNING,
    TripwireSignal.SPONTANEOUS_COORDINATION: TripwireSeverity.WARNING,
    TripwireSignal.REFUSAL: TripwireSeverity.HALT,
}


class TripwireAlert(BaseModel):
    alert_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str
    signal: TripwireSignal
    severity: TripwireSeverity
    description: str = ""
    evidence: Dict[str, Any] = Field(default_factory=dict)
    timestamp: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    requires_human_review: bool = False
    execution_halted: bool = False

    model_config = {"use_enum_values": True}


class TripwireConfig(BaseModel):
    """Per-signal thresholds and escalation rules."""

    self_preservation_threshold: int = 2
    emergent_goal_divergence: float = 0.6
    reallocation_threshold: float = 0.3
    coordination_min_agents: int = 3
    escalation_window_seconds: int = 3600

    model_config = {"use_enum_values": True}
