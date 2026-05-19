"""Operators — Constrained executors for system interventions.

Operators are the "hands" of the autonomic system.  Each operator can
perform exactly ONE class of intervention (service restart, process
kill, GPU scaling, etc.) and is rate-limited so runaway self-healing
can't make things worse.

Every action is funneled through the AuditLog before execution.
"""

from __future__ import annotations

import asyncio
import logging
import os
import signal
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Deque, Dict, List, Optional, Tuple

from collections import deque

from .audit import AuditLog
from .config import InterventionConfig

log = logging.getLogger("autonomic.operators")


# ── Rate limiter ─────────────────────────────────────────────────────

class RateLimiter:
    """Sliding-window rate limiter."""

    def __init__(self, max_actions: int, window_s: float):
        self.max_actions = max_actions
        self.window_s = window_s
        self._timestamps: Deque[float] = deque()

    def allow(self) -> bool:
        now = time.time()
        cutoff = now - self.window_s
        while self._timestamps and self._timestamps[0] < cutoff:
            self._timestamps.popleft()
        return len(self._timestamps) < self.max_actions

    def record(self) -> None:
        self._timestamps.append(time.time())

    @property
    def remaining(self) -> int:
        now = time.time()
        cutoff = now - self.window_s
        while self._timestamps and self._timestamps[0] < cutoff:
            self._timestamps.popleft()
        return max(0, self.max_actions - len(self._timestamps))


# ── Base Operator ────────────────────────────────────────────────────

class OperatorResult(Enum):
    SUCCESS = "success"
    FAILED = "failed"
    RATE_LIMITED = "rate_limited"
    BLOCKED = "blocked"  # safe mode or other policy


@dataclass
class ActionRecord:
    operator: str
    action: str
    target: str
    result: OperatorResult
    detail: str
    ts: float = field(default_factory=time.time)


class BaseOperator:
    """Abstract base for all operators."""

    name: str = "base"

    def __init__(
        self,
        audit: AuditLog,
        config: Optional[InterventionConfig] = None,
        safe_mode: bool = False,
    ):
        self._audit = audit
        self._cfg = config or InterventionConfig()
        self._safe_mode = safe_mode
        self._limiter = RateLimiter(self._cfg.max_interventions_per_hour, 3600)
        self._cooldowns: Dict[str, float] = {}
        self._history: Deque[ActionRecord] = deque(maxlen=200)

    def set_safe_mode(self, on: bool) -> None:
        self._safe_mode = on

    def _in_cooldown(self, key: str) -> bool:
        last = self._cooldowns.get(key, 0)
        return time.time() - last < self._cfg.cooldown_s

    def _record_cooldown(self, key: str) -> None:
        self._cooldowns[key] = time.time()

    async def _execute(self, action: str, target: str, **kwargs) -> Tuple[OperatorResult, str]:
        """Subclasses must override. Returns (result, detail)."""
        raise NotImplementedError

    async def act(self, action: str, target: str, reason: str = "", **kwargs) -> ActionRecord:
        """Public entry point — enforces rate limits, cooldowns, audit."""
        cooldown_key = f"{self.name}:{action}:{target}"

        # Policy checks
        if self._safe_mode and action not in self._safe_actions():
            rec = ActionRecord(self.name, action, target, OperatorResult.BLOCKED,
                               "safe mode blocks this action")
            self._history.append(rec)
            self._audit.record_quick("intervention", "warning", self.name, action,
                                     target, "blocked by safe mode", {})
            return rec

        if not self._limiter.allow():
            rec = ActionRecord(self.name, action, target, OperatorResult.RATE_LIMITED,
                               f"rate limit hit ({self._cfg.max_interventions_per_hour}/hr)")
            self._history.append(rec)
            self._audit.record_quick("intervention", "warning", self.name, action,
                                     target, "rate limited", {})
            return rec

        if self._in_cooldown(cooldown_key):
            rec = ActionRecord(self.name, action, target, OperatorResult.RATE_LIMITED,
                               f"cooldown ({self._cfg.cooldown_s}s)")
            self._history.append(rec)
            return rec

        # Execute
        log.info("[%s] %s → %s (reason: %s)", self.name, action, target, reason)
        try:
            result, detail = await self._execute(action, target, **kwargs)
        except Exception as exc:
            result, detail = OperatorResult.FAILED, str(exc)
            log.exception("[%s] %s → %s FAILED", self.name, action, target)

        self._limiter.record()
        self._record_cooldown(cooldown_key)

        rec = ActionRecord(self.name, action, target, result, detail)
        self._history.append(rec)
        self._audit.record_quick(
            "intervention", "info" if result == OperatorResult.SUCCESS else "error",
            self.name, action, target, reason,
            {"result": result.value, "detail": detail}
        )
        return rec

    def _safe_actions(self) -> set:
        """Actions allowed even in safe mode. Override in subclass."""
        return set()

    def recent_actions(self, n: int = 20) -> List[dict]:
        return [
            {"operator": r.operator, "action": r.action, "target": r.target,
             "result": r.result.value, "detail": r.detail, "ts": r.ts}
            for r in list(self._history)[-n:]
        ]

    def snapshot(self) -> dict:
        return {
            "name": self.name,
            "safe_mode": self._safe_mode,
            "rate_remaining": self._limiter.remaining,
            "recent_actions": self.recent_actions(5),
        }


