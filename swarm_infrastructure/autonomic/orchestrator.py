"""Autonomic Orchestrator — The brain of the X3 control plane.

The Orchestrator ties everything together:
  1. Consumes HealthEngine snapshots
  2. Matches health patterns to remediation playbooks
  3. Dispatches actions through Operators
  4. Enforces guardrails (safe mode, circuit breakers, rate limits)
  5. Exposes a RESTish API facade for the swarm server

Architecture invariant:
  Sentinels OBSERVE → Health Engine SCORES → Orchestrator DECIDES → Operators ACT

The Orchestrator never touches the OS directly.  All mutations go
through the Operator layer, which enforces its own whitelists and
rate limits.
"""

from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional, Tuple

from .config import AutonomicConfig, SafeModeProfile
from .metrics_bus import MetricsBus
from .health_engine import HealthEngine
from .state_machine import RecoveryStateMachine, SystemState
from .circuit_breaker import CircuitBreakerRegistry, CBState
from .operators import OperatorRegistry, OperatorResult, ActionRecord
from .audit import AuditLog

log = logging.getLogger("autonomic.orchestrator")


# ── Playbook / Remediation Rules ────────────────────────────────────

class Severity(Enum):
    INFO = "info"
    WARNING = "warning"
    CRITICAL = "critical"


@dataclass
class PlaybookAction:
    """A single step in a remediation playbook."""
    operator: str
    action: str
    target: str
    kwargs: Dict[str, Any] = field(default_factory=dict)
    description: str = ""


@dataclass
class Playbook:
    """A named remediation sequence triggered by a condition."""
    name: str
    description: str
    severity: Severity
    actions: List[PlaybookAction]
    cooldown_s: float = 120.0  # don't re-trigger for this long

    # Condition: these are evaluated by the orchestrator
    condition_component: Optional[str] = None
    condition_score_below: Optional[float] = None
    condition_state: Optional[SystemState] = None


# Built-in playbooks
DEFAULT_PLAYBOOKS: List[Playbook] = [
    Playbook(
        name="gpu_overheat",
        description="GPU temp critical — reduce power limit and drain",
        severity=Severity.CRITICAL,
        condition_component="gpu",
        condition_score_below=40,
        actions=[
            PlaybookAction("gpu", "set_power_limit", "0", {"watts": 100},
                           "Reduce GPU 0 power to 100W"),
            PlaybookAction("gpu", "set_power_limit", "1", {"watts": 100},
                           "Reduce GPU 1 power to 100W"),
            PlaybookAction("gpu", "set_power_limit", "2", {"watts": 100},
                           "Reduce GPU 2 power to 100W"),
        ],
        cooldown_s=300,
    ),

    Playbook(
        name="ollama_restart",
        description="Ollama service unhealthy — restart",
        severity=Severity.WARNING,
        condition_component="swarm",
        condition_score_below=50,
        actions=[
            PlaybookAction("service", "restart", "ollama_server", {},
                           "Restart Ollama wrapper"),
        ],
        cooldown_s=180,
    ),

    Playbook(
        name="oom_response",
        description="OOM detected in logs — GC agents and pause queue",
        severity=Severity.CRITICAL,
        condition_component="logs",
        condition_score_below=40,
        actions=[
            PlaybookAction("swarm", "gc_agents", "", {},
                           "Garbage collect idle agents"),
            PlaybookAction("swarm", "pause_queue", "default", {},
                           "Pause default job queue"),
        ],
        cooldown_s=300,
    ),

    Playbook(
        name="enter_safe_mode",
        description="System entering safe mode — scale down everything",
        severity=Severity.CRITICAL,
        condition_state=SystemState.SAFE_MODE,
        actions=[
            PlaybookAction("gpu", "set_power_limit", "0", {"watts": 80},
                           "Reduce GPU 0 to minimum"),
            PlaybookAction("gpu", "set_power_limit", "1", {"watts": 80},
                           "Reduce GPU 1 to minimum"),
            PlaybookAction("gpu", "set_power_limit", "2", {"watts": 80},
                           "Reduce GPU 2 to minimum"),
            PlaybookAction("swarm", "pause_queue", "default", {},
                           "Pause all job queues"),
        ],
        cooldown_s=600,
    ),
]


# ── Orchestrator ────────────────────────────────────────────────────

