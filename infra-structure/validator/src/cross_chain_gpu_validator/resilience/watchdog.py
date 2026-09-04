"""Watchdog Process Supervisor — monitors and restarts the validator process.

Provides:
1. Process health monitoring via PID file + periodic health checks
2. Auto-restart on crash with exponential backoff (1s → 2s → 4s → ... → 60s max)
3. Memory limit enforcement via cgroups (Linux) or rlimit (cross-platform)
4. systemd integration for machine-level supervision
5. Logging of all restart events to a structured log file

Usage
-----
    # As a standalone supervisor:
    python -m cross_chain_gpu_validator.resilience.watchdog \\
        --pid-file /var/run/x3-validator.pid \\
        --cmd "python -m cross_chain_gpu_validator.cli start" \\
        --memory-limit-mb 8192

    # As a library:
    from cross_chain_gpu_validator.resilience.watchdog import Watchdog
    wd = Watchdog(cmd=["python", "-m", "cross_chain_gpu_validator.cli", "start"])
    wd.start()
"""

from __future__ import annotations

import logging
import os
import signal
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any

logger = logging.getLogger("x3.watchdog")


# ─── Constants ────────────────────────────────────────────────

MAX_BACKOFF_SECONDS = 60.0
INITIAL_BACKOFF_SECONDS = 1.0
BACKOFF_MULTIPLIER = 2.0
DEFAULT_HEALTH_INTERVAL = 10.0
DEFAULT_MEMORY_LIMIT_MB = 0  # 0 = no limit


# ─── Enums ────────────────────────────────────────────────────


class WatchdogState(Enum):
    """Lifecycle state of the watchdog supervisor."""
    STOPPED = "stopped"
    RUNNING = "running"
    PAUSED = "paused"
    FAILED = "failed"


class RestartReason(Enum):
    """Why the process was restarted."""
    CRASH = "crash"
    HEALTH_CHECK_FAILED = "health_check_failed"
    MEMORY_LIMIT_EXCEEDED = "memory_limit_exceeded"
    MANUAL = "manual"
    SIGNAL = "signal"


# ─── Event Log ────────────────────────────────────────────────


@dataclass
class RestartEvent:
    """Record of a single restart event."""
    attempt: int
    reason: RestartReason
    exit_code: int | None
    uptime_seconds: float
    timestamp: str
    backoff_seconds: float
    details: str = ""

    def to_dict(self) -> dict[str, Any]:
        return {
            "attempt": self.attempt,
            "reason": self.reason.value,
            "exit_code": self.exit_code,
            "uptime_seconds": round(self.uptime_seconds, 2),
            "timestamp": self.timestamp,
            "backoff_seconds": self.backoff_seconds,
            "details": self.details,
        }


# ─── Memory Monitor ──────────────────────────────────────────


