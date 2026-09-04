"""Accuracy scoreboard — ranks agents by prediction accuracy.

Agents below accuracy thresholds receive warning and critical events
that trigger goal mutation via the Goal Genome.
"""

from __future__ import annotations

import logging
from collections import defaultdict
from typing import Dict, List, Optional, Tuple

from swarm.event_bus.events import BusEvent, EventType
from swarm.storage.backend import StorageBackend
from swarm.world_sim.schema import PredictionResult

logger = logging.getLogger(__name__)

DEFAULT_WARNING_THRESHOLD = 0.3
DEFAULT_CRITICAL_THRESHOLD = 0.15
CRITICAL_CONSECUTIVE_EPOCHS = 5


class AccuracyScoreboard:
    """Maintains per-agent rolling accuracy metrics.

    Args:
        storage: Persistence backend.
        warning_threshold: Accuracy below this triggers ACCURACY_WARNING.
        critical_threshold: Accuracy below this for N epochs triggers ACCURACY_CRITICAL.
    """

    def __init__(
        self,
        storage: StorageBackend,
        warning_threshold: float = DEFAULT_WARNING_THRESHOLD,
        critical_threshold: float = DEFAULT_CRITICAL_THRESHOLD,
    ) -> None:
        self._storage = storage
        self._warning_threshold = warning_threshold
        self._critical_threshold = critical_threshold
        self._pending_bus_events: List[BusEvent] = []

        # In-memory scoreboard: agent_id → list of (epoch, accuracy)
        self._scores: Dict[str, List[Tuple[int, float]]] = defaultdict(list)

    def update_from_results(
        self, epoch: int, results: List[PredictionResult]
    ) -> None:
        """Update scoreboard from prediction resolution results."""
        # Group by agent
        agent_results: Dict[str, List[PredictionResult]] = defaultdict(list)
        for r in results:
            agent_results[r.agent_id].append(r)

        for agent_id, agent_res in agent_results.items():
            accurate = sum(1 for r in agent_res if r.error_magnitude < 0.5)
            accuracy = accurate / max(len(agent_res), 1)
            self._scores[agent_id].append((epoch, accuracy))

            # Check warning threshold
            if accuracy < self._warning_threshold:
                self._pending_bus_events.append(
                    BusEvent(
                        event_type=EventType.ACCURACY_WARNING,
                        agent_id=agent_id,
                        layer="WORLD_SIM",
                        severity="WARNING",
                        payload={
                            "epoch": epoch,
                            "accuracy": round(accuracy, 4),
                            "threshold": self._warning_threshold,
                        },
                    )
                )
                logger.warning(
                    "ACCURACY_WARNING: agent=%s accuracy=%.4f < %.4f",
                    agent_id,
                    accuracy,
                    self._warning_threshold,
                )

            # Check critical threshold (consecutive epochs)
            if accuracy < self._critical_threshold:
                recent = self._scores[agent_id][-CRITICAL_CONSECUTIVE_EPOCHS:]
                if (
                    len(recent) >= CRITICAL_CONSECUTIVE_EPOCHS
                    and all(a < self._critical_threshold for _, a in recent)
                ):
                    self._pending_bus_events.append(
                        BusEvent(
                            event_type=EventType.ACCURACY_CRITICAL,
                            agent_id=agent_id,
                            layer="WORLD_SIM",
                            severity="ERROR",
                            payload={
                                "epoch": epoch,
                                "accuracy": round(accuracy, 4),
                                "consecutive_epochs": CRITICAL_CONSECUTIVE_EPOCHS,
                            },
                        )
                    )
                    logger.error(
                        "ACCURACY_CRITICAL: agent=%s below %.4f for %d consecutive epochs",
                        agent_id,
                        self._critical_threshold,
                        CRITICAL_CONSECUTIVE_EPOCHS,
                    )

    def get_rankings(self, last_n: int = 10) -> List[Dict]:
        """Return agents ranked by average accuracy (descending)."""
        rankings = []
        for agent_id, scores in self._scores.items():
            recent = scores[-last_n:]
            avg_accuracy = sum(a for _, a in recent) / max(len(recent), 1)
            rankings.append({
                "agent_id": agent_id,
                "avg_accuracy": round(avg_accuracy, 4),
                "total_predictions": len(scores),
            })
        rankings.sort(key=lambda x: x["avg_accuracy"], reverse=True)
        return rankings

    def get_agent_accuracy(self, agent_id: str) -> float:
        """Return most recent accuracy for an agent."""
        scores = self._scores.get(agent_id, [])
        if not scores:
            return 0.0
        return scores[-1][1]

    def get_pending_bus_events(self) -> List[BusEvent]:
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events
