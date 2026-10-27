"""Lane Orchestrator — three-tier execution lanes with deterministic failover.

Architecture
────────────
  Lane 1 (Primary GPU)    ──▶  Full GPU acceleration, lowest latency, signing authority
  Lane 2 (Shadow GPU)     ──▶  Hot standby, GPU-warmed, mirrors workload, NO signing
  Lane 3 (Tertiary CPU)   ──▶  CPU-only degraded mode, guaranteed liveness

Failover is deterministic and health-score-driven:
  - Primary drops below threshold → Shadow promoted in <3 seconds
  - Shadow unhealthy too → Tertiary activated (CPU fallback)
  - When Primary recovers → Shadow drains, Primary re-promoted
  - External validators ALWAYS retain native chain compute — X3 is
    an optional turbocharger, never a consensus dependency.

Integration
───────────
  The ``LaneOrchestrator`` plugs into ``GpuHealthDaemon`` via callbacks and
  wraps ``MultiChainOrchestrator`` so that inbound validation requests are
  routed to the best available lane.
"""

from __future__ import annotations

import logging
import threading
import time
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Any, Callable

from cross_chain_gpu_validator.resilience.circuit import CircuitBreaker
from cross_chain_gpu_validator.resilience.health import NodeHealth

logger = logging.getLogger("x3.lanes")


# ─── Enums ───────────────────────────────────────────────────


class LaneTier(Enum):
    """Execution tier priority (lower ordinal = higher priority)."""
    PRIMARY = 1    # GPU-accelerated, full speed
    SHADOW = 2     # Hot standby, GPU-warmed, no signing
    TERTIARY = 3   # CPU-only degraded fallback


class LaneStatus(Enum):
    """Lifecycle state of an individual lane."""
    ACTIVE = "active"         # Serving requests
    STANDBY = "standby"       # Ready to promote
    PROMOTING = "promoting"   # Being promoted (draining peer)
    DRAINING = "draining"     # Draining in-flight before demotion
    DEGRADED = "degraded"     # Running at reduced capacity
    OFFLINE = "offline"       # Not available


# ─── Lane Dataclass ──────────────────────────────────────────


@dataclass
class AccelerationLane:
    """One execution lane in the 3-tier system."""

    tier: LaneTier
    status: LaneStatus = LaneStatus.STANDBY
    breaker: CircuitBreaker | None = None

    # Performance tracking
    requests_served: int = 0
    last_request_at: float = 0.0
    avg_latency_ms: float = 0.0
    _latency_sum: float = 0.0

    # GPU details (None for TERTIARY)
    gpu_device_id: int | None = None
    gpu_warmed: bool = False

    # Promotion tracking
    promoted_at: float = 0.0
    promotions: int = 0

    @property
    def is_gpu(self) -> bool:
        return self.tier in (LaneTier.PRIMARY, LaneTier.SHADOW)

    @property
    def available(self) -> bool:
        return self.status in (LaneStatus.ACTIVE, LaneStatus.STANDBY, LaneStatus.DEGRADED)

    def record_latency(self, ms: float) -> None:
        self.requests_served += 1
        self.last_request_at = time.monotonic()
        self._latency_sum += ms
        self.avg_latency_ms = self._latency_sum / self.requests_served

    def to_dict(self) -> dict[str, Any]:
        return {
            "tier": self.tier.name,
            "status": self.status.value,
            "requests_served": self.requests_served,
            "avg_latency_ms": round(self.avg_latency_ms, 3),
            "gpu_device_id": self.gpu_device_id,
            "gpu_warmed": self.gpu_warmed,
            "promotions": self.promotions,
        }


# ─── Lane Orchestrator ──────────────────────────────────────


