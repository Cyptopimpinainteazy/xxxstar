"""Inferstructor Resilience Module — Fallback Protection for Cross-Chain GPU Validator.

Provides layered fallback protection for external validators using X3 as a
GPU-accelerated superhighway.  Three execution lanes, health scoring,
signer-lock management, toll-booth access control, and deterministic
degraded-mode operation.

Architecture
────────────
  Lane 1 (Primary)  ──▶  GPU-accelerated, lowest latency
  Lane 2 (Shadow)   ──▶  Hot standby, GPU-warmed, no signing authority
  Lane 3 (Tertiary) ──▶  CPU-only degraded mode, guaranteed liveness

External validators always retain native fallback — X3 is a turbocharger,
never a consensus dependency.

Modules
───────
  health       – GPU / node health scoring daemon
  lanes        – Lane orchestrator with deterministic failover
  tollbooth    – Access control, SLA enforcement, rate limiting
  signer_lock  – Distributed signing authority (prevent double-sign)
  circuit      – Circuit breaker for Redis / RPC / GPU subsystems
  degraded     – CPU-only degraded mode controller
"""

from cross_chain_gpu_validator.resilience.health import (
    GpuHealthDaemon,
    HealthScore,
    NodeHealth,
)
from cross_chain_gpu_validator.resilience.lanes import (
    AccelerationLane,
    LaneOrchestrator,
    LaneStatus,
    LaneTier,
)
from cross_chain_gpu_validator.resilience.tollbooth import (
    AccessTier,
    TollBooth,
    ValidatorTicket,
)
from cross_chain_gpu_validator.resilience.signer_lock import (
    SignerAuthority,
    SignerLock,
)
from cross_chain_gpu_validator.resilience.circuit import (
    CircuitBreaker,
    CircuitState,
)
from cross_chain_gpu_validator.resilience.degraded import (
    DegradedModeController,
    OperatingMode,
)
from cross_chain_gpu_validator.resilience.orchestrator import (
    ResilientOrchestrator,
)

__all__ = [
    # Health
    "GpuHealthDaemon",
    "HealthScore",
    "NodeHealth",
    # Lanes
    "AccelerationLane",
    "LaneOrchestrator",
    "LaneStatus",
    "LaneTier",
    # Toll booth
    "AccessTier",
    "TollBooth",
    "ValidatorTicket",
    # Signer lock
    "SignerAuthority",
    "SignerLock",
    # Circuit breaker
    "CircuitBreaker",
    "CircuitState",
    # Degraded mode
    "DegradedModeController",
    "OperatingMode",
    # Integration
    "ResilientOrchestrator",
]
