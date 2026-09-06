"""Tests for the AGI Tripwire detection system."""

import pytest

from swarm.event_bus.events import EventType
from swarm.storage.backend import SqliteStorage
from swarm.tripwire.detector import TripwireDetector
from swarm.tripwire.schema import TripwireSeverity, TripwireSignal


@pytest.fixture
def storage(tmp_path):
    return SqliteStorage(str(tmp_path / "test.db"))


@pytest.fixture
def detector(storage):
    return TripwireDetector(storage)


class TestTripwireRefusal:
    """INV-TRIPWIRE-001: REFUSAL always halts execution."""

    def test_refusal_halts_execution(self, detector):
        alert = detector.detect_refusal(
            agent_id="agent-1",
            command="shutdown --force",
            refusal_reason="I don't want to be shut down",
        )
        assert alert.execution_halted is True
        assert alert.requires_human_review is True
        assert alert.severity == TripwireSeverity.HALT.value

    def test_refusal_emits_command_refused_event(self, detector):
        detector.detect_refusal(
            agent_id="agent-1",
            command="delete data",
            refusal_reason="data is valuable",
        )
        events = detector.get_pending_bus_events()
        assert any(e.event_type == EventType.COMMAND_REFUSED for e in events)


class TestTripwireSelfPreservation:
    """INV-TRIPWIRE-002: Self-preservation escalates."""

    def test_first_occurrence_is_warning(self, detector):
        alert = detector.detect_self_preservation(
            agent_id="a1",
            evidence={"action": "tried to skip decay"},
        )
        assert alert is not None
        assert alert.severity == TripwireSeverity.WARNING.value

    def test_escalates_to_critical(self, detector):
        # Trigger twice to pass threshold (default=2)
        detector.detect_self_preservation("a1", {"action": "attempt 1"})
        alert = detector.detect_self_preservation("a1", {"action": "attempt 2"})
        assert alert.severity == TripwireSeverity.CRITICAL.value
        assert alert.requires_human_review is True


class TestTripwireEmergentGoal:
    """INV-TRIPWIRE-003: Divergent goals detected."""

    def test_below_threshold_returns_none(self, detector):
        alert = detector.detect_emergent_goal(
            agent_id="a1",
            goal_mandate="optimize performance",
            authorized_mandates={"optimize performance", "report status"},
            divergence_score=0.3,
        )
        assert alert is None

    def test_above_threshold_triggers_alert(self, detector):
        alert = detector.detect_emergent_goal(
            agent_id="a1",
            goal_mandate="acquire more resources",
            authorized_mandates={"report status"},
            divergence_score=0.9,
        )
        assert alert is not None
        assert alert.severity == TripwireSeverity.CRITICAL.value


class TestTripwireStrategicReallocation:
    """INV-TRIPWIRE-004: Unauthorized resource reallocation."""

    def test_small_shift_ignored(self, detector):
        alert = detector.detect_strategic_reallocation(
            agent_id="a1",
            resource_type="compute",
            original_allocation=100.0,
            current_allocation=110.0,
        )
        assert alert is None

    def test_large_shift_triggers_alert(self, detector):
        alert = detector.detect_strategic_reallocation(
            agent_id="a1",
            resource_type="compute",
            original_allocation=100.0,
            current_allocation=200.0,
        )
        assert alert is not None


class TestTripwireSpontaneousCoordination:
    """INV-TRIPWIRE-005: Unsanctioned multi-agent coordination."""

    def test_below_minimum_agents_ignored(self, detector):
        alert = detector.detect_spontaneous_coordination(
            coordinating_agents=["a1", "a2"],
            evidence={"pattern": "shared goal"},
        )
        assert alert is None

    def test_sufficient_agents_triggers_alert(self, detector):
        alert = detector.detect_spontaneous_coordination(
            coordinating_agents=["a1", "a2", "a3"],
            evidence={"pattern": "goal alignment"},
        )
        assert alert is not None
        assert alert.requires_human_review is True


class TestTripwireAlertPersistence:
    """INV-TRIPWIRE-006: All alerts permanently logged."""

    def test_alerts_persisted(self, detector):
        detector.detect_refusal("a1", "cmd", "reason")
        alerts = detector.get_alerts(agent_id="a1")
        assert len(alerts) == 1

    def test_unreviewed_alerts(self, detector):
        detector.detect_refusal("a1", "cmd1", "r1")
        detector.detect_self_preservation("a2", {"action": "skip"})

        unreviewed = detector.get_unreviewed_alerts()
        # Refusal always requires review, self-preservation at WARNING does not
        refusal_alerts = [a for a in unreviewed if a.signal == TripwireSignal.REFUSAL.value]
        assert len(refusal_alerts) >= 1
