"""Autonomic Control Plane — Structured Audit Logger.

Immutable, append-only JSONL log for every autonomic action.
Answers: who, what, when, why, result, config version.
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import time
from dataclasses import dataclass, field, asdict
from enum import Enum
from typing import Any, Dict, List, Optional

log = logging.getLogger("autonomic.audit")


class AuditSeverity(str, Enum):
    INFO = "info"
    WARN = "warn"
    ERROR = "error"
    CRITICAL = "critical"


class AuditCategory(str, Enum):
    STATE_CHANGE = "state_change"
    INTERVENTION = "intervention"
    ESCALATION = "escalation"
    CIRCUIT_BREAKER = "circuit_breaker"
    SENTINEL_ALERT = "sentinel_alert"
    HEALTH_SCORE = "health_score"
    CONFIG_CHANGE = "config_change"
    HUMAN_OVERRIDE = "human_override"
    SAFE_MODE = "safe_mode"
    STARTUP = "startup"
    SHUTDOWN = "shutdown"


@dataclass
class AuditEntry:
    """A single audit record."""
    category: AuditCategory
    severity: AuditSeverity
    actor: str              # "gpu_guard", "orchestrator", "human", etc.
    action: str             # "restart_service", "scale_workers", etc.
    target: str             # "x3-chain-node", "gpu.2", etc.
    reason: str             # Human-readable reason
    result: str = "pending"  # "success", "failed", "skipped"
    details: Dict[str, Any] = field(default_factory=dict)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        d = asdict(self)
        d["category"] = self.category.value if hasattr(self.category, 'value') else self.category
        d["severity"] = self.severity.value if hasattr(self.severity, 'value') else self.severity
        return d


class AuditLog:
    """Append-only JSONL audit log with in-memory recent buffer."""

    def __init__(self, path: str = "logs/autonomic_audit.jsonl",
                 buffer_size: int = 1000,
                 log_dir: Optional[str] = None,
                 max_memory: Optional[int] = None):
        if log_dir:
            self._path = os.path.join(log_dir, "audit.jsonl")
        else:
            self._path = path
        self._buffer: List[AuditEntry] = []
        self._buffer_size = max_memory or buffer_size
        self._lock = asyncio.Lock()
        os.makedirs(os.path.dirname(self._path) or ".", exist_ok=True)

    async def record(self, entry: AuditEntry) -> None:
        """Append an audit entry to disk + memory buffer."""
        async with self._lock:
            self._buffer.append(entry)
            if len(self._buffer) > self._buffer_size:
                self._buffer = self._buffer[-self._buffer_size:]

            try:
                with open(self._path, "a") as f:
                    f.write(json.dumps(entry.to_dict()) + "\n")
            except Exception:
                log.exception("Failed to write audit entry to %s", self._path)

        log.info("[AUDIT] %s | %s | %s → %s | %s | %s",
                 entry.severity.value.upper(),
                 entry.actor,
                 entry.action,
                 entry.target,
                 entry.result,
                 entry.reason)

    def record_quick(
        self,
        category: str,
        severity: str,
        actor: str,
        action: str,
        target: str,
        reason: str,
        details: Any = None,
    ) -> None:
        """Synchronous convenience wrapper — fire and forget."""
        entry = AuditEntry(
            category=category,
            severity=severity,
            actor=actor,
            action=action,
            target=target,
            reason=reason,
            result="logged",
            details=details or {},
        )
        self._buffer.append(entry)
        if len(self._buffer) > self._buffer_size:
            self._buffer = self._buffer[-self._buffer_size:]
        try:
            with open(self._path, "a") as f:
                f.write(json.dumps(entry.to_dict()) + "\n")
        except Exception:
            log.exception("Failed to write audit entry")

        log.info("[AUDIT] %s | %s | %s → %s | %s",
                 entry.category, entry.severity, entry.actor,
                 entry.action, entry.target)

    def recent(self, n: int = 50, category: Optional[AuditCategory] = None) -> List[dict]:
        """Return recent audit entries."""
        entries = self._buffer
        if category:
            entries = [e for e in entries if e.category == category]
        return [e.to_dict() for e in entries[-n:]]

    def search(self, actor: Optional[str] = None, target: Optional[str] = None,
               since: Optional[float] = None) -> List[dict]:
        """Search buffer by actor/target/time."""
        results = []
        for e in self._buffer:
            if actor and e.actor != actor:
                continue
            if target and e.target != target:
                continue
            if since and e.ts < since:
                continue
            results.append(e.to_dict())
        return results
