"""Cemetery — persistent archive of dead goals.

Every dead goal is stored with full metadata: lineage, fitness history,
death reason.  Cemetery data is queried by the mutator to avoid repeating
failed mutation paths (anti-pattern detection).

Graves are PERMANENT.  No resurrection.
"""

from __future__ import annotations

import logging
from datetime import datetime, timezone
from typing import Dict, List, Optional

from swarm.goal_genome.schema import Goal
from swarm.storage.backend import StorageBackend

logger = logging.getLogger(__name__)

NAMESPACE = "goal_cemetery"


class Cemetery:
    """Persistent archive of dead goals for forensic analysis."""

    def __init__(self, storage: StorageBackend) -> None:
        self._storage = storage

    def bury(self, goal: Goal) -> None:
        """Archive a dead goal."""
        if goal.is_alive:
            raise ValueError(
                f"Cannot bury goal {goal.goal_id} — it is still alive!"
            )
        self._storage.save(
            NAMESPACE,
            goal.goal_id,
            goal.model_dump(mode="json"),
        )
        logger.info(
            "Buried goal %s (gen %d) — reason: %s",
            goal.goal_id[:8],
            goal.generation,
            goal.death_reason or "unknown",
        )

    def get(self, goal_id: str) -> Optional[Goal]:
        """Retrieve a dead goal by ID."""
        data = self._storage.load(NAMESPACE, goal_id)
        if data is None:
            return None
        return Goal.model_validate(data)

    def query_by_domain(self, domain: str) -> List[Goal]:
        """List dead goals in a specific domain."""
        rows = self._storage.query(
            NAMESPACE, filters={"domain": domain}
        )
        return [Goal.model_validate(r) for r in rows]

    def query_by_generation(self, generation: int) -> List[Goal]:
        """List dead goals of a specific generation."""
        rows = self._storage.query(
            NAMESPACE, filters={"generation": generation}
        )
        return [Goal.model_validate(r) for r in rows]

    def query_by_death_reason(self, reason: str) -> List[Goal]:
        """List dead goals with a specific death reason."""
        rows = self._storage.query(
            NAMESPACE, filters={"death_reason": reason}
        )
        return [Goal.model_validate(r) for r in rows]

    def list_all(self) -> List[Goal]:
        """Return all dead goals."""
        keys = self._storage.list_keys(NAMESPACE)
        results = self._storage.load_many(NAMESPACE, keys)
        return [Goal.model_validate(r) for r in results if r is not None]

    def get_failed_mandates(self) -> List[str]:
        """Return mandates of all dead goals for anti-pattern detection."""
        goals = self.list_all()
        return [g.mandate for g in goals]
