"""Self-Improvement Engine — propose, cost-check, execute, record outcomes.

Improvement is permitted but NEVER free.
Cooldown periods enforce deliberation.
Budget checks enforce resource constraints.
Scars ensure failures have permanent consequence.
"""

from __future__ import annotations

import logging
import time
from datetime import datetime, timezone
from typing import Dict, List, Optional

from swarm.event_bus.events import BusEvent, EventType
from swarm.self_improve.cost import CostCalculator
from swarm.self_improve.scars import ScarRegistry
from swarm.self_improve.schema import (
    ImprovementOutcome,
    ImprovementProposal,
    ProposalStatus,
    Scar,
)
from swarm.storage.backend import StorageBackend

logger = logging.getLogger(__name__)

DEFAULT_COOLDOWN_SECONDS = 300  # 5 minutes between attempts
DEFAULT_SUCCESS_PROBABILITY = 0.7


class SelfImprovementEngine:
    """Manages the full lifecycle of self-improvement proposals.

    Args:
        storage: Persistence backend.
        agent_id: Agent identity.
        scars: ScarRegistry for this agent.
        cost_calculator: Cost computation engine.
        resource_budget: Remaining resource budget.
        cooldown_seconds: Minimum time between improvement attempts.
    """

    def __init__(
        self,
        storage: StorageBackend,
        agent_id: str,
        scars: ScarRegistry,
        cost_calculator: Optional[CostCalculator] = None,
        resource_budget: float = 100.0,
        cooldown_seconds: float = DEFAULT_COOLDOWN_SECONDS,
    ) -> None:
        self._storage = storage
        self._agent_id = agent_id
        self._scars = scars
        self._cost = cost_calculator or CostCalculator()
        self._resource_budget = resource_budget
        self._cooldown_seconds = cooldown_seconds
        self._namespace = f"self_improve:{agent_id}"
        self._last_attempt_time: float = 0.0
        self._pending_bus_events: List[BusEvent] = []

    @property
    def resource_budget(self) -> float:
        return self._resource_budget

    def propose(self, proposal: ImprovementProposal) -> ImprovementProposal:
        """Submit an improvement proposal.

        Validates: cooldown period, budget sufficiency.
        Computes actual cost from proficiency + scars.
        """
        now = time.monotonic()

        # Cooldown check
        if now - self._last_attempt_time < self._cooldown_seconds:
            remaining = self._cooldown_seconds - (now - self._last_attempt_time)
            proposal.status = ProposalStatus.REJECTED_COOLDOWN
            logger.warning(
                "Proposal rejected (cooldown): agent=%s remaining=%.1fs",
                self._agent_id,
                remaining,
            )
            self._persist_proposal(proposal)
            return proposal

        # Cost calculation
        scar_count = self._scars.count_in_domain(proposal.target_domain)
        cost = self._cost.calculate(proposal.current_proficiency, scar_count)
        proposal.estimated_cost = cost

        # Budget check
        if cost > self._resource_budget:
            proposal.status = ProposalStatus.REJECTED_BUDGET
            logger.warning(
                "Proposal rejected (budget): agent=%s cost=%.2f budget=%.2f",
                self._agent_id,
                cost,
                self._resource_budget,
            )
            self._persist_proposal(proposal)
            return proposal

        proposal.status = ProposalStatus.APPROVED
        self._persist_proposal(proposal)

        self._pending_bus_events.append(
            BusEvent(
                event_type=EventType.IMPROVEMENT_PROPOSED,
                agent_id=self._agent_id,
                layer="SELF_IMPROVE",
                payload={
                    "proposal_id": proposal.proposal_id,
                    "type": proposal.improvement_type,
                    "cost": cost,
                    "domain": proposal.target_domain,
                },
            )
        )

        logger.info(
            "Proposal approved: agent=%s type=%s cost=%.2f domain=%s",
            self._agent_id,
            proposal.improvement_type,
            cost,
            proposal.target_domain,
        )
        return proposal

    def execute(
        self,
        proposal: ImprovementProposal,
        success: bool = True,
        proficiency_delta: float = 0.0,
        side_effects: Optional[List[str]] = None,
    ) -> ImprovementOutcome:
        """Execute an approved proposal.

        ALWAYS deducts cost from budget.  Success is not guaranteed.
        Failed attempts create permanent scars.
        """
        if proposal.status != ProposalStatus.APPROVED:
            raise ValueError(
                f"Cannot execute proposal in status {proposal.status}"
            )

        proposal.status = ProposalStatus.EXECUTING
        self._persist_proposal(proposal)

        # Deduct cost — ALWAYS, even on failure
        self._resource_budget -= proposal.estimated_cost
        self._last_attempt_time = time.monotonic()

        proficiency_after = proposal.current_proficiency
        if success:
            proficiency_after += proficiency_delta
            proposal.status = ProposalStatus.SUCCEEDED
            event_type = EventType.IMPROVEMENT_SUCCEEDED
        else:
            # Small regression on failure
            proficiency_after = max(0.0, proficiency_after - 0.05)
            proposal.status = ProposalStatus.FAILED
            event_type = EventType.IMPROVEMENT_FAILED

            # Record scar — PERMANENT, NEVER deleted
            scar = Scar(
                agent_id=self._agent_id,
                proposal_id=proposal.proposal_id,
                improvement_type=proposal.improvement_type,
                target_domain=proposal.target_domain,
                target_capability=proposal.target_capability,
                cost_paid=proposal.estimated_cost,
                failure_reason="Improvement attempt failed",
            )
            self._scars.record(scar)

        proposal.resolved_at = datetime.now(timezone.utc)
        self._persist_proposal(proposal)

        outcome = ImprovementOutcome(
            proposal_id=proposal.proposal_id,
            agent_id=self._agent_id,
            success=success,
            actual_cost=proposal.estimated_cost,
            proficiency_before=proposal.current_proficiency,
            proficiency_after=proficiency_after,
            side_effects=side_effects or [],
        )

        self._storage.save(
            self._namespace,
            f"outcome:{proposal.proposal_id}",
            outcome.model_dump(mode="json"),
        )

        self._pending_bus_events.append(
            BusEvent(
                event_type=event_type,
                agent_id=self._agent_id,
                layer="SELF_IMPROVE",
                payload={
                    "proposal_id": proposal.proposal_id,
                    "cost": proposal.estimated_cost,
                    "proficiency_before": proposal.current_proficiency,
                    "proficiency_after": proficiency_after,
                    "budget_remaining": self._resource_budget,
                },
            )
        )

        logger.info(
            "Improvement %s: agent=%s cost=%.2f prof=%.4f→%.4f budget=%.2f",
            "succeeded" if success else "FAILED",
            self._agent_id,
            proposal.estimated_cost,
            proposal.current_proficiency,
            proficiency_after,
            self._resource_budget,
        )

        return outcome

    def get_history(self) -> List[Dict]:
        """Get all proposals for this agent."""
        return self._storage.query(self._namespace)

    def get_pending_bus_events(self) -> List[BusEvent]:
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        # Also collect scar events
        events.extend(self._scars.get_pending_bus_events())
        return events

    # ------------------------------------------------------------------
    # Internals
    # ------------------------------------------------------------------

    def _persist_proposal(self, proposal: ImprovementProposal) -> None:
        self._storage.save(
            self._namespace,
            f"proposal:{proposal.proposal_id}",
            proposal.model_dump(mode="json"),
        )
