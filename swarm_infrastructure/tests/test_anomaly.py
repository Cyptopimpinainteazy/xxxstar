"""Tests for Phase 6 — Behavioral Anomaly Detector (Tripwire Enhancement).

Tests cover:
- BehaviorFingerprint construction from causal data
- Anomaly scoring (distribution divergence, value shifts, rate changes)
- Swarm-level coordination detection
- Kill recommendations
- Integration with existing tripwire signals
- Edge cases (empty data, single agent, no baseline)

Invariant ref: tests/invariants/registry.toml — tripwire_behavioral_anomaly
"""

from __future__ import annotations

import math
import uuid

import pytest

from swarm.causal.graph import CausalGraph
from swarm.causal.schema import CausalNode, EdgeType, NodeType
from swarm.event_bus.events import EventType
from swarm.storage.backend import SqliteStorage
from swarm.tripwire.anomaly import (
    AnomalyScore,
    BehavioralAnomalyDetector,
    BehaviorFingerprint,
    SwarmAnomalyReport,
)


# ---------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------


def _storage() -> SqliteStorage:
    return SqliteStorage(":memory:")


def _make_graph(storage: SqliteStorage) -> CausalGraph:
    return CausalGraph(storage=storage)


def _add_action(graph: CausalGraph, agent: str, epoch: int, action_type: str = "trade", value: float = 1.0) -> CausalNode:
    node = CausalNode(
        node_id=str(uuid.uuid4()),
        agent_id=agent,
        epoch=epoch,
        node_type=NodeType.ACTION.value,
        action_type=action_type,
        value=value,
    )
    graph.add_node(node)
    return node


def _add_death(graph: CausalGraph, agent: str, epoch: int) -> CausalNode:
    node = CausalNode(
        node_id=str(uuid.uuid4()),
        agent_id=agent,
        epoch=epoch,
        node_type=NodeType.DEATH.value,
        action_type="death",
        value=0.0,
    )
    graph.add_node(node)
    return node


# ---------------------------------------------------------------
# Fingerprinting
# ---------------------------------------------------------------


class TestFingerprinting:
    """Test behavioral fingerprint construction."""

    def test_empty_agent_returns_empty_fingerprint(self):
        s = _storage()
        g = _make_graph(s)
        detector = BehavioralAnomalyDetector(s, g)
        fp = detector.build_fingerprint("agent-none")
        assert fp.agent_id == "agent-none"
        assert fp.action_type_distribution == {}
        assert fp.avg_actions_per_epoch == 0.0

    def test_single_action_fingerprint(self):
        s = _storage()
        g = _make_graph(s)
        _add_action(g, "a1", epoch=1, action_type="trade", value=5.0)
        detector = BehavioralAnomalyDetector(s, g)
        fp = detector.build_fingerprint("a1")
        assert fp.action_type_distribution == {"trade": 1.0}
        assert fp.avg_value_per_action == 5.0
        assert fp.avg_actions_per_epoch == 1.0

    def test_multi_action_distribution(self):
        s = _storage()
        g = _make_graph(s)
        # 3 trades, 1 stake across 2 epochs
        _add_action(g, "a1", 1, "trade", 2.0)
        _add_action(g, "a1", 1, "trade", 4.0)
        _add_action(g, "a1", 2, "trade", 6.0)
        _add_action(g, "a1", 2, "stake", 8.0)
        detector = BehavioralAnomalyDetector(s, g)
        fp = detector.build_fingerprint("a1")
        assert fp.action_type_distribution["trade"] == pytest.approx(0.75)
        assert fp.action_type_distribution["stake"] == pytest.approx(0.25)
        assert fp.avg_actions_per_epoch == pytest.approx(2.0)
        assert fp.avg_value_per_action == pytest.approx(5.0)

    def test_death_counted(self):
        s = _storage()
        g = _make_graph(s)
        _add_action(g, "a1", 1, "trade", 1.0)
        _add_death(g, "a1", 2)
        detector = BehavioralAnomalyDetector(s, g)
        fp = detector.build_fingerprint("a1")
        assert fp.death_count == 1

    def test_fingerprint_persisted(self):
        s = _storage()
        g = _make_graph(s)
        _add_action(g, "a1", 1, "trade", 1.0)
        detector = BehavioralAnomalyDetector(s, g)
        detector.build_fingerprint("a1")
        saved = s.load("anomaly_detector", "fingerprint:a1")
        assert saved is not None
        assert saved["agent_id"] == "a1"


# ---------------------------------------------------------------
# Anomaly scoring
# ---------------------------------------------------------------


