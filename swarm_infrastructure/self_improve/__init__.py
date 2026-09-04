"""X3 AGI Substrate — Costly Recursive Self-Improvement.

Self-modification is permitted but NEVER free.  Costs escalate
exponentially with proficiency.  Failures leave permanent scars.
No erasing history.  No free upgrades.
"""

from swarm.self_improve.engine import SelfImprovementEngine
from swarm.self_improve.cost import CostCalculator
from swarm.self_improve.scars import ScarRegistry

__all__ = [
    "SelfImprovementEngine",
    "CostCalculator",
    "ScarRegistry",
]
