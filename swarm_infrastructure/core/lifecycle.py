"""Epoch Lifecycle Orchestrator — the heartbeat of the swarm.

The epoch loop is the central coordination mechanism.  It does NOT
make decisions for agents.  It sequences operations, collects events,
and enforces timing constraints.

12-Step Epoch Cycle:
 1. Advance world state epoch
 2. Agents perceive world
 3. Agents select actions (act)
 4. Environment applies consequences
 5. Agents update self-models
 6. Resolve predictions from previous epoch
 7. Agents evaluate epoch fitness
 8. Reaper evaluates mortality
 9. Execute kills & generate postmortems
10. Propagate scars to survivors
11. Goal genome mutation pass
12. Flush bus events & persist

Analysis must lag action by 1 epoch.
"""

from __future__ import annotations

import asyncio
import logging
import time
from typing import Any, Callable, Dict, List, Optional

from swarm.core.agent import Agent, AgentConfig, Consequence
from swarm.core.enums import Domain
from swarm.event_bus.bus import AsyncEventBus
from swarm.event_bus.events import BusEvent, EventType
from swarm.goal_genome.schema import MutationTrigger
from swarm.gpu_bridge.client import GpuTaskClient
from swarm.gpu_bridge.schema import (
    GpuTask,
    GpuTaskResult,
    GpuTaskStatus,
    GpuTaskType,
)
from swarm.reaper import PostmortemAnalyzer, ReaperEngine, ScarPropagator
from swarm.reaper.schema import DeathLevel
from swarm.storage.backend import StorageBackend
from swarm.tripwire.detector import TripwireDetector
from swarm.causal.graph import CausalGraph
from swarm.causal.schema import CausalNode, CausalEdge, NodeType, EdgeType
from swarm.world_sim.prediction import PredictionMarket
from swarm.world_sim.scoreboard import AccuracyScoreboard
from swarm.world_sim.state_graph import WorldStateGraph

logger = logging.getLogger(__name__)


class EpochStats:
    """Statistics for a single epoch."""

    __slots__ = (
        "epoch",
        "agents_alive",
        "agents_killed",
        "agents_born",
        "total_reward",
        "total_cost",
        "predictions_resolved",
        "scars_propagated",
        "goals_mutated",
        "bus_events_published",
        "gpu_tasks_submitted",
        "gpu_tasks_completed",
        "duration_seconds",
    )

    def __init__(self, epoch: int) -> None:
        self.epoch = epoch
        self.agents_alive = 0
        self.agents_killed = 0
        self.agents_born = 0
        self.total_reward = 0.0
        self.total_cost = 0.0
        self.predictions_resolved = 0
        self.scars_propagated = 0
        self.goals_mutated = 0
        self.bus_events_published = 0
        self.gpu_tasks_submitted = 0
        self.gpu_tasks_completed = 0
        self.duration_seconds = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "epoch": self.epoch,
            "agents_alive": self.agents_alive,
            "agents_killed": self.agents_killed,
            "agents_born": self.agents_born,
            "total_reward": round(self.total_reward, 4),
            "total_cost": round(self.total_cost, 4),
            "predictions_resolved": self.predictions_resolved,
            "scars_propagated": self.scars_propagated,
            "goals_mutated": self.goals_mutated,
            "bus_events_published": self.bus_events_published,
            "gpu_tasks_submitted": self.gpu_tasks_submitted,
            "gpu_tasks_completed": self.gpu_tasks_completed,
            "duration_seconds": round(self.duration_seconds, 4),
        }


