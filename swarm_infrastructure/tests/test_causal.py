"""Tests for the Causal Analysis Layer.

Invariant refs: tests/invariants/registry.toml — CAUSAL_DAG_ACYCLIC, ATTRIBUTION_SUM_BOUNDED

Tests cover:
- CausalGraph: node/edge CRUD, DAG invariants, traversal, chain extraction
- AttributionEngine: path_weight, shapley, recency methods
- CounterfactualEngine: causal_removal, marginal_impact, death analysis
- Lifecycle integration: causal nodes created during epoch loop
"""

from __future__ import annotations

import pytest

from swarm.causal.graph import CausalGraph
from swarm.causal.schema import (
    CausalChain,
    CausalEdge,
    CausalNode,
    EdgeType,
    NodeType,
    AttributionScore,
    Counterfactual,
)
from swarm.causal.attribution import AttributionEngine
from swarm.causal.counterfactual import CounterfactualEngine
from swarm.storage.backend import SqliteStorage


# =====================================================================
# Fixtures
# =====================================================================


@pytest.fixture
def storage():
    return SqliteStorage(":memory:")


@pytest.fixture
def graph(storage):
    return CausalGraph(storage=storage)


@pytest.fixture
def attribution(graph):
    return AttributionEngine(graph)


@pytest.fixture
def counterfactual(graph, attribution):
    return CounterfactualEngine(graph, attribution)


def _make_node(agent_id="agent-1", epoch=0, node_type=NodeType.ACTION,
               action_type="test", value=0.0, **kwargs) -> CausalNode:
    """Helper to create a CausalNode."""
    return CausalNode(
        agent_id=agent_id,
        epoch=epoch,
        node_type=node_type,
        action_type=action_type,
        value=value,
        **kwargs,
    )


# =====================================================================
# CausalGraph tests
# =====================================================================

