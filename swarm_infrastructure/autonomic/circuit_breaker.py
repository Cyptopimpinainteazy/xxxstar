"""Autonomic Control Plane — Circuit Breaker.

Per-module failure isolation. Prevents runaway restart loops and cascade failures.

States:
    CLOSED     → Normal operation. Failures counted in sliding window.
    OPEN       → Module quarantined. All calls rejected. Timer running.
    HALF_OPEN  → Trial period. Limited calls allowed to test recovery.
"""

from __future__ import annotations

import asyncio
import logging
import time
from collections import deque
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, Deque, Dict, Optional

log = logging.getLogger("autonomic.circuit_breaker")


class CBState(str, Enum):
    CLOSED = "closed"
    OPEN = "open"
    HALF_OPEN = "half_open"


@dataclass
class CircuitBreakerStats:
    """Observability snapshot for a single breaker."""
    module: str
    state: CBState
    failure_count: int
    success_count: int
    total_calls: int
    last_failure_ts: Optional[float]
    opened_at: Optional[float]
    half_open_successes: int
    trips: int               # total times tripped open

    def to_dict(self) -> dict:
        return {
            "module": self.module,
            "state": self.state.value,
            "failure_count": self.failure_count,
            "success_count": self.success_count,
            "total_calls": self.total_calls,
            "last_failure_ts": self.last_failure_ts,
            "opened_at": self.opened_at,
            "half_open_successes": self.half_open_successes,
            "trips": self.trips,
        }


class CircuitBreaker:
    """Circuit breaker for a single module/subsystem."""

    def __init__(
        self,
        module: str,
        failure_threshold: int = 5,
        recovery_timeout_s: float = 60.0,
        half_open_max: int = 2,
        window_s: float = 300.0,
        on_state_change: Optional[Callable] = None,
    ):
        self.module = module
        self.failure_threshold = failure_threshold
        self.recovery_timeout_s = recovery_timeout_s
        self.half_open_max = half_open_max
        self.window_s = window_s
        self._on_state_change = on_state_change

        self._state = CBState.CLOSED
        self._failures: Deque[float] = deque()   # timestamps of failures
        self._success_count = 0
        self._total_calls = 0
        self._last_failure_ts: Optional[float] = None
        self._opened_at: Optional[float] = None
        self._half_open_successes = 0
        self._trips = 0

    @property
    def state(self) -> CBState:
        # Auto-transition OPEN → HALF_OPEN after timeout
        if self._state == CBState.OPEN and self._opened_at:
            if time.time() - self._opened_at >= self.recovery_timeout_s:
                self._transition(CBState.HALF_OPEN)
        return self._state

    @property
    def is_available(self) -> bool:
        """Can calls go through?"""
        s = self.state  # triggers auto-transition check
        if s == CBState.CLOSED:
            return True
        if s == CBState.HALF_OPEN:
            return True
        return False

    def record_success(self) -> None:
        """Record a successful call."""
        self._total_calls += 1
        self._success_count += 1

        if self._state == CBState.HALF_OPEN:
            self._half_open_successes += 1
            if self._half_open_successes >= self.half_open_max:
                self._transition(CBState.CLOSED)

    def record_failure(self, reason: str = "") -> None:
        """Record a failed call."""
        now = time.time()
        self._total_calls += 1
        self._failures.append(now)
        self._last_failure_ts = now

        # Evict old failures outside window
        cutoff = now - self.window_s
        while self._failures and self._failures[0] < cutoff:
            self._failures.popleft()

        if self._state == CBState.HALF_OPEN:
            # Any failure in half-open → back to OPEN
            self._transition(CBState.OPEN)
            return

        if len(self._failures) >= self.failure_threshold:
            self._transition(CBState.OPEN)

    def reset(self) -> None:
        """Manual reset (human override)."""
        self._failures.clear()
        self._half_open_successes = 0
        self._transition(CBState.CLOSED)

    def trip(self, reason: str = "manual") -> None:
        """Manually trip the breaker open."""
        self._transition(CBState.OPEN)

    def stats(self) -> CircuitBreakerStats:
        return CircuitBreakerStats(
            module=self.module,
            state=self.state,
            failure_count=len(self._failures),
            success_count=self._success_count,
            total_calls=self._total_calls,
            last_failure_ts=self._last_failure_ts,
            opened_at=self._opened_at,
            half_open_successes=self._half_open_successes,
            trips=self._trips,
        )

    def _transition(self, new_state: CBState) -> None:
        old = self._state
        if old == new_state:
            return
        self._state = new_state

        if new_state == CBState.OPEN:
            self._opened_at = time.time()
            self._half_open_successes = 0
            self._trips += 1
            log.warning("Circuit breaker OPEN: %s (trip #%d)", self.module, self._trips)
        elif new_state == CBState.HALF_OPEN:
            self._half_open_successes = 0
            log.info("Circuit breaker HALF_OPEN: %s", self.module)
        elif new_state == CBState.CLOSED:
            self._failures.clear()
            self._opened_at = None
            log.info("Circuit breaker CLOSED: %s", self.module)

        if self._on_state_change:
            try:
                self._on_state_change(self.module, old, new_state)
            except Exception:
                log.exception("Circuit breaker state-change callback failed")


# ---------------------------------------------------------------------------
# Registry — manages all breakers
# ---------------------------------------------------------------------------
class CircuitBreakerRegistry:
    """Central registry of all circuit breakers."""

    def __init__(self, default_config: Optional[Dict[str, Any]] = None):
        self._breakers: Dict[str, CircuitBreaker] = {}
        self._defaults = default_config or {}

    def register(self, name: str, **kwargs) -> CircuitBreaker:
        """Register (create) a named circuit breaker."""
        merged = {**self._defaults, **kwargs, "module": name}
        cb = CircuitBreaker(**merged)
        self._breakers[name] = cb
        return cb

    def get_or_create(self, module: str, **overrides) -> CircuitBreaker:
        """Get existing breaker or create with defaults."""
        if module not in self._breakers:
            return self.register(module, **overrides)
        return self._breakers[module]

    def get(self, module: str) -> Optional[CircuitBreaker]:
        return self._breakers.get(module)

    def all(self) -> Dict[str, CircuitBreaker]:
        """Return all registered breakers."""
        return dict(self._breakers)

    def all_stats(self) -> Dict[str, dict]:
        return {name: cb.stats().to_dict() for name, cb in self._breakers.items()}

    def any_open(self) -> bool:
        return any(cb.state == CBState.OPEN for cb in self._breakers.values())

    def open_breakers(self) -> list[str]:
        return [name for name, cb in self._breakers.items() if cb.state == CBState.OPEN]

    def reset_all(self) -> None:
        """Human override: reset all breakers."""
        for cb in self._breakers.values():
            cb.reset()
