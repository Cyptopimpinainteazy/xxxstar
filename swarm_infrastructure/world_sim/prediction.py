"""Agent prediction interface — the prediction market."""

from __future__ import annotations

import logging
from typing import Dict, List, Optional

from swarm.event_bus.events import BusEvent, EventType
from swarm.storage.backend import StorageBackend
from swarm.world_sim.schema import Prediction

logger = logging.getLogger(__name__)

NAMESPACE = "predictions"


class PredictionMarket:
    """Manage agent predictions against the World State."""

    def __init__(self, storage: StorageBackend) -> None:
        self._storage = storage
        self._pending_bus_events: List[BusEvent] = []

    def submit_prediction(
        self,
        agent_id: str,
        target_path: str,
        predicted_value: float,
        confidence: float,
        stake: float,
        horizon_epochs: int,
        current_epoch: int,
    ) -> Prediction:
        """Submit a prediction."""
        pred = Prediction(
            agent_id=agent_id,
            target_state_path=target_path,
            predicted_value=predicted_value,
            confidence=confidence,
            horizon_epoch=current_epoch + horizon_epochs,
            stake=stake,
        )

        self._storage.save(
            NAMESPACE, pred.prediction_id, pred.model_dump(mode="json")
        )

        self._pending_bus_events.append(
            BusEvent(
                event_type=EventType.PREDICTION_SUBMITTED,
                agent_id=agent_id,
                layer="WORLD_SIM",
                payload={
                    "prediction_id": pred.prediction_id,
                    "target_path": target_path,
                    "horizon_epoch": pred.horizon_epoch,
                    "stake": stake,
                },
            )
        )

        logger.info(
            "Prediction: agent=%s path=%s value=%.4f stake=%.2f horizon=%d",
            agent_id,
            target_path,
            predicted_value,
            stake,
            pred.horizon_epoch,
        )
        return pred

    def get_pending_predictions(
        self,
        agent_id: Optional[str] = None,
        epoch: Optional[int] = None,
    ) -> List[Prediction]:
        """Get unresolved predictions, optionally filtered."""
        filters: Dict[str, object] = {}
        if agent_id:
            filters["agent_id"] = agent_id
        if epoch is not None:
            filters["horizon_epoch"] = epoch

        rows = self._storage.query(NAMESPACE, filters=filters if filters else None)
        return [Prediction.model_validate(r) for r in rows]

    def get_agent_accuracy(
        self, agent_id: str, last_n_predictions: int = 50
    ) -> float:
        """Rolling accuracy from resolved predictions (stored in results ns)."""
        results = self._storage.query(
            "prediction_results",
            filters={"agent_id": agent_id},
            limit=last_n_predictions,
        )
        if not results:
            return 0.5  # No data ≠ bad accuracy; neutral until proven otherwise
        accurate = sum(
            1 for r in results if r.get("error_magnitude", 1.0) < 0.5
        )
        return accurate / len(results)

    def remove_prediction(self, prediction_id: str) -> None:
        """Remove a resolved prediction from the pending pool."""
        self._storage.delete(NAMESPACE, prediction_id)

    def get_pending_bus_events(self) -> List[BusEvent]:
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events
