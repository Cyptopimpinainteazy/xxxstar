"""Self-Improvement Strategy Optimizer — Phase 5.

Uses causal analysis and Bayesian skill profiles to decide:
1. WHAT to improve  — which capability has highest expected ROI
2. WHEN to improve  — cooldown-adjusted timing based on streak/budget
3. HOW MUCH to risk — Kelly-criterion stake sizing for proposals

Built on top of:
- SelfImprovementEngine (Phase 1 — propose/execute/scar)
- CausalGraph (Phase 2 — blame/credit chains)
- EnhancedPredictionMarket (Phase 4 — Bayesian profiles)
"""

from __future__ import annotations

import math
from collections import defaultdict
from typing import Any, Dict, List, Optional, Tuple

from pydantic import BaseModel, Field

from swarm.causal.attribution import AttributionEngine
from swarm.causal.graph import CausalGraph
from swarm.causal.schema import NodeType
from swarm.self_improve.schema import ImprovementProposal, ImprovementType, Scar
from swarm.storage.backend import StorageBackend

NAMESPACE = "improve_strategy"


# ──────────────────────────────────────────────────────────────────
# Schemas
# ──────────────────────────────────────────────────────────────────

class ImprovementCandidate(BaseModel):
    """A scored candidate for self-improvement."""
    capability: str
    domain: str
    current_proficiency: float
    expected_roi: float          # Expected return on investment
    risk_score: float            # 0=safe, 1=risky
    causal_blame: float          # How much this capability caused failures
    scar_count: int
    priority_rank: int = 0


class StrategyReport(BaseModel):
    """Analysis of improvement opportunities for an agent."""
    agent_id: str
    epoch: int
    budget_remaining: float
    candidates: List[ImprovementCandidate] = Field(default_factory=list)
    recommended: Optional[ImprovementCandidate] = None
    should_wait: bool = False    # True if cooldown/budget says wait
    wait_reason: str = ""


# ──────────────────────────────────────────────────────────────────
# Core Engine
# ──────────────────────────────────────────────────────────────────

