"""Resource Monitor Sentinel — CPU, RAM, Disk, Load, File Descriptors.

Publishes:
    system.ram_used_pct
    system.swap_used_pct
    system.disk_used_pct
    system.load_1m / load_5m / load_15m
    system.cpu_temp_c
    system.fd_used_pct
    system.health               ComponentHealth score
"""

from __future__ import annotations

import asyncio
import logging
import os
import time
from typing import Any, Dict, List, Optional

from ..metrics_bus import MetricsBus, MetricPoint, ComponentHealth
from ..config import ResourceMonitorConfig

log = logging.getLogger("autonomic.sentinel.resource")


class ResourceMonitor:
    """Sentinel that monitors system-level resources."""

    def __init__(self, bus: MetricsBus, config: Optional[ResourceMonitorConfig] = None):
        self._bus = bus
        self._cfg = config or ResourceMonitorConfig()
        self._running = False
        self._task: Optional[asyncio.Task] = None
        self._cpu_count = os.cpu_count() or 1

    async def start(self) -> None:
        self._running = True
        self._task = asyncio.create_task(self._poll_loop())
        log.info("Resource Monitor started (poll every %.1fs)", self._cfg.poll_interval_s)

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
                points, health = self._collect()
                await self._bus.publish_many(points)
                await self._bus.publish_health(health)
            except asyncio.CancelledError:
                break
            except Exception:
                log.exception("Resource Monitor poll error")
            await asyncio.sleep(self._cfg.poll_interval_s)

    def _collect(self) -> tuple[List[MetricPoint], ComponentHealth]:
        score = 100
        issues: List[str] = []
        points: List[MetricPoint] = []

        # ── RAM ───────────────────────────────────────────────────────
        mem = self._read_meminfo()
        if mem:
            ram_pct = (1 - mem["available"] / max(mem["total"], 1)) * 100
            swap_pct = 0.0
            if mem["swap_total"] > 0:
                swap_pct = (mem["swap_total"] - mem["swap_free"]) / mem["swap_total"] * 100

            points.append(MetricPoint("system", "ram_used_pct", round(ram_pct, 1)))
            points.append(MetricPoint("system", "ram_available_mib", mem["available"] // 1024))
            points.append(MetricPoint("system", "swap_used_pct", round(swap_pct, 1)))

            if ram_pct >= self._cfg.ram_crit_pct:
                score -= 30
                issues.append(f"RAM CRITICAL {ram_pct:.0f}%")
            elif ram_pct >= self._cfg.ram_warn_pct:
                score -= 10
                issues.append(f"RAM high {ram_pct:.0f}%")

            if swap_pct >= self._cfg.swap_warn_pct:
                score -= 10
                issues.append(f"Swap high {swap_pct:.0f}%")

        # ── Disk ──────────────────────────────────────────────────────
        try:
            st = os.statvfs("/")
            disk_total = st.f_blocks * st.f_frsize
            disk_free = st.f_bavail * st.f_frsize
            disk_pct = (1 - disk_free / max(disk_total, 1)) * 100
            points.append(MetricPoint("system", "disk_used_pct", round(disk_pct, 1)))
            points.append(MetricPoint("system", "disk_free_gib", round(disk_free / (1024**3), 1)))

            if disk_pct >= self._cfg.disk_crit_pct:
                score -= 25
                issues.append(f"Disk CRITICAL {disk_pct:.0f}%")
            elif disk_pct >= self._cfg.disk_warn_pct:
                score -= 10
                issues.append(f"Disk high {disk_pct:.0f}%")
        except Exception:
            pass

        # ── Load Average ──────────────────────────────────────────────
        try:
            load1, load5, load15 = os.getloadavg()
            points.append(MetricPoint("system", "load_1m", round(load1, 2)))
            points.append(MetricPoint("system", "load_5m", round(load5, 2)))
            points.append(MetricPoint("system", "load_15m", round(load15, 2)))

            threshold = self._cpu_count * self._cfg.load_warn_multiplier
            if load1 > threshold:
                score -= 10
                issues.append(f"Load {load1:.1f} > {threshold:.0f} threshold")
        except Exception:
            pass

        # ── CPU Temperature ───────────────────────────────────────────
        cpu_temp = self._read_cpu_temp()
        if cpu_temp is not None:
            points.append(MetricPoint("system", "cpu_temp_c", cpu_temp))
            if cpu_temp >= self._cfg.cpu_temp_crit_c:
                score -= 25
                issues.append(f"CPU CRITICAL temp {cpu_temp}°C")
            elif cpu_temp >= self._cfg.cpu_temp_warn_c:
                score -= 10
                issues.append(f"CPU high temp {cpu_temp}°C")

        # ── File Descriptors ──────────────────────────────────────────
        fd_info = self._read_fd_usage()
        if fd_info:
            fd_pct = fd_info["used"] / max(fd_info["max"], 1) * 100
            points.append(MetricPoint("system", "fd_used_pct", round(fd_pct, 1)))
            points.append(MetricPoint("system", "fd_used", fd_info["used"]))
            if fd_pct >= self._cfg.fd_warn_pct:
                score -= 15
                issues.append(f"FD usage high {fd_pct:.0f}%")

        score = max(0, min(100, score))
        status = "healthy" if score >= 75 else "degraded" if score >= 40 else "critical"

        health = ComponentHealth("system", score, status,
                                 {"issues": issues, "cpu_count": self._cpu_count})
        return points, health

    # ── /proc readers ─────────────────────────────────────────────────
    @staticmethod
    def _read_meminfo() -> Optional[Dict[str, int]]:
        try:
            data = {}
            with open("/proc/meminfo") as f:
                for line in f:
                    parts = line.split()
                    if len(parts) >= 2:
                        key = parts[0].rstrip(":")
                        val = int(parts[1])  # kB
                        data[key] = val
            return {
                "total": data.get("MemTotal", 0) * 1024,
                "available": data.get("MemAvailable", 0) * 1024,
                "swap_total": data.get("SwapTotal", 0) * 1024,
                "swap_free": data.get("SwapFree", 0) * 1024,
            }
        except Exception:
            return None

    @staticmethod
    def _read_cpu_temp() -> Optional[int]:
        """Read CPU temp from thermal zones (k10temp / coretemp)."""
        try:
            # Try common thermal zones
            for i in range(10):
                path = f"/sys/class/thermal/thermal_zone{i}/temp"
                tpath = f"/sys/class/thermal/thermal_zone{i}/type"
                if os.path.exists(path) and os.path.exists(tpath):
                    with open(tpath) as f:
                        ttype = f.read().strip()
                    if ttype in ("x86_pkg_temp", "k10temp", "coretemp"):
                        with open(path) as f:
                            return int(f.read().strip()) // 1000
            # Fallback: just read zone 0
            path = "/sys/class/thermal/thermal_zone0/temp"
            if os.path.exists(path):
                with open(path) as f:
                    return int(f.read().strip()) // 1000
        except Exception:
            pass
        return None

    @staticmethod
    def _read_fd_usage() -> Optional[Dict[str, int]]:
        try:
            with open("/proc/sys/fs/file-nr") as f:
                parts = f.read().strip().split()
                return {"used": int(parts[0]), "max": int(parts[2])}
        except Exception:
            return None

    def snapshot(self) -> dict:
        points, health = self._collect()
        return {
            "running": self._running,
            "health": health.to_dict(),
            "metrics": {p.name: p.value for p in points},
        }
