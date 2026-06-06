"""Autonomic Control Plane — Configuration schema & defaults.

All tunables live here. Override via env vars or autonomic_config.json.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional

_CONFIG_PATH = os.getenv(
    "X3_AUTONOMIC_CONFIG",
    os.path.join(os.path.dirname(__file__), "..", "config", "autonomic_config.json"),
)


# ---------------------------------------------------------------------------
# Thresholds
# ---------------------------------------------------------------------------
@dataclass
class HealthThresholds:
    """Score thresholds that trigger state transitions."""
    normal_min: int = 75         # Below → DEGRADED
    degraded_min: int = 60       # Below → CONTAINMENT
    containment_min: int = 40    # Below → SAFE_MODE
    safe_mode_min: int = 20      # Below → MANUAL_REQUIRED


@dataclass
class GPUGuardConfig:
    """GPU-specific sentinel tunables."""
    poll_interval_s: float = 5.0
    xid_window_s: float = 600.0          # 10 min sliding window
    xid_threshold: int = 3               # 3 Xid faults → escalate
    temp_warn_c: int = 80
    temp_crit_c: int = 88
    vram_warn_pct: float = 90.0
    vram_crit_pct: float = 96.0
    util_floor_pct: int = 5              # Below = likely hung
    auto_scale_workers: bool = True
    min_workers: int = 1
    max_workers: int = 32


@dataclass
class ResourceMonitorConfig:
    """System-level resource sentinel tunables."""
    poll_interval_s: float = 10.0
    ram_warn_pct: float = 85.0
    ram_crit_pct: float = 95.0
    swap_warn_pct: float = 50.0
    disk_warn_pct: float = 85.0
    disk_crit_pct: float = 95.0
    load_warn_multiplier: float = 2.0     # warn if load > N * cpu_count
    cpu_temp_warn_c: int = 80
    cpu_temp_crit_c: int = 95
    fd_warn_pct: float = 80.0


@dataclass
class LogWatcherConfig:
    """Kernel / system-log sentinel tunables."""
    poll_interval_s: float = 2.0
    patterns: List[str] = field(default_factory=lambda: [
        "NVRM: Xid",
        "GPU has fallen off the bus",
        "Out of memory",
        "oom_reaper",
        "oom-kill",
        "segfault",
        "kernel panic",
        "BUG:",
        "Call Trace:",
        "watchdog",
        "EXT4-fs error",
        "I/O error",
        "CUDA_ERROR",
        "rm_init_adapter failed",
        "nvidia-modeset: ERROR",
        "Skipping invalid relocation target",
    ])


@dataclass
class CircuitBreakerConfig:
    """Per-module circuit breaker defaults."""
    failure_threshold: int = 5        # failures before OPEN
    recovery_timeout_s: float = 60.0  # seconds in OPEN before HALF_OPEN
    half_open_max: int = 2            # successes needed to CLOSE
    window_s: float = 300.0           # sliding window for failure counting


@dataclass
class InterventionConfig:
    """Rate-limiting & cooldown for operator actions."""
    cooldown_s: float = 30.0          # min seconds between same intervention
    max_interventions_per_hour: int = 20
    escalation_after: int = 3         # escalate to next level after N retries
    require_simulation: bool = False  # Phase 3: simulate before act


@dataclass
class SafeModeProfile:
    """What happens in SAFE_MODE."""
    max_gpu_workers: int = 2
    max_strategies: int = 1
    disable_mutations: bool = True
    reduce_rpc_concurrency: int = 4
    gpu_power_limit_pct: int = 70
    log_level: str = "WARNING"


# ---------------------------------------------------------------------------
# Top-Level Config
# ---------------------------------------------------------------------------
@dataclass
class AutonomicConfig:
    """Full control-plane configuration."""
    enabled: bool = True
    health: HealthThresholds = field(default_factory=HealthThresholds)
    gpu: GPUGuardConfig = field(default_factory=GPUGuardConfig)
    resource: ResourceMonitorConfig = field(default_factory=ResourceMonitorConfig)
    log_watcher: LogWatcherConfig = field(default_factory=LogWatcherConfig)
    circuit_breaker: CircuitBreakerConfig = field(default_factory=CircuitBreakerConfig)
    intervention: InterventionConfig = field(default_factory=InterventionConfig)
    safe_mode: SafeModeProfile = field(default_factory=SafeModeProfile)
    audit_log_dir: str = "logs/autonomic"
    audit_max_memory: int = 1000
    metrics_retention_s: float = 3600.0    # 1 hour in-memory
    health_publish_interval_s: float = 10.0
    swarm_api_url: str = "http://127.0.0.1:8080"

    def to_dict(self) -> dict:
        return asdict(self)

    @classmethod
    def load(cls, path: Optional[str] = None) -> "AutonomicConfig":
        """Load from JSON file with env-var override, falling back to defaults."""
        p = path or _CONFIG_PATH
        cfg = cls()
        if os.path.isfile(p):
            try:
                with open(p) as f:
                    data = json.load(f)
                cfg = _merge(cfg, data)
            except Exception:
                pass  # fall back to defaults
        return cfg


def _merge(cfg: AutonomicConfig, data: dict) -> AutonomicConfig:
    """Shallow-merge a dict into the config dataclass."""
    for key, val in data.items():
        if hasattr(cfg, key):
            attr = getattr(cfg, key)
            if hasattr(attr, "__dataclass_fields__") and isinstance(val, dict):
                for k2, v2 in val.items():
                    if hasattr(attr, k2):
                        setattr(attr, k2, v2)
            else:
                setattr(cfg, key, val)
    return cfg
