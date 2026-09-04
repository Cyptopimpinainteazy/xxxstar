"""Tests for Phase 4 — Enhanced Prediction Market.

Invariant refs: tests/invariants/registry.toml — BAYESIAN_SUM_BOUNDED, CONSENSUS_MONOTONIC

Tests cover:
- Bayesian profile: updates, posterior convergence, profile persistence
- Kelly criterion: stake sizing, edge cases
- Market consensus: aggregation, weighted mean, spread
- Conditional predictions: submit, evaluate, condition matching
- Scoring: Brier score, log loss
- Lifecycle integration: Bayesian updates during epoch resolution
"""

from __future__ import annotations

import math

import pytest

from swarm.storage.backend import SqliteStorage
from swarm.world_sim.market import (
    AgentBayesianProfile,
    ConditionalPrediction,
    EnhancedPredictionMarket,
    MarketConsensus,
)
from swarm.world_sim.schema import Prediction, PredictionResult


# =====================================================================
# Fixtures
# =====================================================================


@pytest.fixture
def storage():
    return SqliteStorage(":memory:")


@pytest.fixture
def market(storage):
    return EnhancedPredictionMarket(storage=storage)


# =====================================================================
# Bayesian Profile Tests
# =====================================================================


class TestBayesianProfile:
    """Test Bayesian confidence tracking."""

    def test_new_profile_defaults(self, market):
        profile = market.get_profile("agent-new")
        assert profile.prior_accuracy == 0.5
        assert profile.alpha == 1.0
        assert profile.beta_param == 1.0
        assert profile.prediction_count == 0

    def test_correct_prediction_increases_accuracy(self, market):
        p0 = market.get_profile("a1")
        assert p0.prior_accuracy == 0.5

        p1 = market.update_bayesian("a1", was_correct=True)
        assert p1.prior_accuracy > 0.5
        assert p1.alpha == 2.0
        assert p1.beta_param == 1.0
        assert p1.correct_count == 1

    def test_wrong_prediction_decreases_accuracy(self, market):
        p = market.update_bayesian("a1", was_correct=False)
        assert p.prior_accuracy < 0.5
        assert p.beta_param == 2.0

    def test_convergence_with_many_correct(self, market):
        for _ in range(20):
            market.update_bayesian("skilled", was_correct=True)
        profile = market.get_profile("skilled")
        assert profile.prior_accuracy > 0.9

    def test_convergence_with_many_wrong(self, market):
        for _ in range(20):
            market.update_bayesian("bad", was_correct=False)
        profile = market.get_profile("bad")
        assert profile.prior_accuracy < 0.1

    def test_mixed_predictions(self, market):
        # 7 correct, 3 wrong → posterior ~ 0.727
        for _ in range(7):
            market.update_bayesian("mixed", was_correct=True)
        for _ in range(3):
            market.update_bayesian("mixed", was_correct=False)

        profile = market.get_profile("mixed")
        expected = 8.0 / 12.0  # (1+7) / (1+7 + 1+3)
        assert abs(profile.prior_accuracy - expected) < 0.01

    def test_streak_tracking(self, market):
        market.update_bayesian("s1", was_correct=True)
        market.update_bayesian("s1", was_correct=True)
        market.update_bayesian("s1", was_correct=True)
        assert market.get_profile("s1").streak == 3

        market.update_bayesian("s1", was_correct=False)
        assert market.get_profile("s1").streak == -1

    def test_profile_persistence(self, storage, market):
        market.update_bayesian("persist-test", was_correct=True)
        market.update_bayesian("persist-test", was_correct=True)

        # Create new market from same storage
        market2 = EnhancedPredictionMarket(storage=storage)
        profile = market2.get_profile("persist-test")
        assert profile.alpha == 3.0
        assert profile.correct_count == 2


# =====================================================================
# Kelly Criterion / Stake Tests
# =====================================================================


