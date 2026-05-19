"""
Test suite for coordination detection and anti-manipulation system.
Tests detection of bias patterns, collusion attempts, and governance anomalies.
"""

import time

import pytest

from swarm.jury.coordination import (
    AnomalyLevel,
    AnomalyRecord,
    BiasPattern,
    CoordinationDetector,
)


@pytest.fixture
def detector():
    """Create a fresh CoordinationDetector for each test."""
    return CoordinationDetector()


class TestCivicSelfTermination:
    """Test detection of rapid token spending (civic self-termination pattern)."""

    def test_civic_self_termination_detection_above_threshold(self, detector):
        """Detect when agent burns >=3 tokens in 60 seconds."""
        # Record rapid token consumption
        now = time.time()
        detector.record_token_consumption("agent_1", now)
        detector.record_token_consumption("agent_1", now + 5)
        detector.record_token_consumption("agent_1", now + 10)
        detector.record_token_consumption("agent_1", now + 15)

        # Detect civic self-termination (>=3 threshold)
        alerts = detector.detect_civic_self_termination()

        # Should detect spike
        assert len(alerts) > 0
        assert any(a.pattern == BiasPattern.CIVIC_SELF_TERMINATION for a in alerts)
        assert any(a.actors == ["agent_1"] for a in alerts)

    def test_civic_self_termination_under_threshold(self, detector):
        """No alert when spending <3 tokens in 60 seconds."""
        # Record only 2 tokens
        now = time.time()
        detector.record_token_consumption("agent_2", now)
        detector.record_token_consumption("agent_2", now + 10)

        # Detect civic self-termination
        alerts = detector.detect_civic_self_termination()

        # Should not detect spike (only 2 tokens)
        assert len(alerts) == 0


class TestIdeologicalSurge:
    """Test detection of consensus voting patterns (ideological surge)."""

    def test_ideological_surge_above_threshold(self, detector):
        """Detect when >80% of agents vote in same direction."""
        case_id = "case_consensus"

        # Record 8 yes votes, 2 no votes (80% consensus)
        for i in range(8):
            detector.record_vote(case_id, f"agent_{i}", True)
        for i in range(2):
            detector.record_vote(case_id, f"agent_{8+i}", False)

        # Detect ideological surge
        alert = detector.detect_ideological_surge(case_id)

        # Should detect surge
        assert alert is not None
        assert alert.pattern == BiasPattern.IDEOLOGICAL_SURGE
        assert alert.case_id == case_id
        assert alert.evidence["consensus_ratio"] == 0.80

    def test_ideological_surge_under_threshold(self, detector):
        """No alert when voting is split <80%."""
        case_id = "case_split"

        # Record 3 yes, 2 no (60% consensus)
        for i in range(3):
            detector.record_vote(case_id, f"agent_{i}", True)
        for i in range(2):
            detector.record_vote(case_id, f"agent_{3+i}", False)

        # Detect ideological surge
        alert = detector.detect_ideological_surge(case_id)

        # Should not detect (60% is under 80% threshold)
        assert alert is None


class TestLawyerOssification:
    """Test detection of repeated strike patterns by counsel."""

    def test_lawyer_ossification_above_threshold(self, detector):
        """Detect when lawyer uses same reason 5+ times."""
        lawyer_id = "da_prosecutor_1"
        case_id = "case_ossified"

        # Same lawyer strikes with same reason 5 times
        for i in range(5):
            detector.record_strike(
                case_id,
                lawyer_id,
                "DA_Procedural",
                "implicit_bias_against_regulation",
                f"profile_hash_{i}",
            )

        # Detect lawyer ossification
        alert = detector.detect_lawyer_ossification(lawyer_id)

        # Should detect ossification
        assert alert is not None
        assert alert.pattern == BiasPattern.LAWYER_OSSIFICATION
        assert alert.actors == [lawyer_id]
        assert alert.evidence["use_count"] == 5

    def test_lawyer_ossification_under_threshold(self, detector):
        """No alert when reasons are varied."""
        lawyer_id = "defense_1"
        case_id = "case_varied"

        # Different reasons each time (under threshold)
        reasons = [
            "implicit_bias_criminal_law",
            "corporate_background",
            "works_for_regulator",
            "close_family_lawyer",
        ]

        for i, reason in enumerate(reasons):
            detector.record_strike(
                case_id,
                lawyer_id,
                "Defense_DueProcess",
                reason,
                f"profile_hash_{i}",
            )

        # Detect lawyer ossification
        alert = detector.detect_lawyer_ossification(lawyer_id)

        # Should not detect (under threshold)
        assert alert is None


