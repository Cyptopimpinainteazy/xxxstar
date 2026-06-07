"""X3 AGI Substrate — Goal Genome with mutation and mortality.

Agent goals are living organisms.  They compete, mutate, fork, and die.
No human approval is required for mutation.  Only consequences decide.
"""

from swarm.goal_genome.schema import (
    Goal,
    GoalFitnessReport,
    GoalMutation,
    MutationType,
    MutationTrigger,
    Recommendation,
)
from swarm.goal_genome.genome import GoalGenome

__all__ = [
    "Goal",
    "GoalFitnessReport",
    "GoalGenome",
    "GoalMutation",
    "MutationType",
    "MutationTrigger",
    "Recommendation",
]