class TestCausalGraph:

    def test_add_node(self, graph):
        """Nodes are added and retrievable."""
        node = _make_node()
        result = graph.add_node(node)
        assert result.node_id == node.node_id
        assert graph.node_count == 1

    def test_add_duplicate_node(self, graph):
        """Adding the same node twice returns the existing one."""
        node = _make_node()
        graph.add_node(node)
        result = graph.add_node(node)
        assert result.node_id == node.node_id
        assert graph.node_count == 1

    def test_add_edge(self, graph):
        """Edges connect two existing nodes."""
        n1 = graph.add_node(_make_node(action_type="a1", epoch=0))
        n2 = graph.add_node(_make_node(action_type="a2", epoch=1))
        edge = graph.add_edge(n1.node_id, n2.node_id)
        assert edge.cause_node_id == n1.node_id
        assert edge.effect_node_id == n2.node_id
        assert graph.edge_count == 1

    def test_edge_missing_node_raises(self, graph):
        """Adding an edge with a missing node raises ValueError."""
        n1 = graph.add_node(_make_node())
        with pytest.raises(ValueError, match="not found"):
            graph.add_edge(n1.node_id, "nonexistent")

    def test_cycle_detection(self, graph):
        """Adding an edge that creates a cycle raises ValueError."""
        n1 = graph.add_node(_make_node(action_type="a1", epoch=0))
        n2 = graph.add_node(_make_node(action_type="a2", epoch=1))
        n3 = graph.add_node(_make_node(action_type="a3", epoch=2))
        graph.add_edge(n1.node_id, n2.node_id)
        graph.add_edge(n2.node_id, n3.node_id)
        with pytest.raises(ValueError, match="cycle"):
            graph.add_edge(n3.node_id, n1.node_id)

    def test_duplicate_edge_returns_existing(self, graph):
        """Adding the same edge twice returns the existing edge."""
        n1 = graph.add_node(_make_node(action_type="a1"))
        n2 = graph.add_node(_make_node(action_type="a2"))
        e1 = graph.add_edge(n1.node_id, n2.node_id)
        e2 = graph.add_edge(n1.node_id, n2.node_id)
        assert e1.edge_id == e2.edge_id
        assert graph.edge_count == 1

    def test_get_nodes_for_agent(self, graph):
        """Nodes are indexed by agent_id."""
        graph.add_node(_make_node(agent_id="a1", epoch=0))
        graph.add_node(_make_node(agent_id="a1", epoch=1))
        graph.add_node(_make_node(agent_id="a2", epoch=0))
        assert len(graph.get_nodes_for_agent("a1")) == 2
        assert len(graph.get_nodes_for_agent("a2")) == 1

    def test_get_nodes_for_epoch(self, graph):
        """Nodes are indexed by epoch."""
        graph.add_node(_make_node(agent_id="a1", epoch=0))
        graph.add_node(_make_node(agent_id="a2", epoch=0))
        graph.add_node(_make_node(agent_id="a1", epoch=1))
        assert len(graph.get_nodes_for_epoch(0)) == 2
        assert len(graph.get_nodes_for_epoch(1)) == 1

    def test_get_nodes_by_type(self, graph):
        """Nodes are filterable by type."""
        graph.add_node(_make_node(node_type=NodeType.ACTION))
        graph.add_node(_make_node(node_type=NodeType.CONSEQUENCE))
        graph.add_node(_make_node(node_type=NodeType.ACTION))
        assert len(graph.get_nodes_by_type(NodeType.ACTION)) == 2
        assert len(graph.get_nodes_by_type(NodeType.CONSEQUENCE)) == 1

    def test_direct_causes_and_effects(self, graph):
        """Direct cause/effect traversal works."""
        n1 = graph.add_node(_make_node(action_type="cause", epoch=0))
        n2 = graph.add_node(_make_node(action_type="effect", epoch=1))
        graph.add_edge(n1.node_id, n2.node_id)
        assert graph.direct_causes(n2.node_id)[0].node_id == n1.node_id
        assert graph.direct_effects(n1.node_id)[0].node_id == n2.node_id

    def test_ancestors(self, graph):
        """Ancestors traverses the full DAG backwards."""
        n1 = graph.add_node(_make_node(action_type="root", epoch=0))
        n2 = graph.add_node(_make_node(action_type="mid", epoch=1))
        n3 = graph.add_node(_make_node(action_type="leaf", epoch=2))
        graph.add_edge(n1.node_id, n2.node_id)
        graph.add_edge(n2.node_id, n3.node_id)
        ancestors = graph.ancestors(n3.node_id)
        anc_ids = {a.node_id for a in ancestors}
        assert n1.node_id in anc_ids
        assert n2.node_id in anc_ids

    def test_descendants(self, graph):
        """Descendants traverses the full DAG forward."""
        n1 = graph.add_node(_make_node(action_type="root", epoch=0))
        n2 = graph.add_node(_make_node(action_type="mid", epoch=1))
        n3 = graph.add_node(_make_node(action_type="leaf", epoch=2))
        graph.add_edge(n1.node_id, n2.node_id)
        graph.add_edge(n2.node_id, n3.node_id)
        descs = graph.descendants(n1.node_id)
        desc_ids = {d.node_id for d in descs}
        assert n2.node_id in desc_ids
        assert n3.node_id in desc_ids

    def test_roots_and_leaves(self, graph):
        """Root and leaf identification works."""
        n1 = graph.add_node(_make_node(action_type="root", epoch=0))
        n2 = graph.add_node(_make_node(action_type="mid", epoch=1))
        n3 = graph.add_node(_make_node(action_type="leaf", epoch=2))
        graph.add_edge(n1.node_id, n2.node_id)
        graph.add_edge(n2.node_id, n3.node_id)
        roots = graph.roots()
        leaves = graph.leaves()
        assert n1.node_id in [r.node_id for r in roots]
        assert n3.node_id in [l.node_id for l in leaves]

    def test_get_chain_to(self, graph):
        """Chain extraction follows highest-weight path."""
        n1 = graph.add_node(_make_node(action_type="root", epoch=0))
        n2 = graph.add_node(_make_node(action_type="mid", epoch=1))
        n3 = graph.add_node(_make_node(action_type="leaf", epoch=2))
        graph.add_edge(n1.node_id, n2.node_id, weight=0.8)
        graph.add_edge(n2.node_id, n3.node_id, weight=0.9)
        chain = graph.get_chain_to(n3.node_id)
        assert chain.depth == 2
        assert len(chain.nodes) == 3
        # Root first, leaf last
        assert chain.nodes[0].node_id == n1.node_id
        assert chain.nodes[-1].node_id == n3.node_id
        assert abs(chain.total_weight - 0.8 * 0.9) < 0.01

    def test_node_influence(self, graph):
        """Node influence measures downstream impact."""
        n1 = graph.add_node(_make_node(action_type="root", epoch=0))
        n2 = graph.add_node(_make_node(action_type="mid1", epoch=1))
        n3 = graph.add_node(_make_node(action_type="mid2", epoch=1))
        n4 = graph.add_node(_make_node(action_type="leaf", epoch=2))
        graph.add_edge(n1.node_id, n2.node_id, weight=1.0)
        graph.add_edge(n1.node_id, n3.node_id, weight=1.0)
        graph.add_edge(n2.node_id, n4.node_id, weight=1.0)
        # n1 has 3 descendants, n2 has 1
        assert graph.node_influence(n1.node_id) > graph.node_influence(n2.node_id)

    def test_subgraph_by_agent(self, graph):
        """Subgraph extraction filters by agent."""
        graph.add_node(_make_node(agent_id="a1", epoch=0))
        graph.add_node(_make_node(agent_id="a2", epoch=0))
        sub = graph.subgraph(agent_id="a1")
        assert sub.node_count == 1

    def test_subgraph_by_epoch(self, graph):
        """Subgraph extraction filters by epoch range."""
        graph.add_node(_make_node(epoch=0))
        graph.add_node(_make_node(epoch=1))
        graph.add_node(_make_node(epoch=2))
        sub = graph.subgraph(epoch_start=1, epoch_end=1)
        assert sub.node_count == 1

    def test_persistence_rebuild(self, storage):
        """Graph survives rebuild from storage."""
        g1 = CausalGraph(storage=storage)
        n1 = g1.add_node(_make_node(action_type="persist1", epoch=0))
        n2 = g1.add_node(_make_node(action_type="persist2", epoch=1))
        g1.add_edge(n1.node_id, n2.node_id)

        # Rebuild from same storage
        g2 = CausalGraph(storage=storage)
        assert g2.node_count == 2
        assert g2.edge_count == 1
        assert g2.get_node(n1.node_id) is not None


