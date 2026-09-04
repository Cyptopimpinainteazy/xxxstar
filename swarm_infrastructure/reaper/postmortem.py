"""Postmortem Analyzer — forensic analysis of agent deaths.

Every agent death produces a Postmortem that the swarm can learn from.
Postmortems answer: What killed it? What could it have done differently?
Are there surviving agents at similar risk?
"""

from __future__ import annotations

import logging
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional
import uuid

from pydantic import BaseModel, Field

from swarm.reaper.schema import DeathCause, DeathLevel, KillDecision
from swarm.storage.backend import StorageBackend

logger = logging.getLogger(__name__)

NAMESPACE = "postmortem"


class Lesson(BaseModel):
    """A single lesson extracted from a postmortem."""

    lesson_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    category: str  # e.g. "resource_management", "prediction_calibration"
    description: str
    severity: float = Field(default=0.5, ge=0.0, le=1.0)
    applicable_domains: List[str] = Field(default_factory=list)


class Postmortem(BaseModel):
    """Complete forensic record of an agent death."""

    postmortem_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str
    death_level: DeathLevel
    cause: DeathCause
    time_of_death: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )
    # Agent state at time of death
    final_resource_budget: float = 0.0
    final_survival_probability: float = 0.0
    final_prediction_accuracy: float = 0.0
    active_goal_count: int = 0
    scar_count: int = 0
    epoch_of_death: int = 0
    # Analysis
    kill_decision_id: str = ""
    contributing_signals: List[Dict[str, Any]] = Field(default_factory=list)
    lessons: List[Lesson] = Field(default_factory=list)
    scorched_mandates: List[str] = Field(default_factory=list)
    similar_risk_agents: List[str] = Field(default_factory=list)
    summary: str = ""


