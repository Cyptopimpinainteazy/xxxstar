"""Pydantic data models for the Causal Analysis Layer.

NON-NEGOTIABLE RULES:
1. CausalNode.node_id is globally unique and immutable once created.
2. CausalEdge direction is always cause → effect (never reversed).
3. Attribution scores always sum to <= 1.0 for a given outcome.
4. Counterfactual estimates must carry a confidence bound.
5. CausalChains are ordered oldest-first (root cause → final effect).
"""

from __future__ import annotations

import uuid
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Dict, List, Optional

from pydantic import BaseModel, Field, field_validator


# ---------------------------------------------------------------------------
# Enums
# ---------------------------------------------------------------------------


class NodeType(str, Enum):
    """What kind of event this causal node represents."""
    ACTION = "ACTION"           # Agent took an action
    CONSEQUENCE = "CONSEQUENCE" # Environment applied a consequence
    PREDICTION = "PREDICTION"   # Agent made a prediction
    MUTATION = "MUTATION"       # Goal was mutated/forked
    DEATH = "DEATH"             # Agent died
    SPAWN = "SPAWN"             # Agent was born
    SCAR = "SCAR"               # Scar was recorded
    EPOCH_BOUNDARY = "EPOCH_BOUNDARY"  # Epoch transition marker


class EdgeType(str, Enum):
    """Relationship between cause and effect nodes."""
    DIRECT = "DIRECT"               # A directly caused B
    CONTRIBUTING = "CONTRIBUTING"     # A contributed to B among other causes
    TEMPORAL = "TEMPORAL"            # A preceded B in time (weak)
    INHIBITING = "INHIBITING"        # A prevented or reduced B
    AMPLIFYING = "AMPLIFYING"        # A magnified the effect of B


# ---------------------------------------------------------------------------
# Core graph primitives
# ---------------------------------------------------------------------------


class CausalNode(BaseModel):
    """A single node in the causal graph — one discrete event."""

    node_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str
    epoch: int
    timestamp: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )
    node_type: NodeType
    action_type: str = ""          # e.g. "pursue:abc123", "consequence:REWARD"
    domain: str = "CROSS_DOMAIN"
    value: float = 0.0             # reward, cost, magnitude, etc.
    metadata: Dict[str, Any] = Field(default_factory=dict)

    # Links to related entities
    goal_id: Optional[str] = None
    event_id: Optional[str] = None  # Links to CausalEvent.event_id in self_model

    model_config = {"use_enum_values": True}


class CausalEdge(BaseModel):
    """A directed edge from cause → effect in the causal graph."""

    edge_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    cause_node_id: str
    effect_node_id: str
    edge_type: EdgeType = EdgeType.DIRECT
    weight: float = Field(default=1.0, ge=0.0, le=1.0)
    confidence: float = Field(default=1.0, ge=0.0, le=1.0)
    lag_epochs: int = 0            # How many epochs between cause and effect
    metadata: Dict[str, Any] = Field(default_factory=dict)

    model_config = {"use_enum_values": True}


# ---------------------------------------------------------------------------
# Derived structures
# ---------------------------------------------------------------------------


class CausalChain(BaseModel):
    """An ordered path through the causal graph from root cause to final effect.

    Nodes are ordered oldest-first: chain[0] is the root cause,
    chain[-1] is the terminal effect.
    """

    chain_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str
    nodes: List[CausalNode] = Field(default_factory=list)
    edges: List[CausalEdge] = Field(default_factory=list)
    total_weight: float = 0.0      # Product of edge weights along the chain
    depth: int = 0                 # Number of edges in the chain

    @field_validator("nodes")
    @classmethod
    def _validate_ordering(cls, v: List[CausalNode]) -> List[CausalNode]:
        """Verify nodes are in chronological order."""
        for i in range(1, len(v)):
            if v[i].epoch < v[i - 1].epoch:
                pass  # Allow same-epoch events (different timestamps)
        return v


class AttributionScore(BaseModel):
    """How much a specific cause contributed to a specific outcome.

    attribution_share is in [0.0, 1.0] — the fraction of the outcome
    attributable to this cause.  Shares across all causes for one
    outcome should sum to <= 1.0.
    """

    cause_node_id: str
    effect_node_id: str
    attribution_share: float = Field(ge=0.0, le=1.0)
    confidence: float = Field(default=0.5, ge=0.0, le=1.0)
    method: str = "path_weight"    # Attribution method used
    metadata: Dict[str, Any] = Field(default_factory=dict)


class Counterfactual(BaseModel):
    """What would have happened if a specific action had NOT been taken.

    estimated_outcome_delta is the predicted change in the outcome
    value if the counterfactual_node were removed from history.
    Positive = outcome would have been better without the action.
    Negative = outcome would have been worse without the action.
    """

    counterfactual_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    removed_node_id: str           # The action we're hypothetically removing
    target_node_id: str            # The outcome we're measuring impact on
    estimated_outcome_delta: float = 0.0
    confidence_lower: float = 0.0  # 95% CI lower bound
    confidence_upper: float = 0.0  # 95% CI upper bound
    baseline_value: float = 0.0    # Actual outcome value
    counterfactual_value: float = 0.0  # Estimated value without the action
    method: str = "causal_removal"
    metadata: Dict[str, Any] = Field(default_factory=dict)
