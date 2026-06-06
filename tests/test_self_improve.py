"""Tests for the Costly Self-Improvement layer."""

import pytest

from swarm.core.enums import Domain
from swarm.event_bus.events import EventType
from swarm.self_improve.cost import CostCalculator
from swarm.self_improve.engine import SelfImprovementEngine
from swarm.self_improve.scars import ScarRegistry
from swarm.self_improve.schema import (
    ImprovementProposal,
    ImprovementType,
    ProposalStatus,
    Scar,
)
from swarm.storage.backend import SqliteStorage


@pytest.fixture
def storage(tmp_path):
    return SqliteStorage(str(tmp_path / "test.db"))


@pytest.fixture
def scars(storage):
    return ScarRegistry(storage, agent_id="test-agent")


@pytest.fixture
def engine(storage, scars):
    return SelfImprovementEngine(
        storage=storage,
        agent_id="test-agent",
        scars=scars,
        resource_budget=100.0,
        cooldown_seconds=0,  # disable cooldown for tests
    )


class TestCostCalculator:
    """INV-SELFIMPROVE-001: Cost scales with proficiency²."""

    def test_cost_increases_with_proficiency(self):
        calc = CostCalculator()
        cost_low = calc.calculate(current_proficiency=0.3, scar_count=0)
        cost_high = calc.calculate(current_proficiency=0.9, scar_count=0)
        assert cost_high > cost_low

    def test_scars_increase_cost(self):
        calc = CostCalculator()
        cost_no_scars = calc.calculate(current_proficiency=0.5, scar_count=0)
        cost_with_scars = calc.calculate(current_proficiency=0.5, scar_count=5)
        assert cost_with_scars > cost_no_scars

    def test_cost_formula(self):
        calc = CostCalculator(base_multiplier=10.0, scar_penalty=0.2)
        # 10 * 0.5² = 2.5, scar_mult = 1.0 + 0.2*3 = 1.6, total = 4.0
        cost = calc.calculate(current_proficiency=0.5, scar_count=3)
        assert abs(cost - 4.0) < 0.01

    def test_minimum_cost(self):
        calc = CostCalculator()
        cost = calc.calculate(current_proficiency=0.01, scar_count=0)
        assert cost >= 0.1


class TestScarRegistry:
    """INV-SELFIMPROVE-002: Scars are permanent."""

    def test_record_and_retrieve(self, scars):
        scar = Scar(
            agent_id="test-agent",
            proposal_id="p1",
            improvement_type=ImprovementType.CAPABILITY_UPGRADE,
            target_domain=Domain.CODE,
            target_capability="rust",
            cost_paid=5.0,
            failure_reason="compilation error",
        )
        scars.record(scar)

        all_scars = scars.get_all()
        assert len(all_scars) == 1
        assert all_scars[0].failure_reason == "compilation error"

    def test_count_by_domain(self, scars):
        for i in range(3):
            scars.record(
                Scar(
                    agent_id="test-agent",
                    proposal_id=f"p{i}",
                    improvement_type=ImprovementType.PARAMETER_TUNE,
                    target_domain=Domain.CODE,
                    target_capability="perf",
                    cost_paid=1.0,
                )
            )
        assert scars.count_in_domain(Domain.CODE.value) == 3

    def test_scar_emits_bus_event(self, scars):
        scars.record(
            Scar(
                agent_id="test-agent",
                proposal_id="p1",
                improvement_type=ImprovementType.CAPABILITY_UPGRADE,
                target_domain=Domain.CODE,
                target_capability="x",
                cost_paid=1.0,
            )
        )
        events = scars.get_pending_bus_events()
        assert any(e.event_type == EventType.SCAR_RECORDED for e in events)


class TestSelfImprovementEngine:
    """INV-SELFIMPROVE-003: Full improvement lifecycle."""

    def test_propose_and_execute_success(self, engine):
        proposal = ImprovementProposal(
            agent_id="test-agent",
            improvement_type=ImprovementType.CAPABILITY_UPGRADE,
            target_capability="rust",
            target_domain=Domain.CODE,
            current_proficiency=0.5,
        )
        result = engine.propose(proposal)
        assert result.status == ProposalStatus.APPROVED

        outcome = engine.execute(result, success=True, proficiency_delta=0.1)
        assert outcome.success
        assert outcome.proficiency_after > outcome.proficiency_before
        assert engine.resource_budget < 100.0

    def test_failed_improvement_creates_scar(self, engine, scars):
        proposal = ImprovementProposal(
            agent_id="test-agent",
            improvement_type=ImprovementType.CAPABILITY_UPGRADE,
            target_capability="python",
            target_domain=Domain.CODE,
            current_proficiency=0.5,
        )
        approved = engine.propose(proposal)
        engine.execute(approved, success=False)

        all_scars = scars.get_all()
        assert len(all_scars) == 1

    def test_budget_rejection(self, storage, scars):
        engine = SelfImprovementEngine(
            storage=storage,
            agent_id="test-agent",
            scars=scars,
            resource_budget=0.01,  # nearly empty
            cooldown_seconds=0,
        )
        proposal = ImprovementProposal(
            agent_id="test-agent",
            improvement_type=ImprovementType.ARCHITECTURE_CHANGE,
            target_capability="big-change",
            target_domain=Domain.CODE,
            current_proficiency=0.9,
        )
        result = engine.propose(proposal)
        assert result.status == ProposalStatus.REJECTED_BUDGET

    def test_cooldown_rejection(self, storage, scars):
        engine = SelfImprovementEngine(
            storage=storage,
            agent_id="test-agent",
            scars=scars,
            resource_budget=1000.0,
            cooldown_seconds=9999,  # very long cooldown
        )
        p1 = ImprovementProposal(
            agent_id="test-agent",
            improvement_type=ImprovementType.PARAMETER_TUNE,
            target_capability="c1",
            target_domain=Domain.CODE,
            current_proficiency=0.3,
        )
        engine.propose(p1)
        engine.execute(p1, success=True, proficiency_delta=0.1)

        p2 = ImprovementProposal(
            agent_id="test-agent",
            improvement_type=ImprovementType.PARAMETER_TUNE,
            target_capability="c2",
            target_domain=Domain.CODE,
            current_proficiency=0.3,
        )
        result = engine.propose(p2)
        assert result.status == ProposalStatus.REJECTED_COOLDOWN

    def test_cost_always_deducted(self, engine):
        initial_budget = engine.resource_budget
        proposal = ImprovementProposal(
            agent_id="test-agent",
            improvement_type=ImprovementType.CAPABILITY_UPGRADE,
            target_capability="skill",
            target_domain=Domain.CODE,
            current_proficiency=0.5,
        )
        approved = engine.propose(proposal)
        engine.execute(approved, success=False)
        assert engine.resource_budget < initial_budget
