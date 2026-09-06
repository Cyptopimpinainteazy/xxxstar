"""Tests for x3_operator.governance"""

from x3_operator.governance import (
    AttackType,
    GovernanceSimulator,
    SimulationResult,
)


def test_whale_attack():
    sim = GovernanceSimulator(seed=42)
    report = sim.simulate_whale_attack()
    assert report.attack_type == AttackType.WHALE
    assert report.result in SimulationResult
    assert report.num_voters > 0
    assert report.attacker_stake > 0


def test_sybil_attack():
    sim = GovernanceSimulator(seed=42)
    report = sim.simulate_sybil_attack()
    assert report.attack_type == AttackType.SYBIL
    assert report.result == SimulationResult.DEFENDED  # sybil defense should work


def test_bribery_attack():
    sim = GovernanceSimulator(seed=42)
    report = sim.simulate_bribery_attack()
    assert report.attack_type == AttackType.BRIBERY
    assert report.defense_triggered


def test_speed_attack():
    sim = GovernanceSimulator(seed=42)
    report = sim.simulate_speed_attack()
    assert report.attack_type == AttackType.SPEED


def test_full_suite():
    sim = GovernanceSimulator(seed=42)
    reports = sim.run_full_suite()
    assert len(reports) == 4
    types = {r.attack_type for r in reports}
    assert types == {AttackType.WHALE, AttackType.SYBIL, AttackType.BRIBERY, AttackType.SPEED}


def test_deterministic():
    """Same seed → same results."""
    sim1 = GovernanceSimulator(seed=42)
    r1 = sim1.run_full_suite()

    sim2 = GovernanceSimulator(seed=42)
    r2 = sim2.run_full_suite()

    for a, b in zip(r1, r2, strict=False):
        assert a.result == b.result
        assert a.proposal.aye_power == b.proposal.aye_power
        assert a.proposal.nay_power == b.proposal.nay_power


def test_different_seeds():
    sim1 = GovernanceSimulator(seed=1)
    sim2 = GovernanceSimulator(seed=999)
    r1 = sim1.simulate_whale_attack()
    r2 = sim2.simulate_whale_attack()
    # Different seeds should produce different voter distributions
    assert r1.proposal.nay_power != r2.proposal.nay_power


def test_summary():
    sim = GovernanceSimulator(seed=42)
    sim.run_full_suite()
    summary = sim.summary()
    assert summary["total_simulations"] == 4
    assert "resilience_score" in summary
    assert "attacks" in summary
    assert summary["captured"] + summary["defended"] == 4


def test_report_to_dict():
    sim = GovernanceSimulator(seed=42)
    report = sim.simulate_whale_attack()
    d = report.to_dict()
    assert "attack_type" in d
    assert "result" in d
    assert "proposal_passed" in d
    assert "attacker_fraction" in d

def test_whale_high_fraction():
    sim = GovernanceSimulator(seed=42)
    report = sim.simulate_whale_attack(whale_stake_fraction=0.60)
    assert report.defense_triggered


def test_sybil_low_count():
    sim = GovernanceSimulator(seed=42)
    report = sim.simulate_sybil_attack(n_sybils=5, sybil_stake_each=10000)
    # Low sybil count with high stake - harder to detect
    assert report.attack_type == AttackType.SYBIL