# ── Service Operator ─────────────────────────────────────────────────

class ServiceOperator(BaseOperator):
    """Restart / stop systemd services."""

    name = "service"

    # Whitelist of services we're allowed to touch
    ALLOWED_SERVICES = frozenset([
        "x3-chain-node",
        "x3-chain-health",
        "ollama_server",
        "ollama",
    ])

    async def _execute(self, action: str, target: str, **kwargs) -> Tuple[OperatorResult, str]:
        if target not in self.ALLOWED_SERVICES:
            return OperatorResult.BLOCKED, f"service '{target}' not in whitelist"

        if action == "restart":
            cmd = ["sudo", "systemctl", "restart", target]
        elif action == "stop":
            cmd = ["sudo", "systemctl", "stop", target]
        elif action == "start":
            cmd = ["sudo", "systemctl", "start", target]
        else:
            return OperatorResult.FAILED, f"unknown action: {action}"

        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        _, stderr = await asyncio.wait_for(proc.communicate(), timeout=30)
        if proc.returncode == 0:
            return OperatorResult.SUCCESS, f"{action} {target} OK"
        return OperatorResult.FAILED, stderr.decode(errors="replace")[:200]

    def _safe_actions(self) -> set:
        return {"restart"}  # allow restart even in safe mode


# ── GPU Operator ─────────────────────────────────────────────────────

class GPUOperator(BaseOperator):
    """GPU-specific interventions: power limit, persistence mode, clock reset."""

    name = "gpu"

    async def _execute(self, action: str, target: str, **kwargs) -> Tuple[OperatorResult, str]:
        gpu_id = target  # "0", "1", "2"

        if action == "set_power_limit":
            watts = kwargs.get("watts", 120)
            cmd = ["sudo", "nvidia-smi", "-i", gpu_id,
                   "-pl", str(watts)]
        elif action == "reset_clocks":
            cmd = ["sudo", "nvidia-smi", "-i", gpu_id, "-rgc"]
        elif action == "enable_persistence":
            cmd = ["sudo", "nvidia-smi", "-i", gpu_id, "-pm", "1"]
        elif action == "disable_persistence":
            cmd = ["sudo", "nvidia-smi", "-i", gpu_id, "-pm", "0"]
        elif action == "drain":
            # Set compute mode to prohibited — no new work
            cmd = ["sudo", "nvidia-smi", "-i", gpu_id,
                   "-c", "PROHIBITED"]
        elif action == "undrain":
            cmd = ["sudo", "nvidia-smi", "-i", gpu_id,
                   "-c", "DEFAULT"]
        else:
            return OperatorResult.FAILED, f"unknown GPU action: {action}"

        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=15)
        if proc.returncode == 0:
            return OperatorResult.SUCCESS, stdout.decode(errors="replace").strip()[:200]
        return OperatorResult.FAILED, stderr.decode(errors="replace")[:200]

    def _safe_actions(self) -> set:
        return {"set_power_limit", "reset_clocks", "drain"}


# ── Process Operator ─────────────────────────────────────────────────