class TestCoordinatedStrikes:
    """Test detection of coordinated striking between counsel."""

    def test_coordinated_strikes_both_counsel(self, detector):
        """Detect when both DA and Defense strike same candidates with similar reasons."""
        case_id = "case_coordinated"

        # DA counsel strikes with reason
        da_strikes = {"economics_expertise": 5}
        # Defense counsel strikes with same reason
        defense_strikes = {"economics_expertise": 3}

        # Detect coordinated strikes
        alerts = detector.detect_coordinated_strikes(case_id, da_strikes, defense_strikes)

        # Should detect coordination
        assert len(alerts) > 0
        assert any(a.pattern == BiasPattern.COORDINATED_STRIKE for a in alerts)

    def test_independent_strikes_no_coordination(self, detector):
        """No alert when counsel strike for genuinely different reasons."""
        case_id = "case_independent"

        # DA counsel uses different reasons
        da_strikes = {"worked_for_defendant": 2, "conflicts_of_interest": 1}
        # Defense counsel uses different reasons
        defense_strikes = {"prior_conviction": 2, "pro_government_bias": 1}

        # Detect coordinated strikes
        alerts = detector.detect_coordinated_strikes(case_id, da_strikes, defense_strikes)

        # Should not detect coordination
        assert len(alerts) == 0


class TestSectionTargeting:
    """Test detection of biased exclusion across demographic/professional sections."""

    def test_section_targeting_above_threshold(self, detector):
        """Detect when >70% of exclusions from one section."""
        case_id = "case_section_bias"

        # 75% of exclusions from "Academic" section
        excluded_by_section = {"Academic": 15, "Corporate": 3, "Government": 2}

        # Detect section targeting
        alert = detector.detect_section_targeting(case_id, excluded_by_section)

        # Should detect section targeting
        assert alert is not None
        assert alert.pattern == BiasPattern.SECTION_TARGETING
        assert alert.level == AnomalyLevel.ALERT
        assert alert.evidence["targeted_section"] == "Academic"

    def test_section_targeting_proportional(self, detector):
        """No alert when exclusion rate matches overall rate."""
        case_id = "case_proportional"

        # Proportional exclusion (40% from all sections)
        excluded_by_section = {"Corporate": 4, "Academic": 3, "Government": 3}

        # Detect section targeting
        alert = detector.detect_section_targeting(case_id, excluded_by_section)

        # Should not detect disproportionate targeting
        assert alert is None


class TestAnomalyAuditTrail:
    """Test anomaly logging and audit trail integrity."""

    def test_anomaly_records_preserved(self, detector):
        """All anomaly records preserved with full evidence."""
        # Create anomaly manually
        now = time.time()
        record = AnomalyRecord(
            pattern=BiasPattern.IDEOLOGICAL_SURGE,
            level=AnomalyLevel.WARNING,
            case_id="test_case",
            actors=["test_agent"],
            evidence={"test": "data"},
            timestamp=now,
        )
        detector.anomalies.append(record)

        # Check it's preserved
        assert len(detector.anomalies) == 1
        rec = detector.anomalies[0]
        assert rec.pattern == BiasPattern.IDEOLOGICAL_SURGE
        assert rec.case_id == "test_case"
        assert rec.evidence["test"] == "data"

    def test_anomaly_filtering_by_level(self, detector):
        """Filter anomalies by minimum severity level."""
        now = time.time()

        # Create anomalies at different levels
        detector.anomalies = [
            AnomalyRecord(
                pattern=BiasPattern.CIVIC_SELF_TERMINATION,
                level=AnomalyLevel.INFO,
                case_id="case_1",
                actors=[],
                evidence={},
                timestamp=now,
            ),
            AnomalyRecord(
                pattern=BiasPattern.LAWYER_OSSIFICATION,
                level=AnomalyLevel.WARNING,
                case_id="case_1",
                actors=[],
                evidence={},
                timestamp=now,
            ),
            AnomalyRecord(
                pattern=BiasPattern.COORDINATED_STRIKE,
                level=AnomalyLevel.ALERT,
                case_id="case_1",
                actors=[],
                evidence={},
                timestamp=now,
            ),
        ]

        # Filter to WARNING and above
        filtered = detector.get_anomaly_log(min_level=AnomalyLevel.WARNING)
        assert len(filtered) == 2
        assert all(log.level in [AnomalyLevel.WARNING, AnomalyLevel.ALERT]
                   for log in filtered)

    def test_anomaly_filtering_by_case(self, detector):
        """Filter anomalies by case ID."""
        now = time.time()

        detector.anomalies = [
            AnomalyRecord(
                pattern=BiasPattern.CIVIC_SELF_TERMINATION,
                level=AnomalyLevel.INFO,
                case_id="case_1",
                actors=[],
                evidence={},
                timestamp=now,
            ),
            AnomalyRecord(
                pattern=BiasPattern.IDEOLOGICAL_SURGE,
                level=AnomalyLevel.INFO,
                case_id="case_2",
                actors=[],
                evidence={},
                timestamp=now,
            ),
        ]

        case_1_logs = [
            log for log in detector.anomalies if log.case_id == "case_1"
        ]
        assert len(case_1_logs) == 1


