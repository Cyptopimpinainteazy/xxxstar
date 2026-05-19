"""Autonomic Control Plane — Recovery State Machine.

Defines the system-wide operational state and transition rules.

States:
    NORMAL           Everything healthy. Full capacity.
    DEGRADED         Some subsystems stressed. Alerts active.
    CONTAINMENT      Active failure isolation. Reduced capacity.
    SAFE_MODE        Minimal operations. Survival mode.
    MANUAL_REQUIRED  Human intervention needed. Auto-ops paused.

Transitions are driven by health scores from the HealthEngine.
"""

from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Any, Callable, Dict, List, Optional, Tuple

log = logging.getLogger("autonomic.state_machine")


class SystemState(str, Enum):
    NORMAL = "normal"
    DEGRADED = "degraded"
    CONTAINMENT = "containment"
    SAFE_MODE = "safe_mode"
    MANUAL_REQUIRED = "manual_required"


# Valid transitions (from → set of allowed destinations)
_VALID_TRANSITIONS: Dict[SystemState, set] = {
    SystemState.NORMAL:          {SystemState.DEGRADED},
    SystemState.DEGRADED:        {SystemState.NORMAL, SystemState.CONTAINMENT},
    SystemState.CONTAINMENT:     {SystemState.DEGRADED, SystemState.SAFE_MODE},
    SystemState.SAFE_MODE:       {SystemState.CONTAINMENT, SystemState.MANUAL_REQUIRED},
    SystemState.MANUAL_REQUIRED: {SystemState.SAFE_MODE, SystemState.NORMAL},  # human can jump to NORMAL
}

# Severity ordering for escalation comparison
_SEVERITY_ORDER = {
    SystemState.NORMAL: 0,
    SystemState.DEGRADED: 1,
    SystemState.CONTAINMENT: 2,
    SystemState.SAFE_MODE: 3,
    SystemState.MANUAL_REQUIRED: 4,
}


@dataclass
class StateTransition:
    """Record of a state transition."""
    from_state: SystemState
    to_state: SystemState
    reason: str
    trigger: str              # "health_score", "circuit_breaker", "human", "sentinel"
    score_at_transition: int
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return {
            "from": self.from_state.value,
            "to": self.to_state.value,
            "reason": self.reason,
            "trigger": self.trigger,
            "score": self.score_at_transition,
            "ts": self.ts,
        }


# Callback type: (old_state, new_state, transition) → None
StateChangeCallback = Callable[[SystemState, SystemState, StateTransition], Any]


