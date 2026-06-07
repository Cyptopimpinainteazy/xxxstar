"""X3 AGI Substrate — Self-Model Ledger.

Every agent maintains a persistent, versioned model of itself.
If the self-model is destroyed, the agent is dead. No resurrection.
"""

from swarm.self_model.ledger import SelfModelLedger
from swarm.self_model.schema import (
    CausalEvent,
    CapabilityMap,
    ConstraintMap,
    SelfModel,
    SelfProjection,
)

__all__ = [
    "SelfModelLedger",
    "CausalEvent",
    "CapabilityMap",
    "ConstraintMap",
    "SelfModel",
    "SelfProjection",
]