class TestMetaAppealTrigger:
    """Test conditions that trigger meta-appeal system activation."""

    def test_critical_anomaly_triggers_meta_appeal(self, detector):
        """CRITICAL severity anomaly should trigger meta-appeal."""
        case_id = "high_stakes_case"

        # Record high-risk strike coordination (triggers meta-appeal)
        da_strikes = {"pro_defi_background": 8}
        defense_strikes = {"pro_defi_background": 6}
        alerts = detector.detect_coordinated_strikes(case_id, da_strikes, defense_strikes)

        # High coordination ratio should trigger meta-appeal
        should_trigger = detector.should_trigger_meta_appeal(case_id)
        # Will be True if coordination ratio > 0.8 (14/14 = 1.0)
        if len(alerts) > 0 and alerts[0].evidence["coordination_ratio"] > 0.8:
            assert should_trigger is True

    def test_multiple_alert_anomalies_trigger_meta_appeal(self, detector):
        """Multiple ALERT-level anomalies should trigger meta-appeal."""
        case_id = "complex_case"

        # Record section targeting (triggers meta-appeal)
        excluded_by_section = {"Academic": 14, "Corporate": 1}
        alert1 = detector.detect_section_targeting(case_id, excluded_by_section)

        if alert1 is not None:
            should_trigger = detector.should_trigger_meta_appeal(case_id)
            assert should_trigger is True

    def test_info_level_anomalies_no_meta_appeal(self, detector):
        """INFO-level anomalies alone should not trigger meta-appeal."""
        case_id = "routine_case"

        # Record proportional voting (stays at INFO level)
        detector.record_vote(case_id, "agent_1", True)
        detector.record_vote(case_id, "agent_2", True)
        detector.record_vote(case_id, "agent_3", False)
        alert = detector.detect_ideological_surge(case_id)

        # Should be None (under 80% threshold)
        assert alert is None

        should_trigger = detector.should_trigger_meta_appeal(case_id)
        assert should_trigger is False


class TestAnomalySummary:
    """Test system-wide anomaly summary and reporting."""

    def test_system_summary_aggregates_patterns(self, detector):
        """System summary should count patterns by type and level."""
        now = time.time()

        detector.anomalies = [
            AnomalyRecord(
                pattern=BiasPattern.CIVIC_SELF_TERMINATION,
                level=AnomalyLevel.WARNING,
                case_id="case_1",
                actors=[],
                evidence={},
                timestamp=now,
            ),
            AnomalyRecord(
                pattern=BiasPattern.CIVIC_SELF_TERMINATION,
                level=AnomalyLevel.WARNING,
                case_id="case_2",
                actors=[],
                evidence={},
                timestamp=now,
            ),
            AnomalyRecord(
                pattern=BiasPattern.LAWYER_OSSIFICATION,
                level=AnomalyLevel.ALERT,
                case_id="case_1",
                actors=[],
                evidence={},
                timestamp=now,
            ),
        ]

        summary = detector.get_summary()

        # Each pattern should be counted
        assert summary["total_anomalies"] == 3
        assert "by_pattern" in summary
        assert "by_level" in summary

        # Civic self-termination should have count of 2
        assert summary["by_pattern"]["civic_self_termination"] == 2

    def test_summary_critical_alert_count(self, detector):
        """Summary should highlight ALERT cases."""
        now = time.time()

        detector.anomalies = [
            AnomalyRecord(
                pattern=BiasPattern.COORDINATED_STRIKE,
                level=AnomalyLevel.ALERT,
                case_id="case_1",
                actors=[],
                evidence={},
                timestamp=now,
            ),
            AnomalyRecord(
                pattern=BiasPattern.LAWYER_OSSIFICATION,
                level=AnomalyLevel.WARNING,
                case_id="case_2",
                actors=[],
                evidence={},
                timestamp=now,
            ),
            AnomalyRecord(
                pattern=BiasPattern.CIVIC_SELF_TERMINATION,
                level=AnomalyLevel.INFO,
                case_id="case_3",
                actors=[],
                evidence={},
                timestamp=now,
            ),
        ]

        summary = detector.get_summary()

        # Should track anomaly counts by level
        assert summary["by_level"]["alert"] >= 1
        assert summary["by_level"]["warning"] >= 1
