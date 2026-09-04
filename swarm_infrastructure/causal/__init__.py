"""Causal Analysis Layer — tracing cause and effect through agent lifetimes.

Provides:
- ``CausalGraph``     — directed acyclic graph of action → outcome edges
- ``AttributionEngine`` — scores how much each action contributed to an outcome
- ``CounterfactualEngine`` — estimates what would have happened without an action
- ``CausalChainBuilder``  — reconstructs the full causal chain leading to an event

The causal graph is the spine of agent introspection.  Every agent
action creates a node; every observed consequence creates an edge.
The Reaper uses causal chains to justify Level 3 kills, and
the Self-Improvement engine uses attribution scores to decide
which capabilities to invest in.
"""

from swarm.causal.schema import (
    CausalNode,
    CausalEdge,
    CausalChain,
    AttributionScore,
    Counterfactual,
)
from swarm.causal.graph import CausalGraph
from swarm.causal.attribution import AttributionEngine
from swarm.causal.counterfactual import CounterfactualEngine

__all__ = [
    "CausalNode",
    "CausalEdge",
    "CausalChain",
    "AttributionScore",
    "Counterfactual",
    "CausalGraph",
    "AttributionEngine",
    "CounterfactualEngine",
]
