"""
X3 Operator CLI & Runtime
~~~~~~~~~~~~~~~~~~~~~~~~~

Production-grade operator onboarding, bonding, slashing, and monitoring
for the X3 blockchain network.

Modules:
    cli         - Command-line interface
    config      - Centralized configuration
    identity    - Operator identity management
    bonding     - Stake bonding and escrow
    slashing    - Deterministic slashing engine
    health      - Hardware health checks
    supervisor  - Agent supervisor with kill-switch
    storage     - Storage provider protocol
    governance  - Governance capture simulation
    genesis     - Genesis ceremony tools
    telemetry   - Structured logging + metrics
"""

__version__ = "0.1.0"

from .config import X3Config, OperatorRole, NetworkPhase
from .identity import OperatorIdentity, generate_operator_identity, load_operator_identity
from .health import HealthReport, HealthStatus, run_health_check
from .bonding import BondLedger, BondRecord, BondStatus
from .slashing import SlashingEngine, SlashEvidence, SlashVerdict, FaultType
from .supervisor import AgentSupervisor, PolicyManifest, AgentState
from .storage import StorageRegistry, StorageDeal, ContentID, StorageProof, DealStatus
from .governance import GovernanceSimulator, SimulationResult, AttackType
from .genesis import GenesisCeremony, GenesisConfig, GenesisParticipant, CeremonyStep
from .telemetry import MetricsRegistry, setup_structured_logging, create_operator_metrics
from .command_center import CommandCenterState, CommandCenterHandler, run_server

__all__ = [
    "X3Config", "OperatorRole", "NetworkPhase",
    "OperatorIdentity", "generate_operator_identity", "load_operator_identity",
    "HealthReport", "HealthStatus", "run_health_check",
    "BondLedger", "BondRecord", "BondStatus",
    "SlashingEngine", "SlashEvidence", "SlashVerdict", "FaultType",
    "AgentSupervisor", "PolicyManifest", "AgentState",
    "StorageRegistry", "StorageDeal", "ContentID", "StorageProof", "DealStatus",
    "GovernanceSimulator", "SimulationResult", "AttackType",
    "GenesisCeremony", "GenesisConfig", "GenesisParticipant", "CeremonyStep",
    "MetricsRegistry", "setup_structured_logging", "create_operator_metrics",
    "CommandCenterState", "CommandCenterHandler", "run_server",
]