# =====================================================================
# Attribution Engine tests
# =====================================================================

class TestAttributionEngine:

    def _build_diamond_graph(self, graph):
        """Build A → B, A → C, B → D, C → D (diamond shape)."""
        a = graph.add_node(_make_node(action_type="A", epoch=0, value=1.0))
        b = graph.add_node(_make_node(action_type="B", epoch=1, value=2.0))
        c = graph.add_node(_make_node(action_type="C", epoch=1, value=0.5))
        d = graph.add_node(_make_node(action_type="D", epoch=2, value=5.0,
                                      node_type=NodeType.CONSEQUENCE))
        graph.add_edge(a.node_id, b.node_id, weight=0.8)
        graph.add_edge(a.node_id, c.node_id, weight=0.6)
        graph.add_edge(b.node_id, d.node_id, weight=0.9)
        graph.add_edge(c.node_id, d.node_id, weight=0.4)
        return a, b, c, d

    def test_path_weight_attribution(self, graph, attribution):
        """Path-weight produces attribution scores that sum to <= 1.0."""
        _, b, c, d = self._build_diamond_graph(graph)
        scores = attribution.attribute(d.node_id, method="path_weight")
        assert len(scores) > 0
        total_share = sum(s.attribution_share for s in scores)
        assert total_share <= 1.01  # Allow rounding

    def test_shapley_attribution(self, graph, attribution):
        """Shapley produces attribution for direct causes."""
        _, b, c, d = self._build_diamond_graph(graph)
        scores = attribution.attribute(d.node_id, method="shapley")
        assert len(scores) >= 2  # B and C are direct causes
        cause_ids = {s.cause_node_id for s in scores}
        assert b.node_id in cause_ids
        assert c.node_id in cause_ids

    def test_recency_attribution(self, graph, attribution):
        """Recency favors recent causes over distant ones."""
        n0 = graph.add_node(_make_node(action_type="old", epoch=0, value=1.0))
        n1 = graph.add_node(_make_node(action_type="recent", epoch=5, value=1.0))
        n2 = graph.add_node(_make_node(action_type="target", epoch=6, value=3.0))
        graph.add_edge(n0.node_id, n1.node_id, weight=0.5)
        graph.add_edge(n1.node_id, n2.node_id, weight=0.9)
        scores = attribution.attribute(n2.node_id, method="recency")
        if len(scores) >= 2:
            # The recent node should have higher attribution
            by_id = {s.cause_node_id: s for s in scores}
            if n0.node_id in by_id and n1.node_id in by_id:
                assert by_id[n1.node_id].attribution_share >= by_id[n0.node_id].attribution_share

    def test_top_contributors(self, graph, attribution):
        """top_contributors returns at most N results."""
        _, _, _, d = self._build_diamond_graph(graph)
        top = attribution.top_contributors(d.node_id, n=2)
        assert len(top) <= 2

    def test_blame_chain_filters_negative(self, graph, attribution):
        """blame_chain only returns negative-valued or negative-type nodes."""
        a = graph.add_node(_make_node(action_type="good", epoch=0, value=5.0))
        b = graph.add_node(_make_node(action_type="bad", epoch=1, value=-3.0,
                                      node_type=NodeType.SCAR))
        c = graph.add_node(_make_node(action_type="death", epoch=2, value=0.0,
                                      node_type=NodeType.DEATH))
        graph.add_edge(a.node_id, b.node_id, weight=0.9)
        graph.add_edge(b.node_id, c.node_id, weight=1.0)
        blame = attribution.blame_chain(c.node_id)
        blame_ids = {s.cause_node_id for s in blame}
        # 'b' is a SCAR node — should appear in blame
        assert b.node_id in blame_ids

    def test_credit_chain_filters_positive(self, graph, attribution):
        """credit_chain only returns positive-valued nodes."""
        a = graph.add_node(_make_node(action_type="win", epoch=0, value=10.0))
        b = graph.add_node(_make_node(action_type="loss", epoch=1, value=-2.0))
        c = graph.add_node(_make_node(action_type="outcome", epoch=2, value=5.0))
        graph.add_edge(a.node_id, c.node_id, weight=0.9)
        graph.add_edge(b.node_id, c.node_id, weight=0.3)
        credit = attribution.credit_chain(c.node_id)
        credit_ids = {s.cause_node_id for s in credit}
        assert a.node_id in credit_ids
        # Negative node should NOT be in credit
        assert b.node_id not in credit_ids

    def test_no_ancestors_returns_empty(self, graph, attribution):
        """Attribution on a root node returns empty list."""
        n = graph.add_node(_make_node())
        scores = attribution.attribute(n.node_id)
        assert scores == []


