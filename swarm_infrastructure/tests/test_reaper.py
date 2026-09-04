"""Tests for the Reaper engine, postmortem analyzer, and scar propagation.

Invariant refs: tests/invariants/registry.toml — DEATH_PERMANENT, SCARS_NEVER_HEAL
"""

from __future__ import annotations

import pytest

from swarm.reaper.engine import ReaperEngine
from swarm.reaper.postmortem import PostmortemAnalyzer, Postmortem, Lesson
from swarm.reaper.scar_mechanics import ScarPropagator
from swarm.reaper.schema import (
    DeathCause,
    DeathLevel,
    KillDecision,
    MortalitySignal,
    ReaperConfig,
)
from swarm.self_improve.scars import ScarRegistry
from swarm.storage.backend import SqliteStorage


@pytest.fixture
def storage():
    return SqliteStorage(":memory:")


@pytest.fixture
def reaper(storage):
    config = ReaperConfig(evaluation_cooldown=0.0)  # No cooldown for tests
    return ReaperEngine(storage=storage, config=config)


@pytest.fixture
def postmortem_analyzer(storage):
    return PostmortemAnalyzer(storage=storage)


@pytest.fixture
def scar_propagator(storage):
    return ScarPropagator(storage=storage)


# =====================================================================
# ReaperEngine tests
# =====================================================================


class TestReaperEngine:
    """Test the core reaper death evaluation logic."""

    def test_healthy_agent_not_killed(self, reaper):
        """Healthy agent survives reaper evaluation."""
        decision = reaper.evaluate(
            agent_id="agent-1",
            resource_budget=500.0,
            survival_probability=0.8,
            prediction_accuracy=0.6,
        )
        assert not decision.should_kill
        assert decision.agent_id == "agent-1"

    def test_resource_exhaustion_kills(self, reaper):
        """Agent with zero budget is killed."""
        decision = reaper.evaluate(
            agent_id="agent-broke",
            resource_budget=0.0,
            survival_probability=0.5,
            prediction_accuracy=0.5,
        )
        assert decision.should_kill
        assert decision.cause == DeathCause.RESOURCE_EXHAUSTION
        assert decision.death_level in (DeathLevel.HARD, DeathLevel.CAUSAL)

    def test_tripwire_halt_causal_death(self, reaper):
        """Tripwire HALT triggers Level 3 causal death."""
        decision = reaper.evaluate(
            agent_id="agent-rogue",
            resource_budget=100.0,
            survival_probability=0.5,
            prediction_accuracy=0.5,
            tripwire_halt=True,
            active_mandates=["trade_sol", "arb_usdc"],
        )
        assert decision.should_kill
        assert decision.cause == DeathCause.TRIPWIRE_HALT
        assert decision.death_level == DeathLevel.CAUSAL
        assert len(decision.scorched_mandates) > 0

    def test_fitness_collapse_kills(self, reaper):
        """Agent with consistently low fitness is killed."""
        decision = reaper.evaluate(
            agent_id="agent-unfit",
            resource_budget=100.0,
            survival_probability=0.5,
            prediction_accuracy=0.5,
            fitness_scores=[0.05, 0.08, 0.03],
        )
        assert decision.should_kill
        assert decision.cause == DeathCause.FITNESS_COLLAPSE

    def test_prediction_failure_kills(self, reaper):
        """Agent with terrible prediction accuracy is killed."""
        decision = reaper.evaluate(
            agent_id="agent-blind",
            resource_budget=100.0,
            survival_probability=0.5,
            prediction_accuracy=0.05,
        )
        assert decision.should_kill
        assert decision.cause == DeathCause.PREDICTION_FAILURE

    def test_survival_collapse_kills(self, reaper):
        """Agent with near-zero survival probability is killed."""
        decision = reaper.evaluate(
            agent_id="agent-doomed",
            resource_budget=100.0,
            survival_probability=0.01,
            prediction_accuracy=0.5,
        )
        assert decision.should_kill

    def test_jury_verdict_causal_death(self, reaper):
        """Jury kill verdict triggers Level 3 causal death."""
        decision = reaper.evaluate(
            agent_id="agent-convicted",
            resource_budget=100.0,
            survival_probability=0.5,
            prediction_accuracy=0.5,
            jury_verdict_kill=True,
            active_mandates=["governance_hack"],
        )
        assert decision.should_kill
        assert decision.cause == DeathCause.JURY_VERDICT
        assert decision.death_level == DeathLevel.CAUSAL

    def test_execute_kill_scorches_mandates(self, reaper):
        """Level 3 kill scorches mandate space permanently."""
        decision = KillDecision(
            agent_id="agent-x",
            should_kill=True,
            death_level=DeathLevel.CAUSAL,
            cause=DeathCause.TRIPWIRE_HALT,
            scorched_mandates=["toxic_mandate_1", "toxic_mandate_2"],
            reason="Tripwire HALT",
        )
        reaper.execute_kill(decision)

        assert reaper.is_mandate_scorched("toxic_mandate_1")
        assert reaper.is_mandate_scorched("toxic_mandate_2")
        assert not reaper.is_mandate_scorched("safe_mandate")

    def test_execute_kill_emits_death_event(self, reaper):
        """Kill execution produces a death bus event."""
        decision = KillDecision(
            agent_id="agent-y",
            should_kill=True,
            death_level=DeathLevel.HARD,
            cause=DeathCause.RESOURCE_EXHAUSTION,
            reason="Budget = 0",
        )
        reaper.execute_kill(decision)

        events = reaper.get_pending_bus_events()
        assert len(events) == 1
        assert events[0].event_type == "AGENT_DEATH"
        assert events[0].agent_id == "agent-y"

    def test_cooldown_prevents_rapid_eval(self, storage):
        """Evaluation cooldown prevents rapid re-evaluation."""
        config = ReaperConfig(evaluation_cooldown=3600.0)
        reaper = ReaperEngine(storage=storage, config=config)

        d1 = reaper.evaluate(
            agent_id="agent-1",
            resource_budget=0.0,
            survival_probability=0.0,
            prediction_accuracy=0.0,
        )
        assert d1.should_kill  # First eval should kill

        d2 = reaper.evaluate(
            agent_id="agent-1",
            resource_budget=0.0,
            survival_probability=0.0,
            prediction_accuracy=0.0,
        )
        # Second eval should be blocked by cooldown
        assert not d2.should_kill
        assert "cooldown" in d2.reason.lower()

    def test_soft_kill_no_scorched_mandates(self, reaper):
        """Level 1 soft kill does not scorch mandates."""
        decision = KillDecision(
            agent_id="agent-soft",
            should_kill=True,
            death_level=DeathLevel.SOFT,
            cause=DeathCause.MANUAL,
            scorched_mandates=[],
            reason="Manual soft kill",
        )
        reaper.execute_kill(decision)
        assert len(reaper.scorched_mandates) == 0


