"""Hot Standby Process Manager — secondary validator instance for instant failover.

Architecture
────────────
  Primary Instance (Active)     Standby Instance (Hot)
  ┌──────────────────────┐      ┌──────────────────────┐
  │ GPU acceleration     │      │ GPU-warmed, synced   │
  │ Signing authority    │      │ NO signing authority │
  │ Serving requests     │      │ Mirroring workload   │
  └──────────┬───────────┘      └──────────┬───────────┘
             │                             │
             └─────────── Redis ───────────┘
                    (SignerLock + health state)

On primary failure:
  1. Standby detects health score drop via Redis
  2. Standby acquires SignerLock (fencing token)
  3. Standby promotes to active
  4. Old primary is drained and demoted

Key Design Decisions
────────────────────
  - Standby runs the SAME binary with --standby flag
  - Standby mirrors workload but does NOT sign
  - SignerLock prevents double-signing during failover
  - Fencing tokens ensure stale primaries can't sign after failover
  - Works on same machine (different ports) or different machines

Usage
-----
    # Start as primary:
    python -m cross_chain_gpu_validator.resilience.standby \\
        --mode primary --port 9933 --standby-port 9944

    # Start as standby:
    python -m cross_chain_gpu_validator.resilience.standby \\
        --mode standby --port 9944 --primary-port 9933
"""

from __future__ import annotations

import json
import logging
import os
import signal
import subprocess
import threading
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Callable

from cross_chain_gpu_validator.resilience.health import GpuHealthDaemon, NodeHealth
from cross_chain_gpu_validator.resilience.lanes import LaneOrchestrator, LaneTier
from cross_chain_gpu_validator.resilience.signer_lock import SignerLock, SignerAuthority

logger = logging.getLogger("x3.standby")


# ─── Enums ────────────────────────────────────────────────────


class StandbyRole(Enum):
    """Role of this validator instance."""
    PRIMARY = "primary"
    STANDBY = "standby"
    PROMOTING = "promoting"
    DEMOTED = "demoted"


class StandbyState(Enum):
    """Lifecycle state of the standby manager."""
    INITIALIZING = "initializing"
    SYNCING = "syncing"
    READY = "ready"
    ACTIVE = "active"
    FAILED = "failed"
    STOPPED = "stopped"


# ─── Config ───────────────────────────────────────────────────


@dataclass
class StandbyConfig:
    """Configuration for the standby manager."""
    role: StandbyRole = StandbyRole.PRIMARY
    node_id: str = ""
    primary_port: int = 9933
    standby_port: int = 9944
    redis_url: str = "redis://127.0.0.1:6379/0"
    health_check_interval: float = 5.0
    health_threshold: float = 0.5
    promotion_cooldown: float = 10.0
    signer_ttl: float = 30.0
    state_sync_interval: float = 2.0
    data_dir: str = "/tmp/x3-validator"

    @classmethod
    def from_env(cls) -> StandbyConfig:
        """Load configuration from environment variables."""
        return cls(
            role=StandbyRole(os.getenv("X3_STANDBY_ROLE", "primary")),
            node_id=os.getenv("X3_NODE_ID", ""),
            primary_port=int(os.getenv("X3_PRIMARY_PORT", "9933")),
            standby_port=int(os.getenv("X3_STANDBY_PORT", "9944")),
            redis_url=os.getenv("CCGV_REDIS_URL", "redis://127.0.0.1:6379/0"),
            health_check_interval=float(os.getenv("X3_HEALTH_INTERVAL", "5.0")),
            health_threshold=float(os.getenv("X3_HEALTH_THRESHOLD", "0.5")),
            promotion_cooldown=float(os.getenv("X3_PROMOTION_COOLDOWN", "10.0")),
            signer_ttl=float(os.getenv("X3_SIGNER_TTL", "30.0")),
            state_sync_interval=float(os.getenv("X3_STATE_SYNC_INTERVAL", "2.0")),
            data_dir=os.getenv("X3_DATA_DIR", "/tmp/x3-validator"),
        )


# ─── State Sync ───────────────────────────────────────────────


