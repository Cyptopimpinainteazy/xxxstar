# X3 Autonomic Control Plane
"""
swarm/autonomic/ — Self-monitoring, self-healing, self-optimizing control layer.

Architecture (5 layers):
    Layer 0 — MetricsBus         Centralized telemetry pub/sub
    Layer 1 — HealthEngine       Subsystem scoring (0-100)
    Layer 2 — Sentinels          GPU Guard, Log Watcher, Resource Monitor
    Layer 3 — Operators          Constrained executors (restart, scale, rotate)
    Layer 4 — Orchestrator       Decision engine (consume scores → authorize ops)
    Cross    — CircuitBreaker    Failure isolation per module
    Cross    — StateMachine      NORMAL → DEGRADED → CONTAINMENT → SAFE_MODE → MANUAL
    Cross    — AuditLog          Immutable action history
    Cross    — Guardrails        Immutable safety constraints
"""

from .bootstrap import AutonomicControlPlane  # noqa: F401

__all__ = ["AutonomicControlPlane"]

from .bootstrap import AutonomicControlPlane

__all__ = ["AutonomicControlPlane"]