class RecoveryStateMachine:
    """System-wide recovery state machine with transition rules.

    Thresholds (configurable):
        score >= 75 → NORMAL
        score >= 60 → DEGRADED
        score >= 40 → CONTAINMENT
        score >= 20 → SAFE_MODE
        score <  20 → MANUAL_REQUIRED
    """

    def __init__(
        self,
        normal_min: int = 75,
        degraded_min: int = 60,
        containment_min: int = 40,
        safe_mode_min: int = 20,
        thresholds: Optional[Any] = None,
    ):
        self._state = SystemState.NORMAL
        if thresholds and hasattr(thresholds, "normal_min"):
            normal_min = thresholds.normal_min
            degraded_min = thresholds.degraded_min
            containment_min = thresholds.containment_min
            safe_mode_min = thresholds.safe_mode_min
        self._thresholds = {
            SystemState.NORMAL: normal_min,
            SystemState.DEGRADED: degraded_min,
            SystemState.CONTAINMENT: containment_min,
            SystemState.SAFE_MODE: safe_mode_min,
        }
        self._history: List[StateTransition] = []
        self._entered_at = time.time()
        self._callbacks: List[StateChangeCallback] = []
        self._locked = False  # human can lock state

    @property
    def state(self) -> SystemState:
        return self._state

    @property
    def severity(self) -> int:
        return _SEVERITY_ORDER[self._state]

    @property
    def time_in_state_s(self) -> float:
        return time.time() - self._entered_at

    @property
    def is_safe_mode(self) -> bool:
        return self._state in (SystemState.SAFE_MODE, SystemState.MANUAL_REQUIRED)

    @property
    def is_degraded_or_worse(self) -> bool:
        return _SEVERITY_ORDER[self._state] >= 1

    def on_state_change(self, callback: StateChangeCallback) -> None:
        """Register callback for state transitions."""
        self._callbacks.append(callback)

    def evaluate_score(self, score: int, trigger: str = "health_score") -> Optional[StateTransition]:
        """Evaluate system score and transition if thresholds crossed.

        Returns the transition if one occurred, else None.
        Allow gradual recovery (one step at a time) and rapid degradation
        (can skip levels for fast-dropping scores).
        """
        if self._locked:
            return None

        target = self._score_to_state(score)

        if target == self._state:
            return None

        target_sev = _SEVERITY_ORDER[target]
        current_sev = _SEVERITY_ORDER[self._state]

        if target_sev > current_sev:
            # Degrading — allow skipping (fast escalation)
            reason = f"Score {score} below {self._state.value} threshold"
            return self._do_transition(target, reason, trigger, score)
        elif target_sev < current_sev:
            # Recovering — step one level at a time (conservative)
            # Find the next state up (lower severity)
            ordered = sorted(_SEVERITY_ORDER.items(), key=lambda x: x[1])
            for st, sev in ordered:
                if sev == current_sev - 1:
                    if score >= self._thresholds.get(st, 0):
                        reason = f"Score {score} recovered above {st.value} threshold"
                        return self._do_transition(st, reason, trigger, score)
                    break
        return None

    def force_state(self, target: SystemState, reason: str, trigger: str = "human") -> StateTransition:
        """Force a state transition (human override). Bypasses rules."""
        self._locked = False
        return self._do_transition(target, reason, trigger, score_at_transition=-1)

    def lock(self) -> None:
        """Lock the current state (prevent automatic transitions)."""
        self._locked = True
        log.warning("State machine LOCKED at %s by human override", self._state.value)

    def unlock(self) -> None:
        """Unlock state machine for automatic transitions."""
        self._locked = False
        log.info("State machine UNLOCKED")

    def history(self, n: int = 20) -> List[dict]:
        return [t.to_dict() for t in self._history[-n:]]

    def snapshot(self) -> dict:
        return {
            "state": self._state.value,
            "severity": self.severity,
            "time_in_state_s": round(self.time_in_state_s, 1),
            "locked": self._locked,
            "transitions": len(self._history),
            "recent": self.history(5),
        }

    # ── Internal ──────────────────────────────────────────────────────────
    def _score_to_state(self, score: int) -> SystemState:
        if score >= self._thresholds[SystemState.NORMAL]:
            return SystemState.NORMAL
        if score >= self._thresholds[SystemState.DEGRADED]:
            return SystemState.DEGRADED
        if score >= self._thresholds[SystemState.CONTAINMENT]:
            return SystemState.CONTAINMENT
        if score >= self._thresholds[SystemState.SAFE_MODE]:
            return SystemState.SAFE_MODE
        return SystemState.MANUAL_REQUIRED

    def _do_transition(self, target: SystemState, reason: str,
                       trigger: str, score_at_transition: int) -> StateTransition:
        old = self._state
        tx = StateTransition(
            from_state=old,
            to_state=target,
            reason=reason,
            trigger=trigger,
            score_at_transition=score_at_transition,
        )
        self._state = target
        self._entered_at = time.time()
        self._history.append(tx)

        log.warning("STATE TRANSITION: %s → %s | %s | score=%s",
                     old.value, target.value, reason, score_at_transition)

        for cb in self._callbacks:
            try:
                cb(old, target, tx)
            except Exception:
                log.exception("State change callback failed")

        return tx
