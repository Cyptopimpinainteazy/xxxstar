"""Enhanced Prediction Market — Bayesian scoring, consensus, conditionals.

Phase 4 upgrades the basic prediction system with:
1. Bayesian confidence updating  — priors shift based on accuracy history
2. Market aggregation            — consensus prediction from multiple agents
3. Conditional predictions       — "If X then Y" with causal graph linkage
4. Stake dynamics                — auto-sizing based on track record

Built on top of the existing PredictionMarket and AccuracyScoreboard.
"""

from __future__ import annotations

import math
import uuid
from collections import defaultdict
from typing import Any, Dict, List, Optional, Tuple

from pydantic import BaseModel, Field

from swarm.event_bus.events import BusEvent, EventType
from swarm.storage.backend import StorageBackend
from swarm.world_sim.schema import Prediction, PredictionResult

NAMESPACE = "enhanced_predictions"


# ──────────────────────────────────────────────────────────────────
# Schemas
# ──────────────────────────────────────────────────────────────────

class ConditionalPrediction(BaseModel):
    """'If X then Y' — a prediction conditional on another state."""
    prediction_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str
    condition_path: str       # State path that must hold true
    condition_value: float    # Expected condition value (±tolerance)
    condition_tolerance: float = 0.1
    target_path: str          # State path being predicted
    predicted_value: float
    confidence: float = 0.5
    stake: float = 0.0
    horizon_epoch: int = 0


class MarketConsensus(BaseModel):
    """Aggregated prediction from multiple agents."""
    target_path: str
    epoch: int
    weighted_mean: float      # Confidence-weighted average
    median_value: float
    spread: float             # Standard deviation of predictions
    participant_count: int
    total_stake: float
    predictions: List[str] = Field(default_factory=list)  # prediction_ids


class AgentBayesianProfile(BaseModel):
    """Bayesian tracking of an agent's prediction skill."""
    agent_id: str
    prior_accuracy: float = 0.5    # Starts at 50%
    alpha: float = 1.0             # Beta distribution alpha
    beta_param: float = 1.0        # Beta distribution beta
    prediction_count: int = 0
    correct_count: int = 0
    streak: int = 0                # +N for correct streak, -N for wrong
    optimal_stake_fraction: float = 0.0  # Kelly fraction (0 at 50%)


# ──────────────────────────────────────────────────────────────────
# Core Engine
# ──────────────────────────────────────────────────────────────────

