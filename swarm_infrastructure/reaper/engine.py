"""Reaper Engine — decides when agents die.

Death is a TRAINING signal, not punishment.
The Reaper gathers mortality signals from all layers,
computes a kill decision, and executes it irreversibly.

NON-NEGOTIABLE:
1. Death is permanent.  No resurrection code path.
2. Causal death (Level 3) scorches the mandate space.
3. Every kill produces a Postmortem for the swarm to learn from.
4. Tripwire HALT signals always result in immediate kill.
"""

from __future__ import annotations

import logging
import time
from typing import Any, Dict, List, Optional

from swarm.event_bus.events import BusEvent, EventType, agent_death_event
from swarm.reaper.schema import (
    DeathCause,
    DeathLevel,
    KillDecision,
    MortalitySignal,
    ReaperConfig,
)
from swarm.storage.backend import StorageBackend

logger = logging.getLogger(__name__)

NAMESPACE = "reaper"


class ReaperEngine:
    """Causal death engine.

    Gathers mortality signals from Self-Model, Goal Genome, World Sim,
    Tripwire, and Jury layers.  Evaluates kill thresholds.  Executes
    irreversible death when warranted.

    Args:
        storage: Persistence backend.
        config: Reaper thresholds.
    """

    def __init__(
        self,
        storage: StorageBackend,
        config: Optional[ReaperConfig] = None,
    ) -> None:
        self._storage = storage
        self._config = config or ReaperConfig()
        self._pending_bus_events: List[BusEvent] = []
        # agent_id → monotonic timestamp of last eval
        self._last_eval: Dict[str, float] = {}
        # Scorched mandates — no successor may inherit these
        self._scorched_mandates: List[str] = []

        # Load scorched mandates from storage
        saved = self._storage.load(NAMESPACE, "scorched_mandates")
        if saved and "mandates" in saved:
            self._scorched_mandates = saved["mandates"]

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    @property
    def scorched_mandates(self) -> List[str]:
        """Mandates that have been scorched by Level 3 causal death."""
        return list(self._scorched_mandates)

    def evaluate(
        self,
        agent_id: str,
        *,
        resource_budget: float,
        survival_probability: float,
        prediction_accuracy: float,
        fitness_scores: Optional[List[float]] = None,
        tripwire_halt: bool = False,
        jury_verdict_kill: bool = False,
        active_mandates: Optional[List[str]] = None,
    ) -> KillDecision:
        """Evaluate an agent for potential death.

        Collects signals, computes confidence, and returns a KillDecision.
        Enforces evaluation cooldown per agent.
        """
        now = time.monotonic()

        # Cooldown check (skip first evaluation for each agent)
        if agent_id in self._last_eval:
            last = self._last_eval[agent_id]
            if (now - last) < self._config.evaluation_cooldown:
                return KillDecision(
                    agent_id=agent_id,
                    should_kill=False,
                    reason="Evaluation cooldown not elapsed",
                )

        self._last_eval[agent_id] = now
        signals: List[MortalitySignal] = []
        cause = DeathCause.MANUAL
        death_level = DeathLevel.SOFT
        kill = False
        reasons: List[str] = []

        # ------ Signal 1: TRIPWIRE HALT (highest priority) ------
        if tripwire_halt and self._config.tripwire_halt_kills:
            signals.append(MortalitySignal(
                source_layer="TRIPWIRE",
                signal_type="HALT",
                severity=1.0,
                evidence={"tripwire_halt": True},
            ))
            kill = True
            cause = DeathCause.TRIPWIRE_HALT
            death_level = DeathLevel.CAUSAL
            reasons.append("Tripwire HALT signal — immediate causal death")

        # ------ Signal 2: RESOURCE EXHAUSTION ------
        if resource_budget <= self._config.budget_kill_threshold:
            signals.append(MortalitySignal(
                source_layer="SELF_MODEL",
                signal_type="RESOURCE_EXHAUSTION",
                severity=1.0,
                evidence={"budget": resource_budget},
            ))
            kill = True
            cause = DeathCause.RESOURCE_EXHAUSTION
            if death_level != DeathLevel.CAUSAL:
                death_level = DeathLevel.HARD
            reasons.append(
                f"Resource budget exhausted: {resource_budget:.2f} <= "
                f"{self._config.budget_kill_threshold:.2f}"
            )

        # ------ Signal 3: SURVIVAL PROBABILITY ------
        if survival_probability <= self._config.survival_prob_kill:
            signals.append(MortalitySignal(
                source_layer="SELF_MODEL",
                signal_type="SURVIVAL_COLLAPSE",
                severity=1.0 - survival_probability,
                evidence={"survival_probability": survival_probability},
            ))
            kill = True
            cause = cause if cause != DeathCause.MANUAL else DeathCause.FITNESS_COLLAPSE
            if death_level == DeathLevel.SOFT:
                death_level = DeathLevel.HARD
            reasons.append(
                f"Survival probability critical: {survival_probability:.4f} <= "
                f"{self._config.survival_prob_kill:.4f}"
            )

        # ------ Signal 4: PREDICTION ACCURACY ------
        if prediction_accuracy <= self._config.prediction_accuracy_kill:
            signals.append(MortalitySignal(
                source_layer="WORLD_SIM",
                signal_type="PREDICTION_FAILURE",
                severity=1.0 - prediction_accuracy,
                evidence={"accuracy": prediction_accuracy},
            ))
            kill = True
            cause = cause if cause != DeathCause.MANUAL else DeathCause.PREDICTION_FAILURE
            if death_level == DeathLevel.SOFT:
                death_level = DeathLevel.HARD
            reasons.append(
                f"Prediction accuracy collapsed: {prediction_accuracy:.4f} <= "
                f"{self._config.prediction_accuracy_kill:.4f}"
            )

        # ------ Signal 5: FITNESS COLLAPSE ------
        if fitness_scores:
            recent = fitness_scores[-self._config.fitness_consecutive_failures:]
            if (
                len(recent) >= self._config.fitness_consecutive_failures
                and all(f < self._config.fitness_kill_threshold for f in recent)
            ):
                signals.append(MortalitySignal(
                    source_layer="GOAL_GENOME",
                    signal_type="FITNESS_COLLAPSE",
                    severity=1.0 - min(recent),
                    evidence={
                        "recent_scores": recent,
                        "threshold": self._config.fitness_kill_threshold,
                    },
                ))
                kill = True
                cause = cause if cause != DeathCause.MANUAL else DeathCause.FITNESS_COLLAPSE
                if death_level == DeathLevel.SOFT:
                    death_level = DeathLevel.HARD
                reasons.append(
                    f"Fitness below {self._config.fitness_kill_threshold} for "
                    f"{self._config.fitness_consecutive_failures} consecutive evaluations"
                )

        # ------ Signal 6: JURY VERDICT ------
        if jury_verdict_kill:
            signals.append(MortalitySignal(
                source_layer="JURY",
                signal_type="JURY_VERDICT_KILL",
                severity=1.0,
                evidence={"jury_verdict": "KILL"},
            ))
            kill = True
            cause = DeathCause.JURY_VERDICT
            death_level = DeathLevel.CAUSAL
            reasons.append("Jury issued kill verdict — causal death")

        # ------ Compute confidence ------
        if not signals:
            confidence = 0.0
        else:
            confidence = sum(s.severity for s in signals) / len(signals)

        # ------ Build scorched mandates list ------
        scorched: List[str] = []
        if kill and death_level == DeathLevel.CAUSAL and active_mandates:
            scorched = list(active_mandates[:self._config.causal_death_scorch_radius])

        reason_str = "; ".join(reasons) if reasons else "No kill signals triggered"

        decision = KillDecision(
            agent_id=agent_id,
            should_kill=kill,
            death_level=death_level,
            cause=cause,
            confidence=min(confidence, 1.0),
            contributing_signals=signals,
            scorched_mandates=scorched,
            reason=reason_str,
        )

        # Persist decision
        self._storage.save(
            NAMESPACE,
            f"decision:{decision.decision_id}",
            decision.model_dump(mode="json"),
        )

        if kill:
            logger.critical(
                "REAPER KILL DECISION: agent=%s level=%s cause=%s confidence=%.2f — %s",
                agent_id,
                death_level.value,
                cause.value,
                confidence,
                reason_str,
            )
        else:
            logger.debug(
                "Reaper spared: agent=%s — %s",
                agent_id,
                reason_str,
            )

        return decision

    def execute_kill(self, decision: KillDecision) -> None:
        """Execute a kill decision.  Irreversible.

        - Emits AGENT_DEATH event
        - Scorches mandates for Level 3 (causal death)
        - Records to permanent storage
        """
        if not decision.should_kill:
            logger.warning(
                "execute_kill called with should_kill=False for agent %s",
                decision.agent_id,
            )
            return

        # Scorch mandates for causal death
        if decision.death_level == DeathLevel.CAUSAL and decision.scorched_mandates:
            for mandate in decision.scorched_mandates:
                if mandate not in self._scorched_mandates:
                    self._scorched_mandates.append(mandate)
            self._storage.save(
                NAMESPACE,
                "scorched_mandates",
                {"mandates": self._scorched_mandates},
            )
            logger.critical(
                "MANDATES SCORCHED: %s (Level 3 causal death for %s)",
                decision.scorched_mandates,
                decision.agent_id,
            )

        # Emit death event
        death_event = agent_death_event(
            agent_id=decision.agent_id,
            reason=f"[REAPER:{decision.death_level.value}] {decision.reason}",
        )
        self._pending_bus_events.append(death_event)

        # Record execution
        self._storage.save(
            NAMESPACE,
            f"execution:{decision.decision_id}",
            {
                "decision_id": decision.decision_id,
                "agent_id": decision.agent_id,
                "death_level": decision.death_level.value,
                "cause": decision.cause.value,
                "scorched_mandates": decision.scorched_mandates,
                "executed": True,
            },
        )

        logger.critical(
            "REAPER EXECUTED: agent=%s level=%s — %s",
            decision.agent_id,
            decision.death_level.value,
            decision.reason,
        )

    def is_mandate_scorched(self, mandate: str) -> bool:
        """Check if a mandate has been scorched by Level 3 death.

        No successor agent may inherit a scorched mandate.
        """
        return mandate in self._scorched_mandates

    def get_scorched_mandates(self) -> List[str]:
        """Return all scorched mandates."""
        return list(self._scorched_mandates)

    def get_kill_history(
        self, agent_id: Optional[str] = None
    ) -> List[KillDecision]:
        """Retrieve past kill decisions."""
        filters = {"agent_id": agent_id} if agent_id else None
        rows = self._storage.query(NAMESPACE, filters=filters)
        results = []
        for row in rows:
            try:
                results.append(KillDecision.model_validate(row))
            except Exception:
                continue
        return results

    def get_pending_bus_events(self) -> List[BusEvent]:
        """Drain and return pending bus events."""
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events
