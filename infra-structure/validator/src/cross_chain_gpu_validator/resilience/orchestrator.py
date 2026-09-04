"""Resilient Orchestrator — wraps MultiChainOrchestrator with full
Inferstructor resilience: lane routing, circuit breakers, toll booth,
signer lock, health-driven degradation, and metrics propagation.

This is the drop-in replacement for ``MultiChainOrchestrator`` that wires
together all resilience subsystems.  External validators interact with this
exactly like the original orchestrator — the protection is transparent.
"""

from __future__ import annotations

import logging
import time
from typing import Any

from cross_chain_gpu_validator.chain_registry import ChainRegistry
from cross_chain_gpu_validator.config import Settings, load_settings
from cross_chain_gpu_validator.metrics import MetricsStore
from cross_chain_gpu_validator.orchestrator.orchestrator import (
    MultiChainOrchestrator,
    MultiChainSwapPayload,
)
from cross_chain_gpu_validator.orchestrator.registry import AtomicSwapRegistry

from cross_chain_gpu_validator.resilience.circuit import CircuitBreaker
from cross_chain_gpu_validator.resilience.degraded import DegradedModeController, OperatingMode
from cross_chain_gpu_validator.resilience.health import GpuHealthDaemon, NodeHealth
from cross_chain_gpu_validator.resilience.lanes import LaneOrchestrator, LaneTier
from cross_chain_gpu_validator.resilience.signer_lock import SignerLock
from cross_chain_gpu_validator.resilience.tollbooth import TollBooth, AccessTier

logger = logging.getLogger("x3.resilient")