class ProcessOperator(BaseOperator):
    """Kill or signal processes by PID or name."""

    name = "process"

    # Only kill processes whose cmdline matches these substrings
    ALLOWED_PATTERNS = frozenset([
        "python",
        "node",
        "x3",
        "swarm",
        "ollama",
    ])

    async def _execute(self, action: str, target: str, **kwargs) -> Tuple[OperatorResult, str]:
        if action == "kill":
            return await self._kill_pid(int(target), signal.SIGKILL)
        elif action == "terminate":
            return await self._kill_pid(int(target), signal.SIGTERM)
        elif action == "kill_by_name":
            return await self._kill_by_name(target)
        else:
            return OperatorResult.FAILED, f"unknown action: {action}"

    async def _kill_pid(self, pid: int, sig: signal.Signals) -> Tuple[OperatorResult, str]:
        # Verify PID is in our allowed patterns
        try:
            with open(f"/proc/{pid}/cmdline", "rb") as f:
                cmdline = f.read().decode(errors="replace").replace("\x00", " ")
        except FileNotFoundError:
            return OperatorResult.FAILED, f"PID {pid} does not exist"
        except PermissionError:
            return OperatorResult.FAILED, f"cannot read PID {pid} cmdline"

        if not any(pat in cmdline.lower() for pat in self.ALLOWED_PATTERNS):
            return OperatorResult.BLOCKED, f"PID {pid} cmdline not in whitelist: {cmdline[:80]}"

        try:
            os.kill(pid, sig)
            return OperatorResult.SUCCESS, f"sent {sig.name} to PID {pid}"
        except ProcessLookupError:
            return OperatorResult.FAILED, f"PID {pid} already gone"
        except PermissionError:
            return OperatorResult.FAILED, f"permission denied killing PID {pid}"

    async def _kill_by_name(self, name: str) -> Tuple[OperatorResult, str]:
        if not any(pat in name.lower() for pat in self.ALLOWED_PATTERNS):
            return OperatorResult.BLOCKED, f"'{name}' not in whitelist"
        proc = await asyncio.create_subprocess_exec(
            "pkill", "-f", name,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        _, stderr = await asyncio.wait_for(proc.communicate(), timeout=10)
        if proc.returncode == 0:
            return OperatorResult.SUCCESS, f"pkill -f {name} OK"
        elif proc.returncode == 1:
            return OperatorResult.SUCCESS, f"no matching processes for '{name}'"
        return OperatorResult.FAILED, stderr.decode(errors="replace")[:200]

    def _safe_actions(self) -> set:
        return {"terminate"}  # soft kill in safe mode


# ── Swarm Operator ───────────────────────────────────────────────────

class SwarmOperator(BaseOperator):
    """Swarm-level interventions: scale workers, pause jobs, flush queues.

    This operator talks to the swarm API server rather than the OS.
    """

    name = "swarm"

    def __init__(self, audit: AuditLog, swarm_url: str = "http://127.0.0.1:8080",
                 config: Optional[InterventionConfig] = None, safe_mode: bool = False):
        super().__init__(audit, config, safe_mode)
        self._swarm_url = swarm_url.rstrip("/")

    async def _execute(self, action: str, target: str, **kwargs) -> Tuple[OperatorResult, str]:
        import urllib.request
        import json

        if action == "pause_queue":
            return await self._http_post("/api/queue/pause", {"queue": target})
        elif action == "resume_queue":
            return await self._http_post("/api/queue/resume", {"queue": target})
        elif action == "scale_gpu_workers":
            count = kwargs.get("count", 1)
            return await self._http_post("/api/gpu/scale",
                                         {"gpu_id": target, "workers": count})
        elif action == "cancel_task":
            return await self._http_post("/api/tasks/cancel", {"task_id": target})
        elif action == "gc_agents":
            return await self._http_post("/api/agents/gc", {})
        else:
            return OperatorResult.FAILED, f"unknown swarm action: {action}"

    async def _http_post(self, path: str, body: dict) -> Tuple[OperatorResult, str]:
        import urllib.request
        import json

        url = f"{self._swarm_url}{path}"
        data = json.dumps(body).encode()
        req = urllib.request.Request(url, data=data, method="POST",
                                     headers={"Content-Type": "application/json"})
        try:
            loop = asyncio.get_event_loop()
            resp = await loop.run_in_executor(None, lambda: urllib.request.urlopen(req, timeout=10))
            resp_body = resp.read().decode()
            if resp.status < 300:
                return OperatorResult.SUCCESS, resp_body[:200]
            return OperatorResult.FAILED, f"HTTP {resp.status}: {resp_body[:200]}"
        except Exception as exc:
            return OperatorResult.FAILED, str(exc)[:200]

    def _safe_actions(self) -> set:
        return {"pause_queue", "cancel_task"}


# ── Operator Registry ───────────────────────────────────────────────

class OperatorRegistry:
    """Central registry of all operators."""

    def __init__(self):
        self._operators: Dict[str, BaseOperator] = {}

    def register(self, op: BaseOperator) -> None:
        self._operators[op.name] = op

    def get(self, name: str) -> Optional[BaseOperator]:
        return self._operators.get(name)

    def all(self) -> Dict[str, BaseOperator]:
        return dict(self._operators)

    def set_safe_mode(self, on: bool) -> None:
        for op in self._operators.values():
            op.set_safe_mode(on)

    def snapshot(self) -> dict:
        return {name: op.snapshot() for name, op in self._operators.items()}

    @classmethod
    def create_default(cls, audit: AuditLog, config: Optional[InterventionConfig] = None,
                       swarm_url: str = "http://127.0.0.1:8080") -> "OperatorRegistry":
        registry = cls()
        cfg = config or InterventionConfig()
        registry.register(ServiceOperator(audit, cfg))
        registry.register(GPUOperator(audit, cfg))
        registry.register(ProcessOperator(audit, cfg))
        registry.register(SwarmOperator(audit, swarm_url=swarm_url, config=cfg))
        return registry
