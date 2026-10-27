"""Circuit Breaker — prevents cascading failures in GPU / Redis / RPC subsystems.

States: CLOSED (healthy) → OPEN (tripped) → HALF_OPEN (probing recovery).

When a subsystem fails ``failure_threshold`` times within ``window_seconds``,
the breaker opens and all calls short-circuit for ``recovery_seconds``.
After recovery timeout a single probe call is allowed; if it succeeds the
breaker closes, otherwise it stays open.
"""

from __future__ import annotations

import threading
import time
from enum import Enum
from typing import Callable, TypeVar, Any

T = TypeVar("T")


class CircuitState(Enum):
    CLOSED = "closed"
    OPEN = "open"
    HALF_OPEN = "half_open"


class CircuitBreaker:
    """Thread-safe circuit breaker for any subsystem.

    Parameters
    ----------
    name : str
        Human-readable identifier (e.g. ``"redis"``, ``"gpu"``, ``"rpc-eth"``).
    failure_threshold : int
        Consecutive failures to trip (default 5).
    recovery_seconds : float
        How long to stay open before probing (default 30).
    window_seconds : float
        Sliding window for failure counting (default 60).
    on_open : callable
        Invoked when breaker trips open.
    on_close : callable
        Invoked when breaker returns to closed.
    """

    def __init__(
        self,
        name: str,
        failure_threshold: int = 5,
        recovery_seconds: float = 30.0,
        window_seconds: float = 60.0,
        on_open: Callable[[str], None] | None = None,
        on_close: Callable[[str], None] | None = None,
    ) -> None:
        self.name = name
        self._threshold = failure_threshold
        self._recovery_seconds = recovery_seconds
        self._window_seconds = window_seconds
        self._on_open = on_open
        self._on_close = on_close

        self._lock = threading.Lock()
        self._state = CircuitState.CLOSED
        self._failures: list[float] = []
        self._last_failure: float = 0.0
        self._opened_at: float = 0.0
        self._total_trips: int = 0

    @property
    def state(self) -> CircuitState:
        with self._lock:
            return self._get_state()

    def _get_state(self) -> CircuitState:
        """Compute current state (must be called under lock)."""
        if self._state == CircuitState.OPEN:
            if time.monotonic() - self._opened_at >= self._recovery_seconds:
                self._state = CircuitState.HALF_OPEN
        return self._state

    def record_success(self) -> None:
        """Record a successful call — resets breaker to CLOSED."""
        with self._lock:
            was_open = self._state != CircuitState.CLOSED
            self._failures.clear()
            self._state = CircuitState.CLOSED
            if was_open and self._on_close:
                try:
                    self._on_close(self.name)
                except Exception:
                    pass

    def record_failure(self) -> None:
        """Record a failure — may trip the breaker."""
        with self._lock:
            now = time.monotonic()
            self._last_failure = now

            # Trim old failures outside window
            cutoff = now - self._window_seconds
            self._failures = [t for t in self._failures if t > cutoff]
            self._failures.append(now)

            if len(self._failures) >= self._threshold and self._state == CircuitState.CLOSED:
                self._state = CircuitState.OPEN
                self._opened_at = now
                self._total_trips += 1
                if self._on_open:
                    try:
                        self._on_open(self.name)
                    except Exception:
                        pass

    def allow_request(self) -> bool:
        """Check if a request should be allowed through."""
        with self._lock:
            state = self._get_state()
            if state == CircuitState.CLOSED:
                return True
            if state == CircuitState.HALF_OPEN:
                return True  # Allow one probe request
            return False

    def call(self, fn: Callable[..., T], *args: Any, **kwargs: Any) -> T:
        """Execute ``fn`` through the breaker.  Raises ``CircuitOpenError``
        if the breaker is open."""
        if not self.allow_request():
            raise CircuitOpenError(self.name)
        try:
            result = fn(*args, **kwargs)
            self.record_success()
            return result
        except Exception as exc:
            self.record_failure()
            raise

    def to_dict(self) -> dict:
        with self._lock:
            return {
                "name": self.name,
                "state": self._get_state().value,
                "failures_in_window": len(self._failures),
                "threshold": self._threshold,
                "total_trips": self._total_trips,
                "recovery_seconds": self._recovery_seconds,
            }


class CircuitOpenError(Exception):
    """Raised when a circuit breaker is open and rejects the call."""

    def __init__(self, breaker_name: str) -> None:
        self.breaker_name = breaker_name
        super().__init__(f"Circuit breaker '{breaker_name}' is OPEN — request rejected")