class StateSyncTracker:
    """Tracks state sync progress between primary and standby.

    Uses a simple block height comparison. In production this would
    use the actual chain's sync protocol.
    """

    def __init__(self, sync_interval: float = 2.0) -> None:
        self._interval = sync_interval
        self._primary_height: int = 0
        self._standby_height: int = 0
        self._lag_blocks: int = 0
        self._synced: bool = False
        self._lock = threading.Lock()

    def update_primary_height(self, height: int) -> None:
        with self._lock:
            self._primary_height = height
            self._recalculate()

    def update_standby_height(self, height: int) -> None:
        with self._lock:
            self._standby_height = height
            self._recalculate()

    def _recalculate(self) -> None:
        self._lag_blocks = self._primary_height - self._standby_height
        self._synced = self._lag_blocks <= 3  # Within 3 blocks = synced

    @property
    def lag_blocks(self) -> int:
        with self._lock:
            return self._lag_blocks

    @property
    def is_synced(self) -> bool:
        with self._lock:
            return self._synced

    @property
    def primary_height(self) -> int:
        with self._lock:
            return self._primary_height

    @property
    def standby_height(self) -> int:
        with self._lock:
            return self._standby_height

    def to_dict(self) -> dict[str, Any]:
        with self._lock:
            return {
                "primary_height": self._primary_height,
                "standby_height": self._standby_height,
                "lag_blocks": self._lag_blocks,
                "synced": self._synced,
            }


# ─── Standby Manager ──────────────────────────────────────────


