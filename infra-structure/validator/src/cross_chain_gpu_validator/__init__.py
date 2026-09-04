"""Cross-chain GPU validator package."""

from __future__ import annotations

from .chain_adapter import (
    ChainValidator,
    ChainConfig,
    ChainTransaction,
    SignatureAlgorithm,
    HashAlgorithm,
)
from .chain_registry import ChainRegistry, load_default_chain_configs
from .orchestrator import MultiChainOrchestrator, CrossChainOrchestrator, MultiChainSwapPayload
from .evm import EvmValidator, EvmTransaction
from .svm import SvmValidator, SvmTransaction
from .cosmos import CosmosValidator
from .substrate import SubstrateValidator

__all__ = [
    "__version__",
    # Core interfaces
    "ChainValidator",
    "ChainConfig",
    "ChainTransaction",
    "SignatureAlgorithm",
    "HashAlgorithm",
    # Registry
    "ChainRegistry",
    "load_default_chain_configs",
    # Orchestrators
    "MultiChainOrchestrator",
    "CrossChainOrchestrator",
    "MultiChainSwapPayload",
    # Chain validators
    "EvmValidator",
    "EvmTransaction",
    "SvmValidator",
    "SvmTransaction",
    "CosmosValidator",
    "SubstrateValidator",
]

__version__ = "0.2.0"  # Updated for multi-chain support
