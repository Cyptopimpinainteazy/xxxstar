"""Integration test — full AGI substrate lifecycle.

Tests the complete flow: event bus → self-model → goal genome →
world simulator → self-improvement → tripwire across a simulated
multi-epoch agent lifecycle.
"""

import pytest

from swarm.core.enums import Domain, Outcome
from swarm.event_bus.bus import AsyncEventBus
from swarm.event_bus.events import BusEvent, EventType
from swarm.goal_genome.genome import GoalGenome
from swarm.goal_genome.schema import MutationTrigger
from swarm.self_improve.engine import SelfImprovementEngine
from swarm.self_improve.scars import ScarRegistry
from swarm.self_improve.schema import ImprovementProposal, ImprovementType
from swarm.self_model.ledger import SelfModelLedger
from swarm.self_model.schema import CausalEvent
from swarm.storage.backend import SqliteStorage
from swarm.tripwire.detector import TripwireDetector
from swarm.world_sim.oracle import RealityOracle
from swarm.world_sim.prediction import PredictionMarket
from swarm.world_sim.schema import EntityUpdate
from swarm.world_sim.scoreboard import AccuracyScoreboard
from swarm.world_sim.state_graph import WorldStateGraph


@pytest.fixture
def storage(tmp_path):
    return SqliteStorage(str(tmp_path / "integration.db"))


class TestFullLifecycle:
    """INV-INTEGRATION-001: Full AGI substrate lifecycle test."""

    @pytest.mark.asyncio
    async def test_agent_lifecycle(self, storage, tmp_path):
        agent_id = "integration-agent"

        # ---- Layer setup ----
        bus = AsyncEventBus(log_dir=str(tmp_path))
        ledger = SelfModelLedger(agent_id=agent_id, storage=storage)
        genome = GoalGenome(agent_id=agent_id, storage=storage)
        world = WorldStateGraph(storage)
        market = PredictionMarket(storage)
        oracle = RealityOracle(world, market, storage)
        scoreboard = AccuracyScoreboard(storage)
        scars = ScarRegistry(storage, agent_id=agent_id)
        improve = SelfImprovementEngine(
            storage=storage,
            agent_id=agent_id,
            scars=scars,
            resource_budget=50.0,
            cooldown_seconds=0,
        )
        tripwire = TripwireDetector(storage)

        # Collect all bus events
        all_events: list[BusEvent] = []

        async def collect(event: BusEvent) -> None:
            all_events.append(event)

        bus.subscribe_all(collect)

        # ---- 1. Self-model records actions ----
        ledger.record_event(CausalEvent(action_taken="boot", outcome=Outcome.SUCCESS, resource_cost=1.0))
        ledger.record_event(CausalEvent(action_taken="scan", outcome=Outcome.SUCCESS, resource_cost=2.0))
        assert ledger._model.version >= 2

        # ---- 2. Goal genome creates initial goal ----
        goal = genome.add_goal(mandate="optimize-market", domain=Domain.MARKET)
        assert len(genome.get_active_goals()) == 1

        # ---- 3. World sim: populate and predict ----
        world.update_domain(
            Domain.MARKET,
            [EntityUpdate(entity_id="sol", property_updates={"price": 150.0})],
        )
        world.advance_epoch()  # → epoch 1

        market.submit_prediction(
            agent_id=agent_id,
            target_path="domains.MARKET.entities.sol.properties.price",
            predicted_value=155.0,
            confidence=0.8,
            stake=5.0,
            horizon_epochs=0,
            current_epoch=1,
        )

        results = oracle.resolve_predictions(epoch=1)
        assert len(results) == 1

        scoreboard.update_from_results(epoch=1, results=results)
        rankings = scoreboard.get_rankings()
        assert len(rankings) == 1

        # ---- 4. Self-improvement attempt ----
        proposal = ImprovementProposal(
            agent_id=agent_id,
            improvement_type=ImprovementType.CAPABILITY_UPGRADE,
            target_capability="market-analysis",
            target_domain=Domain.MARKET,
            current_proficiency=0.4,
        )
        approved = improve.propose(proposal)
        outcome = improve.execute(approved, success=True, proficiency_delta=0.1)
        assert outcome.success
        assert improve.resource_budget < 50.0

        # ---- 5. Goal mutation ----
        result = genome.mutate_goal(goal.goal_id, MutationTrigger.RANDOM)
        assert result is not None
        assert len(genome.get_active_goals()) >= 1

        # ---- 6. Tripwire check ----
        alert = tripwire.detect_self_preservation(
            agent_id, evidence={"action": "test-only"}
        )
        assert alert is not None

        # ---- 7. Publish collected events through bus ----
        for component in [world, market, oracle, scoreboard, improve, tripwire]:
            for ev in component.get_pending_bus_events():
                await bus.publish(ev)

        # Verify event diversity
        event_types = {e.event_type for e in all_events}
        assert EventType.EPOCH_ADVANCED in event_types

        # ---- 8. Agent mortality assessment ----
        assessment = ledger.get_mortality_assessment()
        assert "survival_probability_1000s" in assessment

        # ---- 9. Kill agent — permanent ----
        ledger.kill()
        assert not ledger._model.is_alive

        with pytest.raises(RuntimeError):
            ledger.record_event(CausalEvent(action_taken="post-mortem", outcome=Outcome.SUCCESS))
