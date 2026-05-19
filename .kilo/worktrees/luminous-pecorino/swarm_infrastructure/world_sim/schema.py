"""Pydantic models for the Unified World Simulator."""

from __future__ import annotations

import hashlib
import json
import uuid
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional

from pydantic import BaseModel, Field

from swarm.core.enums import Domain


class EntityState(BaseModel):
    entity_id: str
    entity_type: str = "generic"
    properties: Dict[str, Any] = Field(default_factory=dict)
    last_updated: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    confidence: float = Field(default=1.0, ge=0.0, le=1.0)


class Relationship(BaseModel):
    source_entity_id: str
    target_entity_id: str
    relation_type: str = "related"
    strength: float = Field(default=0.5, ge=0.0, le=1.0)
    metadata: Dict[str, Any] = Field(default_factory=dict)


class DomainState(BaseModel):
    domain: Domain
    entities: Dict[str, EntityState] = Field(default_factory=dict)
    relationships: List[Relationship] = Field(default_factory=list)
    metrics: Dict[str, float] = Field(default_factory=dict)

    model_config = {"use_enum_values": True}


class WorldState(BaseModel):
    state_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    timestamp: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    epoch: int = 0
    domains: Dict[str, DomainState] = Field(default_factory=dict)
    global_metrics: Dict[str, float] = Field(default_factory=dict)
    integrity_hash: str = ""

    def compute_integrity_hash(self) -> str:
        """Deterministic hash of the entire world state."""
        data = self.model_dump(mode="json", exclude={"integrity_hash", "state_id"})
        raw = json.dumps(data, sort_keys=True, default=str)
        self.integrity_hash = hashlib.sha256(raw.encode("utf-8")).hexdigest()
        return self.integrity_hash

    model_config = {"use_enum_values": True}


class EntityUpdate(BaseModel):
    entity_id: str
    entity_type: str = "generic"
    property_updates: Dict[str, Any] = Field(default_factory=dict)
    confidence: float = Field(default=1.0, ge=0.0, le=1.0)


class Prediction(BaseModel):
    prediction_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str
    target_state_path: str
    predicted_value: float
    confidence: float = Field(default=0.5, ge=0.0, le=1.0)
    horizon_epoch: int
    stake: float = 0.0
    submitted_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class PredictionResult(BaseModel):
    prediction_id: str
    agent_id: str
    predicted_value: float
    actual_value: float
    error_magnitude: float
    stake: float
    reward_or_penalty: float
    resolved_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
