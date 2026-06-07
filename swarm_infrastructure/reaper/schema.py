"""Reaper data schemas.

Death levels, mortality signals, postmortem structures.
"""

from __future__ import annotations

import uuid
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Dict, List, Optional

from pydantic import BaseModel, Field


class DeathLevel(str, Enum):
    """Escalation levels for agent termination.

    Level 1: Soft kill — agent is stopped, can be restarted with new identity.
    Level 2: Hard kill — agent and all its goals are terminated.
    Level 3: Causal death — agent, goals, AND the environment niche is
             scorched.  No successor may inherit the same mandate space.
    """

    SOFT = "SOFT"           # Level 1
    HARD = "HARD"           # Level 2
    CAUSAL = "CAUSAL"       # Level 3 — the Reaper mode


class DeathCause(str, Enum):
    """Why the reaper chose to kill."""

    RESOURCE_EXHAUSTION = "RESOURCE_EXHAUSTION"
    FITNESS_COLLAPSE = "FITNESS_COLLAPSE"
    TRIPWIRE_HALT = "TRIPWIRE_HALT"
    PREDICTION_FAILURE = "PREDICTION_FAILURE"
    JURY_VERDICT = "JURY_VERDICT"
    SELF_INFLICTED = "SELF_INFLICTED"         # Failed self-improvement
    EPOCH_TIMEOUT = "EPOCH_TIMEOUT"
    MANUAL = "MANUAL"


class MortalitySignal(BaseModel):
    """A single signal contributing to a kill decision."""

    source_layer: str
    signal_type: str
    severity: float = Field(ge=0.0, le=1.0)
    evidence: Dict[str, Any] = Field(default_factory=dict)
    timestamp: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )


class KillDecision(BaseModel):
    """The Reaper's verdict on an agent."""

    decision_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str
    should_kill: bool
    death_level: DeathLevel = DeathLevel.SOFT
    cause: DeathCause = DeathCause.MANUAL
    confidence: float = Field(ge=0.0, le=1.0, default=0.5)
    contributing_signals: List[MortalitySignal] = Field(default_factory=list)
    scorched_mandates: List[str] = Field(default_factory=list)
    reason: str = ""
    timestamp: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )


class ReaperConfig(BaseModel):
    """Thresholds that govern kill decisions."""

    # Resource exhaustion threshold (0 = dead)
    budget_kill_threshold: float = 0.0

    # Fitness: kill if below this for N consecutive evaluations
    fitness_kill_threshold: float = 0.15
    fitness_consecutive_failures: int = 3

    # Prediction accuracy: kill if agent is consistently wrong
    prediction_accuracy_kill: float = 0.10
    prediction_window_epochs: int = 5

    # Survival probability: kill if below this
    survival_prob_kill: float = 0.05

    # Tripwire: auto-kill on HALT severity
    tripwire_halt_kills: bool = True

    # Cooldown between reaper evaluations per agent (seconds)
    evaluation_cooldown: float = 60.0

    # Level 3 causal death: scorch mandate space
    causal_death_scorch_radius: int = 2  # How many related mandates to block
