"""CUDA runtime detection and kernel loader stubs."""

from __future__ import annotations

from dataclasses import dataclass
import os
import shutil


@dataclass(frozen=True)
class CudaRuntime:
    """CUDA availability metadata."""

    available: bool
    nvcc_path: str | None
    visible_devices: str

    @classmethod
    def detect(cls) -> "CudaRuntime":
        nvcc_path = shutil.which("nvcc")
        visible_devices = os.getenv("CUDA_VISIBLE_DEVICES", "")
        available = nvcc_path is not None
        return cls(available=available, nvcc_path=nvcc_path, visible_devices=visible_devices)

    def require(self) -> None:
        if not self.available:
            raise RuntimeError(
                "CUDA runtime not available. Ensure nvcc is installed and "
                "CUDA_VISIBLE_DEVICES is set."
            )