# =====================================================================
# Counterfactual Engine tests
# =====================================================================

class TestCounterfactualEngine:

    def _build_chain(self, graph):
        """Build A → B → C (linear chain)."""
        a = graph.add_node(_make_node(action_type="A", epoch=0, value=1.0))
        b = graph.add_node(_make_node(action_type="B", epoch=1, value=2.0))
        c = graph.add_node(_make_node(action_type="C", epoch=2, value=5.0))
        graph.add_edge(a.node_id, b.node_id, weight=0.8)
        graph.add_edge(b.node_id, c.node_id, weight=0.9)
        return a, b, c

    def test_causal_removal_single_path(self, graph, counterfactual):
        """Removing a node on the only path should have high impact."""
        a, b, c = self._build_chain(graph)
        cf = counterfactual.what_if_removed(
            a.node_id, c.node_id, method="causal_removal"
        )
        assert cf.baseline_value == 5.0
        # With only one path through A, removing A should have big impact
        assert abs(cf.estimated_outcome_delta) > 0

    def test_causal_removal_with_alternative(self, graph, counterfactual):
        """Removing one of multiple paths should have less impact."""
        a = graph.add_node(_make_node(action_type="A", epoch=0, value=1.0))
        b = graph.add_node(_make_node(action_type="B", epoch=0, value=1.0))
        c = graph.add_node(_make_node(action_type="C", epoch=1, value=5.0))
        graph.add_edge(a.node_id, c.node_id, weight=0.5)
        graph.add_edge(b.node_id, c.node_id, weight=0.5)
        cf = counterfactual.what_if_removed(
            a.node_id, c.node_id, method="causal_removal"
        )
        # With an alternative path, impact should be partial
        assert abs(cf.estimated_outcome_delta) < abs(cf.baseline_value)

    def test_marginal_impact(self, graph, counterfactual):
        """Marginal impact uses attribution scores."""
        a, b, c = self._build_chain(graph)
        cf = counterfactual.what_if_removed(
            b.node_id, c.node_id, method="marginal_impact"
        )
        assert cf.method == "marginal_impact"
        assert cf.baseline_value == 5.0

    def test_was_death_avoidable(self, graph, counterfactual):
        """Death avoidability analysis returns counterfactuals."""
        a = graph.add_node(_make_node(
            action_type="risky_action", epoch=0, value=-5.0,
            node_type=NodeType.ACTION,
        ))
        scar = graph.add_node(_make_node(
            action_type="scar", epoch=1, value=-2.0,
            node_type=NodeType.SCAR,
        ))
        death = graph.add_node(_make_node(
            action_type="death", epoch=2, value=0.0,
            node_type=NodeType.DEATH,
        ))
        graph.add_edge(a.node_id, scar.node_id, weight=0.9)
        graph.add_edge(scar.node_id, death.node_id, weight=1.0)
        results = counterfactual.was_death_avoidable(death.node_id, top_n=3)
        assert isinstance(results, list)
        # Should return at least one counterfactual
        assert len(results) >= 1

    def test_retrospective_analysis(self, graph, counterfactual):
        """Retrospective analyzes all actions for impact on a target."""
        a1 = graph.add_node(_make_node(
            agent_id="x", action_type="action1", epoch=0, value=2.0,
            node_type=NodeType.ACTION,
        ))
        a2 = graph.add_node(_make_node(
            agent_id="x", action_type="action2", epoch=1, value=3.0,
            node_type=NodeType.ACTION,
        ))
        outcome = graph.add_node(_make_node(
            agent_id="x", action_type="outcome", epoch=2, value=10.0,
            node_type=NodeType.CONSEQUENCE,
        ))
        graph.add_edge(a1.node_id, a2.node_id, weight=0.5)
        graph.add_edge(a2.node_id, outcome.node_id, weight=0.9)
        results = counterfactual.retrospective_analysis("x", outcome.node_id)
        assert isinstance(results, dict)

    def test_find_toxic_patterns(self, graph, counterfactual):
        """Toxic patterns are actions that lead to multiple negative outcomes."""
        toxic = graph.add_node(_make_node(
            agent_id="x", action_type="toxic_action", epoch=0,
            node_type=NodeType.ACTION,
        ))
        scar1 = graph.add_node(_make_node(
            agent_id="x", action_type="scar1", epoch=1,
            node_type=NodeType.SCAR,
        ))
        scar2 = graph.add_node(_make_node(
            agent_id="x", action_type="scar2", epoch=2,
            node_type=NodeType.SCAR,
        ))
        graph.add_edge(toxic.node_id, scar1.node_id, weight=0.9)
        graph.add_edge(toxic.node_id, scar2.node_id, weight=0.8)
        result = counterfactual.find_toxic_patterns("x")
        assert isinstance(result, list)
        # The toxic action appears in 2 negative outcomes
        if result:
            toxic_ids = {n.node_id for n in result}
            assert toxic.node_id in toxic_ids

    def test_no_path_returns_zero_delta(self, graph, counterfactual):
        """Counterfactual with no causal path returns zero delta."""
        n1 = graph.add_node(_make_node(action_type="isolated1", epoch=0))
        n2 = graph.add_node(_make_node(action_type="isolated2", epoch=1))
        # No edge between them
        cf = counterfactual.what_if_removed(
            n1.node_id, n2.node_id, method="causal_removal"
        )
        assert cf.estimated_outcome_delta == 0.0


