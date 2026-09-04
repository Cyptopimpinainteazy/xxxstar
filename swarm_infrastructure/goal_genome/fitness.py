"""Goal fitness evaluation engine.

Fitness formula:
  (reward_earned / max(resource_spent, 0.01))
  * (successes / max(successes + failures, 1))
  * (1 - environmental_resistance)

Recommendation thresholds:
  fitness < 0.2 for 3 consecutive evals → KILL
  fitness < 0.4                         → MUTATE
  else                                  → CONTINUE
"""

from __future__ import annotations

import logging
from typing import List

from swarm.core.enums import Outcome
from swarm.goal_genome.schema import (
    Goal,
    GoalFitnessReport,
    Recommendation,
)

logger = logging.getLogger(__name__)

KILL_THRESHOLD = 0.2
MUTATE_THRESHOLD = 0.4
CONSECUTIVE_KILL_COUNT = 3


class FitnessEvaluator:
    """Evaluates goal fitness from outcome data."""

    def evaluate(
        self,
        goal: Goal,
        successes: int,
        failures: int,
        resource_spent: float,
        reward_earned: float,
        evaluation_period_seconds: int = 0,
    ) -> GoalFitnessReport:
        """Compute fitness and generate recommendation."""
        # Core fitness formula
        efficiency = reward_earned / max(resource_spent, 0.01)
        success_rate = successes / max(successes + failures, 1)
        resistance_factor = 1.0 - goal.environmental_resistance

        computed_fitness = min(1.0, efficiency * success_rate * resistance_factor)
        computed_fitness = max(0.0, computed_fitness)

        # Update goal's fitness
        goal.fitness_score = computed_fitness
        goal.fitness_history.append(computed_fitness)
        goal.pursuit_cost_cumulative += resource_spent

        # Determine recommendation
        recommendation = self._recommend(goal)

        report = GoalFitnessReport(
            goal_id=goal.goal_id,
            evaluation_period_seconds=evaluation_period_seconds,
            successes=successes,
            failures=failures,
            resource_spent=resource_spent,
            reward_earned=reward_earned,
            computed_fitness=computed_fitness,
            recommendation=recommendation,
        )

        logger.info(
            "Fitness eval: goal=%s fitness=%.4f rec=%s",
            goal.goal_id[:8],
            computed_fitness,
            recommendation,
        )
        return report

    def _recommend(self, goal: Goal) -> Recommendation:
        """Determine recommendation based on fitness history."""
        history = goal.fitness_history

        # Check for KILL: 3 consecutive evals below threshold
        if len(history) >= CONSECUTIVE_KILL_COUNT:
            recent = history[-CONSECUTIVE_KILL_COUNT:]
            if all(f < KILL_THRESHOLD for f in recent):
                return Recommendation.KILL

        # Check for MUTATE
        if history and history[-1] < MUTATE_THRESHOLD:
            return Recommendation.MUTATE

        return Recommendation.CONTINUE
