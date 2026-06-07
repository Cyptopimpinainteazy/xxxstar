"""Autonomic Control Plane — Metrics Bus.

Centralized in-memory pub/sub for structured telemetry.
All sentinels publish here. The HealthEngine and Orchestrator consume.

Metrics are time-series points with component tags.
Retention is configurable (default 1 hour sliding window).
"""

from __future__ import annotations

import asyncio
import logging
import time
from collections import defaultdict, deque
from dataclasses import dataclass, field, asdict
from enum import Enum
from typing import Any, Callable, Deque, Dict, List, Optional, Set

log = logging.getLogger("autonomic.metrics")


# ---------------------------------------------------------------------------
# Metric point
# ---------------------------------------------------------------------------
class MetricKind(str, Enum):
    GAUGE = "gauge"
    COUNTER = "counter"
    EVENT = "event"


@dataclass
class MetricPoint:
    """A single telemetry sample."""
    component: str            # e.g. "gpu.0", "system.ram", "rpc.mesh"
    name: str                 # e.g. "temperature_c", "vram_used_pct"
    value: float
    kind: MetricKind = MetricKind.GAUGE
    tags: Dict[str, str] = field(default_factory=dict)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        d = asdict(self)
        d["kind"] = self.kind.value
        return d


@dataclass
class ComponentHealth:
    """Snapshot of a single component's health."""
    component: str
    score: int              # 0-100
    status: str             # "healthy", "degraded", "critical", "unknown"
    details: Dict[str, Any] = field(default_factory=dict)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


# ---------------------------------------------------------------------------
# Subscription callback type
# ---------------------------------------------------------------------------
MetricsCallback = Callable[[MetricPoint], Any]
HealthCallback = Callable[[ComponentHealth], Any]


# ---------------------------------------------------------------------------
# MetricsBus
# ---------------------------------------------------------------------------
class MetricsBus:
    """In-memory metrics pub/sub with sliding-window retention.

    Usage:
        bus = MetricsBus(retention_s=3600)
        bus.subscribe("gpu.*", my_callback)       # wildcards OK
        await bus.publish(MetricPoint(...))
    """

    def __init__(self, retention_s: float = 3600.0):
        self._retention_s = retention_s
        # component → deque of MetricPoint (time-ordered)
        self._series: Dict[str, Deque[MetricPoint]] = defaultdict(
            lambda: deque(maxlen=10_000)
        )
        # Health scores per component
        self._health: Dict[str, ComponentHealth] = {}
        # Subscribers: pattern → list of callbacks
        self._metric_subs: Dict[str, List[MetricsCallback]] = defaultdict(list)
        self._health_subs: Dict[str, List[HealthCallback]] = defaultdict(list)
        self._lock = asyncio.Lock()

    # ── Publish ───────────────────────────────────────────────────────────
    async def publish(self, point: MetricPoint) -> None:
        """Publish a metric point, notify subscribers, evict stale data."""
        async with self._lock:
            self._series[point.component].append(point)
            self._evict(point.component)

        # Notify subscribers (fire-and-forget)
        for pattern, cbs in self._metric_subs.items():
            if self._match(pattern, point.component):
                for cb in cbs:
                    try:
                        result = cb(point)
                        if asyncio.iscoroutine(result):
                            await result
                    except Exception:
                        log.exception("Metric subscriber error: %s", pattern)

    async def publish_health(self, health: ComponentHealth) -> None:
        """Publish a component health score."""
        self._health[health.component] = health

        for pattern, cbs in self._health_subs.items():
            if self._match(pattern, health.component):
                for cb in cbs:
                    try:
                        result = cb(health)
                        if asyncio.iscoroutine(result):
                            await result
                    except Exception:
                        log.exception("Health subscriber error: %s", pattern)

    async def publish_many(self, points: List[MetricPoint]) -> None:
        """Batch-publish. More efficient for sentinel loops."""
        for p in points:
            await self.publish(p)

    # ── Subscribe ─────────────────────────────────────────────────────────
    def subscribe(self, pattern: str, callback: MetricsCallback) -> None:
        """Subscribe to metrics matching a component pattern (supports * wildcard)."""
        self._metric_subs[pattern].append(callback)

    def subscribe_health(self, pattern: str, callback: HealthCallback) -> None:
        """Subscribe to health score updates."""
        self._health_subs[pattern].append(callback)

    # ── Query ─────────────────────────────────────────────────────────────
    def latest(self, component: str) -> Optional[MetricPoint]:
        """Most recent metric for a component."""
        dq = self._series.get(component)
        return dq[-1] if dq else None

    def series(self, component: str, window_s: Optional[float] = None) -> List[MetricPoint]:
        """Time-series for a component within window."""
        dq = self._series.get(component, deque())
        if window_s is None:
            return list(dq)
        cutoff = time.time() - window_s
        return [p for p in dq if p.ts >= cutoff]

    def all_health(self) -> Dict[str, ComponentHealth]:
        """Current health scores for all components."""
        return dict(self._health)

    def component_health(self, component: str) -> Optional[ComponentHealth]:
        """Health score for a specific component."""
        return self._health.get(component)

    def components(self) -> Set[str]:
        """All known component names."""
        return set(self._series.keys())

    def system_score(self) -> int:
        """Weighted average of all component health scores (0-100)."""
        scores = [h.score for h in self._health.values()]
        if not scores:
            return 100
        return int(sum(scores) / len(scores))

    def snapshot(self) -> Dict[str, Any]:
        """Full system snapshot for API/dashboard."""
        return {
            "system_score": self.system_score(),
            "components": {k: v.to_dict() for k, v in self._health.items()},
            "series_counts": {k: len(v) for k, v in self._series.items()},
            "ts": time.time(),
        }

    # ── Internal ──────────────────────────────────────────────────────────
    def _evict(self, component: str) -> None:
        """Remove points older than retention window."""
        dq = self._series[component]
        cutoff = time.time() - self._retention_s
        while dq and dq[0].ts < cutoff:
            dq.popleft()

    @staticmethod
    def _match(pattern: str, component: str) -> bool:
        """Simple wildcard matching:  'gpu.*' matches 'gpu.0', 'gpu.1' etc."""
        if pattern == "*":
            return True
        if "*" not in pattern:
            return pattern == component
        prefix = pattern.rstrip("*").rstrip(".")
        return component.startswith(prefix)
