"""Core Self-Model Ledger — the soul of an X3 agent.

NON-NEGOTIABLE INVARIANTS:
1. ``kill()`` is irreversible.  No code path may set ``is_alive`` back to True.
2. Memory eviction is permanent.  Evicted events cannot be restored.
3. Every mutation increments ``version`` and recomputes ``integrity_hash``.
4. ``anchor_to_chain()`` follows the pattern in ``swarm/jury/anchorer.py``.
"""

from __future__ import annotations

import hashlib
import json
import logging
from typing import Any, Callable, Dict, List, Optional

from swarm.core.enums import Domain
from swarm.event_bus.bus import AsyncEventBus
from swarm.event_bus.events import (
    BusEvent,
    EventType,
    agent_death_event,
)
from swarm.self_model.decay import DecayEngine
from swarm.self_model.projector import ProjectionEngine
from swarm.self_model.schema import (
    CapabilityMap,
    CausalEvent,
    ConstraintMap,
    SelfModel,
    SelfProjection,
)
from swarm.storage.backend import StorageBackend

logger = logging.getLogger(__name__)

NAMESPACE = "self_model"


class SelfModelLedger:
    """Persistent, versioned self-model for a single agent.

    Args:
        agent_id: Unique identifier for the owning agent.
        storage: Persistence backend (SQLite for dev, Postgres for prod).
        event_bus: Optional async event bus for cross-layer events.
        decay_engine: Optional custom decay engine.
        projection_engine: Optional custom projection engine.
    """

    def __init__(
        self,
        agent_id: str,
        storage: StorageBackend,
        event_bus: Optional[AsyncEventBus] = None,
        decay_engine: Optional[DecayEngine] = None,
        projection_engine: Optional[ProjectionEngine] = None,
        anchor_writer: Optional[Callable[[str, int, str], str]] = None,
    ) -> None:
        self.agent_id = agent_id
        self._storage = storage
        self._bus = event_bus
        self._decay = decay_engine or DecayEngine()
        self._projector = projection_engine or ProjectionEngine()
        self._anchor_writer = anchor_writer

        # Load or create
        saved = self._storage.load(NAMESPACE, agent_id)
        if saved is not None:
            self._model = SelfModel.model_validate(saved)
        else:
            self._model = SelfModel(agent_id=agent_id)
            self._persist()

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def model(self) -> SelfModel:
        return self._model

    @property
    def is_alive(self) -> bool:
        return self._model.is_alive

    @property
    def version(self) -> int:
        return self._model.version

    # ------------------------------------------------------------------
    # Core operations
    # ------------------------------------------------------------------

    def record_event(self, event: CausalEvent) -> None:
        """Append a causal event, run decay, bump version, persist."""
        self._assert_alive()
        self._model.past.append(event)
        self._run_decay_pass()
        self._bump_version()
        self._persist()
        logger.info(
            "Recorded event %s for agent %s (v%d)",
            event.event_id,
            self.agent_id,
            self._model.version,
        )

    def update_capabilities(
        self, capabilities: List[CapabilityMap]
    ) -> None:
        """Replace capability map and log the delta."""
        self._assert_alive()
        old_ids = {c.capability_id for c in self._model.present_capabilities}
        new_ids = {c.capability_id for c in capabilities}
        added = new_ids - old_ids
        removed = old_ids - new_ids
        if added:
            logger.info("Agent %s: new capabilities %s", self.agent_id, added)
        if removed:
            logger.info("Agent %s: lost capabilities %s", self.agent_id, removed)

        self._model.present_capabilities = capabilities
        self._bump_version()
        self._persist()

    def update_constraints(self, constraints: ConstraintMap) -> None:
        """Update the constraint map."""
        self._assert_alive()
        self._model.present_constraints = constraints
        self._bump_version()
        self._persist()

    def project_future(
        self, horizon_seconds: int = 1000
    ) -> SelfProjection:
        """Generate a self-projection.

        Must answer: "What will kill me if I continue like this?"
        """
        self._assert_alive()
        projection = self._projector.project(
            past=self._model.past,
            capabilities=self._model.present_capabilities,
            constraints=self._model.present_constraints,
            horizon_seconds=horizon_seconds,
        )

        # Keep max 5 projections
        self._model.future_projections.append(projection)
        if len(self._model.future_projections) > 5:
            self._model.future_projections = self._model.future_projections[-5:]

        self._persist()
        return projection

    def decay_pass(self) -> int:
        """Run an explicit decay pass. Returns count of evicted events."""
        self._assert_alive()
        return self._run_decay_pass()

    def kill(self, reason: str = "Terminated") -> None:
        """Permanently kill this agent.  IRREVERSIBLE.

        - Sets ``is_alive`` to ``False``
        - Writes final snapshot
        - Emits ``AGENT_DEATH`` event
        """
        if not self._model.is_alive:
            logger.warning("Agent %s is already dead", self.agent_id)
            return

        self._model.is_alive = False
        self._bump_version()
        self._persist()

        logger.critical(
            "AGENT DEATH: %s (v%d) — %s",
            self.agent_id,
            self._model.version,
            reason,
        )

        # Emit death event (always created; caller drains via collect_bus_events)
        self._death_event = agent_death_event(self.agent_id, reason)

    def get_death_event(self) -> Optional[BusEvent]:
        """Return pending death event for async publishing."""
        return getattr(self, "_death_event", None)

    def get_mortality_assessment(self) -> Dict[str, Any]:
        """Return a survival summary.

        Keys:
        - top_failure_modes: Top 3 predicted failure modes.
        - resource_burn_rate: Average resource cost per event.
        - time_to_exhaustion: Estimated seconds until budget runs out.
        - survival_probability_1000s: P(alive) for next 1000 seconds.
        """
        if not self._model.is_alive:
            return {
                "top_failure_modes": [],
                "resource_burn_rate": 0.0,
                "time_to_exhaustion": 0.0,
                "survival_probability_1000s": 0.0,
                "status": "DEAD",
            }

        # Gather failure modes from latest projection
        all_modes = []
        for proj in self._model.future_projections:
            all_modes.extend(proj.predicted_failure_modes)

        # Deduplicate by mode name, keep highest probability
        mode_map: Dict[str, Any] = {}
        for fm in all_modes:
            if fm.mode not in mode_map or fm.probability > mode_map[fm.mode]["probability"]:
                mode_map[fm.mode] = {
                    "mode": fm.mode,
                    "probability": fm.probability,
                    "mitigation": fm.mitigation,
                }

        top_modes = sorted(
            mode_map.values(), key=lambda m: m["probability"], reverse=True
        )[:3]

        # Burn rate
        recent = self._model.past[-100:]
        total_cost = sum(e.resource_cost for e in recent)
        burn_rate = total_cost / max(len(recent), 1)

        # Time to exhaustion
        budget = self._model.present_constraints.resource_budget_remaining
        tte = budget / burn_rate if burn_rate > 0 else float("inf")

        # Survival probability heuristic
        max_fail_prob = max((m["probability"] for m in top_modes), default=0.0)
        survival_prob = max(0.0, 1.0 - max_fail_prob)

        return {
            "top_failure_modes": top_modes,
            "resource_burn_rate": round(burn_rate, 4),
            "time_to_exhaustion": round(tte, 2),
            "survival_probability_1000s": round(survival_prob, 4),
            "status": "ALIVE",
        }

    def serialize(self) -> bytes:
        """Deterministic serialisation for hashing and anchoring."""
        data = self._model.model_dump(mode="json")
        return json.dumps(data, sort_keys=True, default=str).encode("utf-8")

    def anchor_to_chain(self) -> str:
        """Submit integrity hash to an injected chain anchor backend.

        This method now fails fast when no anchor writer has been configured,
        instead of returning a fake transaction hash.
        """
        self._assert_alive()
        self._model.compute_integrity_hash()

        if self._anchor_writer is None:
            raise RuntimeError(
                "No chain anchor backend configured for SelfModelLedger.anchor_to_chain"
            )

        tx_hash = self._anchor_writer(
            self.agent_id,
            self._model.version,
            self._model.integrity_hash,
        )
        if not tx_hash:
            raise RuntimeError("Chain anchor backend returned an empty transaction hash")

        logger.info(
            "Anchoring self-model: agent=%s v=%d hash=%s tx=%s",
            self.agent_id,
            self._model.version,
            self._model.integrity_hash[:16],
            str(tx_hash)[:16],
        )
        return str(tx_hash)

    # ------------------------------------------------------------------
    # Internals
    # ------------------------------------------------------------------

    def _assert_alive(self) -> None:
        """Raise if agent is dead.  No zombies allowed."""
        if not self._model.is_alive:
            raise RuntimeError(
                f"Agent {self.agent_id} is DEAD (v{self._model.version}). "
                "No operations permitted on a dead agent."
            )

    def _bump_version(self) -> None:
        """Increment version and recompute integrity hash."""
        self._model.version += 1
        self._model.compute_integrity_hash()

    def _run_decay_pass(self) -> int:
        """Execute decay and eviction.  Returns evicted count."""
        surviving, bus_events = self._decay.decay_pass(
            self._model.past, self.agent_id
        )
        evicted_count = len(self._model.past) - len(surviving)
        self._model.past = surviving

        # Store eviction events for async publishing
        if bus_events:
            if not hasattr(self, "_pending_bus_events"):
                self._pending_bus_events: List[BusEvent] = []
            self._pending_bus_events.extend(bus_events)

        return evicted_count

    def get_pending_bus_events(self) -> List[BusEvent]:
        """Drain and return any pending bus events."""
        events = getattr(self, "_pending_bus_events", [])
        self._pending_bus_events = []
        return events

    def _persist(self) -> None:
        """Write model to storage."""
        self._storage.save(
            NAMESPACE,
            self.agent_id,
            self._model.model_dump(mode="json"),
        )