class StrategyOptimizer:
    """Decides what and when to improve, using causal + Bayesian signals.

    Args:
        storage: Persistence backend.
        causal_graph: For blame/credit analysis.
        attribution: For computing causal influence scores.
    """

    def __init__(
        self,
        storage: StorageBackend,
        causal_graph: CausalGraph,
        attribution: Optional[AttributionEngine] = None,
    ) -> None:
        self._storage = storage
        self._causal = causal_graph
        self._attribution = attribution or AttributionEngine(causal_graph)

    # ------------------------------------------------------------------
    # Analyze
    # ------------------------------------------------------------------

    def analyze(
        self,
        agent_id: str,
        capabilities: Dict[str, float],
        scars: List[Scar],
        budget: float,
        epoch: int,
        min_budget_fraction: float = 0.2,
    ) -> StrategyReport:
        """Analyze improvement opportunities for an agent.

        Args:
            agent_id: Agent identifier.
            capabilities: {capability_name: proficiency} dict.
            scars: Agent's scar list.
            budget: Current resource budget.
            epoch: Current epoch.
            min_budget_fraction: Don't improve if budget below this fraction of max.

        Returns:
            StrategyReport with ranked candidates and recommendation.
        """
        report = StrategyReport(
            agent_id=agent_id,
            epoch=epoch,
            budget_remaining=budget,
        )

        # Safety check: don't spend when budget is critically low
        if budget < 1.0:
            report.should_wait = True
            report.wait_reason = "Budget critically low"
            return report

        # Build scar index
        scar_counts: Dict[str, int] = defaultdict(int)
        for scar in scars:
            scar_counts[scar.target_capability] += 1

        # Get causal blame for each capability
        blame_scores = self._compute_blame_scores(agent_id, capabilities)

        # Score each capability
        candidates: List[ImprovementCandidate] = []
        for cap_name, proficiency in capabilities.items():
            domain = self._infer_domain(cap_name)
            scar_count = scar_counts.get(cap_name, 0)
            blame = blame_scores.get(cap_name, 0.0)

            # ROI estimate: low proficiency + high blame = high ROI
            # Scars reduce ROI (diminishing returns)
            roi = self._estimate_roi(proficiency, blame, scar_count)
            risk = self._estimate_risk(proficiency, scar_count, budget)

            candidates.append(ImprovementCandidate(
                capability=cap_name,
                domain=domain,
                current_proficiency=proficiency,
                expected_roi=round(roi, 4),
                risk_score=round(risk, 4),
                causal_blame=round(blame, 4),
                scar_count=scar_count,
            ))

        # Rank by ROI (descending), break ties by risk (ascending)
        candidates.sort(key=lambda c: (-c.expected_roi, c.risk_score))
        for i, c in enumerate(candidates):
            c.priority_rank = i + 1

        report.candidates = candidates

        # Recommend top candidate if ROI is positive and risk acceptable
        if candidates and candidates[0].expected_roi > 0:
            top = candidates[0]
            if top.risk_score < 0.8:  # Don't recommend very risky improvements
                report.recommended = top
            else:
                report.should_wait = True
                report.wait_reason = f"Top candidate too risky (risk={top.risk_score:.2f})"

        return report

    # ------------------------------------------------------------------
    # Generate proposals from strategy
    # ------------------------------------------------------------------

    def generate_proposal(
        self,
        candidate: ImprovementCandidate,
        agent_id: str,
        improvement_type: ImprovementType = ImprovementType.CAPABILITY_UPGRADE,
    ) -> ImprovementProposal:
        """Create a concrete ImprovementProposal from a candidate."""
        return ImprovementProposal(
            agent_id=agent_id,
            improvement_type=improvement_type,
            target_capability=candidate.capability,
            target_domain=candidate.domain,
            description=(
                f"ROI={candidate.expected_roi:.2f} "
                f"blame={candidate.causal_blame:.2f} "
                f"risk={candidate.risk_score:.2f}"
            ),
            current_proficiency=candidate.current_proficiency,
            expected_proficiency_delta=self._expected_delta(
                candidate.current_proficiency,
                candidate.scar_count,
            ),
        )

    # ------------------------------------------------------------------
    # Retrospective learning
    # ------------------------------------------------------------------

    def learn_from_outcome(
        self,
        agent_id: str,
        capability: str,
        success: bool,
        cost: float,
        proficiency_delta: float,
    ) -> Dict[str, Any]:
        """Record an improvement outcome for future strategy decisions.

        Returns a summary dict for logging.
        """
        record = {
            "agent_id": agent_id,
            "capability": capability,
            "success": success,
            "cost": cost,
            "proficiency_delta": proficiency_delta,
            "roi_actual": proficiency_delta / max(cost, 0.01),
        }
        self._storage.save(
            NAMESPACE,
            f"outcome:{agent_id}:{capability}:{self._next_id(agent_id)}",
            record,
        )
        return record

    def historical_success_rate(
        self,
        agent_id: str,
        capability: Optional[str] = None,
    ) -> float:
        """Historical success rate for an agent's improvements."""
        filters: Dict[str, Any] = {"agent_id": agent_id}
        if capability:
            filters["capability"] = capability

        records = self._storage.query(NAMESPACE, filters=filters)
        if not records:
            return 0.5  # Neutral prior

        successes = sum(1 for r in records if r.get("success", False))
        return successes / len(records)

    # ------------------------------------------------------------------
    # Internal scoring functions
    # ------------------------------------------------------------------

    def _compute_blame_scores(
        self,
        agent_id: str,
        capabilities: Dict[str, float],
    ) -> Dict[str, float]:
        """Use causal graph to assign blame to capabilities.

        Looks at negative-value nodes (failures, deaths) and traces
        blame back through action types that correspond to capabilities.
        """
        scores: Dict[str, float] = defaultdict(float)

        nodes = self._causal.get_nodes_for_agent(agent_id)
        negative_nodes = [
            n for n in nodes
            if n.node_type in (
                NodeType.DEATH.value,
                NodeType.CONSEQUENCE.value,
            )
            and n.value < 0
        ]

        for neg_node in negative_nodes:
            # Trace blame to ancestors
            ancestors = self._causal.ancestors(neg_node.node_id)
            for ancestor in ancestors:
                if ancestor.node_type == NodeType.ACTION.value:
                    # Map action_type to capability (heuristic)
                    action_cap = self._action_to_capability(
                        ancestor.action_type, capabilities
                    )
                    if action_cap:
                        scores[action_cap] += abs(neg_node.value) * 0.1

        # Normalize
        max_score = max(scores.values()) if scores else 1.0
        if max_score > 0:
            for k in scores:
                scores[k] /= max_score

        return dict(scores)

    @staticmethod
    def _action_to_capability(
        action_type: str,
        capabilities: Dict[str, float],
    ) -> Optional[str]:
        """Heuristic mapping from action_type to capability name."""
        action_lower = action_type.lower()
        for cap in capabilities:
            if cap.lower() in action_lower or action_lower in cap.lower():
                return cap
        # Fallback: return first capability if any
        if capabilities:
            return next(iter(capabilities))
        return None

    @staticmethod
    def _estimate_roi(
        proficiency: float,
        blame: float,
        scar_count: int,
    ) -> float:
        """Estimate ROI of improving a capability.

        High blame + low proficiency = high ROI.
        Scars reduce expected ROI (harder to improve).
        """
        # Potential gain: how much room to improve
        potential = 1.0 - proficiency
        # Blame amplifies value: fixing blamed capabilities is more valuable
        value = potential * (1.0 + blame * 2.0)
        # Scar discount: each scar reduces expected ROI by 15%
        discount = max(0.1, 1.0 - 0.15 * scar_count)
        return value * discount

    @staticmethod
    def _estimate_risk(
        proficiency: float,
        scar_count: int,
        budget: float,
    ) -> float:
        """Estimate risk of an improvement attempt.

        Risk increases with: more scars, lower budget, lower proficiency.
        """
        scar_risk = min(1.0, scar_count * 0.2)
        budget_risk = max(0.0, 1.0 - budget / 100.0)
        proficiency_risk = 0.0 if proficiency > 0.5 else 0.3

        return min(1.0, (scar_risk + budget_risk + proficiency_risk) / 3.0)

    @staticmethod
    def _expected_delta(
        current_proficiency: float,
        scar_count: int,
    ) -> float:
        """Expected proficiency gain — diminishing returns."""
        base_gain = 0.1 * (1.0 - current_proficiency)
        scar_penalty = max(0.1, 1.0 - 0.1 * scar_count)
        return round(base_gain * scar_penalty, 4)

    @staticmethod
    def _infer_domain(capability: str) -> str:
        """Heuristic: map capability name to domain."""
        cap_lower = capability.lower()
        if any(w in cap_lower for w in ("trade", "trading", "price", "market", "arb")):
            return "MARKET"
        if any(w in cap_lower for w in ("code", "compile", "debug", "build")):
            return "CODE"
        if any(w in cap_lower for w in ("vote", "govern", "proposal")):
            return "GOVERNANCE"
        if any(w in cap_lower for w in ("infra", "deploy", "node", "network")):
            return "INFRASTRUCTURE"
        return "CROSS_DOMAIN"

    def _next_id(self, agent_id: str) -> int:
        """Simple incrementing ID for outcome records."""
        records = self._storage.query(NAMESPACE, filters={"agent_id": agent_id})
        return len(records) + 1
