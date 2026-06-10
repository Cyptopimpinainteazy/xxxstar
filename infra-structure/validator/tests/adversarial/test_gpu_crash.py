#!/usr/bin/env python3
"""Adversarial Test: GPU Crash — simulate GPU failure and verify fallback.

Scenario
--------
Simulate a GPU crash by:
1. Killing the GPU driver process (or simulating GPU unavailability)
2. Verifying the health daemon detects GPU loss
3. Verifying lane failover to shadow/tertiary
4. Verifying degraded mode activates
5. Verifying recovery when GPU comes back

Usage
-----
    python tests/adversarial/test_gpu_crash.py \
        --rpc-endpoint http://127.0.0.1:9933 \
        --health-endpoint http://127.0.0.1:9934/health
"""

import argparse
import json
import logging
import subprocess
import sys
import time
import urllib.request
import urllib.error

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
)
logger = logging.getLogger("adversarial.gpu_crash")

# Timeouts (seconds)
GPU_DETECT_TIMEOUT = 10
FAILOVER_TIMEOUT = 20
RECOVERY_TIMEOUT = 30


def check_health(health_endpoint: str) -> dict | None:
    """Check validator health endpoint."""
    try:
        req = urllib.request.Request(health_endpoint)
        with urllib.request.urlopen(req, timeout=5) as resp:
            return json.loads(resp.read())
    except Exception as exc:
        logger.debug("Health check failed: %s", exc)
        return None


def simulate_gpu_crash():
    """Simulate GPU crash by killing NVIDIA driver processes."""
    logger.info("Simulating GPU crash...")

    # Try to kill NVIDIA driver persistence
    try:
        subprocess.run(
            ["sudo", "pkill", "-9", "nvidia-persistenced"],
            capture_output=True, timeout=5,
        )
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass

    # Try to reset GPU
    try:
        subprocess.run(
            ["sudo", "nvidia-smi", "--gpu-reset", "-i", "0"],
            capture_output=True, timeout=10,
        )
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass

    # Verify GPU is gone
    try:
        result = subprocess.run(
            ["nvidia-smi"], capture_output=True, timeout=5,
        )
        gpu_available = result.returncode == 0
    except (FileNotFoundError, subprocess.TimeoutExpired):
        gpu_available = False

    if gpu_available:
        logger.warning("GPU still appears available — crash simulation may not work")
    else:
        logger.info("GPU appears unavailable — crash simulation active")

    return not gpu_available


def restore_gpu():
    """Attempt to restore GPU availability."""
    logger.info("Restoring GPU...")
    try:
        subprocess.run(
            ["sudo", "nvidia-persistenced", "--persistence-mode"],
            capture_output=True, timeout=10,
        )
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass

    try:
        result = subprocess.run(
            ["nvidia-smi"], capture_output=True, timeout=5,
        )
        return result.returncode == 0
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return False


def main():
    parser = argparse.ArgumentParser(
        description="Adversarial GPU crash test"
    )
    parser.add_argument(
        "--rpc-endpoint", default="http://127.0.0.1:9933",
        help="Validator RPC endpoint",
    )
    parser.add_argument(
        "--health-endpoint", default="http://127.0.0.1:9934/health",
        help="Validator health endpoint",
    )
    parser.add_argument(
        "--skip-gpu-kill", action="store_true",
        help="Skip actual GPU kill (for testing without real GPU)",
    )
    args = parser.parse_args()

    logger.info("Starting GPU crash test")

    # Phase 1: Verify GPU is available
    logger.info("Phase 1: Verifying GPU availability")
    health = check_health(args.health_endpoint)
    if health:
        gpu_available = health.get("gpu", {}).get("available", False)
        logger.info("GPU available: %s", gpu_available)
    else:
        logger.warning("Cannot check health — continuing")
        gpu_available = True

    # Phase 2: Crash the GPU
    logger.info("Phase 2: Crashing GPU")
    if not args.skip_gpu_kill:
        gpu_crashed = simulate_gpu_crash()
    else:
        logger.info("Skipping GPU kill (--skip-gpu-kill)")
        gpu_crashed = True

    if not gpu_crashed:
        logger.warning("GPU crash simulation may not have worked")

    # Phase 3: Verify health detects GPU loss
    logger.info("Phase 3: Verifying GPU loss detection")
    gpu_detected_down = False
    deadline = time.time() + GPU_DETECT_TIMEOUT
    while time.time() < deadline:
        health = check_health(args.health_endpoint)
        if health:
            gpu = health.get("gpu", {})
            if not gpu.get("available", True):
                gpu_detected_down = True
                logger.info("GPU loss detected by health daemon")
                break
        time.sleep(1)

    if not gpu_detected_down:
        logger.warning("GPU loss not detected within %ds", GPU_DETECT_TIMEOUT)

    # Phase 4: Verify lane failover
    logger.info("Phase 4: Verifying lane failover")
    failover_occurred = False
    deadline = time.time() + FAILOVER_TIMEOUT
    while time.time() < deadline:
        health = check_health(args.health_endpoint)
        if health:
            lane = health.get("lane", {})
            if lane.get("tier") in ("shadow", "tertiary", "degraded"):
                failover_occurred = True
                logger.info("Lane failover detected: %s", lane.get("tier"))
                break
        time.sleep(1)

    if not failover_occurred:
        logger.warning("Lane failover not detected within %ds", FAILOVER_TIMEOUT)

    # Phase 5: Restore GPU
    logger.info("Phase 5: Restoring GPU")
    if not args.skip_gpu_kill:
        gpu_restored = restore_gpu()
    else:
        gpu_restored = True

    # Phase 6: Verify recovery
    logger.info("Phase 6: Verifying GPU recovery")
    recovered = False
    deadline = time.time() + RECOVERY_TIMEOUT
    while time.time() < deadline:
        health = check_health(args.health_endpoint)
        if health:
            gpu = health.get("gpu", {})
            lane = health.get("lane", {})
            if gpu.get("available", False) and lane.get("tier") == "primary":
                recovered = True
                logger.info("GPU recovered — back to primary lane")
                break
        time.sleep(1)

    if not recovered:
        logger.warning("GPU recovery not detected within %ds", RECOVERY_TIMEOUT)

    # Results
    passed = gpu_detected_down and failover_occurred

    result = {
        "test": "gpu_crash",
        "passed": passed,
        "gpu_detected_down": gpu_detected_down,
        "failover_occurred": failover_occurred,
        "recovered": recovered,
        "gpu_restored": gpu_restored,
    }

    logger.info("=" * 60)
    logger.info("GPU CRASH TEST %s", "PASSED" if passed else "FAILED")
    logger.info("  GPU loss detected:  %s", gpu_detected_down)
    logger.info("  Failover occurred:  %s", failover_occurred)
    logger.info("  GPU restored:       %s", gpu_restored)
    logger.info("  Recovered:          %s", recovered)
    logger.info("=" * 60)

    result_path = f"/tmp/x3-adversarial-gpu-crash-{int(time.time())}.json"
    with open(result_path, "w") as f:
        json.dump(result, f, indent=2)
    logger.info("Result written to %s", result_path)

    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
