"""Freebuff Agent — GPU Buffer & Memory Management Specialist.

The FreebuffAgent is a specialized swarm agent that operates in the
FREE_BUFF domain.  Its mandates center on GPU memory optimization:
freeing orphaned buffers, defragmenting VRAM allocations, and
proactively reclaiming idle memory on swarm validator nodes.

It extends the base Agent with:

- Buffer lifecycle tracking (open/closed/cached buffers per GPU node)
- Idle buffer reclamation (release buffers held beyond threshold TTL)
- VRAM fragmentation scoring and defrag recommendations
- Memory pressure prediction (anticipate OOM before it happens)
- Cross-node buffer migration orchestration
- **Freebuff CLI integration** — autonomously dispatches coding prompts
  to the freebuff coding agent via the subprocess CLI wrapper

The agent participates in the standard 12-step epoch loop but
performs its domain-specific actions each epoch in place of the
generic goal-pursuit action.
"""

from __future__ import annotations

import logging
import time
from collections import deque
from typing import Any, Deque, Dict, List, Optional, Tuple
from dataclasses import dataclass, field

from swarm.core.agent import Agent, AgentConfig, ActionResult, Consequence
from swarm.core.enums import Domain, Outcome
from swarm.integrations.freebuff_cli import FreebuffCLI, FreebuffResult

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Freebuff-specific data models
# ---------------------------------------------------------------------------

@dataclass
class BufferRecord:
    """Track a single GPU buffer across its lifecycle."""

    buffer_id: str
    gpu_node_id: str
    size_bytes: int
    allocated_at: float  # epoch timestamp
    last_accessed_at: float
    status: str = "active"  # active | idle | reclaiming | freed
    owner_agent_id: Optional[str] = None


@dataclass
class GpuMemorySnapshot:
    """Snapshot of a single GPU node's memory state."""

    gpu_node_id: str
    total_vram_bytes: int
    used_vram_bytes: int
    free_vram_bytes: int
    fragment_count: int  # number of non-contiguous free segments
    idle_buffer_count: int
    reclaimable_bytes: int
    fragmentation_score: float = 0.0  # computed by agent
    timestamp: float = field(default_factory=time.time)


# ---------------------------------------------------------------------------
# FreebuffAgentConfig
# ---------------------------------------------------------------------------

class FreebuffAgentConfig(AgentConfig):
    """Configuration for a FreebuffAgent with memory-management and CLI defaults."""

    __slots__ = (
        "idle_buffer_ttl_epochs",
        "max_reclaim_per_epoch",
        "fragmentation_rebalance_threshold",
        "target_gpu_nodes",
        "cli_enabled",
        "cli_workspace_root",
        "cli_timeout_s",
        "cli_throttle_epochs",
        "cli_max_prompt_length",
    )

    def __init__(
        self,
        agent_id: Optional[str] = None,
        initial_budget: float = 2000.0,
        initial_mandates: Optional[List[str]] = None,
        idle_buffer_ttl_epochs: int = 30,
        max_reclaim_per_epoch: int = 50,
        fragmentation_rebalance_threshold: float = 0.7,
        target_gpu_nodes: Optional[List[str]] = None,
        cli_enabled: bool = True,
        cli_workspace_root: Optional[str] = None,
        cli_timeout_s: float = 120.0,
        cli_throttle_epochs: int = 1,
        cli_max_prompt_length: int = 4096,
    ) -> None:
        super().__init__(
            agent_id=agent_id,
            initial_budget=initial_budget,
            initial_mandates=initial_mandates
            or [
                "reclaim_idle_buffers",
                "defragment_vram",
                "predict_memory_pressure",
                "dispatch_coding_tasks",
            ],
            domain=Domain.FREE_BUFF,
        )
        self.idle_buffer_ttl_epochs = idle_buffer_ttl_epochs
        self.max_reclaim_per_epoch = max_reclaim_per_epoch
        self.fragmentation_rebalance_threshold = fragmentation_rebalance_threshold
        self.target_gpu_nodes = target_gpu_nodes or []
        self.cli_enabled = cli_enabled
        self.cli_workspace_root = cli_workspace_root
        self.cli_timeout_s = cli_timeout_s
        self.cli_throttle_epochs = cli_throttle_epochs
        self.cli_max_prompt_length = cli_max_prompt_length


