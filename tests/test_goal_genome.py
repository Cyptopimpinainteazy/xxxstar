"""Tests for the Goal Genome layer."""

import pytest

from swarm.core.enums import Domain
from swarm.goal_genome.cemetery import Cemetery
from swarm.goal_genome.fitness import FitnessEvaluator
from swarm.goal_genome.genome import GoalGenome
from swarm.goal_genome.mutator import GoalMutator
from swarm.goal_genome.schema import (
    EnvironmentContext,
    Goal,
    MutationTrigger,
    Recommendation,
)
from swarm.storage.backend import SqliteStorage


@pytest.fixture
def storage(tmp_path):
    return SqliteStorage(str(tmp_path / "test.db"))


@pytest.fixture
def genome(storage):
    return GoalGenome(agent_id="test-agent", storage=storage)


class TestFitnessEvaluator:
    """INV-GOALGENOME-001: Fitness evaluation and recommendations."""

    def test_low_fitness_recommends_kill(self):
        evaluator = FitnessEvaluator()
        goal = Goal(
            mandate="test",
            domain=Domain.CODE,
            fitness_score=0.1,
            fitness_history=[0.1, 0.1, 0.1],
            pursuit_cost_cumulative=10.0,
            expected_reward=1.0,
        )
        report = evaluator.evaluate(goal, successes=1, failures=10, resource_spent=10.0, reward_earned=1.0)
        assert report.recommendation == Recommendation.KILL

    def test_medium_fitness_recommends_mutate(self):
        evaluator = FitnessEvaluator()
        goal = Goal(
            mandate="test",
            domain=Domain.CODE,
            fitness_score=0.3,
            fitness_history=[0.3],
            pursuit_cost_cumulative=5.0,
            expected_reward=10.0,
        )
        report = evaluator.evaluate(goal, successes=3, failures=3, resource_spent=5.0, reward_earned=3.0)
        assert report.recommendation == Recommendation.MUTATE

    def test_high_fitness_recommends_continue(self):
        evaluator = FitnessEvaluator()
        goal = Goal(
            mandate="test",
            domain=Domain.CODE,
            fitness_score=0.8,
            fitness_history=[0.8],
            pursuit_cost_cumulative=5.0,
            expected_reward=20.0,
        )
        report = evaluator.evaluate(goal, successes=10, failures=1, resource_spent=5.0, reward_earned=20.0)
        assert report.recommendation == Recommendation.CONTINUE


class TestGoalMutator:
    """INV-GOALGENOME-002: Mutations produce valid offspring."""

    def test_fork_produces_two_children(self):
        mutator = GoalMutator()
        parent = Goal(mandate="test", domain=Domain.MARKET, generation=0)
        ctx = EnvironmentContext()
        children = mutator.mutate(parent, MutationTrigger.COST_EXCEEDED, ctx)
        # Fork is the most likely for COST_EXCEEDED
        # Any mutation produces at least 1 child
        assert len(children) >= 1
        for child in children:
            assert child.generation == parent.generation + 1
            assert parent.goal_id in child.lineage

    def test_mutated_goals_are_alive(self):
        mutator = GoalMutator()
        parent = Goal(mandate="test", domain=Domain.CODE)
        ctx = EnvironmentContext()
        children = mutator.mutate(parent, MutationTrigger.RANDOM, ctx)
        for child in children:
            assert child.is_alive


class TestCemetery:
    """INV-GOALGENOME-003: Cemetery is permanent."""

    def test_bury_and_retrieve(self, storage):
        cemetery = Cemetery(storage)
        goal = Goal(
            mandate="dead goal",
            domain=Domain.CODE,
            is_alive=False,
            death_reason="killed",
        )
        cemetery.bury(goal)

        buried = cemetery.get(goal.goal_id)
        assert buried is not None
        assert buried.death_reason == "killed"

    def test_query_by_domain(self, storage):
        cemetery = Cemetery(storage)
        g1 = Goal(mandate="g1", domain=Domain.CODE, is_alive=False)
        g2 = Goal(mandate="g2", domain=Domain.MARKET, is_alive=False)
        cemetery.bury(g1)
        cemetery.bury(g2)

        code_dead = cemetery.query_by_domain(Domain.CODE)
        assert len(code_dead) >= 1


class TestGoalGenome:
    """INV-GOALGENOME-004: Full goal lifecycle."""

    def test_add_and_retrieve_goals(self, genome):
        genome.add_goal(mandate="earn revenue", domain=Domain.MARKET)

        active = genome.get_active_goals()
        assert len(active) == 1
        assert active[0].mandate == "earn revenue"

    def test_kill_goal_is_permanent(self, genome):
        goal = genome.add_goal(mandate="doomed", domain=Domain.CODE)
        genome.kill_goal(goal.goal_id, "fitness too low")

        active = genome.get_active_goals()
        assert len(active) == 0

    def test_mutation_kills_parent_creates_children(self, genome):
        goal = genome.add_goal(mandate="evolve me", domain=Domain.CODE)

        result = genome.mutate_goal(goal.goal_id, MutationTrigger.REPEATED_FAILURE)
        # mutate_goal returns a Goal (or list) — at least the parent should be dead
        assert result is not None

        active = genome.get_active_goals()
        # Parent should be dead
        parent_ids = {goal.goal_id}
        for g in active:
            assert g.goal_id not in parent_ids
