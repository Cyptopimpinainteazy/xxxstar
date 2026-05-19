"""Degraded Mode Controller — CPU-only fallback when GPU is unavailable.

Manages the transition between operating modes:

  FULL_GPU      → All GPU kernels active, maximum throughput
  DEGRADED_GPU  → Partial GPU (e.g. one kernel falling back to CPU)
  CPU_ONLY      → Full CPU fallback, guaranteed liveness at reduced throughput
  EMERGENCY     → Minimal processing, only consensus-critical operations

The controller integrates with the existing ``allow_failover`` flag in
``Secp256k1BatchVerifier`` and ``KeccakBatchHasher`` and coordinates the
transition so that no requests are dropped during mode changes.

Design: External validators always see a response.  Throughput drops
in CPU mode, but correctness and liveness are preserved.  X3 is a
turbocharger, not a dependency.
"""

from __future__ import annotations

import logging
import threading
import time
from dataclasses import dataclass
from enum import Enum
from typing import Any, Callable

logger = logging.getLogger("x3.degraded")


class OperatingMode(Enum):
    """Operating mode of the GPU validator."""
    FULL_GPU = "full_gpu"           # All GPU kernels active
    DEGRADED_GPU = "degraded_gpu"   # Partial GPU availability
    CPU_ONLY = "cpu_only"           # Full CPU fallback
    EMERGENCY = "emergency"         # Minimal processing


# Throughput caps per mode (as fraction of FULL_GPU capacity)
_MODE_CAPACITY: dict[OperatingMode, float] = {
    OperatingMode.FULL_GPU: 1.0,
    OperatingMode.DEGRADED_GPU: 0.6,
    OperatingMode.CPU_ONLY: 0.15,
    OperatingMode.EMERGENCY: 0.05,
}

# Batch size limits per mode
_MODE_BATCH_LIMIT: dict[OperatingMode, int] = {
    OperatingMode.FULL_GPU: 16384,
    OperatingMode.DEGRADED_GPU: 8192,
    OperatingMode.CPU_ONLY: 2048,
    OperatingMode.EMERGENCY: 256,
}


@dataclass
class ModeTransition:
    """Record of a mode transition."""
    from_mode: OperatingMode
    to_mode: OperatingMode
    reason: str
    timestamp: float
    health_score: float


