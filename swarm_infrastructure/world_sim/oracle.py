"""Reality Oracle — resolves predictions against actual world state.

Truth is not voted upon.  Truth is settled by accuracy.
Agents that model reality well survive.  Agents that model poorly pay.
"""

from __future__ import annotations

import logging
from typing import List

from swarm.event_bus.events import BusEvent, EventType
from swarm.storage.backend import StorageBackend
from swarm.world_sim.prediction import PredictionMarket
from swarm.world_sim.schema import Prediction, PredictionResult
from swarm.world_sim.state_graph import WorldStateGraph

logger = logging.getLogger(__name__)

RESULTS_NAMESPACE = "prediction_results"


class RealityOracle:
    """Resolves predictions by comparing them against actual world state."""

    def __init__(
        self,
        world: WorldStateGraph,
        market: PredictionMarket,
        storage: StorageBackend,
    ) -> None:
        self._world = world
        self._market = market
        self._storage = storage
        self._pending_bus_events: List[BusEvent] = []

    def resolve_predictions(self, epoch: int) -> List[PredictionResult]:
        """Resolve all predictions targeting *epoch*.

        For each prediction:
        - Retrieve actual value from WorldState via dot-notation query.
        - Compute error_magnitude = |predicted - actual| / max(|actual|, 0.001)
        - Compute reward_or_penalty:
            if error < 0.5: stake * (1 - error_magnitude)
            else: -stake * error_magnitude
        """
        predictions = self._market.get_pending_predictions(epoch=epoch)
        results: List[PredictionResult] = []

        for pred in predictions:
            actual = self._world.query(pred.target_state_path)

            if actual is None:
                logger.warning(
                    "Cannot resolve prediction %s — path %s not found",
                    pred.prediction_id[:8],
                    pred.target_state_path,
                )
                continue

            try:
                actual_float = float(actual)
            except (TypeError, ValueError):
                logger.warning(
                    "Cannot resolve prediction %s — non-numeric value at %s",
                    pred.prediction_id[:8],
                    pred.target_state_path,
                )
                continue

            error = abs(pred.predicted_value - actual_float) / max(
                abs(actual_float), 0.001
            )

            if error < 0.5:
                reward = pred.stake * (1.0 - error)
            else:
                reward = -pred.stake * error

            result = PredictionResult(
                prediction_id=pred.prediction_id,
                agent_id=pred.agent_id,
                predicted_value=pred.predicted_value,
                actual_value=actual_float,
                error_magnitude=error,
                stake=pred.stake,
                reward_or_penalty=reward,
            )
            results.append(result)

            # Persist result
            self._storage.save(
                RESULTS_NAMESPACE,
                result.prediction_id,
                result.model_dump(mode="json"),
            )

            # Remove from pending
            self._market.remove_prediction(pred.prediction_id)

            # Emit event
            self._pending_bus_events.append(
                BusEvent(
                    event_type=EventType.PREDICTION_RESOLVED,
                    agent_id=pred.agent_id,
                    layer="WORLD_SIM",
                    payload={
                        "prediction_id": pred.prediction_id,
                        "error": round(error, 4),
                        "reward": round(reward, 4),
                    },
                )
            )

            logger.info(
                "Resolved: agent=%s pred=%.4f actual=%.4f err=%.4f reward=%.4f",
                pred.agent_id,
                pred.predicted_value,
                actual_float,
                error,
                reward,
            )

        return results

    def get_pending_bus_events(self) -> List[BusEvent]:
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events