class StandbyManager:
    """Manages hot standby validator instances with automatic failover.

    Parameters
    ----------
    config : StandbyConfig
        Configuration for this instance.
    validator_cmd : list[str]
        Command to start the validator binary.
    on_promotion : callable
        ``fn()`` called when standby is promoted to primary.
    on_demotion : callable
        ``fn()`` called when primary is demoted to standby.
    """

    def __init__(
        self,
        config: StandbyConfig | None = None,
        validator_cmd: list[str] | None = None,
        on_promotion: Callable[[], None] | None = None,
        on_demotion: Callable[[], None] | None = None,
    ) -> None:
        self._config = config or StandbyConfig.from_env()
        self._validator_cmd = validator_cmd or []
        self._on_promotion = on_promotion
        self._on_demotion = on_demotion

        # Generate node ID if not set
        if not self._config.node_id:
            import uuid
            self._config.node_id = f"node-{uuid.uuid4().hex[:8]}"

        self._role = self._config.role
        self._state = StandbyState.INITIALIZING
        self._lock = threading.Lock()
        self._stop = threading.Event()

        # Sub-components
        self._signer = SignerLock(
            node_id=self._config.node_id,
            redis_url=self._config.redis_url,
            ttl_seconds=self._config.signer_ttl,
            on_acquired=self._on_signer_acquired,
            on_lost=self._on_signer_lost,
        )

        self._health = GpuHealthDaemon(
            interval=self._config.health_check_interval,
            threshold=self._config.health_threshold,
            on_critical=self._on_health_critical,
            on_recovery=self._on_health_recovery,
        )

        self._lanes = LaneOrchestrator(
            health_threshold=self._config.health_threshold,
            promotion_cooldown=self._config.promotion_cooldown,
            on_failover=self._on_lane_failover,
        )

        self._sync_tracker = StateSyncTracker(
            sync_interval=self._config.state_sync_interval,
        )

        # Primary health tracking (for standby mode)
        self._primary_health: NodeHealth | None = None
        self._primary_last_seen: float = 0.0
        self._primary_timeout: float = 15.0  # Consider primary dead after 15s

        # Validator subprocess (for standby mode)
        self._validator_process: subprocess.Popen | None = None

        # Metrics
        self._promotions: int = 0
        self._demotions: int = 0
        self._failover_events: list[dict] = []

    # ── Properties ────────────────────────────────────────────

    @property
    def role(self) -> StandbyRole:
        with self._lock:
            return self._role

    @property
    def state(self) -> StandbyState:
        with self._lock:
            return self._state

    @property
    def is_primary(self) -> bool:
        return self.role == StandbyRole.PRIMARY

    @property
    def is_standby(self) -> bool:
        return self.role == StandbyRole.STANDBY

    @property
    def is_promoting(self) -> bool:
        return self.role == StandbyRole.PROMOTING

    @property
    def promotions(self) -> int:
        with self._lock:
            return self._promotions

    @property
    def sync_status(self) -> dict:
        return self._sync_tracker.to_dict()

    # ── Lifecycle ─────────────────────────────────────────────

    def start(self) -> None:
        """Start the standby manager."""
        logger.info(
            "StandbyManager starting — role=%s, node_id=%s",
            self._config.role.value, self._config.node_id,
        )

        # Create data directory
        os.makedirs(self._config.data_dir, exist_ok=True)

        # Start health daemon
        self._health.start()

        if self._config.role == StandbyRole.PRIMARY:
            self._start_as_primary()
        else:
            self._start_as_standby()

        # Start the supervision loop
        thread = threading.Thread(
            target=self._supervision_loop,
            daemon=True,
            name="standby-supervisor",
        )
        thread.start()

        with self._lock:
            self._state = StandbyState.READY

        logger.info("StandbyManager started — role=%s", self._config.role.value)

    def stop(self) -> None:
        """Gracefully stop the standby manager."""
        logger.info("StandbyManager stopping...")
        self._stop.set()
        self._health.stop()
        self._signer.release()
        self._stop_validator()
        with self._lock:
            self._state = StandbyState.STOPPED
        logger.info("StandbyManager stopped")

    def _start_as_primary(self) -> None:
        """Initialize as primary validator."""
        logger.info("Starting as PRIMARY — acquiring signer lock")
        self._signer.try_acquire()
        with self._lock:
            self._role = StandbyRole.PRIMARY

    def _start_as_standby(self) -> None:
        """Initialize as standby validator."""
        logger.info("Starting as STANDBY — launching validator process")
        with self._lock:
            self._role = StandbyRole.STANDBY
            self._state = StandbyState.SYNCING

        # Launch the validator in standby mode
        self._launch_validator(standby=True)

    def _launch_validator(self, standby: bool = False) -> None:
        """Launch the validator binary."""
        if not self._validator_cmd:
            logger.warning("No validator command configured — skipping launch")
            return

        cmd = list(self._validator_cmd)
        env = dict(os.environ)

        if standby:
            port = self._config.standby_port
            env["X3_STANDBY_MODE"] = "true"
            env["X3_DISABLE_SIGNING"] = "true"
        else:
            port = self._config.primary_port
            env.pop("X3_STANDBY_MODE", None)
            env.pop("X3_DISABLE_SIGNING", None)

        env["X3_RPC_PORT"] = str(port)
        env["X3_NODE_ID"] = self._config.node_id

        try:
            self._validator_process = subprocess.Popen(
                cmd,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                preexec_fn=os.setsid if hasattr(os, "setsid") else None,
            )
            logger.info(
                "Validator launched — PID %d, port=%d, standby=%s",
                self._validator_process.pid, port, standby,
            )
        except FileNotFoundError as exc:
            logger.error("Validator binary not found: %s", exc)
        except OSError as exc:
            logger.error("Failed to launch validator: %s", exc)

    def _stop_validator(self) -> None:
        """Stop the validator subprocess."""
        if self._validator_process is None:
            return
        try:
            pgid = os.getpgid(self._validator_process.pid)
            os.killpg(pgid, signal.SIGTERM)
            self._validator_process.wait(timeout=10.0)
        except (ProcessLookupError, subprocess.TimeoutExpired, OSError):
            try:
                os.killpg(os.getpgid(self._validator_process.pid), signal.SIGKILL)
            except (ProcessLookupError, OSError):
                pass
        self._validator_process = None

    # ── Supervision Loop ──────────────────────────────────────

    def _supervision_loop(self) -> None:
        """Main supervision loop — monitors health and triggers failover."""
        while not self._stop.is_set():
            try:
                if self._config.role == StandbyRole.PRIMARY:
                    self._supervise_primary()
                else:
                    self._supervise_standby()
            except Exception as exc:
                logger.error("Supervision loop error: %s", exc)
            self._stop.wait(self._config.health_check_interval)

    def _supervise_primary(self) -> None:
        """Supervision logic for primary role."""
        health = self._health.health
        score = health.score.overall

        # Update sync tracker
        self._sync_tracker.update_primary_height(health.block_height)

        # Check if we should demote
        if score < self._config.health_threshold or not health.gpu.available:
            logger.warning(
                "Primary health degraded (score=%.2f, gpu=%s) — considering demotion",
                score, "up" if health.gpu.available else "DOWN",
            )
            # The lane orchestrator handles the actual failover
            self._lanes.on_health_update(health)

    def _supervise_standby(self) -> None:
        """Supervision logic for standby role."""
        # Check if primary is alive via Redis signer lock
        primary_alive = self._is_primary_alive()

        if not primary_alive:
            elapsed = time.time() - self._primary_last_seen
            logger.warning(
                "Primary not detected for %.1fs — considering promotion",
                elapsed,
            )

            if elapsed > self._primary_timeout:
                self._initiate_promotion()

        # Update sync tracker
        self._sync_tracker.update_standby_height(
            self._health.health.block_height
        )

    def _is_primary_alive(self) -> bool:
        """Check if the primary validator is alive via Redis."""
        # The primary holds the signer lock. If we can't see it,
        # the primary may be dead.
        state = self._signer.state()
        if state.authority == SignerAuthority.HOLDER:
            # We hold the lock — we're the primary
            return True
        if state.authority == SignerAuthority.STANDBY:
            # Lock held by someone else — primary is alive
            self._primary_last_seen = time.time()
            return True
        return False

    # ── Promotion / Demotion ──────────────────────────────────

    def _initiate_promotion(self) -> None:
        """Promote standby to primary."""
        logger.warning("Initiating promotion to PRIMARY")

        with self._lock:
            self._role = StandbyRole.PROMOTING
            self._state = StandbyState.ACTIVE

        # 1. Acquire the signer lock
        if not self._signer.try_acquire():
            logger.error("Failed to acquire signer lock — promotion aborted")
            with self._lock:
                self._role = StandbyRole.STANDBY
                self._state = StandbyState.READY
            return

        # 2. Verify we're synced
        if not self._sync_tracker.is_synced:
            logger.warning(
                "Promoting with sync lag: %d blocks behind",
                self._sync_tracker.lag_blocks,
            )

        # 3. Restart validator in primary mode
        self._stop_validator()
        self._launch_validator(standby=False)

        # 4. Update role
        with self._lock:
            self._role = StandbyRole.PRIMARY
            self._promotions += 1

        # 5. Record failover event
        self._record_failover("standby_to_primary", "primary_unreachable")

        logger.warning(
            "Promotion complete — node %s is now PRIMARY (promotion #%d)",
            self._config.node_id, self._promotions,
        )

        if self._on_promotion:
            try:
                self._on_promotion()
            except Exception:
                pass

    def _demote_to_standby(self) -> None:
        """Demote primary to standby."""
        logger.warning("Demoting to STANDBY")

        with self._lock:
            self._role = StandbyRole.DEMOTED
            self._state = StandbyState.SYNCING

        # 1. Release signer lock
        self._signer.release()

        # 2. Restart validator in standby mode
        self._stop_validator()
        self._launch_validator(standby=True)

        # 3. Update role
        with self._lock:
            self._role = StandbyRole.STANDBY
            self._state = StandbyState.READY
            self._demotions += 1

        self._record_failover("primary_to_standby", "manual_demotion")

        logger.warning(
            "Demotion complete — node %s is now STANDBY",
            self._config.node_id,
        )

        if self._on_demotion:
            try:
                self._on_demotion()
            except Exception:
                pass

    # ── Callbacks ─────────────────────────────────────────────

    def _on_signer_acquired(self) -> None:
        logger.info("Signer lock acquired — this node may now sign")

    def _on_signer_lost(self) -> None:
        logger.warning("Signer lock lost — this node must NOT sign")
        if self.is_primary:
            self._demote_to_standby()

    def _on_health_critical(self, health: NodeHealth) -> None:
        logger.error(
            "Health CRITICAL: score=%.2f, gpu=%s",
            health.score.overall,
            "up" if health.gpu.available else "DOWN",
        )

    def _on_health_recovery(self, health: NodeHealth) -> None:
        logger.info(
            "Health RECOVERED: score=%.2f", health.score.overall
        )

    def _on_lane_failover(self, from_tier: LaneTier, to_tier: LaneTier) -> None:
        logger.warning("Lane failover: %s → %s", from_tier.name, to_tier.name)

    # ── Event Recording ───────────────────────────────────────

    def _record_failover(self, event_type: str, reason: str) -> None:
        event = {
            "type": event_type,
            "reason": reason,
            "node_id": self._config.node_id,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "promotions": self._promotions,
            "demotions": self._demotions,
        }
        with self._lock:
            self._failover_events.append(event)
            if len(self._failover_events) > 100:
                self._failover_events = self._failover_events[-50:]

    # ── Status ────────────────────────────────────────────────

    def status(self) -> dict[str, Any]:
        with self._lock:
            return {
                "role": self._role.value,
                "state": self._state.value,
                "node_id": self._config.node_id,
                "primary_port": self._config.primary_port,
                "standby_port": self._config.standby_port,
                "promotions": self._promotions,
                "demotions": self._demotions,
                "is_signer": self._signer.is_signer,
                "signer_authority": self._signer.authority.value,
                "sync": self._sync_tracker.to_dict(),
                "health": self._health.health.to_dict(),
                "lanes": self._lanes.status(),
                "recent_failovers": self._failover_events[-5:],
            }


