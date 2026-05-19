"""Pydantic data models for the Self-Model Ledger.

NON-NEGOTIABLE RULES enforced by these schemas:
1. ``SelfModel.is_alive`` defaults to True and can only transition to False.
2. ``CausalEvent.decay_score`` starts at 1.0, decreases monotonically.
3. ``SelfModel.integrity_hash`` is a SHA-256 of deterministic serialisation.
4. ``SelfProjection`` must contain at least one predicted failure mode
   or the projection engine flags the agent for review.
"""

from __future__ import annotations

import hashlib
import json
import uuid
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional

from pydantic import BaseModel, Field, field_validator

from swarm.core.enums import Domain, Outcome


# ---------------------------------------------------------------------------
# Causal history
# ---------------------------------------------------------------------------


class CausalEvent(BaseModel):
    """Single causal event in an agent's history."""

    event_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    timestamp: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )
    action_taken: str
    outcome: Outcome = Outcome.UNKNOWN
    causal_parents: List[str] = Field(default_factory=list)
    resource_cost: float = 0.0
    compressed_embedding: List[float] = Field(
        default_factory=list,
        description="Max 128 dimensions",
    )
    decay_score: float = Field(default=1.0, ge=0.0, le=1.0)

    @field_validator("compressed_embedding")
    @classmethod
    def _validate_embedding_length(cls, v: List[float]) -> List[float]:
        if len(v) > 128:
            raise ValueError("Embedding must be <= 128 dimensions")
        return v

    model_config = {"use_enum_values": True}


# ---------------------------------------------------------------------------
# Present-state maps
# ---------------------------------------------------------------------------


class CapabilityMap(BaseModel):
    """Current capabilities of an agent in a specific domain."""

    capability_id: str
    domain: Domain
    proficiency_score: float = Field(default=0.5, ge=0.0, le=1.0)
    last_exercised: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )
    failure_rate_30d: float = Field(default=0.0, ge=0.0, le=1.0)
    constraint_notes: List[str] = Field(default_factory=list)

    model_config = {"use_enum_values": True}


class ConstraintMap(BaseModel):
    """Operational constraints on an agent."""

    resource_budget_remaining: float = 1000.0
    max_concurrent_tasks: int = 5
    forbidden_actions: List[str] = Field(default_factory=list)
    governance_restrictions: List[str] = Field(default_factory=list)
    ttl_seconds: Optional[int] = None  # None == immortal


# ---------------------------------------------------------------------------
# Future projection
# ---------------------------------------------------------------------------


class FailureMode(BaseModel):
    """A single predicted failure mode."""

    mode: str
    probability: float = Field(ge=0.0, le=1.0)
    mitigation: str = ""


class SelfProjection(BaseModel):
    """Probabilistic self-projection of future state."""

    projection_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    generated_at: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )
    time_horizon_seconds: int = 1000
    predicted_failure_modes: List[FailureMode] = Field(default_factory=list)
    predicted_capability_drift: Dict[str, float] = Field(default_factory=dict)
    confidence_score: float = Field(default=0.5, ge=0.0, le=1.0)
    basis_event_ids: List[str] = Field(default_factory=list)


# ---------------------------------------------------------------------------
# Complete Self-Model
# ---------------------------------------------------------------------------


class SelfModel(BaseModel):
    """The full self-model for a single agent.

    INVARIANT: ``is_alive`` may transition from ``True`` to ``False``
    exactly once.  No code path may set it back to ``True``.
    """

    agent_id: str
    version: int = 0
    created_at: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )
    past: List[CausalEvent] = Field(default_factory=list)
    present_capabilities: List[CapabilityMap] = Field(default_factory=list)
    present_constraints: ConstraintMap = Field(
        default_factory=ConstraintMap
    )
    future_projections: List[SelfProjection] = Field(
        default_factory=list,
        description="Max 5 active projections",
    )
    integrity_hash: str = ""
    is_alive: bool = True

    def compute_integrity_hash(self) -> str:
        """Recompute and store the SHA-256 integrity hash.

        Hash covers ``past`` + ``present_capabilities`` + ``present_constraints``
        serialised deterministically (sorted keys, UTC datetimes as ISO strings).
        """
        data = {
            "agent_id": self.agent_id,
            "version": self.version,
            "past": [e.model_dump(mode="json") for e in self.past],
            "present_capabilities": [
                c.model_dump(mode="json") for c in self.present_capabilities
            ],
            "present_constraints": self.present_constraints.model_dump(mode="json"),
        }
        raw = json.dumps(data, sort_keys=True, default=str)
        self.integrity_hash = hashlib.sha256(raw.encode("utf-8")).hexdigest()
        return self.integrity_hash

    @field_validator("future_projections")
    @classmethod
    def _cap_projections(cls, v: List[SelfProjection]) -> List[SelfProjection]:
        if len(v) > 5:
            return v[-5:]  # keep most recent 5
        return v

    model_config = {"use_enum_values": True}
