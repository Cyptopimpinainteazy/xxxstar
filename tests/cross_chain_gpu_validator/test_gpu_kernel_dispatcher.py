"""Legacy GPU dispatcher must fail closed for unsupported GPU execution."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys

import pytest


GPU_DIR = (
    Path(__file__).resolve().parents[2]
    / "infra-structure"
    / "validator"
    / "src"
    / "cross_chain_gpu_validator"
    / "gpu"
)


def load_gpu_module(name: str):
    package_name = "gpu_dispatcher_under_test"
    if package_name not in sys.modules:
        package_spec = importlib.util.spec_from_file_location(
            package_name,
            GPU_DIR / "__init__.py",
            submodule_search_locations=[str(GPU_DIR)],
        )
        assert package_spec is not None
        package = importlib.util.module_from_spec(package_spec)
        sys.modules[package_name] = package
        assert package_spec.loader is not None
        package_spec.loader.exec_module(package)

    spec = importlib.util.spec_from_file_location(
        f"{package_name}.{name}",
        GPU_DIR / f"{name}.py",
    )
    assert spec is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


kernels = load_gpu_module("kernels")


def test_sha256_batch_fails_closed_instead_of_cpu_simulation() -> None:
    dispatcher = kernels.GPUKernels(allow_failover=True)

    with pytest.raises(kernels.GPUKernelError, match="x3-vm GPU hostcalls"):
        dispatcher.sha256_batch([b"x" * 32])
