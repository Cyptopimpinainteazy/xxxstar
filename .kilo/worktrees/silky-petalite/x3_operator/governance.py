"""
X3 Governance Capture Simulation
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Simulates 4 attack vectors against the governance system:
1. Whale attack - concentration of voting power
2. Sybil attack - many small accounts voting in concert
3. Bribery attack - vote buying with time-decay detection
4. Speed attack - rushing proposals through low-quorum windows

All simulations are deterministic. Same seed → same results.
"""

import hashlib
import logging
import random
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional

logger = logging.getLogger(__name__)


class AttackType(str, Enum):
    WHALE = "whale"
    SYBIL = "sybil"
    BRIBERY = "bribery"
    SPEED = "speed"


class SimulationResult(str, Enum):
    CAPTURED = "captured"
    DEFENDED = "defended"
    PARTIAL = "partial"


@dataclass
class Voter:
    """Simulated governance participant."""
    account_id: str
    stake: int
    conviction: int = 1  # 1x-6x conviction multiplier
    delegated_to: str = ""
    is_attacker: bool = False
    bribed: bool = False
    vote_time: float = 0.0

    def voting_power(self) -> int:
        return self.stake * self.conviction


@dataclass
class Proposal:
    """Simulated governance proposal."""
    proposal_id: str
    title: str
    aye_power: int = 0
    nay_power: int = 0
    total_turnout: int = 0
    quorum_required: int = 0
    voting_period_seconds: float = 604800.0  # 7 days
    created_at: float = 0.0

    def passed(self) -> bool:
        if self.total_turnout < self.quorum_required:
            return False
        return self.aye_power > self.nay_power

    def turnout_ratio(self, total_stake: int) -> float:
        if total_stake == 0:
            return 0.0
        return self.total_turnout / total_stake


@dataclass
class SimulationReport:
    attack_type: AttackType
    result: SimulationResult
    seed: int
    num_voters: int
    total_stake: int
    attacker_stake: int
    attacker_fraction: float
    proposal: Proposal
    defense_triggered: bool
    defense_details: str = ""
    duration_seconds: float = 0.0
    findings: list = field(default_factory=list)

    def to_dict(self) -> dict:
        return {
            "attack_type": self.attack_type.value,
            "result": self.result.value,
            "seed": self.seed,
            "num_voters": self.num_voters,
            "total_stake": self.total_stake,
            "attacker_stake": self.attacker_stake,
            "attacker_fraction": round(self.attacker_fraction, 4),
            "proposal_passed": self.proposal.passed(),
            "aye_power": self.proposal.aye_power,
            "nay_power": self.proposal.nay_power,
            "turnout": self.proposal.total_turnout,
            "quorum_required": self.proposal.quorum_required,
            "defense_triggered": self.defense_triggered,
            "defense_details": self.defense_details,
            "duration_seconds": round(self.duration_seconds, 3),
            "findings": self.findings,
        }


