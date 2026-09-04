"""Pydantic data models for the Goal Genome layer.

NON-NEGOTIABLE: Goals die.  ``is_alive`` transitions False permanently.
No resurrection path exists or should be created.
"""

from __future__ import annotations

import uuid
from datetime import datetime, timezone
from enum import Enum
from typing import List, Optional

from pydantic import BaseModel, Field

from swarm.core.enums import Domain


# ---------------------------------------------------------------------------
# Enums
# ---------------------------------------------------------------------------


class MutationType(str, Enum):
    FORK = "FORK"
    DRIFT = "DRIFT"
    INVERSION = "INVERSION"
    RECOMBINATION = "RECOMBINATION"


class MutationTrigger(str, Enum):
    REPEATED_FAILURE = "REPEATED_FAILURE"
    COST_EXCEEDED = "COST_EXCEEDED"
    ENVIRONMENT_SHIFT = "ENVIRONMENT_SHIFT"
    RANDOM = "RANDOM"


class Recommendation(str, Enum):
    CONTINUE = "CONTINUE"
    MUTATE = "MUTATE"
    KILL = "KILL"


# ---------------------------------------------------------------------------
# Core models
# ---------------------------------------------------------------------------


class Goal(BaseModel):
    """A living goal organism."""

    goal_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    parent_goal_id: Optional[str] = None
    generation: int = 0
    created_at: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )
    mandate: str
    domain: Domain
    fitness_score: float = Field(default=0.5, ge=0.0, le=1.0)
    pursuit_cost_cumulative: float = 0.0
    expected_reward: float = 0.0
    environmental_resistance: float = Field(default=0.0, ge=0.0, le=1.0)
    mutation_probability: float = Field(default=0.1, ge=0.0, le=1.0)
    is_alive: bool = True
    death_reason: Optional[str] = None
    lineage: List[str] = Field(default_factory=list)

    # Fitness history for consecutive-evaluation tracking
    fitness_history: List[float] = Field(default_factory=list)

    model_config = {"use_enum_values": True}


class GoalMutation(BaseModel):
    """Record of a goal mutation event."""

    mutation_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    source_goal_id: str
    mutated_goal_id: str
    mutation_type: MutationType
    mutation_trigger: MutationTrigger
    delta_description: str
    timestamp: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )

    model_config = {"use_enum_values": True}


class GoalFitnessReport(BaseModel):
    """Result of a fitness evaluation."""

    goal_id: str
    evaluation_period_seconds: int = 0
    successes: int = 0
    failures: int = 0
    resource_spent: float = 0.0
    reward_earned: float = 0.0
    computed_fitness: float = 0.0
    recommendation: Recommendation = Recommendation.CONTINUE

    model_config = {"use_enum_values": True}


class EnvironmentContext(BaseModel):
    """Snapshot of environment state passed to the mutator."""

    current_epoch: int = 0
    domain_metrics: dict = Field(default_factory=dict)
    active_goal_count: int = 0
    recent_mutations: List[str] = Field(default_factory=list)