# =====================================================================
# PostmortemAnalyzer tests
# =====================================================================


class TestPostmortemAnalyzer:
    """Test postmortem analysis and lesson extraction."""

    def test_basic_postmortem(self, postmortem_analyzer):
        """Postmortem is generated with lessons."""
        decision = KillDecision(
            agent_id="agent-dead",
            should_kill=True,
            death_level=DeathLevel.HARD,
            cause=DeathCause.RESOURCE_EXHAUSTION,
            contributing_signals=[
                MortalitySignal(
                    source_layer="SELF_MODEL",
                    signal_type="RESOURCE_EXHAUSTION",
                    severity=1.0,
                )
            ],
            reason="Budget = 0",
        )
        pm = postmortem_analyzer.analyze(
            decision, final_budget=0.0, epoch=5
        )
        assert pm.agent_id == "agent-dead"
        assert pm.cause == DeathCause.RESOURCE_EXHAUSTION
        assert len(pm.lessons) >= 1
        assert pm.epoch_of_death == 5

    def test_causal_death_postmortem_records_scorched(self, postmortem_analyzer):
        """Causal death postmortem records scorched mandates."""
        decision = KillDecision(
            agent_id="agent-scorched",
            should_kill=True,
            death_level=DeathLevel.CAUSAL,
            cause=DeathCause.TRIPWIRE_HALT,
            scorched_mandates=["mandate_a", "mandate_b"],
            reason="Tripwire HALT",
        )
        pm = postmortem_analyzer.analyze(decision)
        assert "mandate_a" in pm.scorched_mandates
        assert "mandate_b" in pm.scorched_mandates

    def test_similar_risk_agents_identified(self, postmortem_analyzer):
        """Postmortem identifies surviving agents at similar risk."""
        decision = KillDecision(
            agent_id="dead-1",
            should_kill=True,
            death_level=DeathLevel.HARD,
            cause=DeathCause.RESOURCE_EXHAUSTION,
            reason="Budget exhausted",
        )
        pm = postmortem_analyzer.analyze(
            decision,
            final_budget=0.0,
            active_agent_ids=["survivor-1", "survivor-2"],
            agent_budgets={
                "survivor-1": 5.0,  # Low budget — at risk
                "survivor-2": 500.0,  # Healthy
            },
        )
        assert "survivor-1" in pm.similar_risk_agents

    def test_lessons_by_cause(self, postmortem_analyzer):
        """Each death cause produces a relevant lesson."""
        causes = [
            DeathCause.RESOURCE_EXHAUSTION,
            DeathCause.FITNESS_COLLAPSE,
            DeathCause.PREDICTION_FAILURE,
            DeathCause.TRIPWIRE_HALT,
            DeathCause.JURY_VERDICT,
            DeathCause.SELF_INFLICTED,
        ]
        for cause in causes:
            decision = KillDecision(
                agent_id=f"agent-{cause.value}",
                should_kill=True,
                death_level=DeathLevel.HARD,
                cause=cause,
                reason=f"Died from {cause.value}",
            )
            pm = postmortem_analyzer.analyze(decision)
            assert len(pm.lessons) >= 1, f"No lessons for {cause.value}"


