"""Tests for the core Agent class.

Invariant refs: tests/invariants/registry.toml — DEATH_PERMANENT, SCARS_NEVER_HEAL
"""

from __future__ import annotations

import pytest

from swarm.core.agent import Agent, AgentConfig, ActionResult, Consequence
from swarm.core.enums import Domain, Outcome
from swarm.reaper.engine import ReaperEngine
from swarm.reaper.schema import ReaperConfig
from swarm.storage.backend import SqliteStorage
from swarm.world_sim.prediction import PredictionMarket
from swarm.world_sim.state_graph import WorldStateGraph


@pytest.fixture
def storage():
    return SqliteStorage(":memory:")


@pytest.fixture
def agent(storage):
    config = AgentConfig(
        agent_id="test-agent-001",
        initial_budget=1000.0,
        domain=Domain.CODE,
    )
    return Agent(config=config, storage=storage)


@pytest.fixture
def agent_with_goal(agent):
    agent.add_goal(
        mandate="Optimize build pipeline",
        domain=Domain.CODE,
        expected_reward=2.0,
    )
    return agent


# =====================================================================
# Agent lifecycle tests
# =====================================================================


class TestAgentLifecycle:
    """Test agent birth, action, and death."""

    def test_agent_born_alive(self, agent):
        """New agent starts alive with full budget."""
        assert agent.is_alive
        assert agent.agent_id == "test-agent-001"
        assert agent.resource_budget == 1000.0

    def test_agent_cannot_act_without_goals(self, agent):
        """Agent with no goals cannot act."""
        result = agent.act(epoch=0)
        assert result is None

    def test_agent_acts_with_goal(self, agent_with_goal):
        """Agent with a goal can act and produces an ActionResult."""
        result = agent_with_goal.act(epoch=0)
        assert result is not None
        assert result.action_type.startswith("pursue:")
        assert result.resource_cost > 0

    def test_agent_death_is_permanent(self, agent):
        """Once killed, agent cannot act or add goals."""
        agent.die("Test kill")
        assert not agent.is_alive
        assert agent.act(epoch=0) is None

        with pytest.raises(RuntimeError, match="dead"):
            agent.add_goal("Should fail", Domain.CODE)

    def test_agent_receives_reward(self, agent):
        """Reward consequence increases total reward."""
        agent.receive_consequence(
            Consequence("REWARD", 10.0, "TEST")
        )
        # Reward is accumulated in _total_reward
        assert agent._total_reward == 10.0

    def test_agent_receives_energy_drain(self, agent):
        """Energy drain reduces resource budget."""
        initial = agent.resource_budget
        agent.receive_consequence(
            Consequence("ENERGY_DRAIN", 50.0, "TEST")
        )
        assert agent.resource_budget == initial - 50.0

    def test_agent_receives_scar(self, agent):
        """Scar consequence creates a permanent scar."""
        agent.receive_consequence(
            Consequence(
                "SCAR", 5.0, "TEST",
                details={"domain": "CODE", "capability": "testing"},
            )
        )
        scars = agent.scars.get_all()
        assert len(scars) == 1
        assert scars[0].target_domain == "CODE"

    def test_agent_receives_trust_decay(self, agent):
        """Trust decay reduces capability proficiency."""
        from swarm.self_model.schema import CapabilityMap
        agent.self_model.update_capabilities([
            CapabilityMap(
                capability_id="cap1",
                domain=Domain.CODE,
                proficiency_score=0.8,
            )
        ])
        agent.receive_consequence(
            Consequence("TRUST_DECAY", 0.3, "TEST")
        )
        caps = agent.self_model.model.present_capabilities
        assert caps[0].proficiency_score == pytest.approx(0.5, abs=0.01)

    def test_agent_receives_space_narrowing(self, agent):
        """Space narrowing adds forbidden actions."""
        initial_forbidden = len(
            agent.self_model.model.present_constraints.forbidden_actions
        )
        agent.receive_consequence(
            Consequence(
                "SPACE_NARROWING", 1.0, "TEST",
                details={"forbidden_action": "deploy_without_review"},
            )
        )
        assert (
            len(agent.self_model.model.present_constraints.forbidden_actions)
            == initial_forbidden + 1
        )

    def test_dead_agent_ignores_consequences(self, agent):
        """Dead agents ignore all consequences."""
        agent.die("Dead")
        budget = agent.resource_budget
        agent.receive_consequence(
            Consequence("ENERGY_DRAIN", 100.0, "TEST")
        )
        # Budget should not change — consequence ignored
        assert agent.resource_budget == budget


