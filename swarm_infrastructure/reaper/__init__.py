"""Reaper — causal death engine for the X3 AGI substrate.

The Reaper decides when and how agents die.  Death is permanent.
Postmortem analysis extracts lessons.  Scars propagate to survivors.
"""

from swarm.reaper.engine import ReaperEngine
from swarm.reaper.postmortem import PostmortemAnalyzer, Postmortem
from swarm.reaper.scar_mechanics import ScarPropagator

__all__ = [
    "ReaperEngine",
    "PostmortemAnalyzer",
    "Postmortem",
    "ScarPropagator",
]
