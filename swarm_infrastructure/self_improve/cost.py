"""Cost calculator for self-improvement.

Cost formula:
    base_cost = 10 * (proficiency ** 2)
    scar_multiplier = 1.0 + 0.2 * scar_count_in_domain
    final_cost = base_cost * scar_multiplier

Higher proficiency → exponentially more expensive.
Past failures (scars) → multiplicative penalty.
This ensures improvement is NEVER free and runaway self-modification is bounded.
"""

from __future__ import annotations

import logging

logger = logging.getLogger(__name__)

BASE_COST_MULTIPLIER = 10.0
SCAR_PENALTY_PER_SCAR = 0.2
MIN_COST = 0.1


class CostCalculator:
    """Calculate the cost of an improvement proposal."""

    def __init__(
        self,
        base_multiplier: float = BASE_COST_MULTIPLIER,
        scar_penalty: float = SCAR_PENALTY_PER_SCAR,
    ) -> None:
        self._base_multiplier = base_multiplier
        self._scar_penalty = scar_penalty

    def calculate(
        self,
        current_proficiency: float,
        scar_count: int,
    ) -> float:
        """Calculate the cost of an improvement attempt.

        Args:
            current_proficiency: The agent's current proficiency (0.0 to 1.0+).
            scar_count: Number of scars in the target domain.

        Returns:
            The total cost.
        """
        base = self._base_multiplier * (current_proficiency ** 2)
        scar_multiplier = 1.0 + self._scar_penalty * scar_count
        cost = max(base * scar_multiplier, MIN_COST)

        logger.debug(
            "Cost calc: proficiency=%.4f scars=%d → base=%.2f × scar_mult=%.2f = %.2f",
            current_proficiency,
            scar_count,
            base,
            scar_multiplier,
            cost,
        )
        return cost
