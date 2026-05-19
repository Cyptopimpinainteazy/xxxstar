"""
X3 Operator Configuration
~~~~~~~~~~~~~~~~~~~~~~~~~

Centralized, validated configuration for all operator roles.
Environment-safe defaults. Validation on startup.
"""

import os
import json
import logging
from dataclasses import dataclass, field, asdict
from enum import Enum
from pathlib import Path
from typing import Optional, Dict, Any

logger = logging.getLogger(__name__)


class OperatorRole(str, Enum):
    VALIDATOR = "validator"
    GPU = "gpu"
    STORAGE = "storage"
    RELAYER = "relayer"


class NetworkPhase(str, Enum):
    DEVNET = "devnet"
    TESTNET = "testnet"
    MAINNET = "mainnet"


@dataclass
class ChainConfig:
    """Chain connection parameters."""
    rpc_url: str = "ws://127.0.0.1:9944"
    chain_id: str = "x3-chain-devnet"
    network_phase: NetworkPhase = NetworkPhase.DEVNET


@dataclass
class BondingConfig:
    """Bonding requirements per role."""
    min_bond_validator: int = 10_000  # X3 tokens
    min_bond_gpu: int = 1_000
    min_bond_storage: int = 2_000
    min_bond_relayer: int = 5_000
    unbonding_delay_blocks: int = 14_400  # ~24 hours at 6s blocks
    unbonding_delay_seconds: int = 86400  # 24 hours
    cooldown_epochs: int = 3


@dataclass
class SlashingConfig:
    """Slashing parameters."""
    severity_table: dict = field(default_factory=lambda: {
        "downtime": 0.01,
        "equivocation": 0.50,
        "invalid_proof": 0.10,
        "missed_heartbeat": 0.005,
        "sla_violation": 0.05,
        "data_corruption": 1.00,
        "governance_abuse": 0.25,
        "agent_violation": 0.10,
    })
    repetition_base: float = 1.5  # exponential multiplier per repeat
    max_slash_fraction: float = 1.0  # can lose entire bond


@dataclass
class HealthConfig:
    """Hardware health thresholds."""
    min_disk_gb: int = 100
    min_ram_gb: int = 8
    min_cpu_cores: int = 4
    gpu_required_for: list = field(default_factory=lambda: ["gpu"])
    min_vram_mb: int = 4_096
    max_temperature_c: float = 85.0
    heartbeat_interval_seconds: int = 30
    heartbeat_timeout_seconds: int = 120


@dataclass
class AgentConfig:
    """Agent supervisor configuration."""
    max_agents_per_operator: int = 10
    max_calls_per_minute: int = 60
    max_agent_memory_mb: int = 512
    kill_switch_delay_seconds: int = 5
    policy_hash_required: bool = True


@dataclass
class StorageConfig:
    """Storage provider configuration."""
    proof_interval_seconds: int = 60  # proof submission interval
    min_availability_bp: int = 9_950  # 99.50% in basis points
    max_latency_ms: int = 500
    min_replication_factor: int = 3


@dataclass
class TelemetryConfig:
    """Observability configuration."""
    log_level: str = "INFO"
    log_format: str = "json"
    metrics_port: int = 9615
    metrics_enabled: bool = True
    debug_mode: bool = False