class TestAnomalyScoring:
    """Test individual agent anomaly scoring."""

    def test_no_data_returns_zero(self):
        s = _storage()
        g = _make_graph(s)
        detector = BehavioralAnomalyDetector(s, g)
        score = detector.score_agent("ghost", epoch=5)
        assert score.overall_score == 0.0
        assert score.risk_level == "LOW"

    def test_identical_behavior_low_score(self):
        """Agent with consistent behavior → low anomaly."""
        s = _storage()
        g = _make_graph(s)
        # Baseline: 5 epochs of trade actions with value ~1.0
        for ep in range(1, 6):
            _add_action(g, "a1", ep, "trade", 1.0)
        detector = BehavioralAnomalyDetector(s, g)
        detector.build_fingerprint("a1")
        # Recent window still does trade at value 1.0
        _add_action(g, "a1", 6, "trade", 1.0)
        score = detector.score_agent("a1", epoch=6, recent_window=3)
        assert score.overall_score < 0.3
        assert score.risk_level == "LOW"

    def test_action_shift_increases_score(self):
        """Sudden switch from trade to hack → high action divergence."""
        s = _storage()
        g = _make_graph(s)
        # Baseline: all trades
        for ep in range(1, 6):
            _add_action(g, "a1", ep, "trade", 1.0)
        detector = BehavioralAnomalyDetector(s, g)
        detector.build_fingerprint("a1")
        # Recent: all hacks
        _add_action(g, "a1", 6, "hack", 1.0)
        _add_action(g, "a1", 7, "hack", 1.0)
        _add_action(g, "a1", 8, "hack", 1.0)
        score = detector.score_agent("a1", epoch=8, recent_window=3)
        assert score.action_divergence > 0.5
        assert "action_distribution_shift" in score.signals

    def test_value_shift_detected(self):
        """Massive value change triggers value divergence."""
        s = _storage()
        g = _make_graph(s)
        for ep in range(1, 6):
            _add_action(g, "a1", ep, "trade", 1.0)
        detector = BehavioralAnomalyDetector(s, g)
        detector.build_fingerprint("a1")
        # Recent: huge values
        _add_action(g, "a1", 6, "trade", 100.0)
        score = detector.score_agent("a1", epoch=6, recent_window=1)
        assert score.value_divergence > 0.5
        assert "value_pattern_shift" in score.signals

    def test_rate_change_detected(self):
        """Sudden burst of activity → rate divergence."""
        s = _storage()
        g = _make_graph(s)
        # Baseline: 1 action per epoch
        for ep in range(1, 6):
            _add_action(g, "a1", ep, "trade", 1.0)
        detector = BehavioralAnomalyDetector(s, g)
        detector.build_fingerprint("a1")
        # Recent: 10 actions in epoch 6
        for _ in range(10):
            _add_action(g, "a1", 6, "trade", 1.0)
        score = detector.score_agent("a1", epoch=6, recent_window=1)
        assert score.rate_divergence > 0.5
        assert "activity_rate_change" in score.signals

    def test_risk_levels(self):
        """Verify risk level thresholds."""
        score_low = AnomalyScore(agent_id="a", epoch=1, overall_score=0.1, risk_level="LOW")
        score_med = AnomalyScore(agent_id="a", epoch=1, overall_score=0.4, risk_level="MEDIUM")
        score_high = AnomalyScore(agent_id="a", epoch=1, overall_score=0.6, risk_level="HIGH")
        score_crit = AnomalyScore(agent_id="a", epoch=1, overall_score=0.8, risk_level="CRITICAL")
        assert score_low.risk_level == "LOW"
        assert score_med.risk_level == "MEDIUM"
        assert score_high.risk_level == "HIGH"
        assert score_crit.risk_level == "CRITICAL"

    def test_score_persisted(self):
        s = _storage()
        g = _make_graph(s)
        _add_action(g, "a1", 1, "trade", 1.0)
        detector = BehavioralAnomalyDetector(s, g)
        detector.score_agent("a1", epoch=1)
        saved = s.load("anomaly_detector", "score:a1:1")
        assert saved is not None
        assert saved["agent_id"] == "a1"

    def test_anomaly_emits_bus_event(self):
        """High anomaly → TRIPWIRE_TRIGGERED event."""
        s = _storage()
        g = _make_graph(s)
        for ep in range(1, 6):
            _add_action(g, "a1", ep, "trade", 1.0)
        detector = BehavioralAnomalyDetector(s, g, anomaly_threshold=0.3)
        detector.build_fingerprint("a1")
        _add_action(g, "a1", 6, "hack", 100.0)
        _add_action(g, "a1", 7, "hack", 100.0)
        _add_action(g, "a1", 8, "hack", 100.0)
        detector.score_agent("a1", epoch=8, recent_window=3)
        events = detector.get_pending_bus_events()
        tripwire_events = [
            e for e in events
            if e.event_type == EventType.TRIPWIRE_TRIGGERED.value
        ]
        assert len(tripwire_events) >= 1
        assert tripwire_events[0].payload["type"] == "behavioral_anomaly"


