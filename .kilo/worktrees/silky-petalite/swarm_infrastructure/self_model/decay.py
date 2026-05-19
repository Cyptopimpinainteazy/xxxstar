"""Memory decay and eviction engine for the Self-Model Ledger.

Design decisions:
- Exponential decay with configurable half-life (default 3600s).
- High-magnitude outcome events (SUCCESS/FAILURE) decay at 50% normal rate.
- Zero-cost events decay at 200% normal rate.
- Eviction below 0.05 threshold is **permanent** and logged.
- Each eviction emits a MEMORY_EVICTED event on the bus.
"""

from __future__ import annotations

import logging
import math
from typing import List, Optional, Tuple

from swarm.self_model.schema import CausalEvent
from swarm.core.enums import Outcome
from swarm.event_bus.events import BusEvent, EventType, memory_evicted_event

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

DEFAULT_HALF_LIFE_SECONDS: float = 3600.0
EVICTION_THRESHOLD: float = 0.05
BASE_DECAY_RATE: float = 0.02  # per-pass base


# ---------------------------------------------------------------------------
# Decay Engine
# ---------------------------------------------------------------------------


class DecayEngine:
    """Exponential memory decay with outcome-aware rate modulation.

    Args:
        half_life_seconds: Time constant for decay.  Events lose half
            their score in this many agent-seconds.
        decay_rate_per_pass: Base amount subtracted each pass.
        eviction_threshold: Score below which events are permanently evicted.
    """

    def __init__(
        self,
        half_life_seconds: float = DEFAULT_HALF_LIFE_SECONDS,
        decay_rate_per_pass: float = BASE_DECAY_RATE,
        eviction_threshold: float = EVICTION_THRESHOLD,
    ) -> None:
        self.half_life_seconds = half_life_seconds
        self.decay_rate_per_pass = decay_rate_per_pass
        self.eviction_threshold = eviction_threshold

    def compute_rate(self, event: CausalEvent) -> float:
        """Return the effective decay rate for *event*.

        - High-magnitude outcomes (SUCCESS, FAILURE) → 50 % of base rate.
        - Zero resource cost → 200 % of base rate.
        - Otherwise → base rate.
        """
        rate = self.decay_rate_per_pass

        # High-magnitude outcomes are memorable — decay slower
        if event.outcome in (Outcome.SUCCESS, Outcome.FAILURE):
            rate *= 0.5

        # Free actions are forgettable — decay faster
        if event.resource_cost == 0.0:
            rate *= 2.0

        return rate

    def decay_pass(
        self,
        events: List[CausalEvent],
        agent_id: str,
    ) -> Tuple[List[CausalEvent], List[BusEvent]]:
        """Run one decay pass over *events*.

        Returns:
            (surviving_events, eviction_bus_events)
        """
        surviving: List[CausalEvent] = []
        bus_events: List[BusEvent] = []

        for event in events:
            rate = self.compute_rate(event)
            new_score = max(0.0, event.decay_score - rate)
            event.decay_score = new_score

            if new_score < self.eviction_threshold:
                # PERMANENT EVICTION — no recovery
                logger.info(
                    "Evicting event %s (agent=%s, score=%.4f)",
                    event.event_id,
                    agent_id,
                    new_score,
                )
                bus_events.append(
                    memory_evicted_event(
                        agent_id=agent_id,
                        event_id=event.event_id,
                        final_decay_score=new_score,
                        reason=f"Decay below threshold ({self.eviction_threshold})",
                    )
                )
            else:
                surviving.append(event)

        return surviving, bus_events
