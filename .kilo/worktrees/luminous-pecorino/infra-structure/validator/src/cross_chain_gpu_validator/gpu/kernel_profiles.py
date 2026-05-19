"""Per-chain kernel profiles for EVM and SVM chains.

Defines which GPU kernels each chain family requires, batch sizes,
and gas cost parameters for the X3 validator pipeline.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional


class ChainFamily(Enum):
    EVM = "evm"        # Ethereum-compatible (secp256k1 + Keccak-256)
    SVM = "svm"        # Solana-compatible (Ed25519 + SHA-256)
    COSMOS = "cosmos"  # Cosmos-compatible (secp256k1 + SHA-256)
    SUBSTRATE = "substrate"  # Polkadot/Kusama (Ed25519/Sr25519 + SHA-256)
    OTHER = "other"


class KernelType(Enum):
    SHA256 = "sha256"
    KECCAK256 = "keccak256"
    ED25519 = "ed25519"
    SECP256K1 = "secp256k1"
    POH = "poh"


@dataclass
class KernelConfig:
    """Configuration for a single GPU kernel invocation."""
    kernel_type: KernelType
    batch_size: int = 4096
    streams: int = 4
    gas_cost: int = 500
    vram_per_batch_mb: int = 32
    multi_gpu: bool = True
    deterministic: bool = True


@dataclass
class ChainProfile:
    """GPU kernel requirements for a chain family."""
    family: ChainFamily
    sig_kernel: KernelConfig
    hash_kernel: KernelConfig
    poh_kernel: Optional[KernelConfig] = None
    description: str = ""

    @property
    def required_kernels(self) -> List[KernelType]:
        kernels = [self.sig_kernel.kernel_type, self.hash_kernel.kernel_type]
        if self.poh_kernel:
            kernels.append(self.poh_kernel.kernel_type)
        return kernels

    @property
    def total_gas_cost(self) -> int:
        cost = self.sig_kernel.gas_cost + self.hash_kernel.gas_cost
        if self.poh_kernel:
            cost += self.poh_kernel.gas_cost
        return cost

    @property
    def vram_estimate_mb(self) -> int:
        vram = self.sig_kernel.vram_per_batch_mb + self.hash_kernel.vram_per_batch_mb
        if self.poh_kernel:
            vram += self.poh_kernel.vram_per_batch_mb
        return vram


# ============================================================================
# Pre-defined chain profiles
# ============================================================================

EVM_PROFILE = ChainProfile(
    family=ChainFamily.EVM,
    sig_kernel=KernelConfig(
        kernel_type=KernelType.SECP256K1,
        batch_size=8192,
        streams=4,
        gas_cost=600,
        vram_per_batch_mb=64,
    ),
    hash_kernel=KernelConfig(
        kernel_type=KernelType.KECCAK256,
        batch_size=16384,
        streams=4,
        gas_cost=500,
        vram_per_batch_mb=32,
    ),
    description="EVM chains: Ethereum, Arbitrum, Optimism, Polygon, Base, etc.",
)

SVM_PROFILE = ChainProfile(
    family=ChainFamily.SVM,
    sig_kernel=KernelConfig(
        kernel_type=KernelType.ED25519,
        batch_size=16384,
        streams=4,
        gas_cost=500,
        vram_per_batch_mb=48,
    ),
    hash_kernel=KernelConfig(
        kernel_type=KernelType.SHA256,
        batch_size=32768,
        streams=4,
        gas_cost=500,
        vram_per_batch_mb=32,
    ),
    poh_kernel=KernelConfig(
        kernel_type=KernelType.POH,
        batch_size=64,
        streams=2,
        gas_cost=500,
        vram_per_batch_mb=16,
    ),
    description="Solana chains: mainnet, devnet, testnet.",
)

COSMOS_PROFILE = ChainProfile(
    family=ChainFamily.COSMOS,
    sig_kernel=KernelConfig(
        kernel_type=KernelType.SECP256K1,
        batch_size=8192,
        streams=4,
        gas_cost=600,
        vram_per_batch_mb=64,
    ),
    hash_kernel=KernelConfig(
        kernel_type=KernelType.SHA256,
        batch_size=32768,
        streams=4,
        gas_cost=500,
        vram_per_batch_mb=32,
    ),
    description="Cosmos chains: Hub, Osmosis, Juno, Sei, THORChain, etc.",
)

SUBSTRATE_PROFILE = ChainProfile(
    family=ChainFamily.SUBSTRATE,
    sig_kernel=KernelConfig(
        kernel_type=KernelType.ED25519,
        batch_size=16384,
        streams=4,
        gas_cost=500,
        vram_per_batch_mb=48,
    ),
    hash_kernel=KernelConfig(
        kernel_type=KernelType.SHA256,
        batch_size=32768,
        streams=4,
        gas_cost=500,
        vram_per_batch_mb=32,
    ),
    description="Substrate chains: Polkadot, Kusama.",
)


# ============================================================================
# Profile registry
# ============================================================================

CHAIN_PROFILES: Dict[ChainFamily, ChainProfile] = {
    ChainFamily.EVM: EVM_PROFILE,
    ChainFamily.SVM: SVM_PROFILE,
    ChainFamily.COSMOS: COSMOS_PROFILE,
    ChainFamily.SUBSTRATE: SUBSTRATE_PROFILE,
}

# Map chain_id → ChainFamily  (canonical EVM chain IDs)
CHAIN_FAMILY_MAP: Dict[str, ChainFamily] = {}

# EVM chains (by chain_id)
_EVM_IDS = [
    "1", "10", "42161", "137", "8453", "43114", "56", "250", "59144",
    "534352", "324", "1101", "100", "25", "1284", "1285", "42220",
    "1666600000", "128", "66", "288", "1088", "1313161554", "122",
    "1116",
    # Testnets
    "5", "11155111", "421613", "420", "80001", "84531", "43113",
    "97", "4002",
]
for _id in _EVM_IDS:
    CHAIN_FAMILY_MAP[_id] = ChainFamily.EVM

# Solana chains
for _id in ["solana-mainnet", "solana-devnet", "solana-testnet",
            "1399811149"]:
    CHAIN_FAMILY_MAP[_id] = ChainFamily.SVM

# Cosmos chains
for _id in ["cosmoshub-4", "osmosis-1", "juno-1", "pacific-1",
            "thorchain-mainnet-v1", "injective-1", "stride-1",
            "evmos_9001-2"]:
    CHAIN_FAMILY_MAP[_id] = ChainFamily.COSMOS

# Substrate chains
for _id in ["polkadot", "kusama"]:
    CHAIN_FAMILY_MAP[_id] = ChainFamily.SUBSTRATE


def get_profile(chain_id: str) -> ChainProfile:
    """Get the kernel profile for a chain. Defaults to EVM."""
    family = CHAIN_FAMILY_MAP.get(chain_id, ChainFamily.EVM)
    return CHAIN_PROFILES[family]


def get_family(chain_id: str) -> ChainFamily:
    """Get the chain family for a chain ID."""
    return CHAIN_FAMILY_MAP.get(chain_id, ChainFamily.EVM)


def required_libraries(chain_id: str) -> List[str]:
    """Return the .so library filenames required for this chain."""
    profile = get_profile(chain_id)
    lib_map = {
        KernelType.SHA256: "libsha256_batch.so",
        KernelType.KECCAK256: "libkeccak256_batch.so",
        KernelType.ED25519: "libed25519_batch.so",
        KernelType.SECP256K1: "libsecp256k1_batch.so",
        KernelType.POH: "libsha256_batch.so",  # PoH uses SHA-256 kernel
    }
    return list({lib_map[k] for k in profile.required_kernels})