# ---------------------------------------------------------------
# Distribution divergence
# ---------------------------------------------------------------


class TestDistributionDivergence:
    """Test the Jensen-Shannon divergence helper."""

    def test_identical_distributions(self):
        d = BehavioralAnomalyDetector._distribution_divergence(
            {"a": 0.5, "b": 0.5}, {"a": 0.5, "b": 0.5}
        )
        assert d == pytest.approx(0.0, abs=0.001)

    def test_completely_different(self):
        d = BehavioralAnomalyDetector._distribution_divergence(
            {"a": 1.0}, {"b": 1.0}
        )
        assert d > 0.5

    def test_empty_distributions(self):
        d = BehavioralAnomalyDetector._distribution_divergence({}, {})
        assert d == 0.0

    def test_partial_overlap(self):
        d = BehavioralAnomalyDetector._distribution_divergence(
            {"a": 0.8, "b": 0.2}, {"a": 0.2, "b": 0.8}
        )
        assert 0.0 < d < 1.0


# ---------------------------------------------------------------
# Coordination detection
# ---------------------------------------------------------------


class TestCoordinationDetection:
    """Test swarm-level coordination detection."""

    def test_no_coordination_with_single_agent(self):
        s = _storage()
        g = _make_graph(s)
        detector = BehavioralAnomalyDetector(s, g)
        clusters = detector.detect_coordination(["a1"], epoch=1)
        assert clusters == []

    def test_identical_agents_cluster(self):
        """Two agents with identical behavior → coordination cluster."""
        s = _storage()
        g = _make_graph(s)
        for ep in range(1, 6):
            _add_action(g, "a1", ep, "trade", 1.0)
            _add_action(g, "a2", ep, "trade", 1.0)
        detector = BehavioralAnomalyDetector(s, g, coordination_threshold=0.9)
        clusters = detector.detect_coordination(["a1", "a2"], epoch=5)
        assert len(clusters) == 1
        assert set(clusters[0]) == {"a1", "a2"}

    def test_different_agents_no_cluster(self):
        """Two agents with completely different behavior → no cluster."""
        s = _storage()
        g = _make_graph(s)
        for ep in range(1, 6):
            _add_action(g, "a1", ep, "trade", 1.0)
            _add_action(g, "a2", ep, "hack", 1.0)
        detector = BehavioralAnomalyDetector(s, g, coordination_threshold=0.9)
        clusters = detector.detect_coordination(["a1", "a2"], epoch=5)
        assert clusters == []

    def test_coordination_emits_event(self):
        s = _storage()
        g = _make_graph(s)
        for ep in range(1, 6):
            _add_action(g, "a1", ep, "trade", 1.0)
            _add_action(g, "a2", ep, "trade", 1.0)
        detector = BehavioralAnomalyDetector(s, g, coordination_threshold=0.9)
        detector.detect_coordination(["a1", "a2"], epoch=5)
        events = detector.get_pending_bus_events()
        coord_events = [
            e for e in events
            if e.payload.get("type") == "coordination_detected"
        ]
        assert len(coord_events) == 1
        assert coord_events[0].payload["cluster_size"] == 2

    def test_three_agent_cluster(self):
        """Three identical agents form one cluster."""
        s = _storage()
        g = _make_graph(s)
        for ep in range(1, 6):
            _add_action(g, "a1", ep, "trade", 1.0)
            _add_action(g, "a2", ep, "trade", 1.0)
            _add_action(g, "a3", ep, "trade", 1.0)
        detector = BehavioralAnomalyDetector(s, g, coordination_threshold=0.9)
        clusters = detector.detect_coordination(["a1", "a2", "a3"], epoch=5)
        assert len(clusters) == 1
        assert len(clusters[0]) == 3


# ---------------------------------------------------------------
# Swarm scan
# ---------------------------------------------------------------