# =====================================================================
# ScarPropagator tests
# =====================================================================


class TestScarPropagator:
    """Test scar propagation across death levels."""

    def test_soft_kill_no_propagation(self, scar_propagator, storage):
        """Level 1 soft kill does not propagate scars."""
        registry = ScarRegistry(storage, "survivor-1")
        decision = KillDecision(
            agent_id="dead",
            should_kill=True,
            death_level=DeathLevel.SOFT,
            cause=DeathCause.MANUAL,
        )
        count = scar_propagator.propagate(
            decision,
            survivor_registries={"survivor-1": registry},
            dead_agent_domains=["CODE"],
        )
        assert count == 0
        assert len(registry.get_all()) == 0

    def test_hard_kill_domain_propagation(self, scar_propagator, storage):
        """Level 2 hard kill propagates scars to domain survivors."""
        reg1 = ScarRegistry(storage, "survivor-1")
        reg2 = ScarRegistry(storage, "survivor-2")

        decision = KillDecision(
            agent_id="dead",
            should_kill=True,
            death_level=DeathLevel.HARD,
            cause=DeathCause.FITNESS_COLLAPSE,
        )
        count = scar_propagator.propagate(
            decision,
            survivor_registries={"survivor-1": reg1, "survivor-2": reg2},
            dead_agent_domains=["CODE"],
        )
        assert count == 2  # Both survivors get a scar
        assert len(reg1.get_all()) == 1
        assert len(reg2.get_all()) == 1

    def test_causal_kill_all_propagation(self, scar_propagator, storage):
        """Level 3 causal death propagates to ALL survivors."""
        reg1 = ScarRegistry(storage, "s1")
        reg2 = ScarRegistry(storage, "s2")
        reg3 = ScarRegistry(storage, "s3")

        decision = KillDecision(
            agent_id="dead",
            should_kill=True,
            death_level=DeathLevel.CAUSAL,
            cause=DeathCause.TRIPWIRE_HALT,
            scorched_mandates=["toxic"],
        )
        count = scar_propagator.propagate(
            decision,
            survivor_registries={"s1": reg1, "s2": reg2, "s3": reg3},
            dead_agent_domains=["MARKET"],
        )
        assert count == 3
        for reg in [reg1, reg2, reg3]:
            scars = reg.get_all()
            assert len(scars) == 1
            assert "causal" in scars[0].failure_reason.lower()

    def test_dead_agent_excluded_from_propagation(
        self, scar_propagator, storage
    ):
        """Dead agent's own registry is not scarred during propagation."""
        dead_reg = ScarRegistry(storage, "dead")
        survivor_reg = ScarRegistry(storage, "survivor")

        decision = KillDecision(
            agent_id="dead",
            should_kill=True,
            death_level=DeathLevel.HARD,
            cause=DeathCause.RESOURCE_EXHAUSTION,
        )
        scar_propagator.propagate(
            decision,
            survivor_registries={
                "dead": dead_reg,
                "survivor": survivor_reg,
            },
            dead_agent_domains=["CODE"],
        )
        assert len(dead_reg.get_all()) == 0
        assert len(survivor_reg.get_all()) == 1
