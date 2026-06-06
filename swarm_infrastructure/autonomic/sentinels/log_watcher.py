"""Log Watcher Sentinel — Watches kernel & system logs for crash patterns.

Monitors journalctl/dmesg for predefined danger patterns and publishes
events to the MetricsBus.

Publishes:
    logs.kernel_errors          Error count in window
    logs.xid_events             NVIDIA Xid events
    logs.oom_events             OOM kill events
    logs.health                 ComponentHealth score
"""

from __future__ import annotations

import asyncio
import logging
import re
import time
from collections import defaultdict, deque
from dataclasses import dataclass
from typing import Any, Deque, Dict, List, Optional, Tuple

from ..metrics_bus import MetricsBus, MetricPoint, MetricKind, ComponentHealth
from ..config import LogWatcherConfig

log = logging.getLogger("autonomic.sentinel.logwatch")


@dataclass
class LogEvent:
    """A matched dangerous log line."""
    pattern: str
    line: str
    source: str  # "dmesg", "journal"
    ts: float


class LogWatcher:
    """Sentinel that watches system logs for crash patterns."""

    def __init__(self, bus: MetricsBus, config: Optional[LogWatcherConfig] = None):
        self._bus = bus
        self._cfg = config or LogWatcherConfig()
        self._running = False
        self._task: Optional[asyncio.Task] = None
        self._events: Deque[LogEvent] = deque(maxlen=500)
        self._pattern_counts: Dict[str, int] = defaultdict(int)
        self._compiled = [re.compile(re.escape(p), re.IGNORECASE) for p in self._cfg.patterns]
        self._last_journal_cursor: Optional[str] = None

    async def start(self) -> None:
        self._running = True
        self._task = asyncio.create_task(self._poll_loop())
        log.info("Log Watcher started (poll every %.1fs, %d patterns)",
                 self._cfg.poll_interval_s, len(self._cfg.patterns))

    async def stop(self) -> None:
        self._running = False
        if self._task:
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass

    async def _poll_loop(self) -> None:
        while self._running:
            try:
                events = await self._check_journal()
                if events:
                    now = time.time()
                    for evt in events:
                        self._events.append(evt)
                        self._pattern_counts[evt.pattern] += 1

                    # Publish metrics
                    points = [
                        MetricPoint("logs", "new_events", len(events), kind=MetricKind.COUNTER),
                    ]

                    # Categorized counts
                    oom = sum(1 for e in events if "oom" in e.pattern.lower())
                    xid = sum(1 for e in events if "xid" in e.pattern.lower() or "nvidia" in e.pattern.lower())
                    segfault = sum(1 for e in events if "segfault" in e.pattern.lower())

                    if oom:
                        points.append(MetricPoint("logs", "oom_events", oom, kind=MetricKind.EVENT))
                    if xid:
                        points.append(MetricPoint("logs", "nvidia_events", xid, kind=MetricKind.EVENT))
                    if segfault:
                        points.append(MetricPoint("logs", "segfault_events", segfault, kind=MetricKind.EVENT))

                    await self._bus.publish_many(points)

                # Always publish health
                health = self._compute_health()
                await self._bus.publish_health(health)

            except asyncio.CancelledError:
                break
            except Exception:
                log.exception("Log Watcher poll error")

            await asyncio.sleep(self._cfg.poll_interval_s)

    async def _check_journal(self) -> List[LogEvent]:
        """Read recent journalctl entries and match patterns."""
        cmd = ["journalctl", "--no-pager", "-p", "err", "-b", "--output=short-monotonic"]
        if self._last_journal_cursor:
            cmd += ["--after-cursor", self._last_journal_cursor]
        else:
            # On first run, only look at last 60 seconds
            cmd += ["--since", "60 seconds ago"]

        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, _ = await asyncio.wait_for(proc.communicate(), timeout=10)
            lines = stdout.decode(errors="replace").splitlines()
        except Exception:
            return []

        events = []
        now = time.time()
        for line in lines:
            # Skip empty or header lines
            if not line.strip() or line.startswith("--"):
                continue
            for i, regex in enumerate(self._compiled):
                if regex.search(line):
                    events.append(LogEvent(
                        pattern=self._cfg.patterns[i],
                        line=line.strip()[:300],  # truncate long lines
                        source="journal",
                        ts=now,
                    ))
                    break  # one match per line is enough

        # Update cursor for next poll (use journalctl --show-cursor)
        try:
            proc2 = await asyncio.create_subprocess_exec(
                "journalctl", "--no-pager", "-p", "err", "-b", "-n", "1",
                "--show-cursor", "--output=short",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout2, _ = await asyncio.wait_for(proc2.communicate(), timeout=5)
            for cline in stdout2.decode().splitlines():
                if cline.startswith("-- cursor:"):
                    self._last_journal_cursor = cline.split(":", 1)[1].strip()
                    break
        except Exception:
            pass

        return events

    def _compute_health(self) -> ComponentHealth:
        """Health based on recent error frequency."""
        window = 300  # 5 min
        cutoff = time.time() - window
        recent = [e for e in self._events if e.ts >= cutoff]
        count = len(recent)

        if count == 0:
            return ComponentHealth("logs", 100, "healthy", {"recent_errors": 0})

        score = 100
        issues = []

        # OOM is very severe
        oom = sum(1 for e in recent if "oom" in e.pattern.lower())
        if oom > 0:
            score -= 40
            issues.append(f"{oom} OOM events")

        # Nvidia/GPU errors
        gpu_errs = sum(1 for e in recent if any(p in e.pattern.lower()
                       for p in ("xid", "nvidia", "gpu", "cuda")))
        if gpu_errs >= 5:
            score -= 30
            issues.append(f"{gpu_errs} GPU errors")
        elif gpu_errs > 0:
            score -= 10
            issues.append(f"{gpu_errs} GPU error(s)")

        # Generic error volume
        if count >= 50:
            score -= 20
            issues.append(f"{count} total errors in 5min")
        elif count >= 20:
            score -= 10
            issues.append(f"{count} errors in 5min")

        score = max(0, min(100, score))
        status = "healthy" if score >= 75 else "degraded" if score >= 40 else "critical"
        return ComponentHealth("logs", score, status,
                               {"recent_errors": count, "issues": issues})

    def recent_events(self, n: int = 20) -> List[dict]:
        return [
            {"pattern": e.pattern, "line": e.line, "source": e.source, "ts": e.ts}
            for e in list(self._events)[-n:]
        ]

    def snapshot(self) -> dict:
        return {
            "running": self._running,
            "total_events": len(self._events),
            "pattern_counts": dict(self._pattern_counts),
            "recent": self.recent_events(10),
        }
