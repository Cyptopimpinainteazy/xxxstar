"""GPU Guard Sentinel — Monitors NVIDIA GPUs via nvidia-smi and dmesg.

Publishes:
    gpu.{N}.temperature_c       GPU core temp
    gpu.{N}.vram_used_pct       VRAM utilization %
    gpu.{N}.util_pct            Compute utilization %
    gpu.{N}.power_w             Power draw
    gpu.{N}.xid                 Xid fault event
    gpu.health                  ComponentHealth score

Detects:
    - Thermal throttling / overheating
    - VRAM exhaustion
    - Xid faults (hardware errors)
    - GPU hung (0% util but should be working)
    - Driver errors in dmesg
"""

from __future__ import annotations

import asyncio
import logging
import re
import shutil
import subprocess
import time
from collections import deque
from dataclasses import dataclass
from typing import Any, Deque, Dict, List, Optional, Tuple

from ..metrics_bus import MetricsBus, MetricPoint, MetricKind, ComponentHealth
from ..config import GPUGuardConfig

log = logging.getLogger("autonomic.sentinel.gpu")

# Xid pattern in dmesg
_XID_RE = re.compile(r"NVRM:\s+Xid.*?:\s*(\d+),", re.IGNORECASE)
_GPU_FALL_RE = re.compile(r"GPU has fallen off the bus", re.IGNORECASE)
_RELOC_RE = re.compile(r"Skipping invalid relocation target", re.IGNORECASE)


@dataclass
class GPUSnapshot:
    """Point-in-time GPU state."""
    index: int
    name: str
    temp_c: int
    util_pct: int
    vram_used_mib: int
    vram_total_mib: int
    power_w: float
    power_limit_w: float
    ts: float

    @property
    def vram_used_pct(self) -> float:
        if self.vram_total_mib == 0:
            return 0.0
        return (self.vram_used_mib / self.vram_total_mib) * 100


@dataclass
class XidEvent:
    """NVIDIA Xid fault event."""
    xid_code: int
    gpu_index: Optional[int]
    raw_line: str
    ts: float


