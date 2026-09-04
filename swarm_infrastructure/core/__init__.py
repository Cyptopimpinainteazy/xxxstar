"""Swarm core package — orchestrator and shared types."""

from swarm.core.agent import Agent, AgentConfig, ActionResult, Consequence
from swarm.core.enums import Domain, Outcome
from swarm.core.lifecycle import EpochOrchestrator, EpochStats

# SubstrateWiring is NOT imported here to avoid circular imports
# (wiring.py imports from swarm.agents which depends on swarm.core).
# Import it directly: from swarm.core.wiring import SubstrateWiring

__all__ = [
    "Agent",
    "AgentConfig",
    "ActionResult",
    "Consequence",
    "Domain",
    "Outcome",
    "EpochOrchestrator",
    "EpochStats",
]
