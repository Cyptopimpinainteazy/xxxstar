"""CUDA runtime detection should work on compiler and runtime-only hosts."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
from unittest.mock import patch


CUDA_LOADER_PATH = (
    Path(__file__).resolve().parents[2]
    / "infra-structure"
    / "validator"
    / "src"
    / "cross_chain_gpu_validator"
    / "gpu"
    / "cuda_loader.py"
)

spec = importlib.util.spec_from_file_location("cuda_loader_under_test", CUDA_LOADER_PATH)
assert spec is not None
cuda_loader = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = cuda_loader
spec.loader.exec_module(cuda_loader)
CudaRuntime = cuda_loader.CudaRuntime


def test_detect_accepts_runtime_only_host(monkeypatch) -> None:
    monkeypatch.delenv("X3_BYPASS_CUDA", raising=False)
    monkeypatch.delenv("CCGV_BYPASS_CUDA", raising=False)
    monkeypatch.setenv("CUDA_VISIBLE_DEVICES", "0,1")

    with patch.object(cuda_loader.shutil, "which", return_value=None), patch.object(
        cuda_loader.ctypes, "CDLL", return_value=object()
    ):
        runtime = CudaRuntime.detect()

    assert runtime.available is True
    assert runtime.nvcc_path is None
    assert runtime.visible_devices == "0,1"


def test_detect_reports_unavailable_without_compiler_or_runtime(monkeypatch) -> None:
    monkeypatch.delenv("X3_BYPASS_CUDA", raising=False)
    monkeypatch.delenv("CCGV_BYPASS_CUDA", raising=False)
    monkeypatch.delenv("CUDA_VISIBLE_DEVICES", raising=False)

    with patch.object(cuda_loader.shutil, "which", return_value=None), patch.object(
        cuda_loader.ctypes, "CDLL", side_effect=OSError
    ):
        runtime = CudaRuntime.detect()

    assert runtime.available is False
    assert runtime.nvcc_path is None
    assert runtime.visible_devices == ""


def test_detect_bypasses_cuda_without_probe(monkeypatch) -> None:
    monkeypatch.setenv("X3_BYPASS_CUDA", "1")
    monkeypatch.setenv("CUDA_VISIBLE_DEVICES", "0")

    with patch.object(cuda_loader.shutil, "which") as which, patch.object(
        cuda_loader.ctypes, "CDLL"
    ) as cdll:
        runtime = CudaRuntime.detect()

    assert runtime.available is False
    assert runtime.nvcc_path is None
    assert runtime.visible_devices == "0"
    which.assert_not_called()
    cdll.assert_not_called()
