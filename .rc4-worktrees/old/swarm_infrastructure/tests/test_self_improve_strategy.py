"""Tests for Phase 5 — Self-Improvement Strategy Optimizer.

Invariant refs: tests/invariants/registry.toml — IMPROVEMENT_COST_POSITIVE, SCAR_PERMANENT

Tests cover:
- StrategyOptimizer: analyze, ROI scoring, risk scoring
- Causal blame: blame propagation from negative nodes
- Proposal generation: from candidates to concrete proposals
- Retrospective learning: outcome recording and success rate
- Edge cases: empty capabilities, zero budget, heavy scars
"""

from __future__ import annotations

import pytest

from swarm.causal.graph import CausalGraph
from swarm.causal.schema import CausalNode, NodeType, EdgeType
from swarm.self_improve.schema import ImprovementType, Scar
from swarm.self_improve.strategy import (
    ImprovementCandidate,
    StrategyOptimizer,
    StrategyReport,
)
from swarm.storage.backend import SqliteStorage


# =====================================================================
# Fixtures
# =====================================================================


@pytest.fixture
def storage():
    return SqliteStorage(":memory:")


@pytest.fixture
def causal(storage):
    return CausalGraph(storage=storage)


@pytest.fixture
def optimizer(storage, causal):
    return StrategyOptimizer(storage=storage, causal_graph=causal)


def _make_scars(agent_id: str, capability: str, count: int) -> list[Scar]:
    return [
        Scar(
            agent_id=agent_id,
            proposal_id=f"p-{i}",
            improvement_type=ImprovementType.CAPABILITY_UPGRADE,
            target_domain="MARKET",
            target_capability=capability,
            cost_paid=5.0,
        )
        for i in range(count)
    ]


# =====================================================================
# Analysis Tests
# =====================================================================


class TestStrategyAnalysis:
    """Test the analyze() method."""

    def test_basic_analysis(self, optimizer):
        report = optimizer.analyze(
            agent_id="a1",
            capabilities={"trading": 0.3, "coding": 0.7},
            scars=[],
            budget=100.0,
            epoch=5,
        )
        assert isinstance(report, StrategyReport)
        assert report.agent_id == "a1"
        assert len(report.candidates) == 2
        assert report.recommended is not None

    def test_candidates_ranked_by_roi(self, optimizer):
        report = optimizer.analyze(
            agent_id="a1",
            capabilities={"low": 0.1, "high": 0.9},
            scars=[],
            budget=100.0,
            epoch=1,
        )
        # "low" proficiency has higher ROI (more room to improve)
        assert report.candidates[0].capability == "low"
        assert report.candidates[0].expected_roi > report.candidates[1].expected_roi

    def test_scars_reduce_roi(self, optimizer):
        scars = _make_scars("a1", "trading", 5)
        report = optimizer.analyze(
            agent_id="a1",
            capabilities={"trading": 0.3},
            scars=scars,
            budget=100.0,
            epoch=1,
        )
        # With 5 scars, ROI should be reduced
        no_scar_report = optimizer.analyze(
            agent_id="a1",
            capabilities={"trading": 0.3},
            scars=[],
            budget=100.0,
            epoch=1,
        )
        assert report.candidates[0].expected_roi < no_scar_report.candidates[0].expected_roi

    def test_low_budget_triggers_wait(self, optimizer):
        report = optimizer.analyze(
            agent_id="a1",
            capabilities={"trading": 0.5},
            scars=[],
            budget=0.5,
            epoch=1,
        )
        assert report.should_wait is True
        assert "Budget" in report.wait_reason

    def test_empty_capabilities(self, optimizer):
        report = optimizer.analyze(
            agent_id="a1",
            capabilities={},
            scars=[],
            budget=100.0,
            epoch=1,
        )
        assert len(report.candidates) == 0
        assert report.recommended is None

    def test_priority_ranks_assigned(self, optimizer):
        report = optimizer.analyze(
            agent_id="a1",
            capabilities={"a": 0.1, "b": 0.5, "c": 0.9},
            scars=[],
            budget=100.0,
            epoch=1,
        )
        ranks = [c.priority_rank for c in report.candidates]
        assert ranks == [1, 2, 3]


