"""
X3 Operator Telemetry
~~~~~~~~~~~~~~~~~~~~

Structured logging and Prometheus-compatible metrics.
"""

import json
import logging
import time
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Optional

logger = logging.getLogger(__name__)


class MetricType(str, Enum):
    COUNTER = "counter"
    GAUGE = "gauge"
    HISTOGRAM = "histogram"


@dataclass
class Metric:
    name: str
    metric_type: MetricType
    value: float = 0.0
    labels: dict = field(default_factory=dict)
    help_text: str = ""
    updated_at: float = 0.0

    def increment(self, amount: float = 1.0):
        self.value += amount
        self.updated_at = time.time()

    def set_value(self, value: float):
        self.value = value
        self.updated_at = time.time()

    def to_prometheus(self) -> str:
        label_str = ""
        if self.labels:
            pairs = [f'{k}="{v}"' for k, v in sorted(self.labels.items())]
            label_str = "{" + ",".join(pairs) + "}"
        lines = []
        if self.help_text:
            lines.append(f"# HELP {self.name} {self.help_text}")
        lines.append(f"# TYPE {self.name} {self.metric_type.value}")
        lines.append(f"{self.name}{label_str} {self.value}")
        return "\n".join(lines)


class MetricsRegistry:
    """Thread-safe metrics collection with Prometheus export."""

    def __init__(self):
        self._metrics: dict[str, Metric] = {}

    def counter(self, name: str, help_text: str = "", labels: Optional[dict] = None) -> Metric:
        key = self._key(name, labels)
        if key not in self._metrics:
            self._metrics[key] = Metric(
                name=name,
                metric_type=MetricType.COUNTER,
                labels=labels or {},
                help_text=help_text,
            )
        return self._metrics[key]

    def gauge(self, name: str, help_text: str = "", labels: Optional[dict] = None) -> Metric:
        key = self._key(name, labels)
        if key not in self._metrics:
            self._metrics[key] = Metric(
                name=name,
                metric_type=MetricType.GAUGE,
                labels=labels or {},
                help_text=help_text,
            )
        return self._metrics[key]

    def export_prometheus(self) -> str:
        """Export all metrics in Prometheus text format."""
        lines = []
        for metric in self._metrics.values():
            lines.append(metric.to_prometheus())
        return "\n\n".join(lines) + "\n"

    def export_json(self) -> str:
        data = {}
        for key, metric in self._metrics.items():
            data[key] = {
                "name": metric.name,
                "type": metric.metric_type.value,
                "value": metric.value,
                "labels": metric.labels,
                "updated_at": metric.updated_at,
            }
        return json.dumps(data, indent=2)

    def _key(self, name: str, labels: Optional[dict]) -> str:
        if labels:
            label_str = ",".join(f"{k}={v}" for k, v in sorted(labels.items()))
            return f"{name}:{label_str}"
        return name


def setup_structured_logging(
    level: str = "INFO",
    log_dir: Optional[Path] = None,
    json_format: bool = True,
) -> logging.Logger:
    """Configure structured JSON logging.

    Returns the root logger configured for the operator system.
    """
    root = logging.getLogger("x3_operator")
    root.setLevel(getattr(logging, level.upper(), logging.INFO))

    # Remove existing handlers
    root.handlers.clear()

    if json_format:
        formatter = _JsonFormatter()
    else:
        formatter = logging.Formatter(
            "%(asctime)s %(levelname)s %(name)s %(message)s",
            datefmt="%Y-%m-%dT%H:%M:%S",
        )

    # Console handler
    console = logging.StreamHandler()
    console.setFormatter(formatter)
    root.addHandler(console)

    # File handler
    if log_dir:
        log_dir.mkdir(parents=True, exist_ok=True)
        log_file = log_dir / "x3_operator.log"
        file_handler = logging.FileHandler(log_file)
        file_handler.setFormatter(formatter)
        root.addHandler(file_handler)

    return root


class _JsonFormatter(logging.Formatter):
    def format(self, record: logging.LogRecord) -> str:
        entry = {
            "ts": time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(record.created)),
            "level": record.levelname,
            "logger": record.name,
            "msg": record.getMessage(),
        }
        if record.exc_info and record.exc_info[1]:
            entry["error"] = str(record.exc_info[1])
            entry["error_type"] = type(record.exc_info[1]).__name__
        return json.dumps(entry)


# Pre-built operator metrics
def create_operator_metrics() -> MetricsRegistry:
    """Create standard operator metrics."""
    registry = MetricsRegistry()

    registry.counter("x3_operator_heartbeats_total", "Total heartbeats sent")
    registry.counter("x3_operator_tasks_completed_total", "Total tasks completed")
    registry.counter("x3_operator_tasks_failed_total", "Total tasks failed")
    registry.counter("x3_operator_slashes_total", "Total slashes received")
    registry.counter("x3_operator_proofs_submitted_total", "Storage proofs submitted")

    registry.gauge("x3_operator_bond_amount", "Current bond amount in planck")
    registry.gauge("x3_operator_effective_stake", "Effective stake after slashes")
    registry.gauge("x3_operator_agents_running", "Number of running agents")
    registry.gauge("x3_operator_gpu_utilization", "GPU utilization percentage")
    registry.gauge("x3_operator_disk_available_gb", "Available disk space in GB")
    registry.gauge("x3_operator_uptime_seconds", "Operator uptime in seconds")

    return registry
