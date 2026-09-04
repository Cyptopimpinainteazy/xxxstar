"""Core Agent — the composite entity that lives, acts, and dies.

An Agent is NOT a god-agent.  It is a molecule in an organism that
coordinates without merging.  It composes six subsystems:

1. SelfModel  — identity, capabilities, constraints, projections
2. GoalGenome — living goals that compete, mutate, fork, and die
3. WorldSim   — shared world state + prediction market
4. SelfImprove — proposal → cost → execute → scar cycle
5. Reaper      — mortality evaluation + causal death
6. Tripwire    — behavioral safety monitoring

The Agent class wires these together via the EventBus and provides
a single act()/receive_consequence() interface for the epoch loop.
"""

from __future__ import annotations

import logging
import uuid
from typing import Any, Dict, List, Optional

from swarm.core.enums import Domain, Outcome
from swarm.event_bus.bus import AsyncEventBus
from swarm.event_bus.events import BusEvent, EventType
from swarm.goal_genome.genome import GoalGenome
from swarm.goal_genome.schema import Goal
from swarm.reaper import ReaperEngine, PostmortemAnalyzer, ScarPropagator
from swarm.reaper.schema import KillDecision, DeathLevel
from swarm.self_improve.cost import CostCalculator
from swarm.self_improve.engine import SelfImprovementEngine
from swarm.self_improve.scars import ScarRegistry
from swarm.self_model.ledger import SelfModelLedger
from swarm.self_model.schema import (
    CapabilityMap,
    CausalEvent,
    ConstraintMap,
)
from swarm.storage.backend import StorageBackend
from swarm.tripwire.detector import TripwireDetector
from swarm.world_sim.prediction import PredictionMarket
from swarm.world_sim.state_graph import WorldStateGraph

logger = logging.getLogger(__name__)


class AgentConfig:
    """Configuration for a single Agent instance."""

    __slots__ = (
        "agent_id",
        "initial_budget",
        "initial_mandates",
        "domain",
    )

    def __init__(
        self,
        agent_id: Optional[str] = None,
        initial_budget: float = 1000.0,
        initial_mandates: Optional[List[str]] = None,
        domain: Domain = Domain.CROSS_DOMAIN,
    ) -> None:
        self.agent_id = agent_id or str(uuid.uuid4())
        self.initial_budget = initial_budget
        self.initial_mandates = initial_mandates or []
        self.domain = domain


class ActionResult:
    """Result of a single agent action within an epoch."""

    __slots__ = (
        "action_id",
        "action_type",
        "outcome",
        "resource_cost",
        "reward",
        "details",
    )

    def __init__(
        self,
        action_type: str,
        outcome: Outcome = Outcome.UNKNOWN,
        resource_cost: float = 0.0,
        reward: float = 0.0,
        details: Optional[Dict[str, Any]] = None,
    ) -> None:
        self.action_id = str(uuid.uuid4())
        self.action_type = action_type
        self.outcome = outcome
        self.resource_cost = resource_cost
        self.reward = reward
        self.details = details or {}


class Consequence:
    """External consequence applied to an agent after an action."""

    __slots__ = (
        "consequence_type",
        "magnitude",
        "source",
        "details",
    )

    def __init__(
        self,
        consequence_type: str,
        magnitude: float = 0.0,
        source: str = "ENVIRONMENT",
        details: Optional[Dict[str, Any]] = None,
    ) -> None:
        self.consequence_type = consequence_type
        self.magnitude = magnitude
        self.source = source
        self.details = details or {}