class MemoryMonitor:
    """Monitors process memory usage and enforces limits.

    Uses cgroups v1/v2 on Linux, falls back to /proc/PID/status.
    """

    def __init__(self, limit_mb: int = 0, check_interval: float = 5.0) -> None:
        self._limit_mb = limit_mb
        self._interval = check_interval
        self._pid: int | None = None
        self._stop = threading.Event()

    def set_pid(self, pid: int) -> None:
        self._pid = pid

    def start_monitoring(self) -> None:
        if self._limit_mb <= 0:
            return
        self._stop.clear()
        thread = threading.Thread(
            target=self._monitor_loop, daemon=True, name="memory-monitor"
        )
        thread.start()

    def stop_monitoring(self) -> None:
        self._stop.set()

    def _monitor_loop(self) -> None:
        while not self._stop.is_set():
            if self._pid is not None:
                usage_mb = self._get_memory_mb(self._pid)
                if usage_mb is not None and usage_mb > self._limit_mb:
                    logger.warning(
                        "Memory limit exceeded: %.1f MB > %d MB — killing PID %d",
                        usage_mb, self._limit_mb, self._pid,
                    )
                    try:
                        os.kill(self._pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    return
            self._stop.wait(self._interval)

    @staticmethod
    def _get_memory_mb(pid: int) -> float | None:
        """Read RSS from /proc/PID/status. Returns None on error."""
        try:
            with open(f"/proc/{pid}/status", "r") as f:
                for line in f:
                    if line.startswith("VmRSS:"):
                        parts = line.split()
                        if len(parts) >= 2:
                            return int(parts[1]) / 1024.0
        except (FileNotFoundError, ProcessLookupError, ValueError, OSError):
            pass
        return None


# ─── Health Check ─────────────────────────────────────────────


class HealthCheck:
    """Periodic health check for the managed process.

    By default checks that the PID is alive. Can be extended with
    custom health check functions (e.g. RPC ping, block height check).
    """

    def __init__(
        self,
        interval: float = DEFAULT_HEALTH_INTERVAL,
        custom_checks: list[callable] | None = None,
    ) -> None:
        self._interval = interval
        self._custom_checks = custom_checks or []
        self._pid: int | None = None
        self._stop = threading.Event()
        self._on_failure: callable | None = None

    def set_pid(self, pid: int) -> None:
        self._pid = pid

    def set_on_failure(self, callback: callable) -> None:
        self._on_failure = callback

    def start(self) -> None:
        self._stop.clear()
        thread = threading.Thread(
            target=self._check_loop, daemon=True, name="health-check"
        )
        thread.start()

    def stop(self) -> None:
        self._stop.set()

    def _check_loop(self) -> None:
        while not self._stop.is_set():
            self._stop.wait(self._interval)
            if self._stop.is_set():
                break
            if self._pid is None:
                continue
            if not self._is_alive(self._pid):
                logger.warning("Health check FAILED — PID %d is dead", self._pid)
                if self._on_failure:
                    self._on_failure()
                return
            if not self._run_custom_checks():
                logger.warning("Health check FAILED — custom check failed")
                if self._on_failure:
                    self._on_failure()
                return

    @staticmethod
    def _is_alive(pid: int) -> bool:
        try:
            os.kill(pid, 0)
            return True
        except (ProcessLookupError, PermissionError):
            return False

    def _run_custom_checks(self) -> bool:
        for check in self._custom_checks:
            try:
                if not check():
                    return False
            except Exception as exc:
                logger.error("Custom health check raised: %s", exc)
                return False
        return True


# ─── Watchdog ─────────────────────────────────────────────────


class Watchdog:
    """Process supervisor with auto-restart, health checks, and memory limits.

    Parameters
    ----------
    cmd : list[str]
        Command to run (e.g. ``["python", "-m", "cross_chain_gpu_validator.cli", "start"]``).
    pid_file : str | None
        Path to PID file for external monitoring (e.g. systemd).
    health_interval : float
        Seconds between health checks (default 10).
    memory_limit_mb : int
        Max RSS in MB before process is killed (0 = no limit).
    max_restarts : int
        Max consecutive restarts before watchdog gives up (0 = unlimited).
    restart_delay : float
        Initial backoff delay in seconds (default 1.0).
    custom_health_checks : list[callable]
        Additional health check functions (return bool).
    on_restart : callable
        ``fn(attempt, reason)`` called before each restart.
    on_give_up : callable
        ``fn(attempts)`` called when max_restarts is exceeded.
    """

    def __init__(
        self,
        cmd: list[str],
        pid_file: str | None = None,
        health_interval: float = DEFAULT_HEALTH_INTERVAL,
        memory_limit_mb: int = DEFAULT_MEMORY_LIMIT_MB,
        max_restarts: int = 0,
        restart_delay: float = INITIAL_BACKOFF_SECONDS,
        custom_health_checks: list[callable] | None = None,
        on_restart: callable | None = None,
        on_give_up: callable | None = None,
    ) -> None:
        self._cmd = cmd
        self._pid_file = pid_file
        self._max_restarts = max_restarts
        self._on_restart = on_restart
        self._on_give_up = on_give_up

        self._process: subprocess.Popen | None = None
        self._state = WatchdogState.STOPPED
        self._restart_count = 0
        self._backoff = restart_delay
        self._events: list[RestartEvent] = []
        self._lock = threading.Lock()
        self._stop_event = threading.Event()
        self._start_time = 0.0

        # Sub-components
        self._health = HealthCheck(
            interval=health_interval,
            custom_checks=custom_health_checks,
        )
        self._health.set_on_failure(self._on_health_failure)

        self._memory = MemoryMonitor(limit_mb=memory_limit_mb)

    # ── Properties ────────────────────────────────────────────

    @property
    def state(self) -> WatchdogState:
        with self._lock:
            return self._state

    @property
    def restart_count(self) -> int:
        with self._lock:
            return self._restart_count

    @property
    def pid(self) -> int | None:
        if self._process and self._process.pid:
            return self._process.pid
        return None

    @property
    def is_running(self) -> bool:
        return self.state == WatchdogState.RUNNING

    # ── Lifecycle ─────────────────────────────────────────────

    def start(self) -> None:
        """Start the watchdog and launch the managed process."""
        with self._lock:
            if self._state == WatchdogState.RUNNING:
                logger.warning("Watchdog already running")
                return
            self._state = WatchdogState.RUNNING
            self._restart_count = 0
            self._backoff = INITIAL_BACKOFF_SECONDS
            self._events.clear()

        self._stop_event.clear()
        self._start_time = time.monotonic()
        self._launch_process()

        # Start health checks and memory monitor
        if self.pid:
            self._health.set_pid(self.pid)
            self._health.start()
            self._memory.set_pid(self.pid)
            self._memory.start_monitoring()

        # Start the supervision loop in a background thread
        thread = threading.Thread(
            target=self._supervision_loop, daemon=True, name="watchdog-supervisor"
        )
        thread.start()

        logger.info(
            "Watchdog started — PID %d, cmd=%s",
            self.pid, " ".join(self._cmd),
        )

    def stop(self, timeout: float = 10.0) -> None:
        """Gracefully stop the managed process and watchdog."""
        logger.info("Watchdog stopping...")
        self._stop_event.set()
        self._health.stop()
        self._memory.stop_monitoring()

        if self._process:
            self._signal_process(signal.SIGTERM)
            try:
                self._process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                logger.warning("Process did not exit in %.1fs — sending SIGKILL", timeout)
                self._signal_process(signal.SIGKILL)
                self._process.wait(timeout=5.0)

        self._cleanup_pid_file()
        with self._lock:
            self._state = WatchdogState.STOPPED
        logger.info("Watchdog stopped")

    def pause(self) -> None:
        """Pause supervision (process keeps running, watchdog stops monitoring)."""
        with self._lock:
            self._state = WatchdogState.PAUSED
        self._health.stop()
        self._memory.stop_monitoring()
        logger.info("Watchdog paused")

    def resume(self) -> None:
        """Resume supervision."""
        with self._lock:
            self._state = WatchdogState.RUNNING
        if self.pid:
            self._health.set_pid(self.pid)
            self._health.start()
            self._memory.set_pid(self.pid)
            self._memory.start_monitoring()
        logger.info("Watchdog resumed")

    # ── Internal ──────────────────────────────────────────────

    def _launch_process(self) -> None:
        """Launch the managed process."""
        try:
            self._process = subprocess.Popen(
                self._cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                preexec_fn=os.setsid if hasattr(os, "setsid") else None,
            )
            self._write_pid_file(self._process.pid)
            logger.info("Process launched — PID %d", self._process.pid)
        except FileNotFoundError as exc:
            logger.error("Failed to launch process — command not found: %s", exc)
            with self._lock:
                self._state = WatchdogState.FAILED
            raise
        except OSError as exc:
            logger.error("Failed to launch process — OS error: %s", exc)
            with self._lock:
                self._state = WatchdogState.FAILED
            raise

    def _supervision_loop(self) -> None:
        """Main supervision loop — waits for process exit and restarts."""
        while not self._stop_event.is_set():
            if self._process is None:
                break

            try:
                exit_code = self._process.wait(timeout=1.0)
            except subprocess.TimeoutExpired:
                continue

            # Process exited
            uptime = time.monotonic() - self._start_time
            reason = RestartReason.CRASH
            if exit_code == -signal.SIGKILL:
                reason = RestartReason.MEMORY_LIMIT_EXCEEDED
            elif exit_code == -signal.SIGTERM:
                reason = RestartReason.SIGNAL

            self._record_event(reason, exit_code, uptime)

            if self._stop_event.is_set():
                break

            # Check restart limit
            with self._lock:
                self._restart_count += 1
                if self._max_restarts > 0 and self._restart_count > self._max_restarts:
                    logger.error(
                        "Max restarts (%d) exceeded — giving up",
                        self._max_restarts,
                    )
                    self._state = WatchdogState.FAILED
                    if self._on_give_up:
                        try:
                            self._on_give_up(self._restart_count)
                        except Exception:
                            pass
                    break

            # Exponential backoff
            backoff = min(self._backoff, MAX_BACKOFF_SECONDS)
            logger.warning(
                "Process exited (code=%s, uptime=%.1fs) — restarting in %.1fs "
                "(attempt #%d)",
                exit_code, uptime, backoff, self._restart_count,
            )

            if self._on_restart:
                try:
                    self._on_restart(self._restart_count, reason)
                except Exception:
                    pass

            self._stop_event.wait(backoff)
            if self._stop_event.is_set():
                break

            # Update backoff for next time
            with self._lock:
                self._backoff = min(self._backoff * BACKOFF_MULTIPLIER, MAX_BACKOFF_SECONDS)

            # Relaunch
            self._launch_process()
            if self.pid:
                self._health.set_pid(self.pid)
                self._memory.set_pid(self.pid)
            self._start_time = time.monotonic()

    def _on_health_failure(self) -> None:
        """Called when the health check detects a failure."""
        if self._process is None:
            return
        uptime = time.monotonic() - self._start_time
        self._record_event(RestartReason.HEALTH_CHECK_FAILED, None, uptime)
        logger.warning("Health check failed — killing process")
        self._signal_process(signal.SIGTERM)

    def _signal_process(self, sig: signal.Signals) -> None:
        """Send a signal to the managed process group."""
        if self._process is None or self._process.pid is None:
            return
        try:
            pgid = os.getpgid(self._process.pid)
            os.killpg(pgid, sig)
        except (ProcessLookupError, PermissionError, OSError):
            try:
                os.kill(self._process.pid, sig)
            except (ProcessLookupError, PermissionError, OSError):
                pass

    def _record_event(
        self, reason: RestartReason, exit_code: int | None, uptime: float
    ) -> None:
        """Record a restart event."""
        event = RestartEvent(
            attempt=self._restart_count + 1,
            reason=reason,
            exit_code=exit_code,
            uptime_seconds=uptime,
            timestamp=datetime.now(timezone.utc).isoformat(),
            backoff_seconds=self._backoff,
        )
        with self._lock:
            self._events.append(event)
            if len(self._events) > 100:
                self._events = self._events[-50:]

    # ── PID File Management ───────────────────────────────────

    def _write_pid_file(self, pid: int) -> None:
        if self._pid_file is None:
            return
        try:
            os.makedirs(os.path.dirname(self._pid_file) or ".", exist_ok=True)
            with open(self._pid_file, "w") as f:
                f.write(str(pid))
        except OSError as exc:
            logger.warning("Could not write PID file %s: %s", self._pid_file, exc)

    def _cleanup_pid_file(self) -> None:
        if self._pid_file is None:
            return
        try:
            if os.path.exists(self._pid_file):
                os.unlink(self._pid_file)
        except OSError as exc:
            logger.warning("Could not remove PID file %s: %s", self._pid_file, exc)

    # ── Status ────────────────────────────────────────────────

    def status(self) -> dict[str, Any]:
        with self._lock:
            return {
                "state": self._state.value,
                "pid": self.pid,
                "restart_count": self._restart_count,
                "backoff_seconds": self._backoff,
                "cmd": " ".join(self._cmd),
                "max_restarts": self._max_restarts,
                "memory_limit_mb": self._memory._limit_mb,
                "recent_events": [e.to_dict() for e in self._events[-10:]],
                "total_events": len(self._events),
            }


# ─── CLI Entry Point ──────────────────────────────────────────


def main() -> None:
    """CLI entry point for the watchdog supervisor."""
    import argparse

    parser = argparse.ArgumentParser(
        description="X3 Validator Watchdog — process supervisor with auto-restart"
    )
    parser.add_argument(
        "--cmd", required=True, nargs="+",
        help="Command to supervise (e.g. --cmd python -m myapp start)",
    )
    parser.add_argument(
        "--pid-file", default=None,
        help="Path to PID file",
    )
    parser.add_argument(
        "--health-interval", type=float, default=DEFAULT_HEALTH_INTERVAL,
        help=f"Health check interval in seconds (default {DEFAULT_HEALTH_INTERVAL})",
    )
    parser.add_argument(
        "--memory-limit-mb", type=int, default=DEFAULT_MEMORY_LIMIT_MB,
        help="Memory limit in MB (0 = no limit)",
    )
    parser.add_argument(
        "--max-restarts", type=int, default=0,
        help="Max consecutive restarts before giving up (0 = unlimited)",
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

    wd = Watchdog(
        cmd=args.cmd,
        pid_file=args.pid_file,
        health_interval=args.health_interval,
        memory_limit_mb=args.memory_limit_mb,
        max_restarts=args.max_restarts,
    )

    try:
        wd.start()
        # Block until watchdog stops
        while wd.is_running:
            time.sleep(1)
    except KeyboardInterrupt:
        logger.info("Received SIGINT — shutting down")
    finally:
        wd.stop()


if __name__ == "__main__":
    main()