class EpochOrchestrator:
    """Runs the 12-step epoch loop for a swarm of agents.

    Args:
        storage: Shared persistence backend.
        event_bus: Shared async event bus.
        world_state: Shared world state graph.
        prediction_market: Shared prediction market.
        scoreboard: Shared accuracy scoreboard.
        reaper: Shared reaper engine.
        postmortem_analyzer: Shared postmortem analyzer.
        scar_propagator: Shared scar propagator.
        tripwire: Shared tripwire detector.
        consequence_fn: Optional callback to compute consequences
                        for agent actions (env → agent).
    """

    def __init__(
        self,
        storage: StorageBackend,
        event_bus: AsyncEventBus,
        world_state: WorldStateGraph,
        prediction_market: PredictionMarket,
        scoreboard: AccuracyScoreboard,
        reaper: ReaperEngine,
        postmortem_analyzer: PostmortemAnalyzer,
        scar_propagator: ScarPropagator,
        tripwire: TripwireDetector,
        causal_graph: Optional[CausalGraph] = None,
        gpu_client: Optional[GpuTaskClient] = None,
        consequence_fn: Optional[
            Callable[[Agent, Any], List[Consequence]]
        ] = None,
    ) -> None:
        self._storage = storage
        self._bus = event_bus
        self._world = world_state
        self._predictions = prediction_market
        self._scoreboard = scoreboard
        self._reaper = reaper
        self._postmortem = postmortem_analyzer
        self._scar_propagator = scar_propagator
        self._tripwire = tripwire
        self._causal = causal_graph or CausalGraph(storage=storage)
        self._gpu_client = gpu_client
        self._consequence_fn = consequence_fn

        self._agents: Dict[str, Agent] = {}
        self._epoch_history: List[EpochStats] = []
        self._gpu_pending: Dict[str, str] = {}  # task_id → agent_id
        self._gpu_results_this_epoch: List[GpuTaskResult] = []
        self._running = False

    # ------------------------------------------------------------------
    # Agent management
    # ------------------------------------------------------------------

    def register_agent(self, agent: Agent) -> None:
        """Register an agent in the swarm."""
        self._agents[agent.agent_id] = agent
        logger.info("Registered agent %s", agent.agent_id)

    def spawn_agent(self, config: AgentConfig) -> Agent:
        """Create and register a new agent."""
        # Check scorched mandates
        for mandate in config.initial_mandates:
            if self._reaper.is_mandate_scorched(mandate):
                raise ValueError(
                    f"Cannot spawn agent with scorched mandate: {mandate}"
                )

        agent = Agent(
            config=config,
            storage=self._storage,
            event_bus=self._bus,
            world_state=self._world,
            prediction_market=self._predictions,
            reaper=self._reaper,
            postmortem_analyzer=self._postmortem,
            scar_propagator=self._scar_propagator,
            tripwire=self._tripwire,
        )

        self._agents[agent.agent_id] = agent
        return agent

    @property
    def living_agents(self) -> List[Agent]:
        return [a for a in self._agents.values() if a.is_alive]

    @property
    def dead_agents(self) -> List[Agent]:
        return [a for a in self._agents.values() if not a.is_alive]

    @property
    def current_epoch(self) -> int:
        return self._world.epoch

    @property
    def causal_graph(self) -> CausalGraph:
        """Access the causal graph for analysis."""
        return self._causal

    @property
    def gpu_client(self) -> Optional[GpuTaskClient]:
        """Access the GPU task client (None if not configured)."""
        return self._gpu_client

    # ------------------------------------------------------------------
    # GPU task helpers
    # ------------------------------------------------------------------

    async def submit_gpu_task(
        self,
        agent_id: str,
        task: GpuTask,
    ) -> Optional[str]:
        """Submit a GPU task on behalf of an agent.

        Returns task_id if a GPU client is available, else None.
        """
        if self._gpu_client is None:
            return None
        task.agent_id = agent_id
        task.epoch = self._world.epoch
        tid = await self._gpu_client.submit_task(task)
        self._gpu_pending[tid] = agent_id
        return tid

    async def _collect_gpu_results(self) -> List[GpuTaskResult]:
        """Poll all pending GPU tasks and collect completed results."""
        if self._gpu_client is None:
            return []

        completed: List[GpuTaskResult] = []
        still_pending: Dict[str, str] = {}

        for tid, agent_id in self._gpu_pending.items():
            result = await self._gpu_client.poll_result(tid)
            if result is not None:
                completed.append(result)
            else:
                still_pending[tid] = agent_id

        self._gpu_pending = still_pending
        return completed

    # ------------------------------------------------------------------
    # Epoch loop
    # ------------------------------------------------------------------

    async def run_epoch(self) -> EpochStats:
        """Execute one complete 12-step epoch.

        Returns epoch statistics.
        """
        start = time.monotonic()
        epoch = self._world.epoch
        stats = EpochStats(epoch)
        all_bus_events: List[BusEvent] = []

        living = self.living_agents
        stats.agents_alive = len(living)

        logger.info(
            "=== EPOCH %d START === agents_alive=%d",
            epoch,
            stats.agents_alive,
        )

        # ---- Step 1: Advance world state ----
        self._world.advance_epoch()
        all_bus_events.extend(self._world.get_pending_bus_events())

        # ---- Step 2: Agents perceive world ----
        # (Agents read world state implicitly via shared WorldStateGraph)

        # ---- Step 3: Agents select actions ----
        action_results = {}
        action_causal_nodes: Dict[str, CausalNode] = {}
        for agent in living:
            result = agent.act(epoch)
            if result:
                action_results[agent.agent_id] = result
                stats.total_cost += result.resource_cost

                # Record causal node for this action
                anode = self._causal.add_node(CausalNode(
                    agent_id=agent.agent_id,
                    epoch=epoch,
                    node_type=NodeType.ACTION,
                    action_type=result.action_type,
                    domain=agent._config.domain.value,
                    value=-result.resource_cost,  # Cost is negative value
                    metadata={"action_id": result.action_id},
                ))
                action_causal_nodes[agent.agent_id] = anode

                # Link to previous epoch's action (temporal chain)
                prev_nodes = self._causal.get_nodes_for_agent(agent.agent_id)
                prev_actions = [
                    n for n in prev_nodes
                    if n.node_type == NodeType.ACTION.value
                    and n.node_id != anode.node_id
                ]
                if prev_actions:
                    try:
                        self._causal.add_edge(
                            prev_actions[-1].node_id,
                            anode.node_id,
                            edge_type=EdgeType.TEMPORAL,
                            weight=0.3,
                        )
                    except ValueError:
                        pass  # Skip if cycle detected

        # ---- Step 4: Environment applies consequences ----
        if self._consequence_fn:
            for agent in living:
                action = action_results.get(agent.agent_id)
                if action:
                    consequences = self._consequence_fn(agent, action)
                    for c in consequences:
                        agent.receive_consequence(c)
                        if c.consequence_type == "REWARD":
                            stats.total_reward += c.magnitude

                        # Record causal node for consequence
                        cnode = self._causal.add_node(CausalNode(
                            agent_id=agent.agent_id,
                            epoch=epoch,
                            node_type=NodeType.CONSEQUENCE,
                            action_type=f"consequence:{c.consequence_type}",
                            domain=agent._config.domain.value,
                            value=c.magnitude if c.consequence_type == "REWARD" else -c.magnitude,
                            metadata={"source": c.source},
                        ))

                        # Link action → consequence (direct cause)
                        action_node = action_causal_nodes.get(agent.agent_id)
                        if action_node:
                            try:
                                self._causal.add_edge(
                                    action_node.node_id,
                                    cnode.node_id,
                                    edge_type=EdgeType.DIRECT,
                                    weight=0.8,
                                )
                            except ValueError:
                                pass

        # ---- Step 5: Agents update self-models ----
        # (Handled within act() and receive_consequence())

        # ---- Step 5b: Collect GPU results from previous epoch ----
        gpu_results = await self._collect_gpu_results()
        self._gpu_results_this_epoch = gpu_results
        stats.gpu_tasks_completed = len(gpu_results)
        for gr in gpu_results:
            if gr.succeeded:
                agent = self._agents.get(gr.agent_id)
                if agent and agent.is_alive:
                    agent.receive_consequence(
                        Consequence(
                            "REWARD",
                            float(gr.compute_units_used) * 0.001,
                            "GPU_COMPUTE",
                        )
                    )
                    stats.total_reward += float(gr.compute_units_used) * 0.001

        # ---- Step 6: Resolve predictions from previous epoch ----
        pending_preds = self._predictions.get_pending_predictions(epoch=epoch)
        if pending_preds:
            from swarm.world_sim.schema import PredictionResult
            results = []
            for pred in pending_preds:
                actual = self._world.query(pred.target_state_path)
                if actual is not None:
                    try:
                        actual_val = float(actual)
                    except (TypeError, ValueError):
                        actual_val = 0.0
                    error = abs(pred.predicted_value - actual_val)
                    reward_penalty = pred.stake * (1.0 - error) if error < 1.0 else -pred.stake
                    result = PredictionResult(
                        prediction_id=pred.prediction_id,
                        agent_id=pred.agent_id,
                        predicted_value=pred.predicted_value,
                        actual_value=actual_val,
                        error_magnitude=error,
                        stake=pred.stake,
                        reward_or_penalty=reward_penalty,
                    )
                    results.append(result)
                    self._predictions.remove_prediction(pred.prediction_id)

                    # Apply reward/penalty to agent
                    agent = self._agents.get(pred.agent_id)
                    if agent and agent.is_alive:
                        if reward_penalty > 0:
                            agent.receive_consequence(
                                Consequence("REWARD", reward_penalty, "PREDICTION_MARKET")
                            )
                        else:
                            agent.receive_consequence(
                                Consequence("ENERGY_DRAIN", abs(reward_penalty), "PREDICTION_MARKET")
                            )

            if results:
                self._scoreboard.update_from_results(epoch, results)
                all_bus_events.extend(self._scoreboard.get_pending_bus_events())
                stats.predictions_resolved = len(results)

        # ---- Step 7: Agents evaluate epoch fitness ----
        agent_summaries: Dict[str, Dict] = {}
        for agent in self.living_agents:
            summary = agent.evaluate_epoch(epoch)
            agent_summaries[agent.agent_id] = summary

        # ---- Step 8: Reaper evaluates mortality ----
        kill_decisions = []
        for agent_id, summary in agent_summaries.items():
            agent = self._agents[agent_id]
            decision = self._reaper.evaluate(
                agent_id=agent_id,
                resource_budget=summary["resource_budget"],
                survival_probability=summary["survival_probability"],
                prediction_accuracy=summary["prediction_accuracy"],
                fitness_scores=summary.get("fitness_history", []),
                active_mandates=agent.active_mandates,
            )
            if decision.should_kill:
                kill_decisions.append((agent, decision, summary))

        # ---- Step 9: Execute kills & generate postmortems ----
        for agent, decision, summary in kill_decisions:
            # Execute kill
            self._reaper.execute_kill(decision)
            all_bus_events.extend(self._reaper.get_pending_bus_events())

            # Kill the agent
            agent.die(decision.reason)

            # Generate postmortem
            agent_budgets = {
                aid: s["resource_budget"]
                for aid, s in agent_summaries.items()
                if self._agents[aid].is_alive
            }
            postmortem = self._postmortem.analyze(
                decision,
                final_budget=summary["resource_budget"],
                final_survival_prob=summary["survival_probability"],
                final_accuracy=summary["prediction_accuracy"],
                active_goal_count=summary["active_goals"],
                scar_count=summary["total_scars"],
                epoch=epoch,
                active_agent_ids=[a.agent_id for a in self.living_agents],
                agent_budgets=agent_budgets,
            )

            # Record death as causal node
            death_node = self._causal.add_node(CausalNode(
                agent_id=agent.agent_id,
                epoch=epoch,
                node_type=NodeType.DEATH,
                action_type=f"death:{decision.cause.value}",
                domain=agent._config.domain.value,
                value=0.0,
                metadata={
                    "death_level": decision.death_level.value,
                    "reason": decision.reason,
                    "confidence": decision.confidence,
                },
            ))

            # Link last action → death
            action_node = action_causal_nodes.get(agent.agent_id)
            if action_node:
                try:
                    self._causal.add_edge(
                        action_node.node_id,
                        death_node.node_id,
                        edge_type=EdgeType.DIRECT,
                        weight=1.0,
                        confidence=decision.confidence,
                    )
                except ValueError:
                    pass

            stats.agents_killed += 1
            logger.critical(
                "EPOCH %d KILL: agent=%s cause=%s level=%s",
                epoch,
                agent.agent_id,
                decision.cause.value,
                decision.death_level.value,
            )

        # ---- Step 10: Propagate scars to survivors ----
        for agent, decision, summary in kill_decisions:
            survivor_registries = {
                a.agent_id: a.scars
                for a in self.living_agents
            }
            propagated = self._scar_propagator.propagate(
                decision=decision,
                survivor_registries=survivor_registries,
                dead_agent_domains=[
                    self._agents[agent.agent_id]._config.domain.value
                ],
            )
            stats.scars_propagated += propagated
            all_bus_events.extend(
                self._scar_propagator.get_pending_bus_events()
            )

        # ---- Step 11: Goal genome mutation pass ----
        for agent in self.living_agents:
            goals = agent.goal_genome.get_active_goals()
            for goal in goals:
                fitness = self._fitness_history_last(agent)
                if fitness < 0.3 and len(goals) > 0:
                    try:
                        agent.goal_genome.mutate_goal(
                            goal.goal_id,
                            MutationTrigger.REPEATED_FAILURE,
                        )
                        stats.goals_mutated += 1
                    except Exception:
                        logger.debug(
                            "Goal mutation failed for agent %s goal %s",
                            agent.agent_id,
                            goal.goal_id[:8],
                        )
                    break  # One mutation per epoch

        # ---- Step 12: Flush bus events ----
        for agent in self._agents.values():
            all_bus_events.extend(agent.collect_bus_events())

        for event in all_bus_events:
            await self._bus.publish(event)
        stats.bus_events_published = len(all_bus_events)

        stats.duration_seconds = time.monotonic() - start
        self._epoch_history.append(stats)

        logger.info(
            "=== EPOCH %d END === alive=%d killed=%d reward=%.2f cost=%.2f duration=%.3fs",
            epoch,
            len(self.living_agents),
            stats.agents_killed,
            stats.total_reward,
            stats.total_cost,
            stats.duration_seconds,
        )

        return stats

    async def run(self, max_epochs: int = 100) -> List[EpochStats]:
        """Run the epoch loop for up to max_epochs.

        Stops early if all agents are dead.
        """
        self._running = True
        results: List[EpochStats] = []

        for _ in range(max_epochs):
            if not self._running:
                break

            if not self.living_agents:
                logger.warning("All agents dead — stopping epoch loop")
                break

            stats = await self.run_epoch()
            results.append(stats)

        self._running = False
        return results

    def stop(self) -> None:
        """Signal the epoch loop to stop after current epoch."""
        self._running = False

    @property
    def epoch_history(self) -> List[EpochStats]:
        return list(self._epoch_history)

    # ------------------------------------------------------------------
    # Internals
    # ------------------------------------------------------------------

    def _fitness_history_last(self, agent: Agent) -> float:
        """Get the last fitness score for an agent."""
        history = agent.fitness_history
        if not history:
            return 1.0
        return history[-1]
