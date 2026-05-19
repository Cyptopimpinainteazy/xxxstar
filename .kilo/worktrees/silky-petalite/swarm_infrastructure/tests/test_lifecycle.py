"""Tests for the Epoch Lifecycle Orchestrator.

Invariant refs: tests/invariants/registry.toml — EPOCH_DETERMINISTIC, DEATH_PERMANENT
"""

from __future__ import annotations

import asyncio
import pytest

from swarm.core.agent import Agent, AgentConfig, Consequence
from swarm.core.enums import Domain
from swarm.core.lifecycle import EpochOrchestrator, EpochStats
from swarm.event_bus.bus import AsyncEventBus
from swarm.reaper import PostmortemAnalyzer, ReaperEngine, ScarPropagator
from swarm.reaper.schema import ReaperConfig
from swarm.storage.backend import SqliteStorage
from swarm.tripwire.detector import TripwireDetector
from swarm.world_sim.prediction import PredictionMarket
from swarm.world_sim.scoreboard import AccuracyScoreboard
from swarm.world_sim.state_graph import WorldStateGraph


@pytest.fixture
def storage():
    return SqliteStorage(":memory:")


@pytest.fixture
def event_bus():
    return AsyncEventBus(log_dir=None)


@pytest.fixture
def world_state(storage):
    return WorldStateGraph(storage=storage)


@pytest.fixture
def prediction_market(storage):
    return PredictionMarket(storage=storage)


@pytest.fixture
def scoreboard(storage):
    return AccuracyScoreboard(storage=storage)


@pytest.fixture
def reaper(storage):
    config = ReaperConfig(evaluation_cooldown=0.0)
    return ReaperEngine(storage=storage, config=config)


@pytest.fixture
def postmortem_analyzer(storage):
    return PostmortemAnalyzer(storage=storage)


@pytest.fixture
def scar_propagator(storage):
    return ScarPropagator(storage=storage)


@pytest.fixture
def tripwire(storage):
    return TripwireDetector(storage=storage)


@pytest.fixture
def orchestrator(
    storage,
    event_bus,
    world_state,
    prediction_market,
    scoreboard,
    reaper,
    postmortem_analyzer,
    scar_propagator,
    tripwire,
):
    return EpochOrchestrator(
        storage=storage,
        event_bus=event_bus,
        world_state=world_state,
        prediction_market=prediction_market,
        scoreboard=scoreboard,
        reaper=reaper,
        postmortem_analyzer=postmortem_analyzer,
        scar_propagator=scar_propagator,
        tripwire=tripwire,
    )


def _spawn_agent_with_goal(orchestrator, agent_id, budget=1000.0):
    """Helper: spawn an agent and give it a goal."""
    config = AgentConfig(
        agent_id=agent_id,
        initial_budget=budget,
        domain=Domain.CODE,
    )
    agent = orchestrator.spawn_agent(config)
    agent.add_goal(f"goal_{agent_id}", Domain.CODE)
    return agent


# =====================================================================
# Orchestrator basic tests
# =====================================================================


class TestEpochOrchestrator:

    def test_spawn_agent(self, orchestrator):
        """Spawning an agent registers it."""
        agent = _spawn_agent_with_goal(orchestrator, "a1")
        assert agent.is_alive
        assert len(orchestrator.living_agents) == 1

    def test_spawn_scorched_mandate_fails(self, orchestrator, reaper):
        """Cannot spawn agent with scorched mandate."""
        from swarm.reaper.schema import KillDecision, DeathLevel, DeathCause
        decision = KillDecision(
            agent_id="dead",
            should_kill=True,
            death_level=DeathLevel.CAUSAL,
            cause=DeathCause.TRIPWIRE_HALT,
            scorched_mandates=["toxic"],
        )
        reaper.execute_kill(decision)

        with pytest.raises(ValueError, match="scorched"):
            config = AgentConfig(
                agent_id="new",
                initial_mandates=["toxic"],
            )
            orchestrator.spawn_agent(config)

    @pytest.mark.asyncio
    async def test_run_single_epoch(self, orchestrator):
        """Single epoch runs and returns stats."""
        _spawn_agent_with_goal(orchestrator, "a1")
        _spawn_agent_with_goal(orchestrator, "a2")

        stats = await orchestrator.run_epoch()
        assert isinstance(stats, EpochStats)
        assert stats.agents_alive >= 1
        assert stats.duration_seconds > 0

    @pytest.mark.asyncio
    async def test_epoch_advances_world(self, orchestrator, world_state):
        """Each epoch advances the world state epoch counter."""
        _spawn_agent_with_goal(orchestrator, "a1")
        initial_epoch = world_state.epoch

        await orchestrator.run_epoch()
        assert world_state.epoch == initial_epoch + 1

    @pytest.mark.asyncio
    async def test_dead_agents_excluded(self, orchestrator):
        """Dead agents don't act in subsequent epochs."""
        agent = _spawn_agent_with_goal(orchestrator, "doomed", budget=1000.0)
        agent.die("Pre-kill for test")

        stats = await orchestrator.run_epoch()
        assert stats.agents_alive == 0

    @pytest.mark.asyncio
    async def test_all_dead_stops_loop(self, orchestrator):
        """Run loop stops when all agents die."""
        agent = _spawn_agent_with_goal(orchestrator, "lonely", budget=1000.0)
        agent.die("Pre-kill")

        results = await orchestrator.run(max_epochs=10)
        # Should stop immediately — no living agents
        assert len(results) == 0

    @pytest.mark.asyncio
    async def test_multiple_epochs(self, orchestrator):
        """Multiple epochs run sequentially."""
        _spawn_agent_with_goal(orchestrator, "a1", budget=5000.0)
        _spawn_agent_with_goal(orchestrator, "a2", budget=5000.0)

        results = await orchestrator.run(max_epochs=3)
        assert len(results) == 3
        for i, stats in enumerate(results):
            assert stats.agents_alive >= 1

    @pytest.mark.asyncio
    async def test_consequence_fn_applied(self, orchestrator):
        """Custom consequence function is called during epoch."""
        consequence_called = []

        def test_consequence_fn(agent, action):
            consequence_called.append(agent.agent_id)
            return [Consequence("REWARD", 1.0, "TEST_ENV")]

        orchestrator._consequence_fn = test_consequence_fn

        _spawn_agent_with_goal(orchestrator, "a1")
        await orchestrator.run_epoch()

        assert "a1" in consequence_called

    @pytest.mark.asyncio
    async def test_epoch_stats_structure(self, orchestrator):
        """Epoch stats contain all required fields."""
        _spawn_agent_with_goal(orchestrator, "a1")

        stats = await orchestrator.run_epoch()
        d = stats.to_dict()

        required_keys = {
            "epoch", "agents_alive", "agents_killed", "agents_born",
            "total_reward", "total_cost", "predictions_resolved",
            "scars_propagated", "goals_mutated", "bus_events_published",
            "duration_seconds",
        }
        assert required_keys.issubset(d.keys())

    @pytest.mark.asyncio
    async def test_stop_halts_loop(self, orchestrator):
        """stop() halts the epoch loop."""
        _spawn_agent_with_goal(orchestrator, "a1", budget=10000.0)

        # Run in background, stop after first epoch check
        async def stop_after_delay():
            await asyncio.sleep(0.01)
            orchestrator.stop()

        task = asyncio.create_task(stop_after_delay())
        results = await orchestrator.run(max_epochs=1000)
        await task

        # Should have run only a few epochs before stop
        assert len(results) < 1000
