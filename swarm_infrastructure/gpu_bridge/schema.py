"""Pydantic schemas mirroring the Rust gpu-swarm crate types.

These models are wire-compatible with the Rust types via serde_json.
The Rust side derives Serialize/Deserialize on all public structs,
so Python ↔ Rust communication uses JSON as the interchange format.

Mirrors:
- crates/gpu-swarm/src/task.rs → GpuTask, GpuTaskType, etc.
- crates/gpu-swarm/src/protocol.rs → GpuExecutionProof
"""

from __future__ import annotations

import uuid
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Dict, List, Optional

from pydantic import BaseModel, Field


class GpuTaskType(str, Enum):
    """Mirrors Rust TaskType."""
    X3_BYTECODE = "X3Bytecode"
    MEMPOOL_SIMULATION = "MempoolSimulation"
    ROUTE_OPTIMIZATION = "RouteOptimization"
    ML_TRAINING = "MLTraining"
    PROOF_GENERATION = "ProofGeneration"
    ARBITRAGE_SEARCH = "ArbitrageSearch"
    CUSTOM = "Custom"

    # AGI substrate extensions
    CAUSAL_ANALYSIS = "CausalAnalysis"
    AGENT_EVALUATION = "AgentEvaluation"
    PREDICTION_BATCH = "PredictionBatch"
    COUNTERFACTUAL = "Counterfactual"


class GpuTaskPriority(str, Enum):
    """Mirrors Rust TaskPriority."""
    LOW = "Low"
    NORMAL = "Normal"
    HIGH = "High"
    CRITICAL = "Critical"


class GpuTaskStatus(str, Enum):
    """Mirrors Rust TaskStatus."""
    PENDING = "Pending"
    ASSIGNED = "Assigned"
    EXECUTING = "Executing"
    VERIFYING = "Verifying"
    COMPLETED = "Completed"
    FAILED = "Failed"
    CANCELLED = "Cancelled"
    TIMED_OUT = "TimedOut"


class GpuTaskMetadata(BaseModel):
    """Task metadata for scheduling hints."""
    description: str = ""
    tags: List[str] = Field(default_factory=list)
    required_capabilities: List[str] = Field(default_factory=list)
    preferred_regions: List[str] = Field(default_factory=list)
    min_reputation: int = 0


class GpuTask(BaseModel):
    """A GPU compute task — Python mirror of Rust Task.

    To submit: create a GpuTask, serialize to JSON, send to
    the Rust coordinator via the GpuTaskClient.
    """
    task_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    task_type: GpuTaskType = GpuTaskType.CUSTOM
    priority: GpuTaskPriority = GpuTaskPriority.NORMAL
    agent_id: str = ""             # Submitting agent
    epoch: int = 0                 # Epoch when submitted
    payload: Dict[str, Any] = Field(default_factory=dict)
    reward: int = 0                # reward in X3 tokens
    timeout_secs: int = 300
    verification_count: int = 2
    metadata: GpuTaskMetadata = Field(default_factory=GpuTaskMetadata)
    created_at: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )

    model_config = {"use_enum_values": True}


class GpuExecutionProof(BaseModel):
    """Proof of GPU execution — mirrors Rust ExecutionProof."""
    device_fingerprint: str = ""
    input_hash: str = ""
    output_hash: str = ""
    checkpoints: List[Dict[str, Any]] = Field(default_factory=list)
    compute_units_used: int = 0
    nonce: int = 0
    timestamp: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )


class GpuTaskResult(BaseModel):
    """Result of a completed GPU task."""
    task_id: str
    status: GpuTaskStatus = GpuTaskStatus.COMPLETED
    agent_id: str = ""
    executor_node: str = ""        # NodeId of the executing node
    result_data: Dict[str, Any] = Field(default_factory=dict)
    result_hash: str = ""
    error: Optional[str] = None
    compute_units_used: int = 0
    execution_proof: Optional[GpuExecutionProof] = None
    started_at: Optional[datetime] = None
    completed_at: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc)
    )

    model_config = {"use_enum_values": True}

    @property
    def succeeded(self) -> bool:
        return self.status == GpuTaskStatus.COMPLETED.value

    @property
    def failed(self) -> bool:
        return self.status in (
            GpuTaskStatus.FAILED.value,
            GpuTaskStatus.TIMED_OUT.value,
        )
