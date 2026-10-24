"""Swarm core exports without loading the agent graph for shared enums."""

from importlib import import_module
from typing import TYPE_CHECKING

from swarm.core.enums import Domain, Outcome

if TYPE_CHECKING:
    from swarm.core.agent import Agent, AgentConfig, ActionResult, Consequence
    from swarm.core.lifecycle import EpochOrchestrator, EpochStats

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


def __getattr__(name: str):
    # Schema modules import core.enums while Reaper/SelfImprove are still loading.
    # Eager Agent imports here re-enter those partially initialized packages.
    if name in {"Agent", "AgentConfig", "ActionResult", "Consequence"}:
        module = "swarm.core.agent"
    elif name in {"EpochOrchestrator", "EpochStats"}:
        module = "swarm.core.lifecycle"
    else:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    value = getattr(import_module(module), name)
    globals()[name] = value
    return value
