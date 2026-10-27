"""GPU accelerator interfaces."""

from __future__ import annotations

from .cuda_loader import CudaRuntime
from .secp256k1_gpu import Secp256k1BatchVerifier
from .ed25519_gpu import Ed25519BatchVerifier
from .keccak_gpu import KeccakBatchHasher
from .multi_gpu_scheduler import MultiGpuScheduler, GpuDevice, ChainWorkload, GpuStatus
from .kernel_profiles import (
    ChainProfile, ChainFamily, KernelType, KernelConfig,
    get_profile, get_family, required_libraries,
    EVM_PROFILE, SVM_PROFILE, COSMOS_PROFILE, SUBSTRATE_PROFILE,
)
from .stream_batcher import StreamBatcher, StreamBatcherConfig, BatchResult

__all__ = [
    "CudaRuntime",
    "Secp256k1BatchVerifier",
    "Ed25519BatchVerifier",
    "KeccakBatchHasher",
    "MultiGpuScheduler",
    "GpuDevice",
    "ChainWorkload",
    "GpuStatus",
    "ChainProfile",
    "ChainFamily",
    "KernelType",
    "KernelConfig",
    "get_profile",
    "get_family",
    "required_libraries",
    "EVM_PROFILE",
    "SVM_PROFILE",
    "COSMOS_PROFILE",
    "SUBSTRATE_PROFILE",
    "StreamBatcher",
    "StreamBatcherConfig",
    "BatchResult",
]