# =====================================================================
# Lifecycle integration tests
# =====================================================================

class TestCausalLifecycleIntegration:

    @pytest.fixture
    def full_stack(self):
        """Create a full orchestrator with causal graph."""
        from swarm.core.agent import AgentConfig, Consequence
        from swarm.core.enums import Domain
        from swarm.core.lifecycle import EpochOrchestrator
        from swarm.event_bus.bus import AsyncEventBus
        from swarm.reaper import PostmortemAnalyzer, ReaperEngine, ScarPropagator
        from swarm.reaper.schema import ReaperConfig
        from swarm.tripwire.detector import TripwireDetector
        from swarm.world_sim.prediction import PredictionMarket
        from swarm.world_sim.scoreboard import AccuracyScoreboard
        from swarm.world_sim.state_graph import WorldStateGraph

        storage = SqliteStorage(":memory:")
        causal = CausalGraph(storage=storage)
        bus = AsyncEventBus(log_dir=None)
        world = WorldStateGraph(storage=storage)
        prediction = PredictionMarket(storage=storage)
        scoreboard = AccuracyScoreboard(storage=storage)
        reaper_cfg = ReaperConfig(evaluation_cooldown=0.0)
        reaper = ReaperEngine(storage=storage, config=reaper_cfg)
        postmortem = PostmortemAnalyzer(storage=storage)
        scar_prop = ScarPropagator(storage=storage)
        tripwire = TripwireDetector(storage=storage)

        def reward_fn(agent, action):
            return [Consequence("REWARD", 1.0, "TEST_ENV")]

        orch = EpochOrchestrator(
            storage=storage,
            event_bus=bus,
            world_state=world,
            prediction_market=prediction,
            scoreboard=scoreboard,
            reaper=reaper,
            postmortem_analyzer=postmortem,
            scar_propagator=scar_prop,
            tripwire=tripwire,
            causal_graph=causal,
            consequence_fn=reward_fn,
        )

        config = AgentConfig(agent_id="test-a1", initial_budget=5000.0, domain=Domain.CODE)
        agent = orch.spawn_agent(config)
        agent.add_goal("build stuff", Domain.CODE)

        return orch, causal, agent

    @pytest.mark.asyncio
    async def test_epoch_creates_causal_nodes(self, full_stack):
        """Running an epoch populates the causal graph with action + consequence nodes."""
        orch, causal, agent = full_stack
        assert causal.node_count == 0
        await orch.run_epoch()
        # Should have at least: ACTION + CONSEQUENCE nodes
        assert causal.node_count >= 2
        actions = causal.get_nodes_by_type(NodeType.ACTION)
        assert len(actions) >= 1
        consequences = causal.get_nodes_by_type(NodeType.CONSEQUENCE)
        assert len(consequences) >= 1

    @pytest.mark.asyncio
    async def test_epoch_creates_causal_edges(self, full_stack):
        """Running an epoch creates edges between action and consequence nodes."""
        orch, causal, agent = full_stack
        await orch.run_epoch()
        assert causal.edge_count >= 1

    @pytest.mark.asyncio
    async def test_multiple_epochs_build_chains(self, full_stack):
        """Running multiple epochs builds temporal chains."""
        orch, causal, agent = full_stack
        await orch.run(max_epochs=3)
        # Should have nodes from all 3 epochs
        assert causal.node_count >= 6  # At least 2 per epoch
        # Should have temporal edges linking actions across epochs
        agent_nodes = causal.get_nodes_for_agent("test-a1")
        assert len(agent_nodes) >= 6

    @pytest.mark.asyncio
    async def test_causal_graph_property(self, full_stack):
        """Orchestrator exposes causal_graph property."""
        orch, causal, agent = full_stack
        assert orch.causal_graph is causal
