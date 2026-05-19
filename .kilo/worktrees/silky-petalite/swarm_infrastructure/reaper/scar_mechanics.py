"""Scar Propagation — when an agent dies, surviving agents may inherit scars.

Scars are training signals: permanent damage that increases future costs
and shapes behavior.  They are NEVER deleted, NEVER healed.

Propagation rules:
- Level 1 (Soft): No scar propagation — only the dead agent carries scars.
- Level 2 (Hard): Surviving agents in the SAME domain receive warning scars.
- Level 3 (Causal): All surviving agents receive awareness scars.
  The mandate space is scorched — no new agent may inherit it.
"""

from __future__ import annotations

import logging
from typing import Dict, List, Optional

from swarm.event_bus.events import BusEvent, EventType
from swarm.reaper.schema import DeathLevel, KillDecision
from swarm.self_improve.schema import Scar
from swarm.self_improve.scars import ScarRegistry
from swarm.storage.backend import StorageBackend

logger = logging.getLogger(__name__)


class ScarPropagator:
    """Propagates scars from dead agents to survivors based on death level.

    Args:
        storage: Persistence backend.
    """

    def __init__(self, storage: StorageBackend) -> None:
        self._storage = storage
        self._pending_bus_events: List[BusEvent] = []

    def propagate(
        self,
        decision: KillDecision,
        survivor_registries: Dict[str, ScarRegistry],
        dead_agent_domains: Optional[List[str]] = None,
    ) -> int:
        """Propagate scars based on death level.

        Args:
            decision: The kill decision that triggered propagation.
            survivor_registries: Map of agent_id → ScarRegistry for living agents.
            dead_agent_domains: Domains the dead agent was active in.

        Returns:
            Count of scars propagated.
        """
        if not decision.should_kill:
            return 0

        if decision.death_level == DeathLevel.SOFT:
            # Level 1: No propagation
            logger.debug(
                "Soft kill for %s — no scar propagation",
                decision.agent_id,
            )
            return 0

        domains = dead_agent_domains or []
        propagated = 0

        if decision.death_level == DeathLevel.HARD:
            # Level 2: Same-domain survivors get warning scars
            propagated = self._propagate_domain_scars(
                decision, survivor_registries, domains
            )

        elif decision.death_level == DeathLevel.CAUSAL:
            # Level 3: ALL survivors get awareness scars
            propagated = self._propagate_causal_scars(
                decision, survivor_registries, domains
            )

        logger.warning(
            "Scar propagation: level=%s dead_agent=%s scars_propagated=%d",
            decision.death_level.value,
            decision.agent_id,
            propagated,
        )

        return propagated

    def _propagate_domain_scars(
        self,
        decision: KillDecision,
        survivor_registries: Dict[str, ScarRegistry],
        domains: List[str],
    ) -> int:
        """Level 2 (Hard): Propagate warning scars to same-domain survivors."""
        count = 0

        for agent_id, registry in survivor_registries.items():
            if agent_id == decision.agent_id:
                continue

            # Check if this survivor shares any domain with the dead agent
            # For now, propagate to all survivors — domain filtering
            # requires more context from the agent's GoalGenome
            for domain in domains:
                scar = Scar(
                    agent_id=agent_id,
                    proposal_id=f"propagated:{decision.decision_id}",
                    improvement_type="STRATEGY_SHIFT",
                    target_domain=domain,
                    target_capability="mortality_awareness",
                    cost_paid=0.0,  # Propagated scars don't cost
                    failure_reason=(
                        f"Inherited warning scar from death of "
                        f"{decision.agent_id} (cause: {decision.cause.value})"
                    ),
                )
                registry.record(scar)
                count += 1

                self._pending_bus_events.append(
                    BusEvent(
                        event_type=EventType.SCAR_RECORDED,
                        agent_id=agent_id,
                        layer="REAPER",
                        severity="WARNING",
                        payload={
                            "scar_type": "propagated_warning",
                            "source_agent": decision.agent_id,
                            "death_cause": decision.cause.value,
                            "domain": domain,
                        },
                    )
                )

        return count

    def _propagate_causal_scars(
        self,
        decision: KillDecision,
        survivor_registries: Dict[str, ScarRegistry],
        domains: List[str],
    ) -> int:
        """Level 3 (Causal): Propagate awareness scars to ALL survivors."""
        count = 0

        for agent_id, registry in survivor_registries.items():
            if agent_id == decision.agent_id:
                continue

            # Causal scars are domain-agnostic: everyone learns
            scar = Scar(
                agent_id=agent_id,
                proposal_id=f"causal:{decision.decision_id}",
                improvement_type="STRATEGY_SHIFT",
                target_domain=domains[0] if domains else "CROSS_DOMAIN",
                target_capability="causal_death_awareness",
                cost_paid=0.0,
                failure_reason=(
                    f"Causal death awareness scar — agent {decision.agent_id} "
                    f"died via Level 3 causal death "
                    f"(cause: {decision.cause.value}). "
                    f"Scorched mandates: {decision.scorched_mandates}"
                ),
            )
            registry.record(scar)
            count += 1

            self._pending_bus_events.append(
                BusEvent(
                    event_type=EventType.SCAR_RECORDED,
                    agent_id=agent_id,
                    layer="REAPER",
                    severity="CRITICAL",
                    payload={
                        "scar_type": "causal_awareness",
                        "source_agent": decision.agent_id,
                        "death_cause": decision.cause.value,
                        "scorched_mandates": decision.scorched_mandates,
                    },
                )
            )

        return count

    def get_pending_bus_events(self) -> List[BusEvent]:
        """Drain and return pending bus events."""
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events
