"""
X3 Operator Health Check
~~~~~~~~~~~~~~~~~~~~~~~~

Preflight hardware validation for operator onboarding.
Fail fast. No silent failures.
"""

import logging
import os
import platform
import shutil
import subprocess
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional

logger = logging.getLogger(__name__)


class HealthStatus(str, Enum):
    PASS = "PASS"
    WARN = "WARN"
    FAIL = "FAIL"


@dataclass
class CheckResult:
    name: str
    status: HealthStatus
    value: str
    required: str
    message: str = ""


@dataclass
class HealthReport:
    overall: HealthStatus = HealthStatus.PASS
    checks: list = field(default_factory=list)
    recommended_roles: list = field(default_factory=list)

    def add(self, result: CheckResult):
        self.checks.append(result)
        if result.status == HealthStatus.FAIL:
            self.overall = HealthStatus.FAIL
        elif result.status == HealthStatus.WARN and self.overall != HealthStatus.FAIL:
            self.overall = HealthStatus.WARN


def _get_disk_gb(path: str = "/") -> float:
    """Available disk space in GB."""
    usage = shutil.disk_usage(path)
    return usage.free / (1024 ** 3)


def _get_ram_gb() -> float:
    """Total system RAM in GB."""
    try:
        with open("/proc/meminfo") as f:
            for line in f:
                if line.startswith("MemTotal:"):
                    kb = int(line.split()[1])
                    return kb / (1024 ** 2)
    except FileNotFoundError:
        pass
    # macOS fallback
    try:
        result = subprocess.run(
            ["sysctl", "-n", "hw.memsize"],
            capture_output=True, text=True, timeout=5,
        )
        if result.returncode == 0:
            return int(result.stdout.strip()) / (1024 ** 3)
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass
    return 0.0


def _get_cpu_cores() -> int:
    return os.cpu_count() or 0


def _check_gpu() -> Optional[dict]:
    """Detect GPU via nvidia-smi."""
    try:
        result = subprocess.run(
            ["nvidia-smi", "--query-gpu=name,memory.total,temperature.gpu,driver_version",
             "--format=csv,noheader,nounits"],
            capture_output=True, text=True, timeout=10,
        )
        if result.returncode == 0:
            parts = result.stdout.strip().split(", ")
            if len(parts) >= 4:
                return {
                    "name": parts[0],
                    "vram_mb": int(parts[1]),
                    "temperature_c": float(parts[2]),
                    "driver": parts[3],
                }
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass
    return None


def _check_clock_sync() -> bool:
    """Verify NTP time sync is active."""
    try:
        result = subprocess.run(
            ["timedatectl", "show", "--property=NTPSynchronized", "--value"],
            capture_output=True, text=True, timeout=5,
        )
        return result.stdout.strip().lower() == "yes"
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass
    return False


def run_health_check(
    min_disk_gb: int = 100,
    min_ram_gb: int = 8,
    min_cpu_cores: int = 4,
    require_gpu: bool = False,
    min_vram_mb: int = 4096,
) -> HealthReport:
    """Execute full hardware health check.

    Returns a HealthReport with pass/warn/fail for each subsystem.
    """
    report = HealthReport()

    # Disk
    disk_gb = _get_disk_gb()
    status = HealthStatus.PASS if disk_gb >= min_disk_gb else HealthStatus.FAIL
    report.add(CheckResult(
        name="disk_space",
        status=status,
        value=f"{disk_gb:.1f} GB",
        required=f">= {min_disk_gb} GB",
    ))

    # RAM
    ram_gb = _get_ram_gb()
    status = HealthStatus.PASS if ram_gb >= min_ram_gb else HealthStatus.FAIL
    report.add(CheckResult(
        name="ram",
        status=status,
        value=f"{ram_gb:.1f} GB",
        required=f">= {min_ram_gb} GB",
    ))

    # CPU
    cores = _get_cpu_cores()
    status = HealthStatus.PASS if cores >= min_cpu_cores else HealthStatus.FAIL
    report.add(CheckResult(
        name="cpu_cores",
        status=status,
        value=str(cores),
        required=f">= {min_cpu_cores}",
    ))

    # GPU
    gpu = _check_gpu()
    if require_gpu:
        if gpu is None:
            report.add(CheckResult(
                name="gpu",
                status=HealthStatus.FAIL,
                value="not detected",
                required="nvidia GPU with nvidia-smi",
            ))
        else:
            vram_ok = gpu["vram_mb"] >= min_vram_mb
            temp_ok = gpu["temperature_c"] < 85.0
            status = HealthStatus.PASS if (vram_ok and temp_ok) else HealthStatus.WARN
            report.add(CheckResult(
                name="gpu",
                status=status,
                value=f"{gpu['name']} ({gpu['vram_mb']} MB, {gpu['temperature_c']}°C)",
                required=f">= {min_vram_mb} MB VRAM, < 85°C",
            ))
    else:
        if gpu:
            report.add(CheckResult(
                name="gpu",
                status=HealthStatus.PASS,
                value=f"{gpu['name']} ({gpu['vram_mb']} MB)",
                required="optional",
                message="GPU detected - eligible for GPU operator role",
            ))

    # Clock sync
    synced = _check_clock_sync()
    report.add(CheckResult(
        name="clock_sync",
        status=HealthStatus.PASS if synced else HealthStatus.WARN,
        value="synchronized" if synced else "not synchronized",
        required="NTP synchronized",
        message="" if synced else "Clock not synced - may cause consensus issues",
    ))

    # OS
    report.add(CheckResult(
        name="os",
        status=HealthStatus.PASS,
        value=f"{platform.system()} {platform.release()} ({platform.machine()})",
        required="Linux recommended",
    ))

    # Recommended roles based on hardware
    roles = ["validator", "relayer"]
    if gpu and gpu["vram_mb"] >= min_vram_mb:
        roles.append("gpu")
    if disk_gb >= 500:
        roles.append("storage")
    report.recommended_roles = roles

    return report