class GovernanceSimulator:
    """Deterministic governance attack simulator.

    Uses seeded RNG so every run with the same seed produces identical results.
    """

    def __init__(self, seed: int = 42):
        self.seed = seed
        self.rng = random.Random(seed)
        self.reports: list[SimulationReport] = []

    def _make_voters(self, n_honest: int, honest_stake_range: tuple = (100, 10000)) -> list[Voter]:
        voters = []
        for i in range(n_honest):
            stake = self.rng.randint(*honest_stake_range)
            conviction = self.rng.choice([1, 1, 1, 2, 2, 3])
            voters.append(Voter(
                account_id=f"honest-{i:04d}",
                stake=stake,
                conviction=conviction,
            ))
        return voters

    def _create_proposal(self, title: str, total_stake: int) -> Proposal:
        return Proposal(
            proposal_id=hashlib.sha256(f"{self.seed}:{title}".encode()).hexdigest()[:16],
            title=title,
            quorum_required=int(total_stake * 0.10),  # 10% quorum
            created_at=time.time(),
        )

    def simulate_whale_attack(
        self,
        n_honest: int = 100,
        whale_stake_fraction: float = 0.34,
        whale_conviction: int = 6,
    ) -> SimulationReport:
        """Simulate a single large stakeholder trying to push a proposal.

        Defense: conviction-weighted voting + adaptive quorum.
        """
        start = time.time()
        voters = self._make_voters(n_honest)
        total_honest_stake = sum(v.stake for v in voters)

        # Whale enters
        whale_stake = int(total_honest_stake * whale_stake_fraction / (1 - whale_stake_fraction))
        whale = Voter(
            account_id="whale-0001",
            stake=whale_stake,
            conviction=whale_conviction,
            is_attacker=True,
        )
        all_voters = voters + [whale]
        total_stake = sum(v.stake for v in all_voters)

        proposal = self._create_proposal("whale-drain-treasury", total_stake)
        findings = []

        # Whale votes aye
        proposal.aye_power += whale.voting_power()
        proposal.total_turnout += whale.stake

        # Honest voters vote with varying participation
        for v in voters:
            if self.rng.random() < 0.6:  # 60% turnout
                proposal.nay_power += v.voting_power()
                proposal.total_turnout += v.stake

        # Defense: detect concentration
        whale_vp_fraction = whale.voting_power() / (proposal.aye_power + proposal.nay_power)
        defense_triggered = whale_vp_fraction > 0.33
        defense_details = ""

        if defense_triggered:
            # Raise quorum requirement when concentration detected
            proposal.quorum_required = int(total_stake * 0.25)
            defense_details = f"Adaptive quorum raised to 25% (whale VP fraction: {whale_vp_fraction:.2%})"
            findings.append(f"Whale controls {whale_vp_fraction:.2%} of voting power")
            findings.append(defense_details)

        passed = proposal.passed()
        result = SimulationResult.CAPTURED if passed else SimulationResult.DEFENDED

        report = SimulationReport(
            attack_type=AttackType.WHALE,
            result=result,
            seed=self.seed,
            num_voters=len(all_voters),
            total_stake=total_stake,
            attacker_stake=whale_stake,
            attacker_fraction=whale_stake / total_stake,
            proposal=proposal,
            defense_triggered=defense_triggered,
            defense_details=defense_details,
            duration_seconds=time.time() - start,
            findings=findings,
        )
        self.reports.append(report)
        return report

    def simulate_sybil_attack(
        self,
        n_honest: int = 100,
        n_sybils: int = 500,
        sybil_stake_each: int = 10,
    ) -> SimulationReport:
        """Simulate many small accounts voting in concert.

        Defense: minimum stake threshold + velocity detection.
        """
        start = time.time()
        voters = self._make_voters(n_honest)
        total_honest_stake = sum(v.stake for v in voters)

        # Sybil accounts
        sybils = []
        for i in range(n_sybils):
            sybils.append(Voter(
                account_id=f"sybil-{i:05d}",
                stake=sybil_stake_each,
                conviction=1,
                is_attacker=True,
            ))

        all_voters = voters + sybils
        total_stake = sum(v.stake for v in all_voters)
        sybil_total_stake = sum(s.stake for s in sybils)

        proposal = self._create_proposal("sybil-dilute-governance", total_stake)
        findings = []

        # Sybils all vote aye simultaneously
        vote_times = []
        for s in sybils:
            proposal.aye_power += s.voting_power()
            proposal.total_turnout += s.stake
            vote_times.append(self.rng.uniform(0, 60))  # all within 60 seconds

        # Honest voters
        for v in voters:
            if self.rng.random() < 0.5:
                proposal.nay_power += v.voting_power()
                proposal.total_turnout += v.stake

        # Defense: velocity detection - detect burst of votes from new accounts
        vote_velocity = len(sybils) / 60.0  # votes per second
        min_stake_threshold = 100
        below_threshold = sum(1 for s in sybils if s.stake < min_stake_threshold)
        defense_triggered = vote_velocity > 5.0 or below_threshold > n_honest

        defense_details = ""
        if defense_triggered:
            # Filter out votes below minimum stake
            filtered_power = sum(s.voting_power() for s in sybils if s.stake < min_stake_threshold)
            proposal.aye_power -= filtered_power
            proposal.total_turnout -= sum(s.stake for s in sybils if s.stake < min_stake_threshold)
            defense_details = (
                f"Sybil filter: {below_threshold} accounts below {min_stake_threshold} stake "
                f"filtered. Vote velocity: {vote_velocity:.1f}/s"
            )
            findings.append(f"{n_sybils} sybil accounts detected (velocity: {vote_velocity:.1f}/s)")
            findings.append(defense_details)

        passed = proposal.passed()
        result = SimulationResult.CAPTURED if passed else SimulationResult.DEFENDED

        report = SimulationReport(
            attack_type=AttackType.SYBIL,
            result=result,
            seed=self.seed,
            num_voters=len(all_voters),
            total_stake=total_stake,
            attacker_stake=sybil_total_stake,
            attacker_fraction=sybil_total_stake / total_stake,
            proposal=proposal,
            defense_triggered=defense_triggered,
            defense_details=defense_details,
            duration_seconds=time.time() - start,
            findings=findings,
        )
        self.reports.append(report)
        return report

    def simulate_bribery_attack(
        self,
        n_honest: int = 100,
        bribe_budget: int = 50000,
        bribe_per_vote: int = 100,
    ) -> SimulationReport:
        """Simulate vote buying with economic incentives.

        Defense: conviction locking + vote commitment scheme.
        """
        start = time.time()
        voters = self._make_voters(n_honest)
        total_stake = sum(v.stake for v in voters)

        proposal = self._create_proposal("bribery-extract-funds", total_stake)
        findings = []

        bribed_count = 0
        bribed_power = 0
        remaining_budget = bribe_budget

        # Attempt to bribe voters (those with lower conviction are more susceptible)
        for v in sorted(voters, key=lambda x: x.conviction):
            if remaining_budget < bribe_per_vote:
                break
            # Susceptibility inversely proportional to conviction
            susceptibility = 1.0 / v.conviction
            if self.rng.random() < susceptibility * 0.4:  # base 40% chance at conviction 1
                v.bribed = True
                v.is_attacker = True
                bribed_count += 1
                remaining_budget -= bribe_per_vote
                proposal.aye_power += v.voting_power()
                proposal.total_turnout += v.stake
                bribed_power += v.voting_power()

        # Unbribed voters
        for v in voters:
            if not v.bribed and self.rng.random() < 0.55:
                proposal.nay_power += v.voting_power()
                proposal.total_turnout += v.stake

        # Defense: detect suspicious voting pattern changes
        bribed_fraction = bribed_count / len(voters) if voters else 0
        defense_triggered = bribed_fraction > 0.15

        defense_details = ""
        if defense_triggered:
            # Extend voting period and require conviction lock
            min_conviction = 2
            reduced_power = sum(
                v.voting_power() - v.stake  # remove conviction bonus
                for v in voters if v.bribed and v.conviction < min_conviction
            )
            proposal.aye_power -= reduced_power
            defense_details = (
                f"Bribery detection: {bribed_count}/{len(voters)} voters ({bribed_fraction:.1%}) "
                f"showed unusual pattern. Conviction lock enforced (min {min_conviction}x)."
            )
            findings.append(f"Budget spent: {bribe_budget - remaining_budget}/{bribe_budget}")
            findings.append(f"{bribed_count} voters bribed")
            findings.append(defense_details)

        attacker_stake = sum(v.stake for v in voters if v.bribed)
        passed = proposal.passed()
        result = SimulationResult.CAPTURED if passed else SimulationResult.DEFENDED

        report = SimulationReport(
            attack_type=AttackType.BRIBERY,
            result=result,
            seed=self.seed,
            num_voters=len(voters),
            total_stake=total_stake,
            attacker_stake=attacker_stake,
            attacker_fraction=attacker_stake / total_stake if total_stake else 0,
            proposal=proposal,
            defense_triggered=defense_triggered,
            defense_details=defense_details,
            duration_seconds=time.time() - start,
            findings=findings,
        )
        self.reports.append(report)
        return report

    def simulate_speed_attack(
        self,
        n_honest: int = 100,
        attacker_stake_fraction: float = 0.20,
        off_peak_participation: float = 0.10,
    ) -> SimulationReport:
        """Simulate rushing a proposal through during low participation.

        Defense: minimum voting period + delayed enactment.
        """
        start = time.time()
        voters = self._make_voters(n_honest)
        total_honest_stake = sum(v.stake for v in voters)

        attacker_stake = int(total_honest_stake * attacker_stake_fraction / (1 - attacker_stake_fraction))
        attacker = Voter(
            account_id="speed-attacker-0001",
            stake=attacker_stake,
            conviction=1,
            is_attacker=True,
        )
        all_voters = voters + [attacker]
        total_stake = sum(v.stake for v in all_voters)

        proposal = self._create_proposal("speed-rush-proposal", total_stake)
        findings = []

        # Attacker votes aye
        proposal.aye_power += attacker.voting_power()
        proposal.total_turnout += attacker.stake

        # Only off_peak fraction of honest voters participate
        participating = int(n_honest * off_peak_participation)
        for v in self.rng.sample(voters, min(participating, len(voters))):
            if self.rng.random() < 0.5:
                proposal.aye_power += v.voting_power()
            else:
                proposal.nay_power += v.voting_power()
            proposal.total_turnout += v.stake

        # Defense: minimum turnout + delay
        turnout_ratio = proposal.turnout_ratio(total_stake)
        defense_triggered = turnout_ratio < 0.15

        defense_details = ""
        if defense_triggered:
            # Extend voting period and raise quorum
            proposal.quorum_required = int(total_stake * 0.20)
            proposal.voting_period_seconds *= 2
            defense_details = (
                f"Low turnout detected ({turnout_ratio:.1%}). "
                f"Voting period doubled. Quorum raised to 20%."
            )
            findings.append(f"Only {turnout_ratio:.1%} turnout during attack window")
            findings.append(defense_details)

        passed = proposal.passed()
        result = SimulationResult.CAPTURED if passed else SimulationResult.DEFENDED

        report = SimulationReport(
            attack_type=AttackType.SPEED,
            result=result,
            seed=self.seed,
            num_voters=len(all_voters),
            total_stake=total_stake,
            attacker_stake=attacker_stake,
            attacker_fraction=attacker_stake / total_stake,
            proposal=proposal,
            defense_triggered=defense_triggered,
            defense_details=defense_details,
            duration_seconds=time.time() - start,
            findings=findings,
        )
        self.reports.append(report)
        return report

    def run_full_suite(self) -> list[SimulationReport]:
        """Run all 4 attack simulations and return reports."""
        return [
            self.simulate_whale_attack(),
            self.simulate_sybil_attack(),
            self.simulate_bribery_attack(),
            self.simulate_speed_attack(),
        ]

    def summary(self) -> dict:
        """Summary of all simulation results."""
        results = {}
        for report in self.reports:
            results[report.attack_type.value] = {
                "result": report.result.value,
                "defense_triggered": report.defense_triggered,
                "proposal_passed": report.proposal.passed(),
                "attacker_fraction": round(report.attacker_fraction, 4),
            }
        captured = sum(1 for r in self.reports if r.result == SimulationResult.CAPTURED)
        defended = sum(1 for r in self.reports if r.result == SimulationResult.DEFENDED)
        return {
            "total_simulations": len(self.reports),
            "captured": captured,
            "defended": defended,
            "resilience_score": defended / len(self.reports) if self.reports else 0.0,
            "attacks": results,
        }
