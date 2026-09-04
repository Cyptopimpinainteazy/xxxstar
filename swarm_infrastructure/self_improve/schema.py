"""Pydantic models for the Self-Improvement layer."""

from __future__ import annotations

import uuid
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Dict, List, Optional

from pydantic import BaseModel, Field

from swarm.core.enums import Domain


class ImprovementType(str, Enum):
    CAPABILITY_UPGRADE = "CAPABILITY_UPGRADE"
    PARAMETER_TUNE = "PARAMETER_TUNE"
    ARCHITECTURE_CHANGE = "ARCHITECTURE_CHANGE"
    STRATEGY_SHIFT = "STRATEGY_SHIFT"


class ProposalStatus(str, Enum):
    PENDING = "PENDING"
    APPROVED = "APPROVED"
    EXECUTING = "EXECUTING"
    SUCCEEDED = "SUCCEEDED"
    FAILED = "FAILED"
    REJECTED_BUDGET = "REJECTED_BUDGET"
    REJECTED_COOLDOWN = "REJECTED_COOLDOWN"


class ImprovementProposal(BaseModel):
    proposal_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str
    improvement_type: ImprovementType
    target_capability: str
    target_domain: Domain
    description: str = ""
    estimated_cost: float = 0.0
    current_proficiency: float = 0.0
    expected_proficiency_delta: float = 0.0
    status: ProposalStatus = ProposalStatus.PENDING
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    resolved_at: Optional[datetime] = None

    model_config = {"use_enum_values": True}


class ImprovementOutcome(BaseModel):
    proposal_id: str
    agent_id: str
    success: bool
    actual_cost: float
    proficiency_before: float
    proficiency_after: float
    side_effects: List[str] = Field(default_factory=list)
    resolved_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class Scar(BaseModel):
    """Permanent record of a failed improvement attempt.

    Scars are NEVER deleted.  They increase the cost of future
    improvement attempts in the same domain.
    """

    scar_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str
    proposal_id: str
    improvement_type: ImprovementType
    target_domain: Domain
    target_capability: str
    cost_paid: float
    failure_reason: str = ""
    recorded_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))

    model_config = {"use_enum_values": True}
