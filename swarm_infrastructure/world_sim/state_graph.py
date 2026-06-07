"""Canonical world state graph.

State transitions are DETERMINISTIC: same inputs → same integrity_hash.
This is critical for on-chain anchoring and dispute resolution.
"""

from __future__ import annotations

import logging
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional

from swarm.core.enums import Domain
from swarm.event_bus.events import BusEvent, EventType
from swarm.storage.backend import StorageBackend
from swarm.world_sim.schema import (
    DomainState,
    EntityState,
    EntityUpdate,
    WorldState,
)

logger = logging.getLogger(__name__)

NAMESPACE = "world_state"


class WorldStateGraph:
    """Canonical world state that all agents perceive and predict against.

    Args:
        storage: Persistence backend.
    """

    def __init__(self, storage: StorageBackend) -> None:
        self._storage = storage
        self._pending_bus_events: List[BusEvent] = []

        # Load or init
        saved = self._storage.load(NAMESPACE, "current")
        if saved is not None:
            self._state = WorldState.model_validate(saved)
        else:
            self._state = WorldState(epoch=0)
            self._persist()

    @property
    def current_state(self) -> WorldState:
        return self._state

    @property
    def epoch(self) -> int:
        return self._state.epoch

    def advance_epoch(self) -> WorldState:
        """Advance to the next epoch.

        Collects all domain updates, recomputes integrity hash,
        persists, and emits EPOCH_ADVANCED event.
        """
        # Archive current epoch
        self._storage.save(
            NAMESPACE,
            f"epoch:{self._state.epoch}",
            self._state.model_dump(mode="json"),
        )

        # Advance
        self._state.epoch += 1
        self._state.timestamp = datetime.now(timezone.utc)
        self._state.compute_integrity_hash()
        self._persist()

        self._pending_bus_events.append(
            BusEvent(
                event_type=EventType.EPOCH_ADVANCED,
                agent_id="WORLD_SIM",
                layer="WORLD_SIM",
                payload={
                    "epoch": self._state.epoch,
                    "integrity_hash": self._state.integrity_hash,
                },
            )
        )

        logger.info(
            "Epoch %d advanced (hash=%s)",
            self._state.epoch,
            self._state.integrity_hash[:16],
        )
        return self._state

    def update_domain(
        self,
        domain: Domain,
        updates: List[EntityUpdate],
    ) -> None:
        """Apply entity updates to a domain."""
        domain_key = domain.value
        if domain_key not in self._state.domains:
            self._state.domains[domain_key] = DomainState(domain=domain)

        ds = self._state.domains[domain_key]
        now = datetime.now(timezone.utc)

        for upd in updates:
            if upd.entity_id in ds.entities:
                entity = ds.entities[upd.entity_id]
                entity.properties.update(upd.property_updates)
                entity.last_updated = now
                entity.confidence = upd.confidence
            else:
                ds.entities[upd.entity_id] = EntityState(
                    entity_id=upd.entity_id,
                    entity_type=upd.entity_type,
                    properties=upd.property_updates,
                    last_updated=now,
                    confidence=upd.confidence,
                )

        # Recalculate domain metrics
        ds.metrics["entity_count"] = float(len(ds.entities))
        ds.metrics["avg_confidence"] = (
            sum(e.confidence for e in ds.entities.values()) / max(len(ds.entities), 1)
        )

        self._persist()

    def query(self, path: str) -> Any:
        """Dot-notation query into current state.

        Example: "domains.MARKET.entities.sol_usdc.properties.price"
        """
        parts = path.split(".")
        current: Any = self._state.model_dump(mode="json")
        for part in parts:
            if isinstance(current, dict):
                current = current.get(part)
            else:
                return None
            if current is None:
                return None
        return current

    def get_state_at_epoch(self, epoch: int) -> Optional[WorldState]:
        """Historical lookback."""
        if epoch == self._state.epoch:
            return self._state
        saved = self._storage.load(NAMESPACE, f"epoch:{epoch}")
        if saved is None:
            return None
        return WorldState.model_validate(saved)

    def diff(self, epoch_a: int, epoch_b: int) -> Dict[str, Any]:
        """Structural diff between two epochs."""
        state_a = self.get_state_at_epoch(epoch_a)
        state_b = self.get_state_at_epoch(epoch_b)

        if state_a is None or state_b is None:
            return {"error": f"Missing state for epoch {epoch_a} or {epoch_b}"}

        diff_result: Dict[str, Any] = {
            "epoch_a": epoch_a,
            "epoch_b": epoch_b,
            "domains_added": [],
            "domains_removed": [],
            "entities_changed": [],
        }

        domains_a = set(state_a.domains.keys())
        domains_b = set(state_b.domains.keys())
        diff_result["domains_added"] = list(domains_b - domains_a)
        diff_result["domains_removed"] = list(domains_a - domains_b)

        for domain_key in domains_a & domains_b:
            ds_a = state_a.domains[domain_key]
            ds_b = state_b.domains[domain_key]
            entities_a = set(ds_a.entities.keys())
            entities_b = set(ds_b.entities.keys())

            for eid in entities_b - entities_a:
                diff_result["entities_changed"].append(
                    {"domain": domain_key, "entity": eid, "change": "added"}
                )
            for eid in entities_a - entities_b:
                diff_result["entities_changed"].append(
                    {"domain": domain_key, "entity": eid, "change": "removed"}
                )
            for eid in entities_a & entities_b:
                if ds_a.entities[eid].properties != ds_b.entities[eid].properties:
                    diff_result["entities_changed"].append(
                        {"domain": domain_key, "entity": eid, "change": "modified"}
                    )

        return diff_result

    # ------------------------------------------------------------------
    # Bus events
    # ------------------------------------------------------------------

    def get_pending_bus_events(self) -> List[BusEvent]:
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events

    # ------------------------------------------------------------------
    # Internals
    # ------------------------------------------------------------------

    def _persist(self) -> None:
        self._storage.save(
            NAMESPACE, "current", self._state.model_dump(mode="json")
        )