# =====================================================================
# Epoch evaluation tests
# =====================================================================


class TestAgentEpochEvaluation:

    def test_epoch_evaluation_returns_summary(self, agent_with_goal):
        """Epoch evaluation returns a dict with required keys."""
        agent_with_goal.act(epoch=0)
        summary = agent_with_goal.evaluate_epoch(epoch=0)

        assert summary["agent_id"] == agent_with_goal.agent_id
        assert summary["alive"] is True
        assert "resource_budget" in summary
        assert "fitness_history" in summary
        assert "total_scars" in summary

    def test_epoch_resets_accumulators(self, agent_with_goal):
        """After evaluation, epoch accumulators are reset."""
        agent_with_goal.act(epoch=0)
        agent_with_goal.receive_consequence(
            Consequence("REWARD", 5.0, "TEST")
        )
        agent_with_goal.evaluate_epoch(epoch=0)

        # Accumulators should be reset
        assert agent_with_goal._total_reward == 0.0
        assert agent_with_goal._total_cost == 0.0

    def test_fitness_history_accumulates(self, agent_with_goal):
        """Fitness history grows with each epoch evaluation."""
        for epoch in range(5):
            agent_with_goal.act(epoch=epoch)
            agent_with_goal.evaluate_epoch(epoch=epoch)

        assert len(agent_with_goal.fitness_history) == 5


# =====================================================================
# Goal management tests
# =====================================================================


class TestAgentGoals:

    def test_add_goal(self, agent):
        """Agent can add goals."""
        goal = agent.add_goal("Test mandate", Domain.CODE)
        assert goal.mandate == "Test mandate"
        assert goal.is_alive

    def test_scorched_mandate_rejected(self, storage):
        """Agent cannot adopt a scorched mandate."""
        config = ReaperConfig(evaluation_cooldown=0.0)
        reaper = ReaperEngine(storage=storage, config=config)

        # Scorch a mandate manually
        from swarm.reaper.schema import KillDecision, DeathLevel, DeathCause
        decision = KillDecision(
            agent_id="dead",
            should_kill=True,
            death_level=DeathLevel.CAUSAL,
            cause=DeathCause.TRIPWIRE_HALT,
            scorched_mandates=["toxic_mandate"],
        )
        reaper.execute_kill(decision)

        agent = Agent(
            config=AgentConfig(initial_budget=100.0),
            storage=storage,
            reaper=reaper,
        )
        with pytest.raises(ValueError, match="scorched"):
            agent.add_goal("toxic_mandate", Domain.CODE)

    def test_active_mandates_property(self, agent):
        """active_mandates reflects living goals."""
        agent.add_goal("mandate_a", Domain.CODE)
        agent.add_goal("mandate_b", Domain.MARKET)
        assert len(agent.active_mandates) == 2
        assert "mandate_a" in agent.active_mandates


# =====================================================================
# Bus event collection tests
# =====================================================================


class TestAgentBusEvents:

    def test_collect_bus_events(self, agent_with_goal):
        """Agent collects events from all subsystems."""
        # Acting should generate self-model events
        agent_with_goal.act(epoch=0)
        events = agent_with_goal.collect_bus_events()
        # Should have at least the goal creation event
        assert isinstance(events, list)

    def test_death_emits_event(self, agent):
        """Agent death should produce a death event for the bus."""
        agent.die("Test death")
        events = agent.collect_bus_events()
        death_events = [
            e for e in events if e.event_type == "AGENT_DEATH"
        ]
        assert len(death_events) >= 1
