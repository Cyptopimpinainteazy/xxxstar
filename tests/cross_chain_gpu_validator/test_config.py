"""Cross-chain GPU validator defaults bypass CUDA."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys


CONFIG_PATH = (
    Path(__file__).resolve().parents[2]
    / "infra-structure"
    / "validator"
    / "src"
    / "cross_chain_gpu_validator"
    / "config.py"
)

spec = importlib.util.spec_from_file_location("ccgv_config_under_test", CONFIG_PATH)
assert spec is not None
config_module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = config_module
spec.loader.exec_module(config_module)


def test_defaults_bypass_cuda_and_do_not_require_gpu(monkeypatch) -> None:
    monkeypatch.delenv("CCGV_BYPASS_CUDA", raising=False)
    monkeypatch.delenv("CCGV_REQUIRE_GPU", raising=False)

    settings = config_module.load_settings()

    assert settings.bypass_cuda is True
    assert settings.require_gpu is False


def test_cuda_can_be_explicitly_reenabled(monkeypatch) -> None:
    monkeypatch.setenv("CCGV_BYPASS_CUDA", "0")
    monkeypatch.setenv("CCGV_REQUIRE_GPU", "1")

    settings = config_module.load_settings()

    assert settings.bypass_cuda is False
    assert settings.require_gpu is True
