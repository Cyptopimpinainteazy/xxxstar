"""Bootstrap — Wire up the entire Autonomic Control Plane.

Usage (standalone):
    python -m swarm.autonomic

Usage (embedded in swarm API server):
    from swarm.autonomic import AutonomicControlPlane
    acp = AutonomicControlPlane.from_config("swarm/config/autonomic_config.json")
    await acp.start()
    ...
    await acp.stop()
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import sys
import time
from pathlib import Path
from typing import Any, Dict, Optional

from .config import AutonomicConfig
from .metrics_bus import MetricsBus
from .audit import AuditLog
from .circuit_breaker import CircuitBreakerRegistry
from .state_machine import RecoveryStateMachine
from .health_engine import HealthEngine
from .operators import OperatorRegistry
from .orchestrator import Orchestrator
from .sentinels.gpu_guard import GPUGuard
from .sentinels.resource_monitor import ResourceMonitor
from .sentinels.log_watcher import LogWatcher

log = logging.getLogger("autonomic")


class AutonomicControlPlane:
    """Top-level facade that owns and manages all autonomic subsystems.

    Lifecycle:
        acp = AutonomicControlPlane.from_config(path)
        await acp.start()     # starts sentinels, health engine, orchestrator
        ...
        await acp.stop()      # graceful teardown in reverse order
    """

    def __init__(self, config: Optional[AutonomicConfig] = None):
        self._cfg = config or AutonomicConfig()
        self._started = False

        # Layer 0 — Telemetry
        self.bus = MetricsBus(retention_s=self._cfg.metrics_retention_s)

        # Cross-cutting
        self.audit = AuditLog(
            log_dir=self._cfg.audit_log_dir,
            max_memory=self._cfg.audit_max_memory,
        )
        self.breakers = CircuitBreakerRegistry()
        self.state_machine = RecoveryStateMachine(
            thresholds=self._cfg.health,
        )

        # Register circuit breakers for each operator
        for name in ("service", "gpu", "process", "swarm"):
            self.breakers.register(
                name,
                failure_threshold=self._cfg.circuit_breaker.failure_threshold,
                recovery_timeout_s=self._cfg.circuit_breaker.recovery_timeout_s,
            )

        # Layer 1 — Health Engine
        self.health_engine = HealthEngine(
            bus=self.bus,
            state_machine=self.state_machine,
            breakers=self.breakers,
            config=self._cfg,
            on_state_change=self._on_state_change,
        )

        # Layer 2 — Sentinels
        self.gpu_guard = GPUGuard(
            bus=self.bus,
            config=self._cfg.gpu,
        )
        self.resource_monitor = ResourceMonitor(
            bus=self.bus,
            config=self._cfg.resource,
        )
        self.log_watcher = LogWatcher(
            bus=self.bus,
            config=self._cfg.log_watcher,
        )

        # Layer 3 — Operators
        self.operators = OperatorRegistry.create_default(
            audit=self.audit,
            config=self._cfg.intervention,
            swarm_url=self._cfg.swarm_api_url,
        )

        # Layer 4 — Orchestrator
        self.orchestrator = Orchestrator(
            bus=self.bus,
            health=self.health_engine,
            state_machine=self.state_machine,
            breakers=self.breakers,
            operators=self.operators,
            audit=self.audit,
            config=self._cfg,
        )

    @classmethod
    def from_config(cls, path: str) -> "AutonomicControlPlane":
        """Create from a JSON config file."""
        config = AutonomicConfig.load(path)
        return cls(config)

    async def start(self) -> None:
        """Start all subsystems in dependency order."""
        if self._started:
            return

        log.info("╔══════════════════════════════════════════════╗")
        log.info("║   X3 AUTONOMIC CONTROL PLANE — STARTING     ║")
        log.info("╚══════════════════════════════════════════════╝")

        self.audit.record_quick("system", "info", "autonomic", "start",
                                "control_plane", "initializing", {})

        # Sentinels first (they feed the bus)
        await self.gpu_guard.start()
        await self.resource_monitor.start()
        await self.log_watcher.start()

        # Health engine (consumes bus)
        await self.health_engine.start()

        # Orchestrator last (consumes health engine)
        await self.orchestrator.start()

        self._started = True
        log.info("Autonomic Control Plane fully operational")
        self.audit.record_quick("system", "info", "autonomic", "started",
                                "control_plane", "all subsystems running", {})

    async def stop(self) -> None:
        """Graceful shutdown in reverse order."""
        if not self._started:
            return

        log.info("Autonomic Control Plane shutting down...")

        await self.orchestrator.stop()
        await self.health_engine.stop()
        await self.log_watcher.stop()
        await self.resource_monitor.stop()
        await self.gpu_guard.stop()

        self.audit.record_quick("system", "info", "autonomic", "stopped",
                                "control_plane", "clean shutdown", {})
        self._started = False
        log.info("Autonomic Control Plane stopped")

    async def _on_state_change(self, old, new) -> None:
        """Called by HealthEngine when system state transitions."""
        from .state_machine import SystemState
        self.audit.record_quick(
            "orchestrator", "warning", "health_engine",
            "state_transition", new.value,
            f"{old.value} → {new.value}", {}
        )

    # ── API surface for the swarm server ─────────────────────────────

    def snapshot(self) -> dict:
        """Full system snapshot for /api/autonomic/status."""
        return {
            "started": self._started,
            "system_score": self.health_engine.current_score(),
            "system_state": self.state_machine.state.value,
            "health": self.health_engine.snapshot(),
            "gpu_guard": self.gpu_guard.snapshot(),
            "resource_monitor": self.resource_monitor.snapshot(),
            "log_watcher": self.log_watcher.snapshot(),
            "operators": self.operators.snapshot(),
            "circuit_breakers": {
                name: {"state": cb.state.value}
                for name, cb in self.breakers.all().items()
            },
            "orchestrator": self.orchestrator.snapshot(),
            "state_machine": self.state_machine.snapshot(),
        }

    def health_summary(self) -> dict:
        """Compact health for /api/autonomic/health."""
        score = self.health_engine.current_score()
        state = self.state_machine.state.value
        return {
            "score": round(score, 1),
            "state": state,
            "ok": score >= 60,
        }

    def audit_recent(self, n: int = 50) -> list:
        return self.audit.recent(n)


# ── Standalone runner ────────────────────────────────────────────────

async def _main() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(name)s] %(levelname)s %(message)s",
    )

    config_path = sys.argv[1] if len(sys.argv) > 1 else None
    if config_path and os.path.exists(config_path):
        acp = AutonomicControlPlane.from_config(config_path)
    else:
        acp = AutonomicControlPlane()

    await acp.start()

    try:
        # Run until interrupted
        while True:
            await asyncio.sleep(30)
            snap = acp.health_summary()
            log.info("Health: score=%.1f state=%s ok=%s",
                     snap["score"], snap["state"], snap["ok"])
    except (KeyboardInterrupt, asyncio.CancelledError):
        pass
    finally:
        await acp.stop()


def main():
    asyncio.run(_main())


if __name__ == "__main__":
    main()