# ---------------------------------------------------------------------------
# FreebuffAgent
# ---------------------------------------------------------------------------

class FreebuffAgent(Agent):
    """A specialized agent for GPU buffer/memory management in the swarm.

    The FreebuffAgent performs these domain-specific actions each epoch:

    1. **Scan idle buffers** — identify buffers dormant beyond TTL.
    2. **Reclaim** — free or migrate idle buffers.
    3. **Dispatch coding task** — send a queued coding prompt to the
       freebuff CLI if available and not throttled.
    4. **Score fragmentation** — compute VRAM fragmentation per GPU node.
    5. **Defragment** — recommend or trigger buffer compaction when
       fragmentation exceeds threshold.
    6. **Predict pressure** — predict OOM risk for the next epoch.

    It still inherits all 6 subsystem layers from the base Agent
    (self-model, goal genome, world simulation, self-improvement,
    reaper, tripwire) and participates fully in the epoch loop.
    """

    def __init__(
        self,
        config: FreebuffAgentConfig,
        storage,
        event_bus=None,
        world_state=None,
        prediction_market=None,
        reaper=None,
        postmortem_analyzer=None,
        scar_propagator=None,
        tripwire=None,
        freebuff_cli: Optional[FreebuffCLI] = None,
    ) -> None:
        super().__init__(
            config=config,
            storage=storage,
            event_bus=event_bus,
            world_state=world_state,
            prediction_market=prediction_market,
            reaper=reaper,
            postmortem_analyzer=postmortem_analyzer,
            scar_propagator=scar_propagator,
            tripwire=tripwire,
        )

        self._freebuff_config: FreebuffAgentConfig = config

        # -- Freebuff state --
        self._buffer_registry: Dict[str, BufferRecord] = {}
        self._gpu_snapshots: Dict[str, GpuMemorySnapshot] = {}
        self._reclaim_history: List[Tuple[float, int, int]] = []
        self._pressure_predictions: List[float] = []

        # -- Freebuff CLI integration --
        self._cli: Optional[FreebuffCLI] = freebuff_cli
        self._coding_queue: Deque[Tuple[str, float]] = deque()
        self._active_conversation_id: Optional[str] = None
        self._last_cli_epoch: float = -999.0
        self._cli_results: List[FreebuffResult] = []

        if config.cli_enabled and self._cli is None:
            logger.debug(
                "FreebuffAgent %s: CLI enabled but no FreebuffCLI instance "
                "injected — coding dispatch disabled",
                self.agent_id[:8],
            )

        logger.info(
            "FreebuffAgent born: id=%s ttl=%d epochs nodes=%s cli=%s",
            self.agent_id,
            config.idle_buffer_ttl_epochs,
            config.target_gpu_nodes or ["*"],
            "enabled" if (config.cli_enabled and self._cli) else "disabled",
        )

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def buffer_registry(self) -> Dict[str, BufferRecord]:
        return self._buffer_registry

    @property
    def gpu_snapshots(self) -> Dict[str, GpuMemorySnapshot]:
        return self._gpu_snapshots

    @property
    def idle_buffer_ttl_epochs(self) -> int:
        return self._freebuff_config.idle_buffer_ttl_epochs

    @property
    def specialization(self) -> str:
        """Return the specialization string for registry integration."""
        return "freebuff"

    @property
    def coding_queue_depth(self) -> int:
        """Number of pending coding prompts in the queue."""
        return len(self._coding_queue)

    @property
    def cli_results(self) -> List[FreebuffResult]:
        """Recent CLI results (most recent last)."""
        return list(self._cli_results)

    @property
    def active_conversation_id(self) -> Optional[str]:
        """The currently active conversation ID, if any."""
        return self._active_conversation_id

    # ------------------------------------------------------------------
    # CLI coding dispatch
    # ------------------------------------------------------------------

    def enqueue_coding_prompt(
        self,
        prompt: str,
        epoch: Optional[int] = None,
    ) -> bool:
        """Queue a coding prompt for dispatch via the freebuff CLI.

        The prompt will be picked up on the next epoch when the CLI
        is available and the throttle window has passed.

        Args:
            prompt: The coding prompt to send.
            epoch: Current epoch (used for TTL).

        Returns:
            True if the prompt was queued, False if rejected.
        """
        if not self._freebuff_config.cli_enabled:
            logger.debug(
                "FreebuffAgent %s: CLI disabled, rejecting prompt",
                self.agent_id[:8],
            )
            return False

        if len(prompt) > self._freebuff_config.cli_max_prompt_length:
            logger.warning(
                "FreebuffAgent %s: prompt too long (%d > %d)",
                self.agent_id[:8],
                len(prompt),
                self._freebuff_config.cli_max_prompt_length,
            )
            return False

        if len(self._coding_queue) >= 100:
            logger.warning(
                "FreebuffAgent %s: coding queue full (100)",
                self.agent_id[:8],
            )
            return False

        epoch_float = float(epoch) if epoch is not None else time.time()
        self._coding_queue.append((prompt.strip(), epoch_float))
        logger.info(
            "FreebuffAgent %s: coding prompt queued (depth=%d)",
            self.agent_id[:8],
            len(self._coding_queue),
        )
        return True

    def _dispatch_coding_task(
        self,
        prompt: str,
        current_epoch: int,
    ) -> Optional[FreebuffResult]:
        """Dispatch a single coding prompt via the CLI.

        Returns the FreebuffResult, or None if the CLI is unavailable.
        """
        if not self._cli:
            return None

        throttle = self._freebuff_config.cli_throttle_epochs
        if (float(current_epoch) - self._last_cli_epoch) < float(throttle):
            logger.debug(
                "FreebuffAgent %s: CLI throttled (last=%s epoch=%d throttle=%d)",
                self.agent_id[:8],
                self._last_cli_epoch,
                current_epoch,
                throttle,
            )
            return None

        cache_key = f"code:{hash(prompt) % 100000}"
        result = self._cli.run_prompt(
            prompt=prompt,
            conversation_id=self._active_conversation_id,
            timeout=self._freebuff_config.cli_timeout_s,
            cache_key=cache_key,
        )

        self._last_cli_epoch = float(current_epoch)

        # Track the conversation
        if result.conversation_id:
            self._active_conversation_id = result.conversation_id

        # Store result history (bounded)
        self._cli_results.append(result)
        if len(self._cli_results) > 50:
            self._cli_results = self._cli_results[-50:]

        logger.info(
            "FreebuffAgent %s: coding dispatch ok=%s duration=%.2fs "
            "output_len=%d conv_id=%s",
            self.agent_id[:8],
            result.ok,
            result.duration_seconds,
            len(result.output),
            result.conversation_id or "-",
        )

        return result

    # ------------------------------------------------------------------
    # Domain actions (called from act())
    # ------------------------------------------------------------------

    def _scan_idle_buffers(self, current_epoch: int) -> List[BufferRecord]:
        """Find buffers that have been idle beyond TTL."""
        idle: List[BufferRecord] = []
        ttl = self._freebuff_config.idle_buffer_ttl_epochs

        for buf in self._buffer_registry.values():
            if buf.status != "active":
                continue
            idle_epochs = current_epoch - int(buf.last_accessed_at)
            if idle_epochs >= ttl:
                idle.append(buf)

        if idle:
            logger.debug(
                "FreebuffAgent %s: %d idle buffers found (≥ %d epochs)",
                self.agent_id[:8],
                len(idle),
                ttl,
            )

        return idle

    def _reclaim_buffers(
        self, idle_buffers: List[BufferRecord], current_epoch: int
    ) -> Tuple[int, int]:
        """Reclaim idle buffers (mark as freed).

        Returns (reclaimed_count, failed_count).
        Capped at max_reclaim_per_epoch.
        """
        max_reclaim = self._freebuff_config.max_reclaim_per_epoch
        reclaimed = 0
        failed = 0

        for buf in idle_buffers[:max_reclaim]:
            try:
                buf.status = "freed"
                reclaimed += 1
                logger.debug(
                    "FreebuffAgent %s: reclaimed buffer %s (%d bytes) on %s",
                    self.agent_id[:8],
                    buf.buffer_id,
                    buf.size_bytes,
                    buf.gpu_node_id,
                )
            except Exception:
                logger.debug(
                    "FreebuffAgent %s: reclaim failed for buffer %s",
                    self.agent_id[:8],
                    buf.buffer_id,
                )
                buf.status = "active"  # rollback status
                failed += 1

        # Update snapshots for affected nodes
        for buf in idle_buffers[:max_reclaim]:
            if buf.gpu_node_id in self._gpu_snapshots:
                snap = self._gpu_snapshots[buf.gpu_node_id]
                freed = buf.size_bytes if buf.status == "freed" else 0
                snap.used_vram_bytes = max(0, snap.used_vram_bytes - freed)
                snap.free_vram_bytes = min(
                    snap.total_vram_bytes,
                    snap.free_vram_bytes + freed,
                )

        return reclaimed, failed

    def _score_fragmentation(self) -> Dict[str, float]:
        """Compute fragmentation scores for all tracked GPU nodes."""
        scores: Dict[str, float] = {}
        for node_id, snap in self._gpu_snapshots.items():
            if snap.free_vram_bytes > 0:
                free_mb = max(1, snap.free_vram_bytes // (1024 * 1024))
                frag = min(1.0, snap.fragment_count / max(1, free_mb))
            else:
                frag = 1.0
            scores[node_id] = round(frag, 4)
            snap.fragmentation_score = frag

        return scores

    def _defrag_recommendations(self, scores: Dict[str, float]) -> List[str]:
        """Return GPU node IDs that exceed the fragmentation threshold."""
        threshold = self._freebuff_config.fragmentation_rebalance_threshold
        return [nid for nid, score in scores.items() if score > threshold]

    def _predict_memory_pressure(self) -> float:
        """Predict OOM risk for the next epoch."""
        if not self._gpu_snapshots:
            return 0.0

        pressures: List[float] = []
        for snap in self._gpu_snapshots.values():
            if snap.total_vram_bytes == 0:
                continue
            usage_ratio = snap.used_vram_bytes / snap.total_vram_bytes
            pressure = 0.6 * usage_ratio + 0.4 * snap.fragmentation_score
            pressures.append(pressure)

        if not pressures:
            return 0.0

        avg_pressure = sum(pressures) / len(pressures)
        self._pressure_predictions.append(avg_pressure)

        if len(self._pressure_predictions) > 100:
            self._pressure_predictions = self._pressure_predictions[-100:]

        return avg_pressure

    # ------------------------------------------------------------------
    # Registration helpers
    # ------------------------------------------------------------------

    def register_buffer(self, record: BufferRecord) -> None:
        """Track a new GPU buffer in the registry."""
        self._buffer_registry[record.buffer_id] = record
        logger.debug(
            "FreebuffAgent %s: registered buffer %s (%d bytes) on %s",
            self.agent_id[:8],
            record.buffer_id,
            record.size_bytes,
            record.gpu_node_id,
        )

    def update_gpu_snapshot(self, snapshot: GpuMemorySnapshot) -> None:
        """Update (or insert) a GPU node memory snapshot."""
        self._gpu_snapshots[snapshot.gpu_node_id] = snapshot

    def touch_buffer(self, buffer_id: str, epoch: int) -> bool:
        """Mark a buffer as recently accessed (resets idle timer)."""
        buf = self._buffer_registry.get(buffer_id)
        if buf is None:
            return False
        buf.last_accessed_at = float(epoch)
        return True

    # ------------------------------------------------------------------
    # Core lifecycle (override Agent.act)
    # ------------------------------------------------------------------

    def act(self, epoch: int) -> Optional[ActionResult]:
        """Freebuff-specific epoch action: reclaim, code, monitor, defrag, predict.

        Action priority:
        1. Reclaim idle buffers (highest priority — frees VRAM immediately)
        2. Dispatch queued coding prompts via the CLI
        3. Monitor fragmentation and memory pressure
        4. Fall through to base goal pursuit
        """
        if not self.is_alive:
            return None

        # ---- Step 1: Scan for idle buffers ----
        idle_buffers = self._scan_idle_buffers(epoch)

        # ---- Step 2: Reclaim if idle buffers exist ----
        if idle_buffers:
            reclaimed, failed = self._reclaim_buffers(idle_buffers, epoch)
            self._reclaim_history.append((float(epoch), reclaimed, failed))

            if len(self._reclaim_history) > 200:
                self._reclaim_history = self._reclaim_history[-200:]

            total_bytes = sum(b.size_bytes for b in idle_buffers[:reclaimed])

            result = ActionResult(
                action_type="freebuff:reclaim",
                outcome=Outcome.SUCCESS if reclaimed > 0 else Outcome.FAILURE,
                resource_cost=max(1.0, 0.1 * reclaimed),
                reward=0.01 * (total_bytes // (1024 * 1024)),
                details={
                    "reclaimed": reclaimed,
                    "failed": failed,
                    "bytes_freed": total_bytes,
                    "idle_found": len(idle_buffers),
                },
            )

            self._epoch_actions.append(result)
            self._total_cost += result.resource_cost

            if result.reward > 0:
                self.receive_consequence(
                    Consequence(
                        "REWARD",
                        result.reward,
                        "FREEBUFF_RECLAIM",
                        details={
                            "bytes_freed": total_bytes,
                            "buffers_reclaimed": reclaimed,
                        },
                    )
                )

            return result

        # ---- Step 3: Dispatch queued coding prompt via CLI ----
        if self._cli and self._coding_queue:
            prompt, _ = self._coding_queue.popleft()
            cli_result = self._dispatch_coding_task(prompt, epoch)

            if cli_result is not None:
                result = ActionResult(
                    action_type="freebuff:code",
                    outcome=Outcome.SUCCESS if cli_result.ok else Outcome.FAILURE,
                    resource_cost=max(1.0, cli_result.duration_seconds * 0.1),
                    reward=0.5 if cli_result.ok else 0.0,
                    details={
                        "prompt": prompt[:200],
                        "output_len": len(cli_result.output),
                        "ok": cli_result.ok,
                        "duration_seconds": cli_result.duration_seconds,
                        "conversation_id": cli_result.conversation_id,
                        "queue_remaining": len(self._coding_queue),
                    },
                )

                self._epoch_actions.append(result)
                self._total_cost += result.resource_cost

                if result.reward > 0:
                    self.receive_consequence(
                        Consequence(
                            "REWARD",
                            result.reward,
                            "FREEBUFF_CODE",
                            details={
                                "output_len": len(cli_result.output),
                                "conversation_id": cli_result.conversation_id,
                            },
                        )
                    )

                return result
            else:
                # CLI throttled or unavailable — re-queue at front
                self._coding_queue.appendleft((prompt, float(epoch)))
                logger.debug(
                    "FreebuffAgent %s: CLI throttled, prompt re-queued",
                    self.agent_id[:8],
                )

        # ---- Step 4: Score fragmentation ----
        frag_scores = self._score_fragmentation()
        high_frag_nodes = self._defrag_recommendations(frag_scores)

        # ---- Step 5: Predict memory pressure ----
        pressure = self._predict_memory_pressure()

        if high_frag_nodes or pressure > 0.5:
            result = ActionResult(
                action_type="freebuff:monitor",
                outcome=Outcome.PARTIAL if high_frag_nodes else Outcome.SUCCESS,
                resource_cost=0.5,
                reward=0.0,
                details={
                    "frag_scores": frag_scores,
                    "high_frag_nodes": high_frag_nodes,
                    "memory_pressure": round(pressure, 4),
                },
            )

            self._epoch_actions.append(result)
            self._total_cost += result.resource_cost

            if high_frag_nodes:
                self.receive_consequence(
                    Consequence(
                        "SPACE_NARROWING",
                        magnitude=float(len(high_frag_nodes)),
                        source="FREEBUFF_FRAGMENTATION",
                        details={
                            "high_frag_nodes": high_frag_nodes,
                            "frag_scores": {n: frag_scores[n] for n in high_frag_nodes},
                        },
                    )
                )

            return result

        # ---- Step 6: Fall through to base goal pursuit ----
        return super().act(epoch)

    # ------------------------------------------------------------------
    # Diagnostics / introspection
    # ------------------------------------------------------------------

    def get_memory_report(self) -> Dict[str, Any]:
        """Produce a human-readable memory and CLI report for this agent."""
        total_buffers = len(self._buffer_registry)
        active_buffers = sum(
            1 for b in self._buffer_registry.values() if b.status == "active"
        )
        freed_buffers = sum(
            1 for b in self._buffer_registry.values() if b.status == "freed"
        )
        total_bytes_tracked = sum(
            b.size_bytes for b in self._buffer_registry.values()
        )

        recent_reclaims = self._reclaim_history[-10:] if self._reclaim_history else []
        total_reclaimed = sum(r[1] for r in self._reclaim_history)
        total_failed = sum(r[2] for r in self._reclaim_history)

        current_pressure = (
            self._pressure_predictions[-1] if self._pressure_predictions else 0.0
        )

        node_summaries = {}
        for node_id, snap in self._gpu_snapshots.items():
            node_summaries[node_id] = {
                "total_mb": snap.total_vram_bytes // (1024 * 1024),
                "used_mb": snap.used_vram_bytes // (1024 * 1024),
                "free_mb": snap.free_vram_bytes // (1024 * 1024),
                "fragmentation": snap.fragmentation_score,
                "idle_buffers": snap.idle_buffer_count,
                "reclaimable_mb": snap.reclaimable_bytes // (1024 * 1024),
            }

        # CLI stats
        cli_stats = {
            "enabled": self._freebuff_config.cli_enabled,
            "connected": self._cli is not None,
            "queue_depth": len(self._coding_queue),
            "conversation_id": self._active_conversation_id,
            "last_epoch": self._last_cli_epoch if self._last_cli_epoch > -999 else None,
            "total_results": len(self._cli_results),
            "recent_ok": sum(1 for r in self._cli_results[-10:] if r.ok)
            if self._cli_results
            else 0,
        }

        return {
            "agent_id": self.agent_id,
            "domain": Domain.FREE_BUFF.value,
            "alive": self.is_alive,
            "buffer_stats": {
                "total": total_buffers,
                "active": active_buffers,
                "freed": freed_buffers,
                "total_bytes_tracked": total_bytes_tracked,
            },
            "reclaim_stats": {
                "total_reclaimed": total_reclaimed,
                "total_failed": total_failed,
                "recent": recent_reclaims,
            },
            "memory_pressure": round(current_pressure, 4),
            "fragmentation_threshold": self._freebuff_config.fragmentation_rebalance_threshold,
            "nodes": node_summaries,
            "cli": cli_stats,
        }
