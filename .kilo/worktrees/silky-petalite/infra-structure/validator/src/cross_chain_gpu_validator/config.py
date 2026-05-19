"""Configuration for the cross-chain GPU validator."""

from __future__ import annotations

from dataclasses import dataclass
import os


@dataclass(frozen=True)
class Settings:
    """Runtime settings loaded from environment variables."""

    svm_rpc_url: str
    evm_rpc_url: str
    redis_url: str
    require_gpu: bool
    atomic_timeout_seconds: int
    batch_size: int
    gpu_parity_check: bool
    metrics_host: str
    metrics_port: int
    dashboard_host: str
    dashboard_port: int
    kernel_dir: str
    log_level: str
    max_workers: int
    redis_pool_size: int
    poll_interval_seconds: float


def _get_bool(name: str, default: bool) -> bool:
    value = os.getenv(name)
    if value is None:
        return default
    return value.strip().lower() in {"1", "true", "yes", "on"}


def _get_int(name: str, default: int) -> int:
    value = os.getenv(name)
    if value is None:
        return default
    return int(value.strip())


def _get_float(name: str, default: float) -> float:
    value = os.getenv(name)
    if value is None:
        return default
    return float(value.strip())


def load_settings() -> Settings:
    """Load settings from environment variables with defaults."""

    return Settings(
        svm_rpc_url=os.getenv("CCGV_SVM_RPC", "http://127.0.0.1:8899"),
        evm_rpc_url=os.getenv("CCGV_EVM_RPC", "http://127.0.0.1:8545"),
        redis_url=os.getenv("CCGV_REDIS_URL", "redis://127.0.0.1:6379/0"),
        require_gpu=_get_bool("CCGV_REQUIRE_GPU", True),
        atomic_timeout_seconds=_get_int("CCGV_ATOMIC_TIMEOUT", 30),
        batch_size=_get_int("CCGV_BATCH_SIZE", 4096),
        gpu_parity_check=_get_bool("CCGV_GPU_PARITY_CHECK", True),
        metrics_host=os.getenv("CCGV_METRICS_HOST", "0.0.0.0"),
        metrics_port=_get_int("CCGV_METRICS_PORT", 9101),
        dashboard_host=os.getenv("CCGV_DASHBOARD_HOST", "0.0.0.0"),
        dashboard_port=_get_int("CCGV_DASHBOARD_PORT", 8080),
        kernel_dir=os.getenv(
            "CCGV_KERNEL_DIR",
            os.path.abspath(
                os.path.join(os.path.dirname(__file__), "..", "..", "..", "kernels")
            ),
        ),
        log_level=os.getenv("CCGV_LOG_LEVEL", "INFO"),
        max_workers=_get_int("CCGV_MAX_WORKERS", 8),
        redis_pool_size=_get_int("CCGV_REDIS_POOL_SIZE", 16),
        poll_interval_seconds=_get_float("CCGV_POLL_INTERVAL", 0.1),
    )