@dataclass
class X3Config:
    """Root configuration for X3 operator."""
    chain: ChainConfig = field(default_factory=ChainConfig)
    bonding: BondingConfig = field(default_factory=BondingConfig)
    slashing: SlashingConfig = field(default_factory=SlashingConfig)
    health: HealthConfig = field(default_factory=HealthConfig)
    agent: AgentConfig = field(default_factory=AgentConfig)
    storage: StorageConfig = field(default_factory=StorageConfig)
    telemetry: TelemetryConfig = field(default_factory=TelemetryConfig)
    data_dir: str = "~/.x3-operator"
    operator_role: Optional[OperatorRole] = None

    def __post_init__(self):
        self.data_dir = str(Path(self.data_dir).expanduser())

    def min_bond_for_role(self, role: OperatorRole) -> int:
        """Return minimum bond requirement for a given role."""
        mapping = {
            OperatorRole.VALIDATOR: self.bonding.min_bond_validator,
            OperatorRole.GPU: self.bonding.min_bond_gpu,
            OperatorRole.STORAGE: self.bonding.min_bond_storage,
            OperatorRole.RELAYER: self.bonding.min_bond_relayer,
        }
        return mapping[role]

    def validate(self) -> list[str]:
        """Validate configuration. Returns list of errors (empty = valid)."""
        errors = []
        if self.slashing.max_slash_fraction > 1.0:
            errors.append("max_slash_fraction cannot exceed 1.0")
        if self.slashing.max_slash_fraction < 0.0:
            errors.append("max_slash_fraction cannot be negative")
        if self.health.heartbeat_timeout_seconds <= self.health.heartbeat_interval_seconds:
            errors.append("heartbeat_timeout must exceed heartbeat_interval")
        if self.storage.min_availability_bp > 10_000:
            errors.append("min_availability_bp cannot exceed 10000 (100%)")
        if self.bonding.unbonding_delay_blocks < 1:
            errors.append("unbonding_delay must be positive")
        return errors

    def to_dict(self) -> Dict[str, Any]:
        """Serialize to dictionary."""
        return asdict(self)

    def save(self, path: Optional[str] = None):
        """Persist config to disk."""
        target = Path(path or os.path.join(self.data_dir, "config.json"))
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(self.to_dict(), indent=2, default=str))
        logger.info("Config saved to %s", target)

    @classmethod
    def load(cls, path) -> "X3Config":
        """Load config from JSON file."""
        data = json.loads(Path(str(path)).read_text())

        chain_data = data.get("chain", {})
        # Restore enum from string
        if "network_phase" in chain_data and isinstance(chain_data["network_phase"], str):
            chain_data["network_phase"] = NetworkPhase(chain_data["network_phase"])
        chain = ChainConfig(**chain_data)

        bonding_data = data.get("bonding", {})
        bonding = BondingConfig(**bonding_data)

        slashing_data = data.get("slashing", {})
        slashing = SlashingConfig(**slashing_data)

        health_data = data.get("health", {})
        health = HealthConfig(**health_data)

        agent_data = data.get("agent", {})
        agent = AgentConfig(**agent_data)

        storage_data = data.get("storage", {})
        storage = StorageConfig(**storage_data)

        telemetry_data = data.get("telemetry", {})
        telemetry = TelemetryConfig(**telemetry_data)

        role_str = data.get("operator_role")
        role = OperatorRole(role_str) if role_str else None
        return cls(
            chain=chain, bonding=bonding, slashing=slashing,
            health=health, agent=agent, storage=storage,
            telemetry=telemetry,
            data_dir=data.get("data_dir", "~/.x3-operator"),
            operator_role=role,
        )

    @classmethod
    def from_env(cls) -> "X3Config":
        """Load config with environment overrides."""
        cfg = cls()
        cfg.chain.rpc_url = os.environ.get("X3_RPC_URL", cfg.chain.rpc_url)
        cfg.chain.chain_id = os.environ.get("X3_CHAIN_ID", cfg.chain.chain_id)
        phase = os.environ.get("X3_NETWORK_PHASE")
        if phase:
            cfg.chain.network_phase = NetworkPhase(phase)
        cfg.data_dir = os.environ.get("X3_DATA_DIR", cfg.data_dir)
        cfg.telemetry.log_level = os.environ.get("X3_LOG_LEVEL", cfg.telemetry.log_level)
        cfg.telemetry.debug_mode = os.environ.get("X3_DEBUG", "").lower() in ("1", "true")
        return cfg