# =====================================================================
# Causal Blame Tests
# =====================================================================


class TestCausalBlame:
    """Test blame propagation through the causal graph."""

    def test_blame_from_negative_consequence(self, optimizer, causal):
        # Create action → negative consequence chain
        action = causal.add_node(CausalNode(
            agent_id="a1",
            epoch=1,
            node_type=NodeType.ACTION,
            action_type="trading",
            value=-5.0,
        ))
        consequence = causal.add_node(CausalNode(
            agent_id="a1",
            epoch=1,
            node_type=NodeType.CONSEQUENCE,
            action_type="loss",
            value=-20.0,
        ))
        causal.add_edge(action.node_id, consequence.node_id,
                        edge_type=EdgeType.DIRECT, weight=1.0)

        report = optimizer.analyze(
            agent_id="a1",
            capabilities={"trading": 0.3},
            scars=[],
            budget=100.0,
            epoch=2,
        )
        # Trading should have blame
        assert report.candidates[0].causal_blame > 0.0

    def test_no_blame_for_positive_outcomes(self, optimizer, causal):
        # Only positive consequences
        action = causal.add_node(CausalNode(
            agent_id="a1",
            epoch=1,
            node_type=NodeType.ACTION,
            action_type="coding",
            value=10.0,
        ))
        consequence = causal.add_node(CausalNode(
            agent_id="a1",
            epoch=1,
            node_type=NodeType.CONSEQUENCE,
            action_type="reward",
            value=50.0,
        ))
        causal.add_edge(action.node_id, consequence.node_id,
                        edge_type=EdgeType.DIRECT, weight=1.0)

        report = optimizer.analyze(
            agent_id="a1",
            capabilities={"coding": 0.5},
            scars=[],
            budget=100.0,
            epoch=2,
        )
        # No negative outcomes → no blame
        assert report.candidates[0].causal_blame == 0.0

    def test_blame_from_death(self, optimizer, causal):
        action = causal.add_node(CausalNode(
            agent_id="a1",
            epoch=1,
            node_type=NodeType.ACTION,
            action_type="risky_trade",
            value=-1.0,
        ))
        death = causal.add_node(CausalNode(
            agent_id="a1",
            epoch=1,
            node_type=NodeType.DEATH,
            action_type="death:RESOURCE_EXHAUSTION",
            value=-100.0,
        ))
        causal.add_edge(action.node_id, death.node_id,
                        edge_type=EdgeType.DIRECT, weight=1.0)

        report = optimizer.analyze(
            agent_id="a1",
            capabilities={"trading": 0.2},
            scars=[],
            budget=50.0,
            epoch=2,
        )
        assert report.candidates[0].causal_blame > 0.0


# =====================================================================
# ROI & Risk Scoring Tests
# =====================================================================


class TestScoringFunctions:
    """Test the internal scoring functions."""

    def test_roi_higher_for_low_proficiency(self, optimizer):
        roi_low = optimizer._estimate_roi(0.1, blame=0.0, scar_count=0)
        roi_high = optimizer._estimate_roi(0.9, blame=0.0, scar_count=0)
        assert roi_low > roi_high

    def test_roi_amplified_by_blame(self, optimizer):
        roi_no_blame = optimizer._estimate_roi(0.3, blame=0.0, scar_count=0)
        roi_high_blame = optimizer._estimate_roi(0.3, blame=1.0, scar_count=0)
        assert roi_high_blame > roi_no_blame

    def test_roi_reduced_by_scars(self, optimizer):
        roi_no_scars = optimizer._estimate_roi(0.3, blame=0.5, scar_count=0)
        roi_many_scars = optimizer._estimate_roi(0.3, blame=0.5, scar_count=5)
        assert roi_many_scars < roi_no_scars

    def test_risk_low_when_healthy(self, optimizer):
        risk = optimizer._estimate_risk(0.7, scar_count=0, budget=100.0)
        assert risk < 0.3

    def test_risk_high_with_scars_and_low_budget(self, optimizer):
        risk = optimizer._estimate_risk(0.2, scar_count=5, budget=5.0)
        assert risk > 0.5

    def test_expected_delta_diminishing(self, optimizer):
        delta_low = optimizer._expected_delta(0.1, scar_count=0)
        delta_high = optimizer._expected_delta(0.9, scar_count=0)
        assert delta_low > delta_high