class LaneOrchestrator:
    """Routes validation requests to the best available execution lane.

    Parameters
    ----------
    health_threshold : float
        Health score below which the active lane fails over (default 0.5).
    promotion_cooldown : float
        Seconds to wait between promotions to prevent flapping (default 10).
    on_failover : callable
        ``fn(from_tier, to_tier)`` invoked on lane failover.
    on_recovery : callable
        ``fn(tier)`` invoked when a lane recovers.
    """

    def __init__(
        self,
        health_threshold: float = 0.5,
        promotion_cooldown: float = 10.0,
        on_failover: Callable[[LaneTier, LaneTier], None] | None = None,
        on_recovery: Callable[[LaneTier], None] | None = None,
    ) -> None:
        self._threshold = health_threshold
        self._cooldown = promotion_cooldown
        self._on_failover = on_failover
        self._on_recovery = on_recovery

        self._lock = threading.Lock()
        self._last_promotion_at = 0.0
        self._active_tier = LaneTier.PRIMARY
        self._failover_count = 0

        # Initialize 3 lanes
        self._lanes: dict[LaneTier, AccelerationLane] = {
            LaneTier.PRIMARY: AccelerationLane(
                tier=LaneTier.PRIMARY,
                status=LaneStatus.ACTIVE,
                breaker=CircuitBreaker("lane-primary", failure_threshold=3, recovery_seconds=15),
                gpu_device_id=0,
            ),
            LaneTier.SHADOW: AccelerationLane(
                tier=LaneTier.SHADOW,
                status=LaneStatus.STANDBY,
                breaker=CircuitBreaker("lane-shadow", failure_threshold=3, recovery_seconds=15),
                gpu_device_id=1,
            ),
            LaneTier.TERTIARY: AccelerationLane(
                tier=LaneTier.TERTIARY,
                status=LaneStatus.STANDBY,
                breaker=CircuitBreaker("lane-tertiary", failure_threshold=10, recovery_seconds=5),
                gpu_device_id=None,
            ),
        }

    # ── Lane Selection ───────────────────────────────────────

    @property
    def active_lane(self) -> AccelerationLane:
        with self._lock:
            return self._lanes[self._active_tier]

    @property
    def active_tier(self) -> LaneTier:
        with self._lock:
            return self._active_tier

    def get_lane(self, tier: LaneTier) -> AccelerationLane:
        return self._lanes[tier]

    def select_lane(self) -> AccelerationLane:
        """Select the best available lane for an incoming request.

        Priority: PRIMARY > SHADOW > TERTIARY.
        Checks circuit breaker state and lane availability.
        """
        with self._lock:
            # Try active lane first
            active = self._lanes[self._active_tier]
            if active.available and (active.breaker is None or active.breaker.allow_request()):
                return active

            # Fallback through tiers
            for tier in LaneTier:
                lane = self._lanes[tier]
                if lane.available and (lane.breaker is None or lane.breaker.allow_request()):
                    return lane

            # Everything down — return tertiary anyway (CPU always available)
            return self._lanes[LaneTier.TERTIARY]

    # ── Execute Through Lane ─────────────────────────────────

    def execute(self, fn: Callable[..., Any], *args: Any, **kwargs: Any) -> Any:
        """Execute a validation function through the best available lane.

        Records latency and drives circuit breaker state.
        If the selected lane fails, automatically tries the next tier.
        """
        attempted: set[LaneTier] = set()

        for tier in LaneTier:
            if tier in attempted:
                continue
            lane = self._lanes[tier]
            if not lane.available:
                attempted.add(tier)
                continue

            attempted.add(tier)
            start = time.monotonic()
            try:
                result = fn(*args, **kwargs)
                elapsed_ms = (time.monotonic() - start) * 1000
                lane.record_latency(elapsed_ms)
                if lane.breaker:
                    lane.breaker.record_success()
                return result
            except Exception as exc:
                elapsed_ms = (time.monotonic() - start) * 1000
                lane.record_latency(elapsed_ms)
                if lane.breaker:
                    lane.breaker.record_failure()
                logger.warning(
                    "Lane %s failed (%.1fms): %s — trying next tier",
                    tier.name, elapsed_ms, exc,
                )
                continue

        tertiary = self._lanes[LaneTier.TERTIARY]
        if tertiary.available:
            start = time.monotonic()
            try:
                result = fn(*args, **kwargs)
                elapsed_ms = (time.monotonic() - start) * 1000
                tertiary.record_latency(elapsed_ms)
                if tertiary.breaker:
                    tertiary.breaker.record_success()
                return result
            except Exception as exc:
                elapsed_ms = (time.monotonic() - start) * 1000
                tertiary.record_latency(elapsed_ms)
                if tertiary.breaker:
                    tertiary.breaker.record_failure()
                logger.warning(
                    "Lane %s final retry failed (%.1fms): %s",
                    LaneTier.TERTIARY.name,
                    elapsed_ms,
                    exc,
                )

        raise RuntimeError("All execution lanes exhausted — no lane could serve the request")

    # ── Health-Driven Failover ───────────────────────────────

    def on_health_update(self, health: NodeHealth) -> None:
        """Called by GpuHealthDaemon on every probe cycle.

        Drives lane promotions/demotions based on health score.
        """
        score = health.score.overall
        gpu_ok = health.gpu.available
        now = time.monotonic()

        with self._lock:
            active = self._lanes[self._active_tier]

            # Check if active lane should be demoted
            if score < self._threshold or (active.is_gpu and not gpu_ok):
                if now - self._last_promotion_at < self._cooldown:
                    return  # Prevent flapping
                self._promote_next(active, now)
                return

            # Check if a higher-priority lane can be recovered
            if self._active_tier != LaneTier.PRIMARY and gpu_ok and score >= self._threshold + 0.1:
                # Hysteresis: require score 0.1 above threshold to re-promote
                if now - self._last_promotion_at < self._cooldown:
                    return
                primary = self._lanes[LaneTier.PRIMARY]
                if primary.breaker is None or primary.breaker.allow_request():
                    self._promote_to(LaneTier.PRIMARY, now)

    def _promote_next(self, current: AccelerationLane, now: float) -> None:
        """Promote the next available tier (must be called under lock)."""
        current_idx = current.tier.value
        for tier in LaneTier:
            if tier.value <= current_idx:
                continue
            candidate = self._lanes[tier]
            if candidate.available or tier == LaneTier.TERTIARY:
                self._do_promote(current.tier, tier, now)
                return

        # If no next tier found, force tertiary
        self._do_promote(current.tier, LaneTier.TERTIARY, now)

    def _promote_to(self, target: LaneTier, now: float) -> None:
        """Promote to a specific tier (must be called under lock)."""
        self._do_promote(self._active_tier, target, now)

    def _do_promote(self, from_tier: LaneTier, to_tier: LaneTier, now: float) -> None:
        """Execute a lane promotion (must be called under lock)."""
        if from_tier == to_tier:
            return

        old = self._lanes[from_tier]
        new = self._lanes[to_tier]

        old.status = LaneStatus.DRAINING
        new.status = LaneStatus.PROMOTING

        self._active_tier = to_tier
        self._last_promotion_at = now
        self._failover_count += 1

        new.status = LaneStatus.ACTIVE
        new.promoted_at = now
        new.promotions += 1

        old.status = LaneStatus.STANDBY if old.is_gpu else LaneStatus.OFFLINE

        logger.warning(
            "Lane failover #%d: %s → %s", self._failover_count, from_tier.name, to_tier.name
        )

        if self._on_failover:
            try:
                self._on_failover(from_tier, to_tier)
            except Exception:
                pass

    # ── Manual Controls ──────────────────────────────────────

    def force_failover(self, to_tier: LaneTier) -> None:
        """Manually force failover to a specific tier."""
        with self._lock:
            self._do_promote(self._active_tier, to_tier, time.monotonic())

    def set_lane_status(self, tier: LaneTier, status: LaneStatus) -> None:
        """Override a lane's status."""
        with self._lock:
            self._lanes[tier].status = status

    def warm_shadow(self) -> None:
        """Mark Shadow lane as GPU-warmed and ready for instant promotion."""
        with self._lock:
            shadow = self._lanes[LaneTier.SHADOW]
            shadow.gpu_warmed = True
            shadow.status = LaneStatus.STANDBY

    # ── Status ───────────────────────────────────────────────

    def status(self) -> dict[str, Any]:
        with self._lock:
            return {
                "active_tier": self._active_tier.name,
                "failover_count": self._failover_count,
                "last_promotion_at": self._last_promotion_at,
                "lanes": {
                    tier.name: lane.to_dict() for tier, lane in self._lanes.items()
                },
            }

    @property
    def failover_count(self) -> int:
        with self._lock:
            return self._failover_count
