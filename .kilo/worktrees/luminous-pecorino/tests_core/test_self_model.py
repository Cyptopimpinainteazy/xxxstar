"""Tests for the Self-Model Ledger."""

import pytest

from swarm.core.enums import Domain, Outcome
from swarm.self_model.decay import DecayEngine
from swarm.self_model.ledger import SelfModelLedger
from swarm.self_model.projector import ProjectionEngine
from swarm.self_model.schema import (
    CapabilityMap,
    CausalEvent,
    ConstraintMap,
    SelfModel,
)
from swarm.storage.backend import SqliteStorage


@pytest.fixture
def storage(tmp_path):
    return SqliteStorage(str(tmp_path / "test.db"))


@pytest.fixture
def ledger(storage):
    return SelfModelLedger(agent_id="test-agent", storage=storage)


class TestSelfModelSchema:
    """INV-SELFMODEL-001: Self-model schema integrity."""

    def test_integrity_hash_deterministic(self):
        model = SelfModel(agent_id="A", version=1)
        h1 = model.compute_integrity_hash()
        h2 = model.compute_integrity_hash()
        assert h1 == h2
        assert len(h1) == 64  # SHA-256 hex

    def test_integrity_hash_changes_on_mutation(self):
        model = SelfModel(agent_id="A", version=1)
        h1 = model.compute_integrity_hash()
        model.version = 2
        h2 = model.compute_integrity_hash()
        assert h1 != h2


class TestDecayEngine:
    """INV-SELFMODEL-002: Memory decay is permanent."""

    def test_decay_evicts_below_threshold(self):
        engine = DecayEngine(eviction_threshold=0.5)
        events = [
            CausalEvent(
                action_taken="test",
                outcome=Outcome.SUCCESS,
                decay_score=0.1,
            ),
            CausalEvent(
                action_taken="test2",
                outcome=Outcome.SUCCESS,
                decay_score=0.9,
            ),
        ]
        surviving, eviction_events = engine.decay_pass(events, "agent-1")
        assert len(surviving) == 1
        assert surviving[0].action_taken == "test2"
        assert len(eviction_events) == 1

    def test_decay_reduces_scores(self):
        engine = DecayEngine(decay_rate_per_pass=0.1)
        events = [
            CausalEvent(
                action_taken="test",
                outcome=Outcome.SUCCESS,
                decay_score=1.0,
            ),
        ]
        surviving, _ = engine.decay_pass(events, "agent-1")
        assert surviving[0].decay_score < 1.0


class TestProjectionEngine:
    """INV-SELFMODEL-003: Projections run under 100ms."""

    def test_projection_generates_failure_modes(self):
        engine = ProjectionEngine()
        events = [
            CausalEvent(
                action_taken=f"action-{i}",
                outcome=Outcome.SUCCESS if i % 3 != 0 else Outcome.FAILURE,
                resource_cost=1.0,
                decay_score=0.9,
            )
            for i in range(100)
        ]
        caps = [
            CapabilityMap(
                capability_id="c1",
                domain=Domain.CODE,
                proficiency_score=0.7,
            )
        ]
        constraints = ConstraintMap(resource_budget_remaining=50.0)

        projection = engine.project(events, caps, constraints, horizon_seconds=3600)
        assert projection.confidence_score > 0.0
        assert len(projection.predicted_failure_modes) > 0


class TestSelfModelLedger:
    """INV-SELFMODEL-004: Ledger lifecycle."""

    def test_record_event_and_version_bump(self, ledger):
        initial_version = ledger._model.version
        event = CausalEvent(
            action_taken="test-action",
            outcome=Outcome.SUCCESS,
            resource_cost=1.0,
        )
        ledger.record_event(event)
        assert ledger._model.version == initial_version + 1
        assert len(ledger._model.past) == 1

    def test_kill_is_permanent(self, ledger):
        ledger.kill()
        assert not ledger._model.is_alive

        with pytest.raises(RuntimeError, match="dead"):
            ledger.record_event(
                CausalEvent(
                    action_taken="should-fail",
                    outcome=Outcome.SUCCESS,
                )
            )

    def test_mortality_assessment(self, ledger):
        for i in range(5):
            ledger.record_event(
                CausalEvent(
                    action_taken=f"action-{i}",
                    outcome=Outcome.SUCCESS,
                    resource_cost=10.0,
                )
            )
        assessment = ledger.get_mortality_assessment()
        assert "resource_burn_rate" in assessment
        assert "survival_probability_1000s" in assessment

    def test_persistence_across_instances(self, storage):
        ledger1 = SelfModelLedger(agent_id="persist-test", storage=storage)
        ledger1.record_event(CausalEvent(action_taken="ev1", outcome=Outcome.SUCCESS))
        version = ledger1._model.version

        ledger2 = SelfModelLedger(agent_id="persist-test", storage=storage)
        assert ledger2._model.version == version
        assert len(ledger2._model.past) == 1