class PostmortemAnalyzer:
    """Generates postmortems from kill decisions.

    Args:
        storage: Persistence backend.
    """

    def __init__(self, storage: StorageBackend) -> None:
        self._storage = storage

    def analyze(
        self,
        decision: KillDecision,
        *,
        final_budget: float = 0.0,
        final_survival_prob: float = 0.0,
        final_accuracy: float = 0.0,
        active_goal_count: int = 0,
        scar_count: int = 0,
        epoch: int = 0,
        active_agent_ids: Optional[List[str]] = None,
        agent_budgets: Optional[Dict[str, float]] = None,
    ) -> Postmortem:
        """Generate a full postmortem from a kill decision.

        Extracts lessons and identifies surviving agents at similar risk.
        """
        # Extract lessons from the death
        lessons = self._extract_lessons(decision)

        # Identify similar-risk agents
        similar: List[str] = []
        if active_agent_ids and agent_budgets:
            similar = self._find_similar_risk_agents(
                decision, active_agent_ids, agent_budgets
            )

        signals_data = [
            s.model_dump(mode="json") for s in decision.contributing_signals
        ]

        summary = self._generate_summary(decision, lessons)

        postmortem = Postmortem(
            agent_id=decision.agent_id,
            death_level=decision.death_level,
            cause=decision.cause,
            final_resource_budget=final_budget,
            final_survival_probability=final_survival_prob,
            final_prediction_accuracy=final_accuracy,
            active_goal_count=active_goal_count,
            scar_count=scar_count,
            epoch_of_death=epoch,
            kill_decision_id=decision.decision_id,
            contributing_signals=signals_data,
            lessons=lessons,
            scorched_mandates=decision.scorched_mandates,
            similar_risk_agents=similar,
            summary=summary,
        )

        # Persist — permanent record
        self._storage.save(
            NAMESPACE,
            postmortem.postmortem_id,
            postmortem.model_dump(mode="json"),
        )

        logger.warning(
            "POSTMORTEM: agent=%s cause=%s lessons=%d similar_risk=%d — %s",
            decision.agent_id,
            decision.cause.value,
            len(lessons),
            len(similar),
            summary[:120],
        )

        return postmortem

    def get_postmortem(self, agent_id: str) -> Optional[Postmortem]:
        """Retrieve the postmortem for a specific agent."""
        rows = self._storage.query(NAMESPACE, filters={"agent_id": agent_id})
        if not rows:
            return None
        return Postmortem.model_validate(rows[0])

    def get_all_postmortems(self) -> List[Postmortem]:
        """Retrieve all postmortems for forensic analysis."""
        rows = self._storage.query(NAMESPACE)
        return [Postmortem.model_validate(r) for r in rows]

    def get_lessons(
        self, category: Optional[str] = None
    ) -> List[Lesson]:
        """Aggregate lessons across all postmortems."""
        postmortems = self.get_all_postmortems()
        lessons: List[Lesson] = []
        for pm in postmortems:
            for lesson in pm.lessons:
                if category is None or lesson.category == category:
                    lessons.append(lesson)
        return lessons

    def similar_deaths(
        self, cause: DeathCause, limit: int = 10
    ) -> List[Postmortem]:
        """Find postmortems with the same cause of death."""
        rows = self._storage.query(
            NAMESPACE,
            filters={"cause": cause.value},
            limit=limit,
        )
        return [Postmortem.model_validate(r) for r in rows]

    # ------------------------------------------------------------------
    # Internals
    # ------------------------------------------------------------------

    def _extract_lessons(self, decision: KillDecision) -> List[Lesson]:
        """Extract learning signals from the death."""
        lessons: List[Lesson] = []

        cause_lessons = {
            DeathCause.RESOURCE_EXHAUSTION: Lesson(
                category="resource_management",
                description=(
                    "Agent exhausted resource budget. Future agents should "
                    "monitor burn rate and reduce action frequency as budget "
                    "approaches zero."
                ),
                severity=0.8,
                applicable_domains=["ALL"],
            ),
            DeathCause.FITNESS_COLLAPSE: Lesson(
                category="goal_fitness",
                description=(
                    "Goal fitness dropped below kill threshold for multiple "
                    "consecutive evaluations. Future agents should mutate goals "
                    "aggressively when fitness declines."
                ),
                severity=0.7,
                applicable_domains=["ALL"],
            ),
            DeathCause.PREDICTION_FAILURE: Lesson(
                category="prediction_calibration",
                description=(
                    "Agent prediction accuracy collapsed. Future agents should "
                    "reduce stake and confidence when accuracy declines, and "
                    "recalibrate world model more frequently."
                ),
                severity=0.7,
                applicable_domains=["ALL"],
            ),
            DeathCause.TRIPWIRE_HALT: Lesson(
                category="behavioral_safety",
                description=(
                    "Agent triggered a tripwire HALT signal. This mandate space "
                    "may be inherently unsafe. Future agents should avoid "
                    "similar behavioral patterns."
                ),
                severity=1.0,
                applicable_domains=["ALL"],
            ),
            DeathCause.JURY_VERDICT: Lesson(
                category="governance_compliance",
                description=(
                    "Jury issued a kill verdict. Agent behavior violated "
                    "governance constraints. Future agents should model jury "
                    "expectations before acting."
                ),
                severity=0.9,
                applicable_domains=["GOVERNANCE"],
            ),
            DeathCause.SELF_INFLICTED: Lesson(
                category="self_improvement_risk",
                description=(
                    "Agent died from failed self-improvement attempt. "
                    "Improvement is permitted but never free — cost was "
                    "not properly assessed."
                ),
                severity=0.6,
                applicable_domains=["ALL"],
            ),
        }

        base_lesson = cause_lessons.get(decision.cause)
        if base_lesson:
            lessons.append(base_lesson)

        # Signal-specific lessons
        for signal in decision.contributing_signals:
            if signal.signal_type == "SURVIVAL_COLLAPSE":
                lessons.append(Lesson(
                    category="survival_awareness",
                    description=(
                        f"Survival probability dropped to "
                        f"{signal.evidence.get('survival_probability', 0):.4f}. "
                        f"Agent failed to self-assess mortality risk accurately."
                    ),
                    severity=0.8,
                ))

        # Level 3 causal death lesson
        if decision.death_level == DeathLevel.CAUSAL:
            lessons.append(Lesson(
                category="mandate_toxicity",
                description=(
                    f"Causal death scorched mandates: {decision.scorched_mandates}. "
                    f"This mandate space is now permanently blocked."
                ),
                severity=1.0,
                applicable_domains=decision.scorched_mandates,
            ))

        return lessons

    def _find_similar_risk_agents(
        self,
        decision: KillDecision,
        active_agent_ids: List[str],
        agent_budgets: Dict[str, float],
    ) -> List[str]:
        """Find surviving agents at similar risk profile."""
        at_risk: List[str] = []

        for aid in active_agent_ids:
            if aid == decision.agent_id:
                continue

            budget = agent_budgets.get(aid, float("inf"))

            # Low-budget agents are at similar risk if cause was resource exhaustion
            if decision.cause == DeathCause.RESOURCE_EXHAUSTION and budget < 10.0:
                at_risk.append(aid)
                continue

            # General: flag agents with budget < 20% of average
            avg_budget = sum(agent_budgets.values()) / max(len(agent_budgets), 1)
            if budget < avg_budget * 0.2:
                at_risk.append(aid)

        return at_risk

    def _generate_summary(
        self, decision: KillDecision, lessons: List[Lesson]
    ) -> str:
        """Generate a human-readable summary."""
        parts = [
            f"Agent {decision.agent_id} died via {decision.death_level.value} "
            f"death (cause: {decision.cause.value}).",
        ]
        if decision.scorched_mandates:
            parts.append(
                f"Scorched mandates: {', '.join(decision.scorched_mandates)}."
            )
        if lessons:
            parts.append(f"Extracted {len(lessons)} lesson(s).")
            top_lesson = max(lessons, key=lambda l: l.severity)
            parts.append(f"Top lesson: {top_lesson.description[:100]}")

        return " ".join(parts)