class TestStakeDynamics:
    """Test optimal stake sizing."""

    def test_neutral_agent_zero_stake(self, market):
        stake = market.suggested_stake("new-agent", available_budget=100.0)
        assert stake == 0.0  # 50% accuracy → no edge → zero Kelly

    def test_skilled_agent_positive_stake(self, market):
        for _ in range(15):
            market.update_bayesian("skilled", was_correct=True)
        for _ in range(5):
            market.update_bayesian("skilled", was_correct=False)

        stake = market.suggested_stake("skilled", available_budget=100.0)
        assert stake > 0.0
        assert stake <= 50.0  # Max Kelly fraction is 0.5

    def test_bad_agent_zero_stake(self, market):
        for _ in range(10):
            market.update_bayesian("bad", was_correct=False)
        stake = market.suggested_stake("bad", available_budget=100.0)
        assert stake == 0.0

    def test_stake_scales_with_budget(self, market):
        for _ in range(15):
            market.update_bayesian("a1", was_correct=True)

        s1 = market.suggested_stake("a1", available_budget=100.0)
        s2 = market.suggested_stake("a1", available_budget=200.0)
        assert s2 == pytest.approx(s1 * 2.0, abs=0.01)


# =====================================================================
# Market Consensus Tests
# =====================================================================


class TestMarketConsensus:
    """Test aggregation of multiple agent predictions."""

    def test_single_prediction_consensus(self, market):
        preds = [
            Prediction(agent_id="a1", target_state_path="price",
                       predicted_value=100.0, confidence=0.8,
                       stake=10.0, horizon_epoch=5),
        ]
        c = market.aggregate_consensus("price", preds, epoch=5)
        assert c.participant_count == 1
        assert c.weighted_mean == pytest.approx(100.0, abs=0.01)
        assert c.spread == 0.0

    def test_multiple_predictions_weighted(self, market):
        # Make agent-a skilled (high Bayesian weight)
        for _ in range(10):
            market.update_bayesian("agent-a", was_correct=True)

        preds = [
            Prediction(agent_id="agent-a", target_state_path="price",
                       predicted_value=100.0, confidence=0.9,
                       stake=10.0, horizon_epoch=5),
            Prediction(agent_id="agent-b", target_state_path="price",
                       predicted_value=200.0, confidence=0.5,
                       stake=5.0, horizon_epoch=5),
        ]
        c = market.aggregate_consensus("price", preds, epoch=5)
        assert c.participant_count == 2
        # agent-a has much higher Bayesian weight → consensus closer to 100
        assert c.weighted_mean < 150.0
        assert c.total_stake == 15.0

    def test_consensus_spread(self, market):
        preds = [
            Prediction(agent_id="a1", target_state_path="vol",
                       predicted_value=10.0, confidence=0.5,
                       stake=1.0, horizon_epoch=1),
            Prediction(agent_id="a2", target_state_path="vol",
                       predicted_value=50.0, confidence=0.5,
                       stake=1.0, horizon_epoch=1),
        ]
        c = market.aggregate_consensus("vol", preds, epoch=1)
        assert c.spread > 0  # Non-zero spread
        assert c.median_value == 30.0  # (10+50)/2

    def test_empty_predictions(self, market):
        c = market.aggregate_consensus("none", [], epoch=0)
        assert c.participant_count == 0
        assert c.weighted_mean == 0.0

    def test_consensus_emits_bus_event(self, market):
        preds = [
            Prediction(agent_id="a1", target_state_path="foo",
                       predicted_value=42.0, confidence=0.5,
                       stake=0, horizon_epoch=1),
        ]
        market.aggregate_consensus("foo", preds, epoch=1)
        events = market.get_pending_bus_events()
        assert len(events) == 1
        assert events[0].payload["target_path"] == "foo"


# =====================================================================
# Conditional Prediction Tests
# =====================================================================


