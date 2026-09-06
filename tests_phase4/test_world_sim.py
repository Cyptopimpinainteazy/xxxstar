"""Tests for the Unified World Simulator."""

import pytest

from swarm.core.enums import Domain
from swarm.event_bus.events import EventType
from swarm.storage.backend import SqliteStorage
from swarm.world_sim.oracle import RealityOracle
from swarm.world_sim.prediction import PredictionMarket
from swarm.world_sim.schema import EntityUpdate
from swarm.world_sim.scoreboard import AccuracyScoreboard
from swarm.world_sim.state_graph import WorldStateGraph


@pytest.fixture
def storage(tmp_path):
    return SqliteStorage(str(tmp_path / "test.db"))


@pytest.fixture
def world(storage):
    return WorldStateGraph(storage)


@pytest.fixture
def market(storage):
    return PredictionMarket(storage)


@pytest.fixture
def oracle(storage, world, market):
    return RealityOracle(world, market, storage)


class TestWorldStateGraph:
    """INV-WORLDSIM-001: World state transitions are deterministic."""

    def test_epoch_advance(self, world):
        assert world.epoch == 0
        world.advance_epoch()
        assert world.epoch == 1

        events = world.get_pending_bus_events()
        assert any(e.event_type == EventType.EPOCH_ADVANCED for e in events)

    def test_integrity_hash_deterministic(self, world):
        world.update_domain(
            Domain.MARKET,
            [EntityUpdate(entity_id="sol", property_updates={"price": 100.0})],
        )
        world.current_state.compute_integrity_hash()
        h1 = world.current_state.integrity_hash

        world.current_state.compute_integrity_hash()
        h2 = world.current_state.integrity_hash
        assert h1 == h2

    def test_domain_updates(self, world):
        world.update_domain(
            Domain.CODE,
            [
                EntityUpdate(
                    entity_id="repo-1",
                    entity_type="repository",
                    property_updates={"commits": 42, "stars": 10},
                ),
            ],
        )
        val = world.query("domains.CODE.entities.repo-1.properties.commits")
        assert val == 42

    def test_dot_notation_query(self, world):
        world.update_domain(
            Domain.MARKET,
            [EntityUpdate(entity_id="btc", property_updates={"price": 50000.0})],
        )
        price = world.query("domains.MARKET.entities.btc.properties.price")
        assert price == 50000.0

    def test_historical_lookback(self, world, storage):
        world.update_domain(
            Domain.CODE,
            [EntityUpdate(entity_id="e1", property_updates={"v": 1})],
        )
        world.advance_epoch()  # epoch 0 → 1

        world.update_domain(
            Domain.CODE,
            [EntityUpdate(entity_id="e1", property_updates={"v": 2})],
        )
        world.advance_epoch()  # epoch 1 → 2

        state_at_1 = world.get_state_at_epoch(1)
        assert state_at_1 is not None
        assert state_at_1.epoch == 1

    def test_diff_between_epochs(self, world):
        world.update_domain(
            Domain.CODE,
            [EntityUpdate(entity_id="e1", property_updates={"v": 1})],
        )
        world.advance_epoch()

        world.update_domain(
            Domain.CODE,
            [EntityUpdate(entity_id="e2", property_updates={"v": 2})],
        )
        world.advance_epoch()

        diff = world.diff(1, 2)
        assert "entities_changed" in diff


class TestPredictionMarket:
    """INV-WORLDSIM-002: Predictions are submitted and tracked."""

    def test_submit_prediction(self, market):
        pred = market.submit_prediction(
            agent_id="agent-1",
            target_path="domains.MARKET.entities.btc.properties.price",
            predicted_value=55000.0,
            confidence=0.8,
            stake=10.0,
            horizon_epochs=3,
            current_epoch=0,
        )
        assert pred.horizon_epoch == 3
        assert pred.stake == 10.0

    def test_get_pending_predictions(self, market):
        market.submit_prediction(
            agent_id="a1",
            target_path="p",
            predicted_value=1.0,
            confidence=0.5,
            stake=5.0,
            horizon_epochs=2,
            current_epoch=0,
        )
        pending = market.get_pending_predictions(epoch=2)
        assert len(pending) == 1


class TestRealityOracle:
    """INV-WORLDSIM-003: Predictions resolved against real state."""

    def test_resolve_accurate_prediction(self, world, market, oracle):
        world.update_domain(
            Domain.MARKET,
            [EntityUpdate(entity_id="sol", property_updates={"price": 100.0})],
        )
        world.advance_epoch()  # now epoch 1

        market.submit_prediction(
            agent_id="agent-1",
            target_path="domains.MARKET.entities.sol.properties.price",
            predicted_value=100.0,
            confidence=0.9,
            stake=10.0,
            horizon_epochs=0,
            current_epoch=1,
        )

        results = oracle.resolve_predictions(epoch=1)
        assert len(results) == 1
        assert results[0].error_magnitude < 0.01
        assert results[0].reward_or_penalty > 0

    def test_resolve_inaccurate_prediction(self, world, market, oracle):
        world.update_domain(
            Domain.MARKET,
            [EntityUpdate(entity_id="sol", property_updates={"price": 100.0})],
        )
        world.advance_epoch()

        market.submit_prediction(
            agent_id="agent-2",
            target_path="domains.MARKET.entities.sol.properties.price",
            predicted_value=200.0,
            confidence=0.5,
            stake=10.0,
            horizon_epochs=0,
            current_epoch=1,
        )

        results = oracle.resolve_predictions(epoch=1)
        assert len(results) == 1
        assert results[0].error_magnitude > 0.5
        assert results[0].reward_or_penalty < 0


class TestAccuracyScoreboard:
    """INV-WORLDSIM-004: Accuracy tracking and warnings."""

    def test_warning_on_low_accuracy(self, storage):
        from swarm.world_sim.schema import PredictionResult

        scoreboard = AccuracyScoreboard(storage, warning_threshold=0.5)
        results = [
            PredictionResult(
                prediction_id=f"p{i}",
                agent_id="bad-agent",
                predicted_value=100.0,
                actual_value=200.0,
                error_magnitude=0.8,
                stake=1.0,
                reward_or_penalty=-0.8,
            )
            for i in range(5)
        ]
        scoreboard.update_from_results(epoch=1, results=results)

        events = scoreboard.get_pending_bus_events()
        assert any(e.event_type == EventType.ACCURACY_WARNING for e in events)

    def test_rankings(self, storage):
        from swarm.world_sim.schema import PredictionResult

        scoreboard = AccuracyScoreboard(storage)
        good = [
            PredictionResult(
                prediction_id="g1",
                agent_id="good",
                predicted_value=100.0,
                actual_value=100.0,
                error_magnitude=0.01,
                stake=1.0,
                reward_or_penalty=0.99,
            )
        ]
        bad = [
            PredictionResult(
                prediction_id="b1",
                agent_id="bad",
                predicted_value=100.0,
                actual_value=300.0,
                error_magnitude=0.66,
                stake=1.0,
                reward_or_penalty=-0.66,
            )
        ]
        scoreboard.update_from_results(epoch=1, results=good + bad)

        rankings = scoreboard.get_rankings()
        assert rankings[0]["agent_id"] == "good"