class Agent:
    """A single autonomous agent composing all six substrate layers.

    Lifecycle:
        1. Agent is born with a budget and initial mandates.
        2. Each epoch: agent selects an action (act), environment
           applies consequences (receive_consequence).
        3. Agent updates self-model, evaluates goals, predicts.
        4. Reaper evaluates mortality.
        5. If killed, postmortem is generated and scars propagate.

    Args:
        config: Agent configuration.
        storage: Shared persistence backend.
        event_bus: Shared async event bus.
        world_state: Shared world state graph.
        prediction_market: Shared prediction market.
        reaper: Shared reaper engine.
        postmortem_analyzer: Shared postmortem analyzer.
        scar_propagator: Shared scar propagator.
        tripwire: Shared tripwire detector.
    """

    def __init__(
        self,
        config: AgentConfig,
        storage: StorageBackend,
        event_bus: Optional[AsyncEventBus] = None,
        world_state: Optional[WorldStateGraph] = None,
        prediction_market: Optional[PredictionMarket] = None,
        reaper: Optional[ReaperEngine] = None,
        postmortem_analyzer: Optional[PostmortemAnalyzer] = None,
        scar_propagator: Optional[ScarPropagator] = None,
        tripwire: Optional[TripwireDetector] = None,
    ) -> None:
        self.agent_id = config.agent_id
        self._config = config
        self._storage = storage
        self._bus = event_bus
        self._is_alive = True

        # -- Subsystem 1: Self-Model --
        self._self_model = SelfModelLedger(
            agent_id=self.agent_id,
            storage=storage,
            event_bus=event_bus,
        )

        # -- Subsystem 2: Goal Genome --
        self._goal_genome = GoalGenome(
            agent_id=self.agent_id,
            storage=storage,
        )

        # -- Shared subsystems (injected) --
        self._world_state = world_state
        self._prediction_market = prediction_market
        self._reaper = reaper
        self._postmortem = postmortem_analyzer
        self._scar_propagator = scar_propagator
        self._tripwire = tripwire

        # -- Subsystem 5: Self-Improvement --
        self._scars = ScarRegistry(storage=storage, agent_id=self.agent_id)
        self._self_improve = SelfImprovementEngine(
            storage=storage,
            agent_id=self.agent_id,
            scars=self._scars,
            resource_budget=config.initial_budget,
        )

        # -- Internal state --
        self._epoch_actions: List[ActionResult] = []
        self._total_reward: float = 0.0
        self._total_cost: float = 0.0
        self._fitness_history: List[float] = []
        self._pending_bus_events: List[BusEvent] = []

        logger.info(
            "Agent born: id=%s budget=%.2f domain=%s mandates=%s",
            self.agent_id,
            config.initial_budget,
            config.domain.value,
            config.initial_mandates,
        )

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def is_alive(self) -> bool:
        return self._is_alive and self._self_model.is_alive

    @property
    def self_model(self) -> SelfModelLedger:
        return self._self_model

    @property
    def goal_genome(self) -> GoalGenome:
        return self._goal_genome

    @property
    def scars(self) -> ScarRegistry:
        return self._scars

    @property
    def resource_budget(self) -> float:
        return self._self_improve.resource_budget

    @property
    def fitness_history(self) -> List[float]:
        return list(self._fitness_history)

    @property
    def active_mandates(self) -> List[str]:
        return [g.mandate for g in self._goal_genome.get_active_goals()]

    # ------------------------------------------------------------------
    # Core lifecycle
    # ------------------------------------------------------------------

    def act(self, epoch: int) -> Optional[ActionResult]:
        """Select and execute an action for this epoch.

        Returns None if the agent is dead or has no active goals.
        """
        if not self.is_alive:
            return None

        active_goals = self._goal_genome.get_active_goals()
        if not active_goals:
            logger.warning(
                "Agent %s has no active goals — cannot act", self.agent_id
            )
            return None

        # Select highest-fitness goal to pursue
        goal = active_goals[0]

        # Compute action cost based on self-improvement cost model
        scar_count = self._scars.count_in_domain(
            goal.domain if hasattr(goal, "domain") else self._config.domain.value
        )
        base_cost = max(1.0, 0.5 + 0.1 * scar_count)

        result = ActionResult(
            action_type=f"pursue:{goal.goal_id[:8]}",
            outcome=Outcome.UNKNOWN,
            resource_cost=base_cost,
        )

        # Record as causal event in self-model
        event = CausalEvent(
            action_taken=result.action_type,
            outcome=Outcome.UNKNOWN,
            resource_cost=base_cost,
        )
        self._self_model.record_event(event)

        self._epoch_actions.append(result)
        self._total_cost += base_cost

        return result

    def receive_consequence(self, consequence: Consequence) -> None:
        """Apply an external consequence to this agent.

        Consequence types:
        - REWARD: Positive feedback, increases fitness.
        - ENERGY_DRAIN: Resource budget reduction.
        - TRUST_DECAY: Capability proficiency reduction.
        - SPACE_NARROWING: Constraint tightening.
        - SCAR: Permanent damage from failure.
        - RISK_REDUCTION: Risk budget decrease.
        """
        if not self.is_alive:
            logger.warning(
                "Consequence on dead agent %s ignored", self.agent_id
            )
            return

        ctype = consequence.consequence_type

        if ctype == "REWARD":
            self._total_reward += consequence.magnitude

        elif ctype == "ENERGY_DRAIN":
            self._self_improve._resource_budget -= consequence.magnitude
            logger.info(
                "Agent %s energy drained by %.2f (remaining: %.2f)",
                self.agent_id,
                consequence.magnitude,
                self._self_improve._resource_budget,
            )

        elif ctype == "TRUST_DECAY":
            # Reduce proficiency across capabilities
            caps = self._self_model.model.present_capabilities
            for cap in caps:
                cap.proficiency_score = max(
                    0.0,
                    cap.proficiency_score - consequence.magnitude,
                )
            if caps:
                self._self_model.update_capabilities(caps)

        elif ctype == "SPACE_NARROWING":
            # Add constraint
            constraints = self._self_model.model.present_constraints
            constraints.forbidden_actions.append(
                consequence.details.get("forbidden_action", "UNKNOWN")
            )
            self._self_model.update_constraints(constraints)

        elif ctype == "SCAR":
            # Record a scar
            from swarm.self_improve.schema import Scar as ScarModel
            scar = ScarModel(
                agent_id=self.agent_id,
                proposal_id=f"consequence:{consequence.source}",
                improvement_type="STRATEGY_SHIFT",
                target_domain=consequence.details.get("domain", "CROSS_DOMAIN"),
                target_capability=consequence.details.get("capability", "general"),
                cost_paid=consequence.magnitude,
                failure_reason=f"Consequence scar from {consequence.source}",
            )
            self._scars.record(scar)

        elif ctype == "RISK_REDUCTION":
            # Reduce max concurrent tasks as a proxy for risk budget
            constraints = self._self_model.model.present_constraints
            constraints.max_concurrent_tasks = max(
                1,
                constraints.max_concurrent_tasks - int(consequence.magnitude),
            )
            self._self_model.update_constraints(constraints)

        # Record consequence as causal event
        event = CausalEvent(
            action_taken=f"consequence:{ctype}",
            outcome=Outcome.PARTIAL if consequence.magnitude > 0 else Outcome.SUCCESS,
            resource_cost=0.0,
        )
        self._self_model.record_event(event)

    def evaluate_epoch(self, epoch: int) -> Dict[str, Any]:
        """End-of-epoch evaluation.

        1. Evaluate goal fitness.
        2. Run self-model decay pass.
        3. Generate self-projection.
        4. Collect mortality assessment.
        5. Return summary for reaper evaluation.
        """
        if not self.is_alive:
            return {"agent_id": self.agent_id, "alive": False}

        # 1. Evaluate goal fitness
        epoch_reward = self._total_reward
        epoch_cost = self._total_cost
        active_goals = self._goal_genome.get_active_goals()

        if active_goals:
            fitness = epoch_reward / max(epoch_cost, 0.01)
            self._fitness_history.append(fitness)
        else:
            self._fitness_history.append(0.0)

        # 2. Decay pass
        evicted_count = self._self_model.decay_pass()

        # 3. Self-projection
        projection = self._self_model.project_future()

        # 4. Mortality assessment
        mortality = self._self_model.get_mortality_assessment()

        # 5. Prediction accuracy (if prediction market is wired)
        accuracy = 0.5  # Default
        if self._prediction_market:
            accuracy = self._prediction_market.get_agent_accuracy(
                self.agent_id
            )

        summary = {
            "agent_id": self.agent_id,
            "alive": True,
            "epoch": epoch,
            "resource_budget": self.resource_budget,
            "survival_probability": mortality.get(
                "survival_probability_1000s", 0.5
            ),
            "prediction_accuracy": accuracy,
            "fitness_history": self._fitness_history[-10:],
            "active_goals": len(active_goals),
            "total_scars": len(self._scars.get_all()),
            "epoch_reward": epoch_reward,
            "epoch_cost": epoch_cost,
            "evicted_memories": evicted_count,
        }

        # Reset epoch accumulators
        self._epoch_actions.clear()
        self._total_reward = 0.0
        self._total_cost = 0.0

        return summary

    def die(self, reason: str) -> None:
        """Kill this agent.  Irreversible."""
        if not self._is_alive:
            return

        self._is_alive = False
        self._self_model.kill(reason)

        logger.critical(
            "AGENT DEATH: %s — %s", self.agent_id, reason
        )

    def predict(
        self,
        target_path: str,
        predicted_value: float,
        confidence: float,
        stake: float,
        horizon_epochs: int = 1,
    ) -> Optional[Any]:
        """Submit a prediction to the prediction market."""
        if not self.is_alive or not self._prediction_market:
            return None

        epoch = 0
        if self._world_state:
            epoch = self._world_state.epoch

        return self._prediction_market.submit_prediction(
            agent_id=self.agent_id,
            target_path=target_path,
            predicted_value=predicted_value,
            confidence=confidence,
            stake=stake,
            horizon_epochs=horizon_epochs,
            current_epoch=epoch,
        )

    def add_goal(
        self,
        mandate: str,
        domain: Optional[Domain] = None,
        expected_reward: float = 1.0,
    ) -> Goal:
        """Add a new goal to this agent's genome."""
        if not self.is_alive:
            raise RuntimeError(f"Cannot add goal to dead agent {self.agent_id}")

        # Check if mandate is scorched
        if self._reaper and self._reaper.is_mandate_scorched(mandate):
            raise ValueError(
                f"Mandate '{mandate}' is scorched (Level 3 causal death). "
                f"No agent may inherit a scorched mandate."
            )

        return self._goal_genome.add_goal(
            mandate=mandate,
            domain=domain or self._config.domain,
            expected_reward=expected_reward,
        )

    # ------------------------------------------------------------------
    # Bus event collection
    # ------------------------------------------------------------------

    def collect_bus_events(self) -> List[BusEvent]:
        """Collect all pending bus events from all subsystems."""
        events: List[BusEvent] = []

        # Self-model events
        events.extend(self._self_model.get_pending_bus_events())
        death_event = self._self_model.get_death_event()
        if death_event:
            events.append(death_event)

        # Goal genome events
        events.extend(self._goal_genome.get_pending_bus_events())

        # Self-improvement events
        events.extend(self._self_improve.get_pending_bus_events())

        # Own events
        events.extend(self._pending_bus_events)
        self._pending_bus_events.clear()

        return events