class TestConditionalPredictions:
    """Test conditional 'If X then Y' predictions."""

    def test_submit_conditional(self, market):
        cond = market.submit_conditional(
            agent_id="a1",
            condition_path="domains.MARKET.metrics.volatility",
            condition_value=0.5,
            target_path="domains.MARKET.entities.sol.properties.price",
            predicted_value=150.0,
            confidence=0.7,
            stake=5.0,
            horizon_epoch=3,
        )
        assert cond.prediction_id
        assert cond.agent_id == "a1"
        assert cond.condition_tolerance == 0.1

    def test_evaluate_condition_met(self, market):
        cond = market.submit_conditional(
            agent_id="a1",
            condition_path="vol",
            condition_value=0.5,
            target_path="price",
            predicted_value=100.0,
            horizon_epoch=1,
        )

        def mock_query(path):
            if path == "vol":
                return 0.5
            if path == "price":
                return 105.0
            return None

        results = market.evaluate_conditionals(epoch=1, state_query_fn=mock_query)
        assert len(results) == 1
        pred, met, actual = results[0]
        assert met is True
        assert actual == 105.0

    def test_evaluate_condition_not_met(self, market):
        market.submit_conditional(
            agent_id="a1",
            condition_path="vol",
            condition_value=0.5,
            condition_tolerance=0.05,
            target_path="price",
            predicted_value=100.0,
            horizon_epoch=1,
        )

        def mock_query(path):
            if path == "vol":
                return 2.0  # Way off
            return None

        results = market.evaluate_conditionals(epoch=1, state_query_fn=mock_query)
        assert len(results) == 1
        _, met, actual = results[0]
        assert met is False
        assert actual is None

    def test_conditionals_only_evaluate_at_horizon(self, market):
        market.submit_conditional(
            agent_id="a1",
            condition_path="x",
            condition_value=1.0,
            target_path="y",
            predicted_value=2.0,
            horizon_epoch=5,
        )

        results = market.evaluate_conditionals(
            epoch=3, state_query_fn=lambda p: 1.0
        )
        assert len(results) == 0  # Not yet at horizon

        results = market.evaluate_conditionals(
            epoch=5, state_query_fn=lambda p: 1.0
        )
        assert len(results) == 1  # Now expired

    def test_submit_emits_bus_event(self, market):
        market.submit_conditional(
            agent_id="a1",
            condition_path="c",
            condition_value=1.0,
            target_path="t",
            predicted_value=2.0,
            horizon_epoch=1,
        )
        events = market.get_pending_bus_events()
        assert len(events) == 1
        assert events[0].payload["type"] == "conditional"


# =====================================================================
# Scoring Tests
# =====================================================================


class TestScoring:
    """Test Brier score and log loss."""

    def test_brier_perfect(self, market):
        # Perfect predictions: all correct with confidence 1.0
        preds = [(1.0, True), (0.0, False)]
        score = market.brier_score(preds)
        assert score == pytest.approx(0.0, abs=0.001)

    def test_brier_worst(self, market):
        # Worst predictions: all wrong with confidence 1.0/0.0
        preds = [(1.0, False), (0.0, True)]
        score = market.brier_score(preds)
        assert score == pytest.approx(1.0, abs=0.001)

    def test_brier_calibrated(self, market):
        # 50/50 → Brier = 0.25
        preds = [(0.5, True), (0.5, False)]
        score = market.brier_score(preds)
        assert score == pytest.approx(0.25, abs=0.001)

    def test_brier_empty(self, market):
        assert market.brier_score([]) == 1.0

    def test_log_loss_perfect_ish(self, market):
        preds = [(0.99, True), (0.01, False)]
        score = market.log_loss(preds)
        assert score < 0.1  # Very low loss

    def test_log_loss_bad(self, market):
        preds = [(0.01, True), (0.99, False)]
        score = market.log_loss(preds)
        assert score > 3.0  # Very high loss

    def test_log_loss_empty(self, market):
        assert market.log_loss([]) == float("inf")


# =====================================================================
# Resolution Integration Tests
# =====================================================================


class TestResolutionIntegration:
    """Test resolve_with_bayesian_update."""

    def test_resolve_updates_profiles(self, market):
        results = [
            PredictionResult(
                prediction_id="p1", agent_id="a1",
                predicted_value=100.0, actual_value=101.0,
                error_magnitude=0.01, stake=10.0, reward_or_penalty=9.9,
            ),
            PredictionResult(
                prediction_id="p2", agent_id="a2",
                predicted_value=100.0, actual_value=200.0,
                error_magnitude=1.0, stake=10.0, reward_or_penalty=-10.0,
            ),
        ]
        updated = market.resolve_with_bayesian_update(results)
        assert updated["a1"].prior_accuracy > 0.5  # Correct
        assert updated["a2"].prior_accuracy < 0.5  # Wrong

    def test_rankings_after_resolution(self, market):
        for _ in range(10):
            market.update_bayesian("best", was_correct=True)
        for _ in range(5):
            market.update_bayesian("mid", was_correct=True)
        for _ in range(5):
            market.update_bayesian("mid", was_correct=False)
        for _ in range(10):
            market.update_bayesian("worst", was_correct=False)

        rankings = market.rank_agents_by_skill()
        assert rankings[0]["agent_id"] == "best"
        assert rankings[-1]["agent_id"] == "worst"
        assert len(rankings) == 3
