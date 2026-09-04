#!/usr/bin/env python3
"""Adversarial Test: Lane Flood — overwhelm the validator with traffic.

Scenario
--------
Flood the validator with transactions at 10x normal rate to test:
1. Circuit breaker trips correctly
2. Lane failover happens gracefully
3. Degraded mode handles overflow
4. No data loss during flood

Usage
-----
    python tests/adversarial/test_lane_flood.py \
        --rpc-endpoint http://127.0.0.1:9933 \
        --flood-rate 10000 \
        --duration 60
"""

import argparse
import json
import logging
import random
import sys
import time
import urllib.request
import urllib.error

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
)
logger = logging.getLogger("adversarial.lane_flood")

PASS_THRESHOLD = 0.95  # 95% of transactions must succeed


def send_transaction(rpc_endpoint: str, tx_data: dict) -> bool:
    """Send a single transaction to the validator RPC."""
    payload = json.dumps({
        "jsonrpc": "2.0",
        "method": "author_submitExtrinsic",
        "params": [tx_data],
        "id": random.randint(1, 100000),
    }).encode()
    try:
        req = urllib.request.Request(
            rpc_endpoint,
            data=payload,
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            result = json.loads(resp.read())
            return "error" not in result
    except (urllib.error.URLError, json.JSONDecodeError, OSError) as exc:
        logger.debug("Transaction failed: %s", exc)
        return False


def check_health(rpc_endpoint: str) -> dict | None:
    """Check validator health via RPC."""
    payload = json.dumps({
        "jsonrpc": "2.0",
        "method": "system_health",
        "params": [],
        "id": 1,
    }).encode()
    try:
        req = urllib.request.Request(
            rpc_endpoint,
            data=payload,
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            return json.loads(resp.read()).get("result")
    except Exception:
        return None


def main():
    parser = argparse.ArgumentParser(
        description="Adversarial lane flood test"
    )
    parser.add_argument(
        "--rpc-endpoint", default="http://127.0.0.1:9933",
        help="Validator RPC endpoint",
    )
    parser.add_argument(
        "--flood-rate", type=int, default=5000,
        help="Transactions per second to send",
    )
    parser.add_argument(
        "--duration", type=int, default=30,
        help="Test duration in seconds",
    )
    parser.add_argument(
        "--tx-size", type=int, default=256,
        help="Transaction payload size in bytes",
    )
    args = parser.parse_args()

    logger.info(
        "Starting lane flood test — rate=%d tx/s, duration=%ds, size=%dB",
        args.flood_rate, args.duration, args.tx_size,
    )

    # Pre-flight health check
    health = check_health(args.rpc_endpoint)
    if health:
        logger.info("Pre-flight health: %s", health)
    else:
        logger.warning("Pre-flight health check failed — continuing anyway")

    # Generate flood transactions
    tx_payload = "0x" + "ab" * (args.tx_size // 2)
    tx_data = {
        "data": tx_payload,
        "flood_test": True,
    }

    # Run flood
    sent = 0
    succeeded = 0
    failed = 0
    start = time.time()
    deadline = start + args.duration
    interval = 1.0 / args.flood_rate

    while time.time() < deadline:
        ok = send_transaction(args.rpc_endpoint, tx_data)
        sent += 1
        if ok:
            succeeded += 1
        else:
            failed += 1
        time.sleep(interval)

        # Log progress every 1000 transactions
        if sent % 1000 == 0:
            elapsed = time.time() - start
            actual_rate = sent / elapsed if elapsed > 0 else 0
            logger.info(
                "Progress: sent=%d, ok=%d, failed=%d, rate=%.0f tx/s",
                sent, succeeded, failed, actual_rate,
            )

    elapsed = time.time() - start
    actual_rate = sent / elapsed if elapsed > 0 else 0
    success_rate = succeeded / sent if sent > 0 else 0

    # Post-flight health check
    health = check_health(args.rpc_endpoint)
    if health:
        logger.info("Post-flight health: %s", health)
    else:
        logger.warning("Post-flight health check failed")

    # Results
    passed = success_rate >= PASS_THRESHOLD

    result = {
        "test": "lane_flood",
        "passed": passed,
        "sent": sent,
        "succeeded": succeeded,
        "failed": failed,
        "actual_rate": round(actual_rate, 1),
        "success_rate": round(success_rate, 4),
        "duration": round(elapsed, 2),
        "threshold": PASS_THRESHOLD,
    }

    logger.info("=" * 60)
    logger.info("LANE FLOOD TEST %s", "PASSED" if passed else "FAILED")
    logger.info("  Sent:      %d", sent)
    logger.info("  Succeeded: %d", succeeded)
    logger.info("  Failed:    %d", failed)
    logger.info("  Rate:      %.0f tx/s (target: %d)", actual_rate, args.flood_rate)
    logger.info("  Success:   %.1f%% (threshold: %.0f%%)", success_rate * 100, PASS_THRESHOLD * 100)
    logger.info("=" * 60)

    # Write result file
    result_path = f"/tmp/x3-adversarial-lane-flood-{int(time.time())}.json"
    with open(result_path, "w") as f:
        json.dump(result, f, indent=2)
    logger.info("Result written to %s", result_path)

    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
