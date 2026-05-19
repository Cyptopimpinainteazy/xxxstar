"""Goal mutation engine.

Mutation types:
  FORK         — split mandate into two sub-goals (parent dies)
  DRIFT        — perturb mandate parameters
  INVERSION    — reverse goal polarity
  RECOMBINATION — combine two goals into one (both parents die)

Selection weights by trigger:
  REPEATED_FAILURE:  40% INVERSION, 30% DRIFT, 20% FORK, 10% RECOMBINATION
  COST_EXCEEDED:     50% FORK, 30% DRIFT, 10% INVERSION, 10% RECOMBINATION
  ENVIRONMENT_SHIFT: 40% DRIFT, 30% RECOMBINATION, 20% FORK, 10% INVERSION
  RANDOM:            25% each
"""

from __future__ import annotations

import logging
import random
from typing import List, Optional

from swarm.goal_genome.schema import (
    EnvironmentContext,
    Goal,
    GoalMutation,
    MutationTrigger,
    MutationType,
)
from swarm.core.enums import Domain

logger = logging.getLogger(__name__)

# Mutation selection weights: trigger → [(type, weight), ...]
MUTATION_WEIGHTS = {
    MutationTrigger.REPEATED_FAILURE: [
        (MutationType.INVERSION, 0.40),
        (MutationType.DRIFT, 0.30),
        (MutationType.FORK, 0.20),
        (MutationType.RECOMBINATION, 0.10),
    ],
    MutationTrigger.COST_EXCEEDED: [
        (MutationType.FORK, 0.50),
        (MutationType.DRIFT, 0.30),
        (MutationType.INVERSION, 0.10),
        (MutationType.RECOMBINATION, 0.10),
    ],
    MutationTrigger.ENVIRONMENT_SHIFT: [
        (MutationType.DRIFT, 0.40),
        (MutationType.RECOMBINATION, 0.30),
        (MutationType.FORK, 0.20),
        (MutationType.INVERSION, 0.10),
    ],
    MutationTrigger.RANDOM: [
        (MutationType.FORK, 0.25),
        (MutationType.DRIFT, 0.25),
        (MutationType.INVERSION, 0.25),
        (MutationType.RECOMBINATION, 0.25),
    ],
}


class GoalMutator:
    """Produces mutated goals from existing ones.

    No human approval.  Only consequences.
    """

    def select_mutation_type(self, trigger: MutationTrigger) -> MutationType:
        """Probabilistically select a mutation type based on trigger."""
        weights = MUTATION_WEIGHTS[trigger]
        types = [w[0] for w in weights]
        probs = [w[1] for w in weights]
        return random.choices(types, weights=probs, k=1)[0]

    def mutate(
        self,
        goal: Goal,
        trigger: MutationTrigger,
        context: EnvironmentContext,
        second_goal: Optional[Goal] = None,
    ) -> List[Goal]:
        """Mutate a goal.  Returns list of new goals (1 or 2).

        The parent goal is NOT killed here — caller is responsible.
        """
        mutation_type = self.select_mutation_type(trigger)

        # RECOMBINATION requires a second goal
        if mutation_type == MutationType.RECOMBINATION and second_goal is None:
            # Fall back to DRIFT if no second goal available
            mutation_type = MutationType.DRIFT

        new_lineage = goal.lineage + [goal.goal_id]
        new_gen = goal.generation + 1

        if mutation_type == MutationType.FORK:
            return self._fork(goal, new_lineage, new_gen)
        elif mutation_type == MutationType.DRIFT:
            return self._drift(goal, new_lineage, new_gen)
        elif mutation_type == MutationType.INVERSION:
            return self._invert(goal, new_lineage, new_gen)
        elif mutation_type == MutationType.RECOMBINATION and second_goal is not None:
            return self._recombine(goal, second_goal, new_lineage, new_gen)
        else:
            return self._drift(goal, new_lineage, new_gen)

    def _fork(
        self, goal: Goal, lineage: List[str], gen: int
    ) -> List[Goal]:
        """Split mandate into two sub-goals."""
        mandate_a = f"[FORK-A] {goal.mandate} — sub-objective alpha"
        mandate_b = f"[FORK-B] {goal.mandate} — sub-objective beta"

        child_a = Goal(
            parent_goal_id=goal.goal_id,
            generation=gen,
            mandate=mandate_a,
            domain=goal.domain,
            expected_reward=goal.expected_reward * 0.6,
            environmental_resistance=goal.environmental_resistance,
            lineage=lineage,
        )
        child_b = Goal(
            parent_goal_id=goal.goal_id,
            generation=gen,
            mandate=mandate_b,
            domain=goal.domain,
            expected_reward=goal.expected_reward * 0.4,
            environmental_resistance=goal.environmental_resistance,
            lineage=lineage,
        )

        logger.info(
            "FORK: %s → %s + %s",
            goal.goal_id[:8],
            child_a.goal_id[:8],
            child_b.goal_id[:8],
        )
        return [child_a, child_b]

    def _drift(
        self, goal: Goal, lineage: List[str], gen: int
    ) -> List[Goal]:
        """Slightly modify mandate parameters."""
        drift_suffix = random.choice([
            "with reduced scope",
            "with adjusted thresholds",
            "targeting adjacent opportunity",
            "with refined constraints",
        ])
        new_mandate = f"[DRIFT] {goal.mandate} — {drift_suffix}"

        child = Goal(
            parent_goal_id=goal.goal_id,
            generation=gen,
            mandate=new_mandate,
            domain=goal.domain,
            expected_reward=goal.expected_reward * random.uniform(0.8, 1.2),
            environmental_resistance=max(
                0.0,
                min(1.0, goal.environmental_resistance + random.uniform(-0.1, 0.1)),
            ),
            lineage=lineage,
        )
        logger.info("DRIFT: %s → %s", goal.goal_id[:8], child.goal_id[:8])
        return [child]

    def _invert(
        self, goal: Goal, lineage: List[str], gen: int
    ) -> List[Goal]:
        """Reverse the goal's polarity."""
        new_mandate = f"[INVERSION] Minimize the cost of NOT: {goal.mandate}"

        child = Goal(
            parent_goal_id=goal.goal_id,
            generation=gen,
            mandate=new_mandate,
            domain=goal.domain,
            expected_reward=goal.expected_reward,
            environmental_resistance=max(
                0.0, goal.environmental_resistance - 0.1
            ),
            lineage=lineage,
        )
        logger.info("INVERSION: %s → %s", goal.goal_id[:8], child.goal_id[:8])
        return [child]

    def _recombine(
        self,
        goal_a: Goal,
        goal_b: Goal,
        lineage: List[str],
        gen: int,
    ) -> List[Goal]:
        """Combine elements of two goals into one."""
        new_mandate = (
            f"[RECOMBINATION] Merge: ({goal_a.mandate}) + ({goal_b.mandate})"
        )

        child = Goal(
            parent_goal_id=goal_a.goal_id,
            generation=gen,
            mandate=new_mandate,
            domain=goal_a.domain,
            expected_reward=(
                goal_a.expected_reward + goal_b.expected_reward
            ) * 0.75,
            environmental_resistance=(
                goal_a.environmental_resistance + goal_b.environmental_resistance
            ) / 2.0,
            lineage=lineage,
        )
        logger.info(
            "RECOMBINATION: %s + %s → %s",
            goal_a.goal_id[:8],
            goal_b.goal_id[:8],
            child.goal_id[:8],
        )
        return [child]