# =====================================================================
# Proposal Generation Tests
# =====================================================================


class TestProposalGeneration:
    """Test generating proposals from candidates."""

    def test_generate_proposal(self, optimizer):
        candidate = ImprovementCandidate(
            capability="trading",
            domain="MARKET",
            current_proficiency=0.3,
            expected_roi=1.5,
            risk_score=0.2,
            causal_blame=0.7,
            scar_count=1,
        )
        proposal = optimizer.generate_proposal(candidate, agent_id="a1")
        assert proposal.agent_id == "a1"
        assert proposal.target_capability == "trading"
        assert proposal.target_domain == "MARKET"
        assert proposal.expected_proficiency_delta > 0
        assert "ROI=" in proposal.description

    def test_generate_different_types(self, optimizer):
        candidate = ImprovementCandidate(
            capability="debug",
            domain="CODE",
            current_proficiency=0.5,
            expected_roi=0.8,
            risk_score=0.3,
            causal_blame=0.0,
            scar_count=0,
        )
        proposal = optimizer.generate_proposal(
            candidate, agent_id="a1",
            improvement_type=ImprovementType.STRATEGY_SHIFT,
        )
        assert proposal.improvement_type == ImprovementType.STRATEGY_SHIFT.value


# =====================================================================
# Retrospective Learning Tests
# =====================================================================


class TestRetrospectiveLearning:
    """Test outcome recording and success rate."""

    def test_record_outcome(self, optimizer):
        result = optimizer.learn_from_outcome(
            agent_id="a1",
            capability="trading",
            success=True,
            cost=5.0,
            proficiency_delta=0.1,
        )
        assert result["success"] is True
        assert result["roi_actual"] == pytest.approx(0.02, abs=0.001)

    def test_success_rate_neutral_for_new_agent(self, optimizer):
        rate = optimizer.historical_success_rate("new-agent")
        assert rate == 0.5

    def test_success_rate_computed(self, optimizer):
        optimizer.learn_from_outcome("a1", "cap", True, 1.0, 0.1)
        optimizer.learn_from_outcome("a1", "cap", True, 1.0, 0.1)
        optimizer.learn_from_outcome("a1", "cap", False, 1.0, 0.0)

        rate = optimizer.historical_success_rate("a1")
        assert rate == pytest.approx(2.0 / 3.0, abs=0.01)

    def test_success_rate_by_capability(self, optimizer):
        optimizer.learn_from_outcome("a1", "trading", True, 1.0, 0.1)
        optimizer.learn_from_outcome("a1", "coding", False, 1.0, 0.0)

        trade_rate = optimizer.historical_success_rate("a1", "trading")
        code_rate = optimizer.historical_success_rate("a1", "coding")
        assert trade_rate == 1.0
        assert code_rate == 0.0


# =====================================================================
# Domain Inference Tests
# =====================================================================


class TestDomainInference:
    """Test heuristic domain mapping."""

    def test_market_capabilities(self, optimizer):
        assert optimizer._infer_domain("trading") == "MARKET"
        assert optimizer._infer_domain("price_analysis") == "MARKET"
        assert optimizer._infer_domain("arb_detection") == "MARKET"

    def test_code_capabilities(self, optimizer):
        assert optimizer._infer_domain("code_review") == "CODE"
        assert optimizer._infer_domain("debug_skill") == "CODE"

    def test_governance_capabilities(self, optimizer):
        assert optimizer._infer_domain("governance_voting") == "GOVERNANCE"

    def test_infra_capabilities(self, optimizer):
        assert optimizer._infer_domain("node_management") == "INFRASTRUCTURE"
        assert optimizer._infer_domain("deploy_pipeline") == "INFRASTRUCTURE"

    def test_unknown_defaults_cross_domain(self, optimizer):
        assert optimizer._infer_domain("general_skill") == "CROSS_DOMAIN"
