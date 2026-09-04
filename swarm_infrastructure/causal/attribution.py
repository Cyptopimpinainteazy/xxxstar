"""Attribution Engine — assigns credit/blame to causal ancestors.

Given an outcome node, the attribution engine walks backwards through
the causal graph and computes how much each ancestor contributed
to the outcome.

Three attribution methods:
1. PATH_WEIGHT  — product of edge weights along each path, normalized
2. SHAPLEY      — simplified Shapley-like marginal contribution analysis
3. RECENCY      — exponential decay favoring recent causes

The engine is used by:
- Self-Improvement: to decide which capabilities to invest in
- Reaper: to justify Level 3 causal kills
- Postmortem: to identify root causes of death
"""

from __future__ import annotations

import logging
import math
from collections import defaultdict
from typing import Dict, List, Optional

from swarm.causal.graph import CausalGraph
from swarm.causal.schema import (
    AttributionScore,
    CausalNode,
    NodeType,
)

logger = logging.getLogger(__name__)


class AttributionEngine:
    """Compute attribution scores for causal ancestors of an outcome.

    Args:
        graph: The causal graph to analyze.
    """

    def __init__(self, graph: CausalGraph) -> None:
        self._graph = graph

    def attribute(
        self,
        target_node_id: str,
        method: str = "path_weight",
        max_depth: int = 15,
        min_weight: float = 0.01,
    ) -> List[AttributionScore]:
        """Compute attribution scores for all ancestors of a target node.

        Args:
            target_node_id: The outcome node to attribute.
            method: One of "path_weight", "shapley", "recency".
            max_depth: Maximum ancestor depth to search.
            min_weight: Minimum attribution share to include in results.

        Returns:
            List of AttributionScore, sorted by attribution_share descending.
        """
        if method == "path_weight":
            return self._attribute_path_weight(target_node_id, max_depth, min_weight)
        elif method == "shapley":
            return self._attribute_shapley(target_node_id, max_depth, min_weight)
        elif method == "recency":
            return self._attribute_recency(target_node_id, max_depth, min_weight)
        else:
            raise ValueError(f"Unknown attribution method: {method}")

    def top_contributors(
        self,
        target_node_id: str,
        n: int = 5,
        method: str = "path_weight",
    ) -> List[AttributionScore]:
        """Get the top N contributors to an outcome."""
        all_scores = self.attribute(target_node_id, method=method)
        return all_scores[:n]

    def blame_chain(
        self,
        target_node_id: str,
        method: str = "path_weight",
    ) -> List[AttributionScore]:
        """Get only the negative-outcome contributors (blame assignment).

        Filters to ancestors that produced negative consequences
        (costs, energy drains, scars, etc.) leading to the target.
        """
        all_scores = self.attribute(target_node_id, method=method)
        blame_types = {
            NodeType.CONSEQUENCE.value,
            NodeType.SCAR.value,
            NodeType.DEATH.value,
        }

        result = []
        for score in all_scores:
            node = self._graph.get_node(score.cause_node_id)
            if node and (
                node.node_type in blame_types
                or node.value < 0
            ):
                result.append(score)

        return result

    def credit_chain(
        self,
        target_node_id: str,
        method: str = "path_weight",
    ) -> List[AttributionScore]:
        """Get only the positive-outcome contributors (credit assignment).

        Filters to ancestors that produced positive consequences
        (rewards, successful predictions, etc.) leading to the target.
        """
        all_scores = self.attribute(target_node_id, method=method)

        result = []
        for score in all_scores:
            node = self._graph.get_node(score.cause_node_id)
            if node and node.value > 0:
                result.append(score)

        return result

    # ------------------------------------------------------------------
    # Attribution methods
    # ------------------------------------------------------------------

    def _attribute_path_weight(
        self,
        target_id: str,
        max_depth: int,
        min_weight: float,
    ) -> List[AttributionScore]:
        """Path-weight attribution: product of edge weights along each path.

        For each ancestor, compute the sum of (product of weights along each
        path from ancestor to target).  Then normalize so all shares sum to 1.0.
        """
        chains = self._graph.get_all_chains_to(
            target_id, max_depth=max_depth, max_chains=50
        )

        # Accumulate raw weight per ancestor node
        raw_scores: Dict[str, float] = defaultdict(float)
        for chain in chains:
            for node in chain.nodes:
                if node.node_id == target_id:
                    continue
                # The node's contribution is proportional to the chain weight
                raw_scores[node.node_id] += chain.total_weight

        if not raw_scores:
            return []

        # Normalize to [0, 1]
        total = sum(raw_scores.values())
        if total <= 0:
            return []

        results = []
        for node_id, raw in raw_scores.items():
            share = raw / total
            if share < min_weight:
                continue
            results.append(
                AttributionScore(
                    cause_node_id=node_id,
                    effect_node_id=target_id,
                    attribution_share=round(share, 6),
                    confidence=min(1.0, share * 2),  # Higher share → higher confidence
                    method="path_weight",
                )
            )

        results.sort(key=lambda s: s.attribution_share, reverse=True)
        return results

    def _attribute_shapley(
        self,
        target_id: str,
        max_depth: int,
        min_weight: float,
    ) -> List[AttributionScore]:
        """Simplified Shapley-like attribution.

        For each direct cause: estimate marginal contribution by comparing
        the graph influence WITH vs WITHOUT that cause.

        This is a simplification — true Shapley values require exponential
        computation over all subsets.  We approximate by using the
        node_influence delta.
        """
        direct_causes = self._graph.direct_causes(target_id)
        if not direct_causes:
            return []

        # Base influence: how much influence does the target node have
        # without removing anything
        target_node = self._graph.get_node(target_id)
        if not target_node:
            return []

        # Compute influence of each direct cause on the target
        raw_scores: Dict[str, float] = {}
        for cause in direct_causes:
            # Use the edge weight as the marginal contribution proxy
            edge_weight = 0.0
            for eid in self._graph._incoming.get(target_id, []):
                edge = self._graph._edges[eid]
                if edge.cause_node_id == cause.node_id:
                    edge_weight = edge.weight * edge.confidence
                    break

            # Also factor in the cause's own influence (transitivity)
            cause_influence = self._graph.node_influence(cause.node_id)
            raw_scores[cause.node_id] = edge_weight * (1.0 + cause_influence * 0.1)

        if not raw_scores:
            return []

        total = sum(raw_scores.values())
        if total <= 0:
            return []

        results = []
        for node_id, raw in raw_scores.items():
            share = raw / total
            if share < min_weight:
                continue
            results.append(
                AttributionScore(
                    cause_node_id=node_id,
                    effect_node_id=target_id,
                    attribution_share=round(share, 6),
                    confidence=0.7,  # Shapley approximation is less confident
                    method="shapley",
                )
            )

        results.sort(key=lambda s: s.attribution_share, reverse=True)
        return results

    def _attribute_recency(
        self,
        target_id: str,
        max_depth: int,
        min_weight: float,
    ) -> List[AttributionScore]:
        """Recency-weighted attribution: exponential decay favoring recent causes.

        Recent actions get more credit/blame than distant ones.
        Decay factor: weight * exp(-0.3 * epoch_distance).
        """
        target = self._graph.get_node(target_id)
        if not target:
            return []

        ancestors = self._graph.ancestors(target_id, max_depth=max_depth)
        if not ancestors:
            return []

        DECAY_RATE = 0.3

        raw_scores: Dict[str, float] = {}
        for ancestor in ancestors:
            epoch_distance = max(0, target.epoch - ancestor.epoch)
            decay = math.exp(-DECAY_RATE * epoch_distance)

            # Also factor in edge weight from the ancestor to target path
            chain = self._graph.get_chain_to(target_id, max_depth=max_depth)
            path_weight = chain.total_weight if chain.nodes else 0.0

            raw_scores[ancestor.node_id] = decay * max(path_weight, 0.01)

        total = sum(raw_scores.values())
        if total <= 0:
            return []

        results = []
        for node_id, raw in raw_scores.items():
            share = raw / total
            if share < min_weight:
                continue
            results.append(
                AttributionScore(
                    cause_node_id=node_id,
                    effect_node_id=target_id,
                    attribution_share=round(share, 6),
                    confidence=0.6,  # Recency heuristic is approximate
                    method="recency",
                )
            )

        results.sort(key=lambda s: s.attribution_share, reverse=True)
        return results
