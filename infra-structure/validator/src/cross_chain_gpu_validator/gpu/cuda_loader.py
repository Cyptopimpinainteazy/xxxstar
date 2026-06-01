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
        """Detect CUDA runtime availability.

        The original implementation only checked for the presence of the ``nvcc``
        compiler binary, which is unsuitable for runtime‑only hosts where the
        compiler is not installed but the CUDA driver/runtime libraries are
        available.  This method now performs a more robust detection:

        1. Look for ``nvcc`` (compiler) – useful for development containers.
        2. Attempt to load the CUDA runtime library (``libcudart.so``) to verify
           that a functional runtime is present.
        3. Record the ``CUDA_VISIBLE_DEVICES`` environment variable.

        The ``available`` flag is true when either a compiler is found *or* the
        runtime library can be loaded.  ``nvcc_path`` is retained for backward
        compatibility and may be ``None`` on runtime‑only hosts.
        """
        nvcc_path = shutil.which("nvcc")
        visible_devices = os.getenv("CUDA_VISIBLE_DEVICES", "")
        # Try loading the CUDA runtime library; ignore errors – absence means no runtime.
        runtime_available = False
        try:
            ctypes.CDLL("libcudart.so")
            runtime_available = True
        except OSError:
            runtime_available = False
        available = (nvcc_path is not None) or runtime_available
        return cls(available=available, nvcc_path=nvcc_path, visible_devices=visible_devices)

    def require(self) -> None:
        if not self.available:
            raise RuntimeError(
                "CUDA runtime not available. Ensure nvcc is installed and "
                "CUDA_VISIBLE_DEVICES is set."
            )