class Orchestrator:
    """The brain — consumes health, matches playbooks, dispatches operators."""

    def __init__(
        self,
        bus: MetricsBus,
        health: HealthEngine,
        state_machine: RecoveryStateMachine,
        breakers: CircuitBreakerRegistry,
        operators: OperatorRegistry,
        audit: AuditLog,
        config: Optional[AutonomicConfig] = None,
        playbooks: Optional[List[Playbook]] = None,
    ):
        self._bus = bus
        self._health = health
        self._sm = state_machine
        self._breakers = breakers
        self._ops = operators
        self._audit = audit
        self._cfg = config or AutonomicConfig()
        self._playbooks = playbooks or list(DEFAULT_PLAYBOOKS)

        self._running = False
        self._task: Optional[asyncio.Task] = None
        self._eval_interval = 10.0  # seconds
        self._playbook_cooldowns: Dict[str, float] = {}
        self._action_history: List[dict] = []
        self._safe_mode_active = False

    async def start(self) -> None:
        self._running = True
        self._task = asyncio.create_task(self._decision_loop())
        log.info("Orchestrator started (eval every %.1fs, %d playbooks)",
                 self._eval_interval, len(self._playbooks))

    async def stop(self) -> None:
        self._running = False
        if self._task:
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass

    async def _decision_loop(self) -> None:
        # Wait a bit for sentinels to populate initial data
        await asyncio.sleep(15)

        while self._running:
            try:
                await self._evaluate()
            except asyncio.CancelledError:
                break
            except Exception:
                log.exception("Orchestrator decision loop error")
            await asyncio.sleep(self._eval_interval)

    async def _evaluate(self) -> None:
        """One decision cycle."""
        snap = self._health.snapshot()
        state = self._sm.state
        now = time.time()

        # Handle safe mode transitions
        if state == SystemState.SAFE_MODE and not self._safe_mode_active:
            self._enter_safe_mode()
        elif state != SystemState.SAFE_MODE and self._safe_mode_active:
            self._exit_safe_mode()

        # Match playbooks
        for pb in self._playbooks:
            if not self._playbook_matches(pb, snap, state):
                continue

            # Check cooldown
            last_run = self._playbook_cooldowns.get(pb.name, 0)
            if now - last_run < pb.cooldown_s:
                continue

            # Execute playbook
            log.warning("Triggering playbook: %s (%s)", pb.name, pb.description)
            self._audit.record_quick(
                "orchestrator", pb.severity.value, "orchestrator",
                "trigger_playbook", pb.name, pb.description,
                {"state": state.value, "score": snap.get("system_score")}
            )

            results = await self._execute_playbook(pb)
            self._playbook_cooldowns[pb.name] = now

            self._action_history.append({
                "playbook": pb.name,
                "ts": now,
                "results": [
                    {"action": f"{r.operator}.{r.action}",
                     "target": r.target,
                     "result": r.result.value,
                     "detail": r.detail}
                    for r in results
                ],
            })

    def _playbook_matches(self, pb: Playbook, snap: dict, state: SystemState) -> bool:
        """Check if a playbook's conditions are met."""
        # State condition
        if pb.condition_state and state != pb.condition_state:
            return False

        # Component score condition
        if pb.condition_component and pb.condition_score_below is not None:
            components = snap.get("components", {})
            comp = components.get(pb.condition_component)
            if comp is None:
                return False
            score = comp.get("score")
            if score is None or score >= pb.condition_score_below:
                return False

        # If no conditions specified, never auto-trigger
        if pb.condition_state is None and pb.condition_component is None:
            return False

        return True

    async def _execute_playbook(self, pb: Playbook) -> List[ActionRecord]:
        """Execute all actions in a playbook sequentially."""
        results = []
        for step in pb.actions:
            op = self._ops.get(step.operator)
            if op is None:
                log.error("Playbook %s: unknown operator '%s'", pb.name, step.operator)
                continue

            # Check circuit breaker
            cb = self._breakers.get(step.operator)
            if cb and not cb.allow():
                log.warning("Playbook %s: operator '%s' circuit breaker open",
                            pb.name, step.operator)
                continue

            record = await op.act(step.action, step.target,
                                   reason=f"playbook:{pb.name}", **step.kwargs)
            results.append(record)

            if record.result == OperatorResult.FAILED:
                if cb:
                    cb.record_failure()
            elif record.result == OperatorResult.SUCCESS:
                if cb:
                    cb.record_success()

        return results

    def _enter_safe_mode(self) -> None:
        log.warning("ENTERING SAFE MODE")
        self._safe_mode_active = True
        self._ops.set_safe_mode(True)
        self._audit.record_quick(
            "orchestrator", "critical", "orchestrator",
            "enter_safe_mode", "system", "health score critical", {}
        )

    def _exit_safe_mode(self) -> None:
        log.info("EXITING SAFE MODE")
        self._safe_mode_active = False
        self._ops.set_safe_mode(False)
        self._audit.record_quick(
            "orchestrator", "info", "orchestrator",
            "exit_safe_mode", "system", "health recovered", {}
        )

    # ── Manual overrides ─────────────────────────────────────────────

    async def force_playbook(self, name: str, reason: str = "manual") -> Optional[List[ActionRecord]]:
        """Manually trigger a playbook, bypassing cooldowns."""
        pb = next((p for p in self._playbooks if p.name == name), None)
        if pb is None:
            return None

        self._audit.record_quick(
            "orchestrator", "warning", "human",
            "force_playbook", name, reason, {}
        )
        results = await self._execute_playbook(pb)
        self._playbook_cooldowns[name] = time.time()
        return results

    def force_state(self, state: str, reason: str = "manual") -> bool:
        """Manually force a system state."""
        try:
            target = SystemState(state)
        except ValueError:
            return False
        self._sm.force_state(target, reason)
        self._audit.record_quick(
            "orchestrator", "warning", "human",
            "force_state", state, reason, {}
        )
        return True

    def reset_circuit_breaker(self, name: str) -> bool:
        cb = self._breakers.get(name)
        if cb is None:
            return False
        cb.reset()
        self._audit.record_quick(
            "orchestrator", "info", "human",
            "reset_circuit_breaker", name, "manual reset", {}
        )
        return True

    def add_playbook(self, pb: Playbook) -> None:
        self._playbooks.append(pb)

    # ── Snapshot ─────────────────────────────────────────────────────

    def snapshot(self) -> dict:
        return {
            "running": self._running,
            "safe_mode": self._safe_mode_active,
            "system_state": self._sm.state.value,
            "system_score": self._health.current_score(),
            "health": self._health.snapshot(),
            "operators": self._ops.snapshot(),
            "circuit_breakers": {
                name: {"state": cb.state.value, "failures": len(cb._failures)}
                for name, cb in self._breakers.all().items()
            },
            "playbooks": [
                {
                    "name": pb.name,
                    "description": pb.description,
                    "severity": pb.severity.value,
                    "cooldown_remaining": max(0, pb.cooldown_s - (
                        time.time() - self._playbook_cooldowns.get(pb.name, 0)
                    )),
                }
                for pb in self._playbooks
            ],
            "recent_actions": self._action_history[-20:],
        }