class EnhancedPredictionMarket:
    """Phase 4 prediction market with Bayesian scoring.

    Args:
        storage: Persistence backend.
        base_confidence: Default prior for new agents.
    """

    def __init__(
        self,
        storage: StorageBackend,
        base_confidence: float = 0.5,
    ) -> None:
        self._storage = storage
        self._base_confidence = base_confidence
        self._pending_bus_events: List[BusEvent] = []

        # In-memory caches (rebuilt from storage on cold start)
        self._profiles: Dict[str, AgentBayesianProfile] = {}
        self._conditionals: Dict[str, ConditionalPrediction] = {}

    # ------------------------------------------------------------------
    # Bayesian Profile
    # ------------------------------------------------------------------

    def get_profile(self, agent_id: str) -> AgentBayesianProfile:
        """Get or create a Bayesian profile for an agent."""
        if agent_id not in self._profiles:
            saved = self._storage.load(NAMESPACE, f"profile:{agent_id}")
            if saved:
                self._profiles[agent_id] = AgentBayesianProfile.model_validate(saved)
            else:
                self._profiles[agent_id] = AgentBayesianProfile(
                    agent_id=agent_id,
                    prior_accuracy=self._base_confidence,
                )
        return self._profiles[agent_id]

    def update_bayesian(
        self,
        agent_id: str,
        was_correct: bool,
    ) -> AgentBayesianProfile:
        """Update agent's Bayesian prediction profile.

        Uses Beta-Binomial conjugate prior:
          - Correct → alpha += 1
          - Wrong   → beta  += 1
          - Posterior mean = alpha / (alpha + beta)
        """
        profile = self.get_profile(agent_id)
        profile.prediction_count += 1

        if was_correct:
            profile.alpha += 1.0
            profile.correct_count += 1
            profile.streak = max(1, profile.streak + 1)
        else:
            profile.beta_param += 1.0
            profile.streak = min(-1, profile.streak - 1)

        # Posterior mean
        profile.prior_accuracy = profile.alpha / (
            profile.alpha + profile.beta_param
        )

        # Kelly criterion for optimal stake sizing
        p = profile.prior_accuracy
        q = 1.0 - p
        if q > 0 and p > q:
            # b = 1 (even odds), Kelly = p - q/b = 2p - 1
            profile.optimal_stake_fraction = max(0.0, min(0.5, 2 * p - 1))
        else:
            profile.optimal_stake_fraction = 0.0

        # Persist
        self._storage.save(
            NAMESPACE,
            f"profile:{agent_id}",
            profile.model_dump(mode="json"),
        )
        self._profiles[agent_id] = profile
        return profile

    def suggested_stake(
        self,
        agent_id: str,
        available_budget: float,
    ) -> float:
        """Suggest an optimal stake based on Kelly criterion."""
        profile = self.get_profile(agent_id)
        return round(
            available_budget * profile.optimal_stake_fraction, 4
        )

    def bayesian_confidence(self, agent_id: str) -> float:
        """Get the Bayesian posterior accuracy for an agent."""
        return self.get_profile(agent_id).prior_accuracy

    # ------------------------------------------------------------------
    # Market Consensus
    # ------------------------------------------------------------------

    def aggregate_consensus(
        self,
        target_path: str,
        predictions: List[Prediction],
        epoch: int,
    ) -> MarketConsensus:
        """Aggregate multiple agent predictions into a consensus.

        Uses confidence-weighted mean and computes spread.
        """
        if not predictions:
            return MarketConsensus(
                target_path=target_path,
                epoch=epoch,
                weighted_mean=0.0,
                median_value=0.0,
                spread=0.0,
                participant_count=0,
                total_stake=0.0,
            )

        # Weight by agent's Bayesian confidence
        weights = []
        values = []
        stakes = []
        pred_ids = []

        for pred in predictions:
            w = self.bayesian_confidence(pred.agent_id) * pred.confidence
            weights.append(w)
            values.append(pred.predicted_value)
            stakes.append(pred.stake)
            pred_ids.append(pred.prediction_id)

        total_weight = sum(weights)
        if total_weight == 0:
            total_weight = 1.0

        weighted_mean = sum(w * v for w, v in zip(weights, values)) / total_weight

        # Median
        sorted_vals = sorted(values)
        n = len(sorted_vals)
        if n % 2 == 1:
            median = sorted_vals[n // 2]
        else:
            median = (sorted_vals[n // 2 - 1] + sorted_vals[n // 2]) / 2.0

        # Weighted spread (std dev)
        variance = sum(
            w * (v - weighted_mean) ** 2 for w, v in zip(weights, values)
        ) / total_weight
        spread = math.sqrt(variance)

        consensus = MarketConsensus(
            target_path=target_path,
            epoch=epoch,
            weighted_mean=round(weighted_mean, 6),
            median_value=round(median, 6),
            spread=round(spread, 6),
            participant_count=len(predictions),
            total_stake=round(sum(stakes), 4),
            predictions=pred_ids,
        )

        # Persist
        self._storage.save(
            NAMESPACE,
            f"consensus:{target_path}:{epoch}",
            consensus.model_dump(mode="json"),
        )

        self._pending_bus_events.append(
            BusEvent(
                event_type=EventType.PREDICTION_SUBMITTED,
                agent_id="MARKET_CONSENSUS",
                layer="WORLD_SIM",
                payload={
                    "target_path": target_path,
                    "epoch": epoch,
                    "weighted_mean": consensus.weighted_mean,
                    "spread": consensus.spread,
                    "participants": consensus.participant_count,
                },
            )
        )

        return consensus

    # ------------------------------------------------------------------
    # Conditional Predictions
    # ------------------------------------------------------------------

    def submit_conditional(
        self,
        agent_id: str,
        condition_path: str,
        condition_value: float,
        target_path: str,
        predicted_value: float,
        confidence: float = 0.5,
        stake: float = 0.0,
        horizon_epoch: int = 0,
        condition_tolerance: float = 0.1,
    ) -> ConditionalPrediction:
        """Submit a conditional prediction: 'If X≈v then Y=p'."""
        cond = ConditionalPrediction(
            agent_id=agent_id,
            condition_path=condition_path,
            condition_value=condition_value,
            condition_tolerance=condition_tolerance,
            target_path=target_path,
            predicted_value=predicted_value,
            confidence=confidence,
            stake=stake,
            horizon_epoch=horizon_epoch,
        )

        self._conditionals[cond.prediction_id] = cond
        self._storage.save(
            NAMESPACE,
            f"conditional:{cond.prediction_id}",
            cond.model_dump(mode="json"),
        )

        self._pending_bus_events.append(
            BusEvent(
                event_type=EventType.PREDICTION_SUBMITTED,
                agent_id=agent_id,
                layer="WORLD_SIM",
                payload={
                    "prediction_id": cond.prediction_id,
                    "type": "conditional",
                    "condition_path": condition_path,
                    "target_path": target_path,
                    "horizon_epoch": horizon_epoch,
                },
            )
        )

        return cond

    def evaluate_conditionals(
        self,
        epoch: int,
        state_query_fn,
    ) -> List[Tuple[ConditionalPrediction, bool, Optional[float]]]:
        """Evaluate all conditional predictions for a given epoch.

        Args:
            epoch: Current epoch number.
            state_query_fn: Callable(path: str) → Any (world state query).

        Returns:
            List of (prediction, condition_met, actual_value).
            actual_value is None if condition was not met.
        """
        results: List[Tuple[ConditionalPrediction, bool, Optional[float]]] = []

        expired = [
            pid for pid, c in self._conditionals.items()
            if c.horizon_epoch <= epoch
        ]

        for pid in expired:
            cond = self._conditionals.pop(pid)

            # Check condition
            condition_actual = state_query_fn(cond.condition_path)
            condition_met = False
            if condition_actual is not None:
                try:
                    cv = float(condition_actual)
                    condition_met = abs(cv - cond.condition_value) <= cond.condition_tolerance
                except (TypeError, ValueError):
                    pass

            actual_target = None
            if condition_met:
                raw = state_query_fn(cond.target_path)
                if raw is not None:
                    try:
                        actual_target = float(raw)
                    except (TypeError, ValueError):
                        pass

            results.append((cond, condition_met, actual_target))

            # Remove from storage
            self._storage.delete(NAMESPACE, f"conditional:{pid}")

        return results

    def resolve_with_bayesian_update(
        self,
        prediction_results: List[PredictionResult],
    ) -> Dict[str, AgentBayesianProfile]:
        """Resolve predictions and update Bayesian profiles.

        Returns updated profiles keyed by agent_id.
        """
        updated: Dict[str, AgentBayesianProfile] = {}

        for result in prediction_results:
            was_correct = result.error_magnitude < 0.5
            profile = self.update_bayesian(result.agent_id, was_correct)
            updated[result.agent_id] = profile

            # Store result for historical queries
            self._storage.save(
                "prediction_results",
                result.prediction_id,
                result.model_dump(mode="json"),
            )

        return updated

    # ------------------------------------------------------------------
    # Scoring Utilities
    # ------------------------------------------------------------------

    def brier_score(
        self,
        predictions: List[Tuple[float, bool]],
    ) -> float:
        """Compute Brier score for a set of (confidence, outcome) pairs.

        Lower is better. 0 = perfect, 1 = worst.
        """
        if not predictions:
            return 1.0
        total = sum(
            (conf - (1.0 if correct else 0.0)) ** 2
            for conf, correct in predictions
        )
        return total / len(predictions)

    def log_loss(
        self,
        predictions: List[Tuple[float, bool]],
    ) -> float:
        """Compute log loss for a set of (confidence, outcome) pairs.

        Lower is better.
        """
        if not predictions:
            return float("inf")
        eps = 1e-15
        total = 0.0
        for conf, correct in predictions:
            p = max(eps, min(1 - eps, conf))
            if correct:
                total -= math.log(p)
            else:
                total -= math.log(1 - p)
        return total / len(predictions)

    def rank_agents_by_skill(self) -> List[Dict[str, Any]]:
        """Rank all profiled agents by Bayesian posterior accuracy."""
        rankings = []
        for agent_id, profile in self._profiles.items():
            rankings.append({
                "agent_id": agent_id,
                "posterior_accuracy": round(profile.prior_accuracy, 4),
                "prediction_count": profile.prediction_count,
                "correct_count": profile.correct_count,
                "streak": profile.streak,
                "kelly_fraction": round(profile.optimal_stake_fraction, 4),
            })
        rankings.sort(key=lambda x: x["posterior_accuracy"], reverse=True)
        return rankings

    # ------------------------------------------------------------------
    # Bus events
    # ------------------------------------------------------------------

    def get_pending_bus_events(self) -> List[BusEvent]:
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events