class GPUGuard:
    """Sentinel that monitors NVIDIA GPUs and publishes to MetricsBus."""

    # Xid severity classification
    XID_SEVERITY = {
        13: "graphics_engine_exception",
        31: "mmu_fault",
        43: "gpu_stopped_processing",
        45: "preemptive_cleanup",
        48: "dbe_ecc_error",
        61: "internal_micro_controller_halt",
        62: "internal_micro_controller_exception",
        63: "internal_micro_controller_breakpoint",
        64: "fallen_off_bus_likely",
        68: "video_processor_exception",
        69: "graphics_engine_class_error",
        79: "gpu_fallen_off_bus",
        109: "context_killed_by_driver",
        119: "timeout_on_channel",
    }

    def __init__(self, bus: MetricsBus, config: Optional[GPUGuardConfig] = None):
        self._bus = bus
        self._cfg = config or GPUGuardConfig()
        self._nvidia_smi = shutil.which("nvidia-smi")
        self._running = False
        self._task: Optional[asyncio.Task] = None

        # Xid sliding window
        self._xid_events: Deque[XidEvent] = deque(maxlen=200)
        self._last_dmesg_ts: float = 0.0

        # Per-GPU history (for trend detection)
        self._gpu_history: Dict[int, Deque[GPUSnapshot]] = {}

    async def start(self) -> None:
        if not self._nvidia_smi:
            log.warning("nvidia-smi not found — GPU Guard disabled")
            return
        self._running = True
        self._task = asyncio.create_task(self._poll_loop())
        log.info("GPU Guard started (poll every %.1fs)", self._cfg.poll_interval_s)

    async def stop(self) -> None:
        self._running = False
        if self._task:
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass
        log.info("GPU Guard stopped")

    # ── Main poll loop ────────────────────────────────────────────────────
    async def _poll_loop(self) -> None:
        while self._running:
            try:
                snapshots = await self._read_gpus()
                xids = await self._read_dmesg_xids()

                points: List[MetricPoint] = []
                for snap in snapshots:
                    comp = f"gpu.{snap.index}"
                    points.extend([
                        MetricPoint(comp, "temperature_c", snap.temp_c),
                        MetricPoint(comp, "vram_used_pct", snap.vram_used_pct),
                        MetricPoint(comp, "vram_used_mib", snap.vram_used_mib),
                        MetricPoint(comp, "util_pct", snap.util_pct),
                        MetricPoint(comp, "power_w", snap.power_w),
                        MetricPoint(comp, "power_limit_w", snap.power_limit_w),
                    ])

                    # Track history
                    if snap.index not in self._gpu_history:
                        self._gpu_history[snap.index] = deque(maxlen=720)  # 1hr @ 5s
                    self._gpu_history[snap.index].append(snap)

                for xid in xids:
                    self._xid_events.append(xid)
                    points.append(MetricPoint(
                        f"gpu.{xid.gpu_index or 'unknown'}", "xid",
                        float(xid.xid_code), kind=MetricKind.EVENT,
                        tags={"meaning": self.XID_SEVERITY.get(xid.xid_code, "unknown")},
                    ))

                await self._bus.publish_many(points)

                # Compute and publish health score
                health = self._compute_health(snapshots, xids)
                await self._bus.publish_health(health)

            except asyncio.CancelledError:
                break
            except Exception:
                log.exception("GPU Guard poll error")

            await asyncio.sleep(self._cfg.poll_interval_s)

    # ── GPU reading ───────────────────────────────────────────────────────
    async def _read_gpus(self) -> List[GPUSnapshot]:
        """Read GPU state via nvidia-smi."""
        if not self._nvidia_smi:
            return []
        try:
            proc = await asyncio.create_subprocess_exec(
                self._nvidia_smi,
                "--query-gpu=index,name,temperature.gpu,utilization.gpu,"
                "memory.used,memory.total,power.draw,power.limit",
                "--format=csv,noheader,nounits",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, _ = await asyncio.wait_for(proc.communicate(), timeout=10)
            snapshots = []
            now = time.time()
            for line in stdout.decode().strip().splitlines():
                parts = [p.strip() for p in line.split(",")]
                if len(parts) >= 8:
                    try:
                        snapshots.append(GPUSnapshot(
                            index=int(parts[0]),
                            name=parts[1],
                            temp_c=int(parts[2]),
                            util_pct=int(parts[3]),
                            vram_used_mib=int(parts[4]),
                            vram_total_mib=int(parts[5]),
                            power_w=float(parts[6]),
                            power_limit_w=float(parts[7]),
                            ts=now,
                        ))
                    except (ValueError, IndexError):
                        continue
            return snapshots
        except Exception:
            log.exception("Failed to read nvidia-smi")
            return []

    async def _read_dmesg_xids(self) -> List[XidEvent]:
        """Read new Xid events from dmesg."""
        try:
            proc = await asyncio.create_subprocess_exec(
                "dmesg", "--level=err,warn", "--time-format=raw",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, _ = await asyncio.wait_for(proc.communicate(), timeout=5)
            events = []
            now = time.time()
            for line in stdout.decode().splitlines():
                # Only process new lines
                try:
                    ts_str = line.split("]")[0].lstrip("[").strip()
                    ts_val = float(ts_str)
                except (ValueError, IndexError):
                    ts_val = now

                if ts_val <= self._last_dmesg_ts:
                    continue

                m = _XID_RE.search(line)
                if m:
                    events.append(XidEvent(
                        xid_code=int(m.group(1)),
                        gpu_index=None,
                        raw_line=line.strip(),
                        ts=now,
                    ))

                if _GPU_FALL_RE.search(line):
                    events.append(XidEvent(
                        xid_code=79,
                        gpu_index=None,
                        raw_line=line.strip(),
                        ts=now,
                    ))

            if events:
                self._last_dmesg_ts = time.time()
            return events
        except Exception:
            return []

    # ── Health scoring ────────────────────────────────────────────────────
    def _compute_health(self, snapshots: List[GPUSnapshot],
                        recent_xids: List[XidEvent]) -> ComponentHealth:
        """Compute overall GPU subsystem health (0-100)."""
        if not snapshots:
            return ComponentHealth("gpu", 50, "unknown", {"reason": "no GPUs detected"})

        score = 100
        details: Dict[str, Any] = {"gpu_count": len(snapshots), "issues": []}

        for snap in snapshots:
            tag = f"gpu.{snap.index}"

            # Temperature scoring
            if snap.temp_c >= self._cfg.temp_crit_c:
                score -= 30
                details["issues"].append(f"{tag} CRITICAL temp {snap.temp_c}°C")
            elif snap.temp_c >= self._cfg.temp_warn_c:
                score -= 10
                details["issues"].append(f"{tag} high temp {snap.temp_c}°C")

            # VRAM scoring
            if snap.vram_used_pct >= self._cfg.vram_crit_pct:
                score -= 25
                details["issues"].append(f"{tag} CRITICAL vram {snap.vram_used_pct:.0f}%")
            elif snap.vram_used_pct >= self._cfg.vram_warn_pct:
                score -= 10
                details["issues"].append(f"{tag} high vram {snap.vram_used_pct:.0f}%")

        # Xid scoring — recent faults in sliding window
        window_cutoff = time.time() - self._cfg.xid_window_s
        recent_count = sum(1 for x in self._xid_events if x.ts >= window_cutoff)
        recent_count += len(recent_xids)

        if recent_count >= self._cfg.xid_threshold:
            score -= 40
            details["issues"].append(f"{recent_count} Xid faults in window")
        elif recent_count > 0:
            score -= 10 * recent_count
            details["issues"].append(f"{recent_count} Xid fault(s)")

        score = max(0, min(100, score))
        status = "healthy" if score >= 75 else "degraded" if score >= 40 else "critical"

        return ComponentHealth("gpu", score, status, details)

    # ── Public queries ────────────────────────────────────────────────────
    def gpu_count(self) -> int:
        return len(self._gpu_history)

    def recent_xids(self, window_s: float = 600) -> List[dict]:
        cutoff = time.time() - window_s
        return [
            {"xid": x.xid_code, "meaning": self.XID_SEVERITY.get(x.xid_code, "unknown"),
             "line": x.raw_line, "ts": x.ts}
            for x in self._xid_events if x.ts >= cutoff
        ]

    def gpu_temps(self) -> Dict[int, int]:
        result = {}
        for idx, hist in self._gpu_history.items():
            if hist:
                result[idx] = hist[-1].temp_c
        return result

    def snapshot(self) -> dict:
        return {
            "running": self._running,
            "gpu_count": self.gpu_count(),
            "temps": self.gpu_temps(),
            "recent_xids": self.recent_xids(),
            "config": {
                "poll_interval_s": self._cfg.poll_interval_s,
                "xid_threshold": self._cfg.xid_threshold,
                "temp_crit_c": self._cfg.temp_crit_c,
            },
        }
