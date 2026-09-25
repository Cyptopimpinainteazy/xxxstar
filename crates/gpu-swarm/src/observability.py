"""
GPU Swarm Advanced Observability Module
Comprehensive instrumentation with tracing, metrics, and structured logging
"""

import os
import json
import time
import logging
from datetime import datetime
from typing import Dict, Any, Optional, Callable
from dataclasses import dataclass, asdict
from functools import wraps
from contextlib import contextmanager

# OpenTelemetry imports
from opentelemetry import trace, metrics
from opentelemetry.exporter.jaeger.thrift import JaegerExporter
from opentelemetry.exporter.prometheus import PrometheusMetricReader
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.resources import Resource
from opentelemetry.instrumentation.flask import FlaskInstrumentor
from opentelemetry.instrumentation.requests import RequestsInstrumentor
from opentelemetry.instrumentation.sqlalchemy import SQLAlchemyInstrumentor
from opentelemetry.instrumentation.logging import LoggingInstrumentor
from opentelemetry.instrumentation.psycopg2 import Psycopg2Instrumentor


# Configure Jaeger exporter for distributed tracing
jaeger_exporter = JaegerExporter(
    agent_host_name=os.getenv("JAEGER_HOST", "localhost"),
    agent_port=int(os.getenv("JAEGER_PORT", 6831)),
)

trace_provider = TracerProvider(
    resource=Resource.create({
        "service.name": "gpu-swarm",
        "service.version": os.getenv("SERVICE_VERSION", "1.0.0"),
        "deployment.environment": os.getenv("ENVIRONMENT", "production"),
    })
)
trace_provider.add_span_processor(BatchSpanProcessor(jaeger_exporter))
trace.set_tracer_provider(trace_provider)

# Configure Prometheus metrics
prometheus_reader = PrometheusMetricReader()
meter_provider = MeterProvider(metric_readers=[prometheus_reader])
metrics.set_meter_provider(meter_provider)

# Get tracer and meter instances
tracer = trace.get_tracer(__name__)
meter = metrics.get_meter(__name__)

# Configure structured logging
logging.basicConfig(
    level=os.getenv("LOG_LEVEL", "INFO"),
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)


@dataclass
class SpanContext:
    """Context for distributed tracing"""
    trace_id: str
    span_id: str
    parent_span_id: Optional[str] = None
    baggage: Dict[str, str] = None

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


