"""Self-projection engine for the Self-Model Ledger.

Answers the critical question:
  "What will kill me if I continue like this?"

Design decisions:
- Simple Markov-chain-like trend extrapolation (no ML dependency).
- Must run < 100 ms for 1000 causal events on a single CPU core.
- Failure modes are synthesised by crossing negative capability drift
  with approaching constraint limits.
- Confidence is inversely proportional to horizon length,
  proportional to basis event count.
"""

from __future__ import annotations

import logging
import time
from collections import defaultdict
from datetime import datetime, timezone
from typing import Dict, List, Optional

from swarm.core.enums import Outcome
from swarm.self_model.schema import (
    CapabilityMap,
    CausalEvent,
    ConstraintMap,
    FailureMode,
    SelfProjection,
)

logger = logging.getLogger(__name__)


class ProjectionEngine:
    """Generates probabilistic self-projections from agent history.

    Args:
        max_basis_events: Number of recent events to consider.
        trend_window: Number of recent exercises per capability for trend.
    """

    def __init__(
        self,
        max_basis_events: int = 100,
        trend_window: int = 10,
    ) -> None:
        self.max_basis_events = max_basis_events
        self.trend_window = trend_window

    def project(
        self,
        past: List[CausalEvent],
        capabilities: List[CapabilityMap],
        constraints: ConstraintMap,
        horizon_seconds: int = 1000,
    ) -> SelfProjection:
        """Generate a self-projection.

        Returns a ``SelfProjection`` with at least one failure mode.
        If none are found, injects a ``NO_FAILURE_MODES_DETECTED`` entry
        and logs a warning — the agent should be flagged for review.
        """
        t0 = time.perf_counter()

        # Trim to recent events
        basis_events = past[-self.max_basis_events :]
        basis_ids = [e.event_id for e in basis_events]

        # --- Capability drift ---
        drift = self._compute_capability_drift(basis_events, capabilities)

        # --- Resource burn rate ---
        burn_rate = self._compute_burn_rate(basis_events)

        # --- Failure modes ---
        failure_modes = self._synthesise_failure_modes(
            drift, burn_rate, constraints, horizon_seconds
        )

        # If no failure modes detected → flag for review
        if not failure_modes:
            logger.warning(
                "ProjectionEngine: no failure modes detected. "
                "Agent should be reviewed."
            )
            failure_modes = [
                FailureMode(
                    mode="NO_FAILURE_MODES_DETECTED",
                    probability=0.1,
                    mitigation="Manual review required — agent may be over-fitted to safe behaviour.",
                )
            ]

        # Confidence = f(basis count, horizon)
        confidence = self._compute_confidence(
            len(basis_events), horizon_seconds
        )

        elapsed_ms = (time.perf_counter() - t0) * 1000
        logger.debug("Projection generated in %.2f ms", elapsed_ms)

        return SelfProjection(
            time_horizon_seconds=horizon_seconds,
            predicted_failure_modes=failure_modes,
            predicted_capability_drift=drift,
            confidence_score=confidence,
            basis_event_ids=basis_ids,
        )

    # ------------------------------------------------------------------
    # Internal methods
    # ------------------------------------------------------------------

    def _compute_capability_drift(
        self,
        events: List[CausalEvent],
        capabilities: List[CapabilityMap],
    ) -> Dict[str, float]:
        """Compute per-capability trend from recent outcomes.

        Positive drift = improving, negative = degrading.
        """
        # Group events by domain (proxy for capability)
        cap_by_id = {c.capability_id: c for c in capabilities}
        # Count recent successes/failures per capability
        outcome_counts: Dict[str, Dict[str, int]] = defaultdict(
            lambda: {"success": 0, "failure": 0, "total": 0}
        )

        for event in events[-self.trend_window * len(capabilities) :]:
            # Map event to capability via action heuristic
            for cap in capabilities:
                if cap.domain.lower() in event.action_taken.lower():
                    counts = outcome_counts[cap.capability_id]
                    counts["total"] += 1
                    if event.outcome == Outcome.SUCCESS:
                        counts["success"] += 1
                    elif event.outcome == Outcome.FAILURE:
                        counts["failure"] += 1

        drift: Dict[str, float] = {}
        for cap_id, counts in outcome_counts.items():
            total = counts["total"]
            if total == 0:
                drift[cap_id] = 0.0
            else:
                # Drift in [-1, 1]: positive = improving
                drift[cap_id] = (counts["success"] - counts["failure"]) / total

        return drift

    def _compute_burn_rate(self, events: List[CausalEvent]) -> float:
        """Average resource cost per event (units per action)."""
        if not events:
            return 0.0
        total_cost = sum(e.resource_cost for e in events)
        return total_cost / len(events)

    def _synthesise_failure_modes(
        self,
        drift: Dict[str, float],
        burn_rate: float,
        constraints: ConstraintMap,
        horizon_seconds: int,
    ) -> List[FailureMode]:
        """Cross negative drift with approaching constraint limits."""
        modes: List[FailureMode] = []

        # Resource exhaustion
        if burn_rate > 0:
            time_to_exhaustion = (
                constraints.resource_budget_remaining / burn_rate
            )
            if time_to_exhaustion < horizon_seconds:
                probability = min(
                    1.0, horizon_seconds / max(time_to_exhaustion, 1)
                )
                modes.append(
                    FailureMode(
                        mode="RESOURCE_EXHAUSTION",
                        probability=min(probability, 0.99),
                        mitigation="Reduce resource consumption or request budget increase.",
                    )
                )

        # TTL expiry
        if constraints.ttl_seconds is not None:
            if constraints.ttl_seconds < horizon_seconds:
                probability = min(
                    1.0, horizon_seconds / max(constraints.ttl_seconds, 1)
                )
                modes.append(
                    FailureMode(
                        mode="TTL_EXPIRY",
                        probability=min(probability, 0.99),
                        mitigation="Request TTL extension or complete critical tasks.",
                    )
                )

        # Capability degradation
        for cap_id, d in drift.items():
            if d < -0.3:
                modes.append(
                    FailureMode(
                        mode=f"CAPABILITY_DEGRADATION:{cap_id}",
                        probability=min(abs(d), 0.95),
                        mitigation=f"Investigate declining performance in {cap_id}.",
                    )
                )

        return modes

    def _compute_confidence(
        self, basis_count: int, horizon_seconds: int
    ) -> float:
        """Confidence inversely proportional to horizon, proportional to data."""
        # More data → higher confidence (caps at 1.0)
        data_factor = min(1.0, basis_count / self.max_basis_events)
        # Longer horizon → lower confidence
        horizon_factor = max(0.1, 1.0 - (horizon_seconds / 10000))
        return round(data_factor * horizon_factor, 4)