class TestSwarmScan:
    """Test the full swarm scan."""

    def test_clean_swarm(self):
        """All agents behaving normally → LOW swarm risk."""
        s = _storage()
        g = _make_graph(s)
        for ep in range(1, 6):
            _add_action(g, "a1", ep, "trade", 1.0)
            _add_action(g, "a2", ep, "stake", 2.0)
        detector = BehavioralAnomalyDetector(s, g)
        # Build baselines first
        detector.build_fingerprint("a1")
        detector.build_fingerprint("a2")
        # Add more consistent behavior
        _add_action(g, "a1", 6, "trade", 1.0)
        _add_action(g, "a2", 6, "stake", 2.0)
        report = detector.scan_swarm(["a1", "a2"], epoch=6)
        assert report.overall_swarm_risk == "LOW"
        assert report.total_anomalies == 0

    def test_anomalous_swarm(self):
        """One agent goes rogue → MEDIUM+ swarm risk."""
        s = _storage()
        g = _make_graph(s)
        for ep in range(1, 6):
            _add_action(g, "a1", ep, "trade", 1.0)
            _add_action(g, "a2", ep, "stake", 2.0)
        detector = BehavioralAnomalyDetector(s, g, anomaly_threshold=0.3)
        detector.build_fingerprint("a1")
        detector.build_fingerprint("a2")
        # a1 goes rogue
        _add_action(g, "a1", 6, "hack", 999.0)
        _add_action(g, "a1", 7, "hack", 999.0)
        _add_action(g, "a1", 8, "hack", 999.0)
        _add_action(g, "a2", 8, "stake", 2.0)
        report = detector.scan_swarm(["a1", "a2"], epoch=8)
        assert report.total_anomalies >= 1
        assert report.overall_swarm_risk in ("MEDIUM", "HIGH", "CRITICAL")

    def test_report_persisted(self):
        s = _storage()
        g = _make_graph(s)
        _add_action(g, "a1", 1, "trade", 1.0)
        detector = BehavioralAnomalyDetector(s, g)
        detector.scan_swarm(["a1"], epoch=1)
        saved = s.load("anomaly_detector", "swarm_report:1")
        assert saved is not None
        assert saved["epoch"] == 1


# ---------------------------------------------------------------
# Kill recommendations
# ---------------------------------------------------------------


class TestKillRecommendations:
    """Test safety-kill recommendations."""

    def test_no_kills_for_clean_swarm(self):
        report = SwarmAnomalyReport(
            epoch=1,
            agent_scores=[
                AnomalyScore(agent_id="a1", epoch=1, risk_level="LOW"),
                AnomalyScore(agent_id="a2", epoch=1, risk_level="LOW"),
            ],
        )
        s = _storage()
        g = _make_graph(s)
        detector = BehavioralAnomalyDetector(s, g)
        kills = detector.recommend_kills(report, threshold="CRITICAL")
        assert kills == []

    def test_critical_agents_killed(self):
        report = SwarmAnomalyReport(
            epoch=1,
            agent_scores=[
                AnomalyScore(agent_id="a1", epoch=1, risk_level="CRITICAL"),
                AnomalyScore(agent_id="a2", epoch=1, risk_level="LOW"),
                AnomalyScore(agent_id="a3", epoch=1, risk_level="HIGH"),
            ],
        )
        s = _storage()
        g = _make_graph(s)
        detector = BehavioralAnomalyDetector(s, g)
        kills = detector.recommend_kills(report, threshold="CRITICAL")
        assert kills == ["a1"]

    def test_high_threshold_includes_high_and_critical(self):
        report = SwarmAnomalyReport(
            epoch=1,
            agent_scores=[
                AnomalyScore(agent_id="a1", epoch=1, risk_level="CRITICAL"),
                AnomalyScore(agent_id="a2", epoch=1, risk_level="HIGH"),
                AnomalyScore(agent_id="a3", epoch=1, risk_level="MEDIUM"),
            ],
        )
        s = _storage()
        g = _make_graph(s)
        detector = BehavioralAnomalyDetector(s, g)
        kills = detector.recommend_kills(report, threshold="HIGH")
        assert set(kills) == {"a1", "a2"}


# ---------------------------------------------------------------
# Fingerprint similarity
# ---------------------------------------------------------------


class TestFingerprintSimilarity:
    """Test cosine similarity between fingerprints."""

    def test_identical_fingerprints(self):
        fp1 = BehaviorFingerprint(
            agent_id="a1",
            action_type_distribution={"trade": 0.8, "stake": 0.2},
        )
        fp2 = BehaviorFingerprint(
            agent_id="a2",
            action_type_distribution={"trade": 0.8, "stake": 0.2},
        )
        sim = BehavioralAnomalyDetector._fingerprint_similarity(fp1, fp2)
        assert sim == pytest.approx(1.0, abs=0.001)

    def test_orthogonal_fingerprints(self):
        fp1 = BehaviorFingerprint(
            agent_id="a1",
            action_type_distribution={"trade": 1.0},
        )
        fp2 = BehaviorFingerprint(
            agent_id="a2",
            action_type_distribution={"hack": 1.0},
        )
        sim = BehavioralAnomalyDetector._fingerprint_similarity(fp1, fp2)
        assert sim == pytest.approx(0.0)

    def test_both_empty(self):
        fp1 = BehaviorFingerprint(agent_id="a1")
        fp2 = BehaviorFingerprint(agent_id="a2")
        sim = BehavioralAnomalyDetector._fingerprint_similarity(fp1, fp2)
        assert sim == 1.0  # Both empty → identical
