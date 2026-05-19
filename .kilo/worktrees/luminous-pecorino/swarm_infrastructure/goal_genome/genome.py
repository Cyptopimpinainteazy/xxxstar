"""Core Goal Genome engine — manages living goals for an agent.

Goals are organisms: they compete, mutate, fork, and die.
No human approval is required for mutation.  Only consequences decide.
"""

from __future__ import annotations

import logging
from typing import Dict, List, Optional

from swarm.core.enums import Domain
from swarm.event_bus.events import (
    BusEvent,
    EventType,
    goal_died_event,
    goal_mutated_event,
)
from swarm.goal_genome.cemetery import Cemetery
from swarm.goal_genome.fitness import FitnessEvaluator
from swarm.goal_genome.mutator import GoalMutator
from swarm.goal_genome.schema import (
    EnvironmentContext,
    Goal,
    GoalFitnessReport,
    MutationTrigger,
    Recommendation,
)
from swarm.storage.backend import StorageBackend

logger = logging.getLogger(__name__)

NAMESPACE = "goal_genome"


class GoalGenome:
    """Manages the full goal lifecycle for a single agent.

    Args:
        agent_id: Owning agent identifier.
        storage: Persistence backend.
    """

    def __init__(self, agent_id: str, storage: StorageBackend) -> None:
        self.agent_id = agent_id
        self._storage = storage
        self._fitness = FitnessEvaluator()
        self._mutator = GoalMutator()
        self._cemetery = Cemetery(storage)
        self._pending_bus_events: List[BusEvent] = []

        # Load existing goals
        self._goals: Dict[str, Goal] = {}
        for key in self._storage.list_keys(f"{NAMESPACE}:{agent_id}"):
            data = self._storage.load(f"{NAMESPACE}:{agent_id}", key)
            if data is not None:
                goal = Goal.model_validate(data)
                self._goals[goal.goal_id] = goal

    # ------------------------------------------------------------------
    # Goal lifecycle
    # ------------------------------------------------------------------

    def add_goal(
        self,
        mandate: str,
        domain: Domain,
        expected_reward: float = 1.0,
    ) -> Goal:
        """Create a new root goal."""
        goal = Goal(
            mandate=mandate,
            domain=domain,
            expected_reward=expected_reward,
            lineage=[],
        )
        self._goals[goal.goal_id] = goal
        self._persist_goal(goal)

        self._pending_bus_events.append(
            BusEvent(
                event_type=EventType.GOAL_CREATED,
                agent_id=self.agent_id,
                layer="GOAL_GENOME",
                payload={"goal_id": goal.goal_id, "mandate": mandate},
            )
        )

        logger.info(
            "New goal: %s — %s (domain=%s)",
            goal.goal_id[:8],
            mandate,
            domain,
        )
        return goal

    def evaluate_fitness(
        self,
        goal_id: str,
        successes: int,
        failures: int,
        resource_spent: float,
        reward_earned: float,
        evaluation_period_seconds: int = 0,
    ) -> GoalFitnessReport:
        """Evaluate fitness of a goal and generate recommendation."""
        goal = self._get_goal(goal_id)
        report = self._fitness.evaluate(
            goal=goal,
            successes=successes,
            failures=failures,
            resource_spent=resource_spent,
            reward_earned=reward_earned,
            evaluation_period_seconds=evaluation_period_seconds,
        )
        self._persist_goal(goal)
        return report

    def execute_recommendation(
        self,
        report: GoalFitnessReport,
        trigger: MutationTrigger = MutationTrigger.REPEATED_FAILURE,
        context: Optional[EnvironmentContext] = None,
    ) -> Optional[Goal]:
        """Execute a fitness recommendation.  No human approval needed.

        Returns the new goal if mutated, None otherwise.
        """
        if context is None:
            context = EnvironmentContext(
                active_goal_count=len(self.get_active_goals())
            )

        if report.recommendation == Recommendation.KILL:
            self.kill_goal(report.goal_id, reason="Fitness below threshold for 3+ evaluations")
            return None
        elif report.recommendation == Recommendation.MUTATE:
            return self.mutate_goal(report.goal_id, trigger, context)
        else:
            return None  # CONTINUE — no action

    def mutate_goal(
        self,
        goal_id: str,
        trigger: MutationTrigger,
        context: Optional[EnvironmentContext] = None,
    ) -> Goal:
        """Mutate a goal. Parent dies.  Returns first new child."""
        if context is None:
            context = EnvironmentContext()

        goal = self._get_goal(goal_id)

        # Find a second goal for potential RECOMBINATION
        active = self.get_active_goals()
        second = None
        for g in active:
            if g.goal_id != goal_id:
                second = g
                break

        new_goals = self._mutator.mutate(
            goal=goal,
            trigger=trigger,
            context=context,
            second_goal=second,
        )

        # Kill parent
        self.kill_goal(goal_id, reason=f"Mutated via {trigger}")

        # Register new goals
        for ng in new_goals:
            self._goals[ng.goal_id] = ng
            self._persist_goal(ng)

        # Emit mutation event for first child
        first_child = new_goals[0]
        self._pending_bus_events.append(
            goal_mutated_event(
                agent_id=self.agent_id,
                source_goal_id=goal_id,
                new_goal_id=first_child.goal_id,
                mutation_type=self._mutator.select_mutation_type(trigger).value
                if hasattr(self._mutator.select_mutation_type(trigger), 'value')
                else str(self._mutator.select_mutation_type(trigger)),
            )
        )

        return first_child

    def kill_goal(self, goal_id: str, reason: str) -> None:
        """Kill a goal.  Permanent.  No resurrection."""
        goal = self._get_goal(goal_id)
        if not goal.is_alive:
            logger.warning("Goal %s is already dead", goal_id[:8])
            return

        goal.is_alive = False
        goal.death_reason = reason
        self._persist_goal(goal)
        self._cemetery.bury(goal)

        self._pending_bus_events.append(
            goal_died_event(
                agent_id=self.agent_id,
                goal_id=goal_id,
                reason=reason,
            )
        )

        logger.warning(
            "GOAL DIED: %s — %s", goal_id[:8], reason
        )

    # ------------------------------------------------------------------
    # Queries
    # ------------------------------------------------------------------

    def get_active_goals(self) -> List[Goal]:
        """Return all living goals."""
        return [g for g in self._goals.values() if g.is_alive]

    def get_goal(self, goal_id: str) -> Optional[Goal]:
        """Return a goal by ID (alive or dead)."""
        return self._goals.get(goal_id)

    def get_lineage(self, goal_id: str) -> List[Goal]:
        """Return full ancestor chain for a goal."""
        goal = self._goals.get(goal_id)
        if goal is None:
            return []

        chain: List[Goal] = []
        for ancestor_id in goal.lineage:
            # Check active goals first, then cemetery
            ancestor = self._goals.get(ancestor_id)
            if ancestor is None:
                ancestor = self._cemetery.get(ancestor_id)
            if ancestor is not None:
                chain.append(ancestor)
        chain.append(goal)
        return chain

    def get_cemetery(self) -> List[Goal]:
        """Return all dead goals for forensic analysis."""
        return self._cemetery.list_all()

    # ------------------------------------------------------------------
    # Bus events
    # ------------------------------------------------------------------

    def get_pending_bus_events(self) -> List[BusEvent]:
        """Drain and return pending bus events."""
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events

    # ------------------------------------------------------------------
    # Internals
    # ------------------------------------------------------------------

    def _get_goal(self, goal_id: str) -> Goal:
        """Fetch goal or raise."""
        goal = self._goals.get(goal_id)
        if goal is None:
            raise KeyError(f"Goal {goal_id} not found")
        return goal

    def _persist_goal(self, goal: Goal) -> None:
        self._storage.save(
            f"{NAMESPACE}:{self.agent_id}",
            goal.goal_id,
            goal.model_dump(mode="json"),
        )
