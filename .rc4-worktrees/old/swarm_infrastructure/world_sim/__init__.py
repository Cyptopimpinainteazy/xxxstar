"""X3 AGI Substrate — Unified World Simulator.

A shared, canonical state graph that all agents perceive, predict against,
and are judged by.  Truth is not voted upon.  Truth is settled by
prediction accuracy.
"""

from swarm.world_sim.state_graph import WorldStateGraph
from swarm.world_sim.prediction import PredictionMarket
from swarm.world_sim.oracle import RealityOracle
from swarm.world_sim.scoreboard import AccuracyScoreboard

__all__ = [
    "WorldStateGraph",
    "PredictionMarket",
    "RealityOracle",
    "AccuracyScoreboard",
]
