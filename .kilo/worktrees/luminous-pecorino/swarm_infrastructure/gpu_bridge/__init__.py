"""GPU Bridge — connects the Python AGI swarm to the Rust GPU compute network.

Provides:
- ``GpuTaskSchema``   — Python mirrors of Rust task types (serde-compatible)
- ``GpuTaskClient``   — submits GPU tasks and polls for results
- ``GpuResultHandler`` — processes completed tasks back into the causal graph
- ``MockGpuProvider``  — in-process mock for testing without Rust FFI

The bridge uses JSON serialization on the Python side, matching the
Rust serde_json derivations in crates/gpu-swarm.
"""

from swarm.gpu_bridge.schema import (
    GpuTask,
    GpuTaskType,
    GpuTaskPriority,
    GpuTaskStatus,
    GpuTaskResult,
    GpuExecutionProof,
)
from swarm.gpu_bridge.client import GpuTaskClient, GpuProvider
from swarm.gpu_bridge.provider import MockGpuProvider

__all__ = [
    "GpuTask",
    "GpuTaskType",
    "GpuTaskPriority",
    "GpuTaskStatus",
    "GpuTaskResult",
    "GpuExecutionProof",
    "GpuTaskClient",
    "GpuProvider",
    "MockGpuProvider",
]
