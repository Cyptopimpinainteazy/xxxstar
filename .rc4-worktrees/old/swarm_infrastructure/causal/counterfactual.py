"""Counterfactual Engine — "what if?" analysis over the causal graph.

Estimates what would have happened if a specific action had NOT been taken.
This is the core of the agent's introspective capability:

  "If I hadn't done X, would I still have died?"
  "If I hadn't pursued goal G, would my fitness be higher?"
  "Was this scar actually caused by my action, or inevitable?"

Two estimation methods:
1. CAUSAL_REMOVAL — remove the node and all downstream edges,
   re-estimate the outcome from remaining paths
2. MARGINAL_IMPACT — use attribution scores to estimate the
   fractional impact of removing one cause

Used by:
- Self-Improvement: "should I avoid this action pattern in the future?"
- Postmortem: "was this death avoidable?"
- Reaper: "is this agent pattern causally toxic?"
"""

from __future__ import annotations

import logging
from typing import Dict, List, Optional, Set

from swarm.causal.attribution import AttributionEngine
from swarm.causal.graph import CausalGraph
from swarm.causal.schema import (
    CausalNode,
    Counterfactual,
    NodeType,
)

logger = logging.getLogger(__name__)


class CounterfactualEngine:
    """Estimate outcomes in hypothetical scenarios where actions are removed.

    Args:
        graph: The causal graph to analyze.
        attribution: The attribution engine for marginal impact estimation.
    """

    def __init__(
        self,
        graph: CausalGraph,
        attribution: Optional[AttributionEngine] = None,
    ) -> None:
        self._graph = graph
        self._attribution = attribution or AttributionEngine(graph)

    def what_if_removed(
        self,
        removed_node_id: str,
        target_node_id: str,
        method: str = "causal_removal",
    ) -> Counterfactual:
        """Estimate what would have happened to target if removed_node never occurred.

        Args:
            removed_node_id: The action to hypothetically remove.
            target_node_id: The outcome to measure impact on.
            method: "causal_removal" or "marginal_impact".

        Returns:
            Counterfactual with estimated outcome delta and confidence bounds.
        """
        if method == "causal_removal":
            return self._causal_removal(removed_node_id, target_node_id)
        elif method == "marginal_impact":
            return self._marginal_impact(removed_node_id, target_node_id)
        else:
            raise ValueError(f"Unknown counterfactual method: {method}")

    def was_death_avoidable(
        self,
        death_node_id: str,
        top_n: int = 3,
    ) -> List[Counterfactual]:
        """Analyze whether an agent's death could have been avoided.

        For each of the top contributors to the death, estimate
        whether removing that action would have prevented death.

        Returns: List of counterfactuals for the top contributors,
        sorted by |estimated_outcome_delta| descending.
        """
        death_node = self._graph.get_node(death_node_id)
        if not death_node:
            return []

        # Get top contributors to death
        scores = self._attribution.top_contributors(
            death_node_id, n=top_n, method="path_weight"
        )

        results = []
        for score in scores:
            cf = self.what_if_removed(
                score.cause_node_id,
                death_node_id,
                method="causal_removal",
            )
            results.append(cf)

        results.sort(key=lambda c: abs(c.estimated_outcome_delta), reverse=True)
        return results

    def retrospective_analysis(
        self,
        agent_id: str,
        target_node_id: str,
    ) -> Dict[str, Counterfactual]:
        """For each ACTION node in agent's history, estimate its impact on target.

        Returns dict: node_id → Counterfactual.
        Useful for the self-improvement engine to learn which action
        patterns to reinforce or avoid.
        """
        agent_actions = self._graph.get_nodes_by_type(
            NodeType.ACTION, agent_id=agent_id
        )

        results = {}
        for action in agent_actions:
            if action.node_id == target_node_id:
                continue
            cf = self.what_if_removed(
                action.node_id,
                target_node_id,
                method="marginal_impact",
            )
            # Only include actions that had measurable impact
            if abs(cf.estimated_outcome_delta) > 0.01:
                results[action.node_id] = cf

        return results

    def find_toxic_patterns(
        self,
        agent_id: str,
        negative_threshold: float = -0.1,
    ) -> List[CausalNode]:
        """Find actions that consistently lead to negative outcomes.

        An action is "toxic" if it has high negative attribution across
        multiple negative outcome nodes (consequences, scars, deaths).
        """
        # Get all negative outcome nodes for this agent
        neg_types = [NodeType.SCAR, NodeType.DEATH]
        negative_nodes = []
        for nt in neg_types:
            negative_nodes.extend(
                self._graph.get_nodes_by_type(nt, agent_id=agent_id)
            )

        # Also include consequences with negative value
        consequences = self._graph.get_nodes_by_type(
            NodeType.CONSEQUENCE, agent_id=agent_id
        )
        negative_nodes.extend([c for c in consequences if c.value < 0])

        if not negative_nodes:
            return []

        # For each action, count how many negative outcomes it contributed to
        action_blame_count: Dict[str, int] = {}
        action_blame_total: Dict[str, float] = {}

        for neg_node in negative_nodes:
            scores = self._attribution.attribute(neg_node.node_id)
            for score in scores:
                cause = self._graph.get_node(score.cause_node_id)
                if cause and cause.node_type == NodeType.ACTION.value:
                    aid = cause.node_id
                    action_blame_count[aid] = action_blame_count.get(aid, 0) + 1
                    action_blame_total[aid] = (
                        action_blame_total.get(aid, 0.0) + score.attribution_share
                    )

        # Filter to actions that appear in multiple negative outcomes
        toxic = []
        for node_id, count in action_blame_count.items():
            if count >= 2 or action_blame_total.get(node_id, 0) > 0.5:
                node = self._graph.get_node(node_id)
                if node:
                    toxic.append(node)

        return toxic

    # ------------------------------------------------------------------
    # Estimation methods
    # ------------------------------------------------------------------

    def _causal_removal(
        self,
        removed_id: str,
        target_id: str,
    ) -> Counterfactual:
        """Remove a node and estimate the target outcome without it.

        Strategy:
        1. Get all causal chains from removed_node to target.
        2. Compute total causal weight flowing through removed_node.
        3. Estimate counterfactual value = baseline - (weight * baseline).
        4. Confidence bounds based on number of alternative paths.
        """
        removed = self._graph.get_node(removed_id)
        target = self._graph.get_node(target_id)

        if not removed or not target:
            return Counterfactual(
                removed_node_id=removed_id,
                target_node_id=target_id,
            )

        baseline = target.value

        # Get all chains to target
        all_chains = self._graph.get_all_chains_to(
            target_id, max_depth=15, max_chains=20
        )

        # Separate chains that go through removed_node vs not
        chains_through = []
        chains_around = []
        for chain in all_chains:
            node_ids = {n.node_id for n in chain.nodes}
            if removed_id in node_ids:
                chains_through.append(chain)
            else:
                chains_around.append(chain)

        # Weight flowing through the removed node
        weight_through = sum(c.total_weight for c in chains_through)
        weight_around = sum(c.total_weight for c in chains_around)
        total_weight = weight_through + weight_around

        if total_weight <= 0:
            # No causal path — removal has no effect
            return Counterfactual(
                removed_node_id=removed_id,
                target_node_id=target_id,
                estimated_outcome_delta=0.0,
                confidence_lower=0.0,
                confidence_upper=0.0,
                baseline_value=baseline,
                counterfactual_value=baseline,
                method="causal_removal",
            )

        # Fraction of causality flowing through the removed node
        fraction_through = weight_through / total_weight

        # Estimated impact: removing the node removes this fraction of the outcome
        impact = baseline * fraction_through

        # Counterfactual value (what the outcome would have been)
        cf_value = baseline - impact

        # Confidence bounds: more alternative paths → tighter bounds
        num_alternatives = len(chains_around)
        if num_alternatives == 0:
            # No alternatives — high confidence it was critical
            confidence = 0.9
        else:
            confidence = 0.5 + 0.4 * (1.0 / (1.0 + num_alternatives))

        ci_width = abs(impact) * (1.0 - confidence)

        return Counterfactual(
            removed_node_id=removed_id,
            target_node_id=target_id,
            estimated_outcome_delta=-impact,  # Negative = outcome would be less
            confidence_lower=-impact - ci_width,
            confidence_upper=-impact + ci_width,
            baseline_value=baseline,
            counterfactual_value=cf_value,
            method="causal_removal",
        )

    def _marginal_impact(
        self,
        removed_id: str,
        target_id: str,
    ) -> Counterfactual:
        """Estimate impact using attribution scores (faster, less accurate).

        Uses the attribution share of removed_node on target_node
        as a direct proxy for the counterfactual delta.
        """
        target = self._graph.get_node(target_id)
        if not target:
            return Counterfactual(
                removed_node_id=removed_id,
                target_node_id=target_id,
            )

        baseline = target.value

        # Get attribution share
        scores = self._attribution.attribute(target_id, method="path_weight")
        share = 0.0
        confidence = 0.5
        for score in scores:
            if score.cause_node_id == removed_id:
                share = score.attribution_share
                confidence = score.confidence
                break

        impact = baseline * share
        cf_value = baseline - impact
        ci_width = abs(impact) * 0.3  # Fixed 30% CI width for marginal method

        return Counterfactual(
            removed_node_id=removed_id,
            target_node_id=target_id,
            estimated_outcome_delta=-impact,
            confidence_lower=-impact - ci_width,
            confidence_upper=-impact + ci_width,
            baseline_value=baseline,
            counterfactual_value=cf_value,
            method="marginal_impact",
            metadata={"attribution_share": share},
        )
