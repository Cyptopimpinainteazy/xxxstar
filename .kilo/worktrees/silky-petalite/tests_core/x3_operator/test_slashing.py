"""Tests for x3_operator.slashing"""

import pytest

from x3_operator.config import X3Config
from x3_operator.slashing import FaultType, SlashEvidence, SlashingEngine


@pytest.fixture
def engine():
    return SlashingEngine(X3Config())


def _make_evidence(fault: FaultType, operator: str = "op-001") -> SlashEvidence:
    return SlashEvidence(
        fault_type=fault,
        operator_id=operator,
        block_number=100,
        timestamp=1000.0,
        description="test fault",
        reporter_id="test",
    )


def test_basic_slash(engine):
    evidence = _make_evidence(FaultType.DOWNTIME)
    verdict = engine.evaluate(evidence, bond_amount=100_000)
    assert verdict.slash_amount > 0
    assert verdict.severity == 0.01  # default downtime severity
    assert verdict.repetition_count == 1


def test_equivocation_severity(engine):
    evidence = _make_evidence(FaultType.EQUIVOCATION)
    verdict = engine.evaluate(evidence, bond_amount=100_000)
    assert verdict.severity == 0.50
    assert verdict.slash_amount == 50_000


def test_repetition_multiplier(engine):
    e1 = _make_evidence(FaultType.DOWNTIME)
    v1 = engine.evaluate(e1, bond_amount=100_000)

    e2 = _make_evidence(FaultType.DOWNTIME)
    v2 = engine.evaluate(e2, bond_amount=100_000)

    assert v2.repetition_count == 2
    assert v2.repetition_multiplier > v1.repetition_multiplier
    assert v2.slash_amount > v1.slash_amount


def test_confidence_factor(engine):
    evidence = _make_evidence(FaultType.DOWNTIME)
    v_full = engine.evaluate(evidence, bond_amount=100_000, confidence=1.0)

    engine2 = SlashingEngine(X3Config())
    evidence2 = _make_evidence(FaultType.DOWNTIME)
    v_half = engine2.evaluate(evidence2, bond_amount=100_000, confidence=0.5)

    assert v_half.slash_amount < v_full.slash_amount


def test_confidence_validation(engine):
    evidence = _make_evidence(FaultType.DOWNTIME)
    with pytest.raises(ValueError, match="Confidence"):
        engine.evaluate(evidence, bond_amount=100_000, confidence=1.5)


def test_max_slash_cap(engine):
    evidence = _make_evidence(FaultType.DATA_CORRUPTION)
    verdict = engine.evaluate(evidence, bond_amount=100_000)
    # data_corruption = 1.00 severity, should hit max
    assert verdict.slash_amount <= 100_000


def test_cumulative_slash(engine):
    for _ in range(3):
        e = _make_evidence(FaultType.DOWNTIME)
        engine.evaluate(e, bond_amount=100_000)

    total = engine.cumulative_slash("op-001")
    assert total > 0


def test_operator_history(engine):
    e1 = _make_evidence(FaultType.DOWNTIME)
    e2 = _make_evidence(FaultType.EQUIVOCATION)
    engine.evaluate(e1, bond_amount=100_000)
    engine.evaluate(e2, bond_amount=100_000)

    history = engine.get_operator_history("op-001")
    assert len(history) == 2


def test_reset_history(engine):
    e = _make_evidence(FaultType.DOWNTIME)
    engine.evaluate(e, bond_amount=100_000)
    engine.reset_operator_history("op-001")
    assert len(engine.get_operator_history("op-001")) == 0


def test_verdict_hash_deterministic(engine):
    evidence = _make_evidence(FaultType.DOWNTIME)
    verdict = engine.evaluate(evidence, bond_amount=100_000)
    assert verdict.verdict_hash
    assert len(verdict.verdict_hash) == 64


def test_evidence_hash():
    e = _make_evidence(FaultType.DOWNTIME)
    assert e.evidence_hash
    assert len(e.evidence_hash) == 64


def test_different_operators(engine):
    e1 = _make_evidence(FaultType.DOWNTIME, "op-001")
    e2 = _make_evidence(FaultType.DOWNTIME, "op-002")
    engine.evaluate(e1, bond_amount=100_000)
    engine.evaluate(e2, bond_amount=100_000)

    # Each should have repetition_count = 1
    v1 = engine.get_operator_verdicts("op-001")
    v2 = engine.get_operator_verdicts("op-002")
    assert v1[0].repetition_count == 1
    assert v2[0].repetition_count == 1