class DegradedModeController:
    """Controls operating mode transitions based on GPU availability.

    Parameters
    ----------
    on_mode_change : callable
        ``fn(from_mode, to_mode, reason)`` invoked on mode transition.
    initial_mode : OperatingMode
        Starting mode (default FULL_GPU).
    gpu_recovery_delay : float
        Seconds to wait after GPU recovery before upgrading mode (default 5.0).
        Prevents flapping on intermittent GPU issues.
    """

    def __init__(
        self,
        on_mode_change: Callable[[OperatingMode, OperatingMode, str], None] | None = None,
        initial_mode: OperatingMode = OperatingMode.FULL_GPU,
        gpu_recovery_delay: float = 5.0,
    ) -> None:
        self._on_mode_change = on_mode_change
        self._mode = initial_mode
        self._recovery_delay = gpu_recovery_delay

        self._lock = threading.Lock()
        self._transition_history: list[ModeTransition] = []
        self._last_transition_at = 0.0
        self._gpu_recovered_at: float | None = None
        self._total_transitions = 0

    # ── Mode Access ──────────────────────────────────────────

    @property
    def mode(self) -> OperatingMode:
        with self._lock:
            return self._mode

    @property
    def capacity(self) -> float:
        """Current throughput capacity as fraction of full GPU (0.0–1.0)."""
        with self._lock:
            return _MODE_CAPACITY[self._mode]

    @property
    def batch_limit(self) -> int:
        """Maximum batch size in current mode."""
        with self._lock:
            return _MODE_BATCH_LIMIT[self._mode]

    @property
    def is_degraded(self) -> bool:
        with self._lock:
            return self._mode != OperatingMode.FULL_GPU

    @property
    def is_emergency(self) -> bool:
        with self._lock:
            return self._mode == OperatingMode.EMERGENCY

    # ── Health-Driven Transitions ────────────────────────────

    def on_health_update(
        self, gpu_available: bool, health_score: float, gpu_temp_c: int = 0
    ) -> None:
        """Called by the health daemon on each probe cycle.

        Determines the appropriate operating mode based on GPU state
        and health score, then transitions if needed.
        """
        with self._lock:
            target = self._compute_target_mode(gpu_available, health_score, gpu_temp_c)
            if target == self._mode:
                # Reset recovery tracking if mode is stable
                if target == OperatingMode.FULL_GPU:
                    self._gpu_recovered_at = None
                return
            self._transition(target, self._build_reason(gpu_available, health_score), health_score)

    def _compute_target_mode(
        self, gpu_available: bool, score: float, temp_c: int
    ) -> OperatingMode:
        """Determine target mode from health signals (must be called under lock)."""
        if not gpu_available:
            if score < 0.1:
                return OperatingMode.EMERGENCY
            return OperatingMode.CPU_ONLY

        if temp_c >= 90:
            # Thermal emergency — throttle to CPU to cool GPU
            return OperatingMode.CPU_ONLY

        if score >= 0.7:
            # Want to upgrade to FULL — check recovery delay
            if self._mode != OperatingMode.FULL_GPU:
                now = time.monotonic()
                if self._gpu_recovered_at is None:
                    self._gpu_recovered_at = now
                if now - self._gpu_recovered_at < self._recovery_delay:
                    return self._mode  # Hold current mode during delay
            return OperatingMode.FULL_GPU

        if score >= 0.4:
            return OperatingMode.DEGRADED_GPU

        if score >= 0.15:
            return OperatingMode.CPU_ONLY

        return OperatingMode.EMERGENCY

    @staticmethod
    def _build_reason(gpu_available: bool, score: float) -> str:
        if not gpu_available:
            return "gpu_unavailable"
        if score < 0.15:
            return f"critical_health_score={score:.2f}"
        if score < 0.4:
            return f"low_health_score={score:.2f}"
        if score < 0.7:
            return f"degraded_health_score={score:.2f}"
        return f"healthy_score={score:.2f}"

    def _transition(
        self, target: OperatingMode, reason: str, health_score: float
    ) -> None:
        """Execute a mode transition (must be called under lock)."""
        old = self._mode
        self._mode = target
        self._last_transition_at = time.monotonic()
        self._total_transitions += 1
        self._gpu_recovered_at = None

        transition = ModeTransition(
            from_mode=old,
            to_mode=target,
            reason=reason,
            timestamp=time.time(),
            health_score=health_score,
        )
        self._transition_history.append(transition)
        # Keep history bounded
        if len(self._transition_history) > 100:
            self._transition_history = self._transition_history[-50:]

        logger.warning(
            "Mode transition #%d: %s → %s (reason: %s, score: %.2f)",
            self._total_transitions, old.value, target.value, reason, health_score,
        )

        if self._on_mode_change:
            try:
                self._on_mode_change(old, target, reason)
            except Exception:
                pass

    # ── Manual Controls ──────────────────────────────────────

    def force_mode(self, mode: OperatingMode, reason: str = "manual") -> None:
        """Force a specific operating mode."""
        with self._lock:
            if mode != self._mode:
                score = _MODE_CAPACITY.get(mode, 0.0)
                self._transition(mode, reason, score)

    def force_emergency(self) -> None:
        """Emergency shutdown of GPU processing."""
        self.force_mode(OperatingMode.EMERGENCY, "manual_emergency")

    def force_recovery(self) -> None:
        """Force recovery to full GPU mode (use with caution)."""
        self.force_mode(OperatingMode.FULL_GPU, "manual_recovery")

    # ── Query ────────────────────────────────────────────────

    def should_use_gpu(self) -> bool:
        """Whether GPU kernels should be used for the current batch."""
        with self._lock:
            return self._mode in (OperatingMode.FULL_GPU, OperatingMode.DEGRADED_GPU)

    def clamp_batch_size(self, requested: int) -> int:
        """Clamp a requested batch size to the current mode's limit."""
        limit = self.batch_limit
        return min(requested, limit)

    def status(self) -> dict[str, Any]:
        with self._lock:
            recent = []
            for t in self._transition_history[-5:]:
                recent.append({
                    "from": t.from_mode.value,
                    "to": t.to_mode.value,
                    "reason": t.reason,
                    "timestamp": t.timestamp,
                    "health_score": round(t.health_score, 3),
                })
            return {
                "mode": self._mode.value,
                "capacity": _MODE_CAPACITY[self._mode],
                "batch_limit": _MODE_BATCH_LIMIT[self._mode],
                "is_degraded": self._mode != OperatingMode.FULL_GPU,
                "total_transitions": self._total_transitions,
                "recent_transitions": recent,
            }