# ─── CLI Entry Point ──────────────────────────────────────────


def main() -> None:
    """CLI entry point for the standby manager."""
    import argparse

    parser = argparse.ArgumentParser(
        description="X3 Hot Standby Manager — automatic failover for validators"
    )
    parser.add_argument(
        "--mode", choices=["primary", "standby"], default="primary",
        help="Run mode (default: primary)",
    )
    parser.add_argument(
        "--port", type=int, default=9933,
        help="RPC port for this instance",
    )
    parser.add_argument(
        "--primary-port", type=int, default=9933,
        help="Primary instance RPC port",
    )
    parser.add_argument(
        "--standby-port", type=int, default=9944,
        help="Standby instance RPC port",
    )
    parser.add_argument(
        "--node-id", default="",
        help="Unique node identifier",
    )
    parser.add_argument(
        "--redis-url", default="redis://127.0.0.1:6379/0",
        help="Redis connection URL",
    )
    parser.add_argument(
        "--validator-cmd", nargs="+", default=[],
        help="Validator binary and args (e.g. --validator-cmd x3-chain-node --dev)",
    )
    parser.add_argument(
        "--health-threshold", type=float, default=0.5,
        help="Health score threshold for failover (default: 0.5)",
    )
    parser.add_argument(
        "--primary-timeout", type=float, default=15.0,
        help="Seconds without primary heartbeat before promoting (default: 15)",
    )
    parser.add_argument(
        "--log-level", default="INFO",
        choices=["DEBUG", "INFO", "WARNING", "ERROR"],
        help="Logging level",
    )

    args = parser.parse_args()

    logging.basicConfig(
        level=getattr(logging, args.log_level),
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )

    config = StandbyConfig(
        role=StandbyRole(args.mode),
        node_id=args.node_id,
        primary_port=args.primary_port,
        standby_port=args.standby_port,
        redis_url=args.redis_url,
        health_threshold=args.health_threshold,
    )

    manager = StandbyManager(
        config=config,
        validator_cmd=args.validator_cmd,
    )

    try:
        manager.start()
        # Block until stopped
        while manager.state not in (StandbyState.STOPPED, StandbyState.FAILED):
            time.sleep(1)
    except KeyboardInterrupt:
        logger.info("Received SIGINT — shutting down")
    finally:
        manager.stop()


if __name__ == "__main__":
    main()
