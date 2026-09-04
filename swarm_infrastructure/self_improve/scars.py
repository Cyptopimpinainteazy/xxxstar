"""Scar Registry — permanent records of failed improvement attempts.

NON-NEGOTIABLE: Scars are NEVER deleted, NEVER modified, NEVER hidden.
History cannot be rewritten.
"""

from __future__ import annotations

import logging
from typing import Dict, List, Optional

from swarm.event_bus.events import BusEvent, EventType
from swarm.storage.backend import StorageBackend
from swarm.self_improve.schema import Scar

logger = logging.getLogger(__name__)


class ScarRegistry:
    """Permanent scar storage.

    Args:
        storage: Persistence backend.
        agent_id: The agent this registry belongs to.
    """

    def __init__(self, storage: StorageBackend, agent_id: str) -> None:
        self._storage = storage
        self._agent_id = agent_id
        self._namespace = f"scars:{agent_id}"
        self._pending_bus_events: List[BusEvent] = []

    def record(self, scar: Scar) -> None:
        """Record a scar. Permanent. No undo."""
        self._storage.save(
            self._namespace, scar.scar_id, scar.model_dump(mode="json")
        )

        self._pending_bus_events.append(
            BusEvent(
                event_type=EventType.SCAR_RECORDED,
                agent_id=self._agent_id,
                layer="SELF_IMPROVE",
                payload={
                    "scar_id": scar.scar_id,
                    "proposal_id": scar.proposal_id,
                    "domain": scar.target_domain,
                    "cost_paid": scar.cost_paid,
                },
            )
        )

        logger.warning(
            "SCAR recorded: agent=%s domain=%s capability=%s cost=%.2f",
            self._agent_id,
            scar.target_domain,
            scar.target_capability,
            scar.cost_paid,
        )

    def get_scars(
        self, domain: Optional[str] = None
    ) -> List[Scar]:
        """Retrieve scars, optionally filtered by domain."""
        filters = {"target_domain": domain} if domain else None
        rows = self._storage.query(self._namespace, filters=filters)
        return [Scar.model_validate(r) for r in rows]

    def count_in_domain(self, domain: str) -> int:
        """Count scars in a specific domain."""
        return len(self.get_scars(domain=domain))

    def get_all(self) -> List[Scar]:
        """All scars for this agent. Permanent record."""
        rows = self._storage.query(self._namespace)
        return [Scar.model_validate(r) for r in rows]

    def get_pending_bus_events(self) -> List[BusEvent]:
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events