class ResilientOrchestrator:
    """Production-grade orchestrator with full Inferstructor resilience.

    Wraps the existing ``MultiChainOrchestrator`` and adds:
    - 3-lane execution tiers with deterministic failover
    - Circuit breakers around Redis, GPU, and RPC subsystems
    - Toll booth access control for external validators
    - Distributed signer lock for double-sign prevention
    - Health-driven degraded mode controller
    - MetricsStore integration (gpu_health finally gets updated)

    Parameters
    ----------
    registry : AtomicSwapRegistry
        Redis-backed swap state machine.
    chain_registry : ChainRegistry
        Registered chain validators.
    metrics : MetricsStore
        Thread-safe metrics store.
    settings : Settings | None
        Runtime config.  Loaded from env if None.
    """

    def __init__(
        self,
        registry: AtomicSwapRegistry,
        chain_registry: ChainRegistry,
        metrics: MetricsStore,
        settings: Settings | None = None,
    ) -> None:
        self._settings = settings or load_settings()
        self._metrics = metrics

        # ── Core orchestrator ────────────────────────────────
        self._orchestrator = MultiChainOrchestrator(
            registry=registry,
            chain_registry=chain_registry,
            metrics=metrics,
        )

        # ── Circuit breakers ─────────────────────────────────
        self._redis_breaker = CircuitBreaker(
            "redis",
            failure_threshold=5,
            recovery_seconds=30,
            on_open=lambda name: logger.error("Circuit OPEN: %s", name),
            on_close=lambda name: logger.info("Circuit CLOSED: %s", name),
        )
        self._gpu_breaker = CircuitBreaker(
            "gpu",
            failure_threshold=3,
            recovery_seconds=15,
            on_open=lambda name: logger.error("Circuit OPEN: %s", name),
            on_close=lambda name: logger.info("Circuit CLOSED: %s", name),
        )

        # ── Lane orchestrator ────────────────────────────────
        self._lanes = LaneOrchestrator(
            health_threshold=0.5,
            promotion_cooldown=10.0,
            on_failover=self._on_lane_failover,
            on_recovery=self._on_lane_recovery,
        )

        # ── Degraded mode controller ─────────────────────────
        self._degraded = DegradedModeController(
            on_mode_change=self._on_mode_change,
        )

        # ── Toll booth ───────────────────────────────────────
        self._tollbooth = TollBooth(
            default_tier=AccessTier.BASE,
            session_ttl=3600.0,
        )

        # ── Signer lock ─────────────────────────────────────
        self._signer = SignerLock(
            redis_url=self._settings.redis_url,
            ttl_seconds=30.0,
            on_acquired=lambda: logger.info("Signer lock ACQUIRED"),
            on_lost=lambda: logger.warning("Signer lock LOST"),
        )

        # ── Health daemon ────────────────────────────────────
        self._health = GpuHealthDaemon(
            interval=5.0,
            threshold=0.5,
            on_critical=self._on_health_critical,
            on_recovery=self._on_health_recovery,
        )

    # ── Lifecycle ────────────────────────────────────────────

    def start(self) -> None:
        """Start all resilience subsystems."""
        logger.info("Starting Inferstructor resilience layer")
        self._health.start()
        self._signer.try_acquire()
        logger.info(
            "Resilient orchestrator started — mode=%s, signer=%s",
            self._degraded.mode.value,
            self._signer.authority.value,
        )

    def stop(self) -> None:
        """Gracefully stop all resilience subsystems."""
        logger.info("Stopping Inferstructor resilience layer")
        self._health.stop()
        self._signer.release()

    # ── Submit + Process (drop-in API) ───────────────────────

    def submit_swap(
        self, payload: MultiChainSwapPayload, validator_id: str | None = None
    ) -> None:
        """Submit a swap with toll booth + circuit breaker protection."""
        # Toll booth check
        if validator_id:
            chain_id = next(iter(payload.chain_transactions.keys()), "unknown")
            ticket = self._tollbooth.admit(validator_id, chain_id)
            if ticket is None:
                raise PermissionError(
                    f"Validator {validator_id} denied — rate limit exceeded"
                )
            if not self._tollbooth.check_batch_size(
                validator_id, sum(len(v) for v in payload.chain_transactions.values())
            ):
                raise ValueError(
                    f"Batch size exceeds tier limit for {validator_id}"
                )

        # Submit through circuit breaker
        self._redis_breaker.call(self._orchestrator.submit_swap, payload)

    def process_pending(self) -> None:
        """Process pending swaps through the lane orchestrator."""
        self._lanes.execute(
            self._redis_breaker.call,
            self._orchestrator.process_pending,
        )

    def validate_swap(self, payload: MultiChainSwapPayload) -> bool:
        """Validate a swap through the best available lane."""
        return self._lanes.execute(self._orchestrator.validate_swap, payload)

    # ── Health Callbacks ─────────────────────────────────────

    def _on_health_critical(self, health: NodeHealth) -> None:
        logger.error(
            "Health CRITICAL: score=%.2f gpu=%s",
            health.score.overall,
            "up" if health.gpu.available else "DOWN",
        )
        self._metrics.update_gpu_health("critical")
        self._lanes.on_health_update(health)
        self._degraded.on_health_update(
            health.gpu.available, health.score.overall, health.gpu.temperature_c
        )

    def _on_health_recovery(self, health: NodeHealth) -> None:
        logger.info(
            "Health RECOVERED: score=%.2f", health.score.overall
        )
        self._metrics.update_gpu_health("healthy")
        self._lanes.on_health_update(health)
        self._degraded.on_health_update(
            health.gpu.available, health.score.overall, health.gpu.temperature_c
        )

    # ── Lane Callbacks ───────────────────────────────────────

    def _on_lane_failover(self, from_tier: LaneTier, to_tier: LaneTier) -> None:
        logger.warning("Lane failover: %s → %s", from_tier.name, to_tier.name)
        if to_tier == LaneTier.TERTIARY:
            self._degraded.force_mode(OperatingMode.CPU_ONLY, "lane_failover_tertiary")
        elif to_tier == LaneTier.SHADOW:
            self._degraded.force_mode(OperatingMode.DEGRADED_GPU, "lane_failover_shadow")

    def _on_lane_recovery(self, tier: LaneTier) -> None:
        logger.info("Lane recovered: %s", tier.name)
        if tier == LaneTier.PRIMARY:
            self._degraded.force_mode(OperatingMode.FULL_GPU, "lane_recovery_primary")

    # ── Mode Callback ────────────────────────────────────────

    def _on_mode_change(
        self, old: OperatingMode, new: OperatingMode, reason: str
    ) -> None:
        logger.warning(
            "Operating mode: %s → %s (reason=%s)", old.value, new.value, reason
        )
        self._metrics.update_gpu_health(new.value)

    # ── Status ───────────────────────────────────────────────

    def status(self) -> dict[str, Any]:
        return {
            "mode": self._degraded.mode.value,
            "capacity": self._degraded.capacity,
            "signer": self._signer.authority.value,
            "health": self._health.health.to_dict(),
            "lanes": self._lanes.status(),
            "tollbooth": self._tollbooth.status(),
            "circuit_breakers": {
                "redis": self._redis_breaker.to_dict(),
                "gpu": self._gpu_breaker.to_dict(),
            },
            "degraded": self._degraded.status(),
        }

    # ── Accessors ────────────────────────────────────────────

    @property
    def health_daemon(self) -> GpuHealthDaemon:
        return self._health

    @property
    def lane_orchestrator(self) -> LaneOrchestrator:
        return self._lanes

    @property
    def toll_booth(self) -> TollBooth:
        return self._tollbooth

    @property
    def signer_lock(self) -> SignerLock:
        return self._signer

    @property
    def degraded_controller(self) -> DegradedModeController:
        return self._degraded