class SwarmObservabilityManager:
    """Main observability manager for GPU Swarm"""

    def __init__(self):
        self.logger = logging.getLogger("gpu-swarm")
        self._setup_metrics()
        self._setup_instrumentation()

    def _setup_metrics(self):
        """Initialize all Prometheus metrics"""
        # Task execution metrics
        self.task_execution_time = meter.create_histogram(
            name="swarm_task_execution_seconds",
            description="Task execution time in seconds",
            unit="s"
        )
        
        self.task_counter = meter.create_counter(
            name="swarm_tasks_total",
            description="Total number of tasks"
        )

        self.task_failures = meter.create_counter(
            name="swarm_task_failures_total",
            description="Total number of failed tasks"
        )

        self.task_queue_depth = meter.create_observable_gauge(
            name="swarm_task_queue_depth",
            description="Current task queue depth",
            callbacks=[self._get_queue_depth]
        )

        # GPU metrics
        self.gpu_utilization = meter.create_histogram(
            name="gpu_utilization_percent",
            description="GPU utilization percentage",
            unit="%"
        )

        self.gpu_memory_used = meter.create_observable_gauge(
            name="gpu_memory_used_bytes",
            description="GPU memory used in bytes",
            unit="By",
            callbacks=[self._get_gpu_memory]
        )

        self.gpu_temperature = meter.create_observable_gauge(
            name="gpu_temperature_celsius",
            description="GPU temperature in celsius",
            unit="°C",
            callbacks=[self._get_gpu_temp]
        )

        # Network metrics
        self.peer_count = meter.create_observable_gauge(
            name="swarm_network_peers_connected",
            description="Number of connected peers"
        )

        self.network_latency = meter.create_histogram(
            name="swarm_peer_latency_milliseconds",
            description="Peer connection latency",
            unit="ms"
        )

        self.network_bytes_sent = meter.create_counter(
            name="swarm_network_bytes_sent_total",
            description="Total bytes sent over network"
        )

        # Reward metrics
        self.rewards_distributed = meter.create_counter(
            name="swarm_rewards_distributed_tokens",
            description="Total tokens distributed as rewards"
        )

        self.slashing_events = meter.create_counter(
            name="swarm_slashing_events_total",
            description="Total slashing events"
        )

        # X3 metrics
        self.x3_compilation_time = meter.create_histogram(
            name="swarm_x3_compilation_time_seconds",
            description="X3 bytecode compilation time",
            unit="s"
        )

        self.x3_gas_used = meter.create_histogram(
            name="swarm_gas_used_total",
            description="Total gas used in X3 execution",
            unit="1"
        )

    def _setup_instrumentation(self):
        """Setup automatic instrumentation for common libraries"""
        FlaskInstrumentor().instrument()
        RequestsInstrumentor().instrument()
        LoggingInstrumentor().instrument()

    def _get_queue_depth(self) -> int:
        """Callback to get current queue depth"""
        # This would be implemented by the actual queue manager
        return 0

    def _get_gpu_memory(self) -> float:
        """Callback to get GPU memory usage"""
        # This would be implemented by GPU monitor
        return 0.0

    def _get_gpu_temp(self) -> float:
        """Callback to get GPU temperature"""
        # This would be implemented by GPU monitor
        return 0.0

    @contextmanager
    def trace_operation(self, operation_name: str, attributes: Dict[str, Any] = None):
        """Context manager for tracing operations"""
        with tracer.start_as_current_span(operation_name) as span:
            if attributes:
                for key, value in attributes.items():
                    span.set_attribute(key, value)
            
            start_time = time.time()
            try:
                yield span
                duration = time.time() - start_time
                span.set_attribute("duration_ms", duration * 1000)
                span.set_attribute("status", "success")
            except Exception as e:
                span.set_attribute("status", "error")
                span.set_attribute("error_type", type(e).__name__)
                span.set_attribute("error_message", str(e))
                raise

    def trace_function(self, func: Callable) -> Callable:
        """Decorator for automatic function tracing"""
        @wraps(func)
        def wrapper(*args, **kwargs):
            with self.trace_operation(
                operation_name=f"{func.__module__}.{func.__name__}",
                attributes={
                    "function": func.__name__,
                    "args_count": len(args),
                    "kwargs_keys": list(kwargs.keys())
                }
            ):
                return func(*args, **kwargs)
        return wrapper

    def record_metric(self, metric_name: str, value: float, attributes: Dict[str, str] = None):
        """Record a custom metric"""
        try:
            if attributes is None:
                attributes = {}
            # Route to appropriate metric based on name
            if metric_name.startswith("task_"):
                self.task_counter.add(value, attributes)
            elif metric_name.startswith("gpu_"):
                self.gpu_utilization.record(value, attributes)
            elif metric_name.startswith("network_"):
                self.network_latency.record(value, attributes)
        except Exception as e:
            self.logger.error(f"Failed to record metric {metric_name}: {e}")

    def log_structured(
        self,
        level: str,
        message: str,
        component: str,
        **context
    ):
        """Log structured event with context"""
        log_entry = {
            "timestamp": datetime.utcnow().isoformat(),
            "level": level,
            "component": component,
            "message": message,
            "context": context,
            "trace_id": trace.get_current_span().get_span_context().trace_id if trace.get_current_span() else None,
        }

        log_json = json.dumps(log_entry)
        
        if level.upper() == "ERROR":
            self.logger.error(log_json)
            self.task_failures.add(1, {"component": component})
        elif level.upper() == "WARN":
            self.logger.warning(log_json)
        else:
            self.logger.info(log_json)

    def record_task_execution(
        self,
        task_id: str,
        duration_seconds: float,
        success: bool,
        gpu_backend: str = None,
        gas_used: int = None,
        error: str = None
    ):
        """Record task execution metrics"""
        attributes = {
            "task_id": task_id,
            "gpu_backend": gpu_backend or "unknown",
            "status": "success" if success else "failure"
        }

        self.task_execution_time.record(duration_seconds, attributes)
        
        if success:
            self.task_counter.add(1, attributes)
            if gas_used:
                self.x3_gas_used.record(gas_used, attributes)
        else:
            self.task_failures.add(1, attributes)
            self.log_structured(
                "ERROR",
                f"Task {task_id} failed",
                "task_executor",
                task_id=task_id,
                duration=duration_seconds,
                error=error
            )

    def record_gpu_metrics(
        self,
        device_id: str,
        utilization: float,
        memory_used: int,
        memory_total: int,
        temperature: float,
        power_watts: float
    ):
        """Record GPU hardware metrics"""
        attributes = {"device_id": device_id}
        
        self.gpu_utilization.record(utilization, attributes)
        self.gpu_memory_used.record(memory_used, attributes)
        self.gpu_temperature.record(temperature, attributes)

    def record_network_event(
        self,
        peer_id: str,
        event_type: str,
        latency_ms: float = None,
        bytes_sent: int = None,
        bytes_received: int = None
    ):
        """Record network metrics"""
        if latency_ms:
            self.network_latency.record(latency_ms, {"peer_id": peer_id})
        
        if bytes_sent:
            self.network_bytes_sent.add(bytes_sent, {"peer_id": peer_id})

    def record_reward_distribution(
        self,
        node_id: str,
        tokens_distributed: float,
        tasks_completed: int
    ):
        """Record reward distribution"""
        attributes = {
            "node_id": node_id,
            "tasks": str(tasks_completed)
        }
        self.rewards_distributed.add(tokens_distributed, attributes)

    def record_slashing_event(
        self,
        node_id: str,
        reason: str,
        tokens_slashed: float
    ):
        """Record slashing event"""
        attributes = {
            "node_id": node_id,
            "reason": reason
        }
        self.slashing_events.add(1, attributes)
        
        self.log_structured(
            "WARN",
            f"Node {node_id} slashed",
            "incentives",
            node_id=node_id,
            reason=reason,
            tokens=tokens_slashed
        )

    def record_x3_compilation(
        self,
        code_size: int,
        duration_seconds: float,
        gas_optimized: int,
        success: bool,
        error: str = None
    ):
        """Record X3 compilation metrics"""
        self.x3_compilation_time.record(duration_seconds)
        
        if success:
            self.log_structured(
                "INFO",
                f"X3 compilation succeeded",
                "x3_executor",
                code_size=code_size,
                duration=duration_seconds,
                gas_optimized=gas_optimized
            )
        else:
            self.log_structured(
                "ERROR",
                f"X3 compilation failed",
                "x3_executor",
                code_size=code_size,
                error=error
            )


# Singleton instance
_observability_manager = None

def get_observability_manager() -> SwarmObservabilityManager:
    """Get or create the observability manager"""
    global _observability_manager
    if _observability_manager is None:
        _observability_manager = SwarmObservabilityManager()
    return _observability_manager


# Export public API
__all__ = [
    'get_observability_manager',
    'SwarmObservabilityManager',
    'SpanContext',
    'tracer',
    'meter'
]
