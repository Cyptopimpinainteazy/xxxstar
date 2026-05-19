"""Health Engine — Aggregates sentinel scores into system-wide health.

The Health Engine is the central nervous system of the autonomic layer.
It:
  1. Subscribes to all ComponentHealth updates on the MetricsBus
  2. Maintains a weighted composite score
  3. Drives the RecoveryStateMachine transitions
  4. Exposes a snapshot for the orchestrator and API

Design rules:
  - Recovery requires N consecutive healthy polls (hysteresis)
  - Degradation is immediate (fail-fast)
  - Safe Mode can only be exited manually or after sustained recovery
"""

from __future__ import annotations

import asyncio
import logging
import time
from collections import deque
from dataclasses import dataclass, field
from typing import Any, Callable, Deque, Dict, List, Optional

from .config import AutonomicConfig, HealthThresholds
from .metrics_bus import MetricsBus, ComponentHealth
from .state_machine import RecoveryStateMachine, SystemState
from .circuit_breaker import CircuitBreakerRegistry

log = logging.getLogger("autonomic.health_engine")

# How many polls a component can miss before we assume it's dead
MAX_SILENT_POLLS = 6


@dataclass
class ComponentRecord:
    """Tracked state for a single component."""
    name: str
    last_health: Optional[ComponentHealth] = None
    last_update: float = 0.0
    consecutive_healthy: int = 0
    consecutive_unhealthy: int = 0
    weight: float = 1.0


@dataclass
class HealthSnapshot:
    """Point-in-time health picture for API / orchestrator."""
    system_score: float
    system_state: str
    components: Dict[str, dict]
    circuit_breakers: Dict[str, str]
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return {
            "system_score": round(self.system_score, 1),
            "system_state": self.system_state,
            "components": self.components,
            "circuit_breakers": self.circuit_breakers,
            "ts": self.ts,
        }


class HealthEngine:
    """Aggregates per-component health into system-wide score and state."""

    # Default weights for known components
    DEFAULT_WEIGHTS: Dict[str, float] = {
        "gpu": 2.0,       # GPUs are mission-critical
        "resources": 1.5,  # RAM/disk important
        "logs": 1.0,       # log anomalies moderate
        "swarm": 1.5,      # swarm API health
    }

    def __init__(
        self,
        bus: MetricsBus,
        state_machine: RecoveryStateMachine,
        breakers: CircuitBreakerRegistry,
        config: Optional[AutonomicConfig] = None,
        on_state_change: Optional[Callable[[SystemState, SystemState], Any]] = None,
    ):
        self._bus = bus
        self._sm = state_machine
        self._breakers = breakers
        self._cfg = config or AutonomicConfig()
        self._thresholds = self._cfg.health
        self._on_state_change = on_state_change

        self._components: Dict[str, ComponentRecord] = {}
        self._history: Deque[HealthSnapshot] = deque(maxlen=360)  # ~30 min @ 5s
        self._running = False
        self._task: Optional[asyncio.Task] = None
        self._poll_interval = 5.0  # seconds
        self._recovery_threshold = 5  # consecutive healthy polls needed

        # Subscribe to all health updates
        self._bus.subscribe_health("*", self._on_health_update)

    async def start(self) -> None:
        self._running = True
        self._task = asyncio.create_task(self._eval_loop())
        log.info("Health Engine started (eval every %.1fs)", self._poll_interval)

    async def stop(self) -> None:
        self._running = False
        if self._task:
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass

    async def _on_health_update(self, health: ComponentHealth) -> None:
        """Callback from MetricsBus for ComponentHealth publications."""
        name = health.component
        if name not in self._components:
            weight = self.DEFAULT_WEIGHTS.get(name, 1.0)
            self._components[name] = ComponentRecord(name=name, weight=weight)

        rec = self._components[name]
        rec.last_health = health
        rec.last_update = time.time()

        if health.score >= self._thresholds.normal_min:
            rec.consecutive_healthy += 1
            rec.consecutive_unhealthy = 0
        else:
            rec.consecutive_unhealthy += 1
            rec.consecutive_healthy = 0

    async def _eval_loop(self) -> None:
        """Periodic evaluation of system health."""
        while self._running:
            try:
                await self._evaluate()
            except asyncio.CancelledError:
                break
            except Exception:
                log.exception("Health Engine eval error")
            await asyncio.sleep(self._poll_interval)

    async def _evaluate(self) -> None:
        now = time.time()
        score = self._compute_system_score(now)
        prev_state = self._sm.state

        # Drive state machine
        self._sm.evaluate_score(score)

        new_state = self._sm.state
        if new_state != prev_state:
            log.warning("System state: %s → %s (score=%.1f)",
                        prev_state.value, new_state.value, score)
            if self._on_state_change:
                try:
                    result = self._on_state_change(prev_state, new_state)
                    if asyncio.iscoroutine(result):
                        await result
                except Exception:
                    log.exception("State change callback failed")

        # Snapshot
        snap = HealthSnapshot(
            system_score=score,
            system_state=new_state.value,
            components={
                name: {
                    "score": rec.last_health.score if rec.last_health else None,
                    "status": rec.last_health.status if rec.last_health else "unknown",
                    "details": rec.last_health.details if rec.last_health else {},
                    "consecutive_healthy": rec.consecutive_healthy,
                    "stale": (now - rec.last_update) > (self._poll_interval * MAX_SILENT_POLLS),
                    "weight": rec.weight,
                }
                for name, rec in self._components.items()
            },
            circuit_breakers={
                name: cb.state.value
                for name, cb in self._breakers.all().items()
            },
        )
        self._history.append(snap)

    def _compute_system_score(self, now: float) -> float:
        """Weighted average of all component scores."""
        if not self._components:
            return 100.0  # no components yet = assume healthy

        total_weight = 0.0
        weighted_sum = 0.0

        for rec in self._components.values():
            if rec.last_health is None:
                continue

            # Stale components get a penalty
            age = now - rec.last_update
            if age > self._poll_interval * MAX_SILENT_POLLS:
                component_score = 30.0  # treat stale as degraded
            else:
                component_score = rec.last_health.score

            # Circuit breaker penalty
            cb = self._breakers.get(rec.name)
            if cb and cb.state.value == "open":
                component_score = min(component_score, 20.0)

            weighted_sum += component_score * rec.weight
            total_weight += rec.weight

        if total_weight == 0:
            return 100.0

        return weighted_sum / total_weight

    def current_score(self) -> float:
        return self._compute_system_score(time.time())

    def current_state(self) -> SystemState:
        return self._sm.state

    def snapshot(self) -> dict:
        if self._history:
            return self._history[-1].to_dict()
        return HealthSnapshot(
            system_score=self.current_score(),
            system_state=self._sm.state.value,
            components={},
            circuit_breakers={},
        ).to_dict()

    def history(self, n: int = 60) -> List[dict]:
        return [s.to_dict() for s in list(self._history)[-n:]]
