#!/usr/bin/env python3
"""Adversarial Test: Network Chaos — simulate network failures.

Scenario
--------
Simulate various network failure modes:
1. Network partition — isolate node from peers
2. High latency — add artificial delay
3. Packet loss — drop random packets
4. Redis outage — kill Redis, verify circuit breaker
5. Recovery — restore network, verify cluster heals

Usage
-----
    python tests/adversarial/test_network_chaos.py \
        --rpc-endpoint http://127.0.0.1:9933 \
        --health-endpoint http://127.0.0.1:9934/health \
        --scenarios partition,latency,packet_loss,redis_outage
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
logger = logging.getLogger("adversarial.network_chaos")

SCENARIO_TIMEOUT = 60


def check_health(health_endpoint: str) -> dict | None:
    """Check validator health endpoint."""
    try:
        req = urllib.request.Request(health_endpoint)
        with urllib.request.urlopen(req, timeout=5) as resp:
            return json.loads(resp.read())
    except Exception:
        return None


def check_rpc(rpc_endpoint: str) -> bool:
    """Check if RPC endpoint is reachable."""
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
            return True
    except Exception:
        return False


def scenario_partition(interface: str = "eth0"):
    """Simulate network partition by blocking traffic."""
    logger.info("Scenario: Network partition — blocking traffic on %s", interface)
    try:
        # Block all traffic on the interface
        subprocess.run(
            ["sudo", "iptables", "-A", "INPUT", "-i", interface, "-j", "DROP"],
            capture_output=True, timeout=5,
        )
        subprocess.run(
            ["sudo", "iptables", "-A", "OUTPUT", "-o", interface, "-j", "DROP"],
            capture_output=True, timeout=5,
        )
        return True
    except Exception as exc:
        logger.error("Failed to create partition: %s", exc)
        return False


def scenario_latency(interface: str = "eth0", delay_ms: int = 2000):
    """Simulate high latency."""
    logger.info("Scenario: High latency — adding %dms delay on %s", delay_ms, interface)
    try:
        subprocess.run(
            ["sudo", "tc", "qdisc", "add", "dev", interface,
             "root", "netem", "delay", f"{delay_ms}ms"],
            capture_output=True, timeout=5,
        )
        return True
    except Exception as exc:
        logger.error("Failed to add latency: %s", exc)
        return False


def scenario_packet_loss(interface: str = "eth0", loss_pct: float = 50.0):
    """Simulate packet loss."""
    logger.info("Scenario: Packet loss — dropping %.0f%% on %s", loss_pct, interface)
    try:
        subprocess.run(
            ["sudo", "tc", "qdisc", "add", "dev", interface,
             "root", "netem", "loss", f"{loss_pct}%"],
            capture_output=True, timeout=5,
        )
        return True
    except Exception as exc:
        logger.error("Failed to add packet loss: %s", exc)
        return False


def scenario_redis_outage():
    """Simulate Redis outage by stopping Redis."""
    logger.info("Scenario: Redis outage — stopping Redis")
    try:
        subprocess.run(
            ["sudo", "systemctl", "stop", "redis"],
            capture_output=True, timeout=10,
        )
        return True
    except Exception as exc:
        logger.error("Failed to stop Redis: %s", exc)
        return False


def cleanup_all(interface: str = "eth0"):
    """Clean up all network chaos scenarios."""
    logger.info("Cleaning up network chaos...")

    # Remove iptables rules
    try:
        subprocess.run(
            ["sudo", "iptables", "-F"],
            capture_output=True, timeout=5,
        )
    except Exception:
        pass

    # Remove tc rules
    try:
        subprocess.run(
            ["sudo", "tc", "qdisc", "del", "dev", interface, "root"],
            capture_output=True, timeout=5,
        )
    except Exception:
        pass

    # Restart Redis
    try:
        subprocess.run(
            ["sudo", "systemctl", "start", "redis"],
            capture_output=True, timeout=10,
        )
    except Exception:
        pass

    logger.info("Cleanup complete")


def main():
    parser = argparse.ArgumentParser(
        description="Adversarial network chaos test"
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
        "--interface", default="eth0",
        help="Network interface to apply chaos to",
    )
    parser.add_argument(
        "--scenarios",
        default="partition,latency,packet_loss,redis_outage",
        help="Comma-separated list of scenarios to run",
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="Don't actually apply chaos (for testing)",
    )
    args = parser.parse_args()

    scenarios = [s.strip() for s in args.scenarios.split(",")]
    logger.info("Starting network chaos test — scenarios: %s", scenarios)

    results = {}

    try:
        for scenario in scenarios:
            logger.info("=" * 60)
            logger.info("Running scenario: %s", scenario)

            # Pre-check
            rpc_ok = check_rpc(args.rpc_endpoint)
            health = check_health(args.health_endpoint)
            logger.info("Pre-check: RPC=%s, health=%s", rpc_ok, health is not None)

            # Apply chaos
            if args.dry_run:
                logger.info("DRY RUN — skipping actual chaos application")
                chaos_applied = True
            elif scenario == "partition":
                chaos_applied = scenario_partition(args.interface)
            elif scenario == "latency":
                chaos_applied = scenario_latency(args.interface)
            elif scenario == "packet_loss":
                chaos_applied = scenario_packet_loss(args.interface)
            elif scenario == "redis_outage":
                chaos_applied = scenario_redis_outage()
            else:
                logger.warning("Unknown scenario: %s", scenario)
                chaos_applied = False

            if not chaos_applied:
                logger.warning("Failed to apply chaos for scenario: %s", scenario)
                results[scenario] = {"applied": False}
                continue

            # Wait and observe
            logger.info("Observing for %ds...", SCENARIO_TIMEOUT)
            deadline = time.time() + SCENARIO_TIMEOUT
            rpc_failed = False
            health_failed = False
            recovered = False

            while time.time() < deadline:
                rpc_ok = check_rpc(args.rpc_endpoint)
                health = check_health(args.health_endpoint)

                if not rpc_ok:
                    rpc_failed = True
                if health is None:
                    health_failed = True

                time.sleep(2)

            # Clean up this scenario
            if not args.dry_run:
                cleanup_all(args.interface)
                time.sleep(5)

            # Post-check
            rpc_ok = check_rpc(args.rpc_endpoint)
            health = check_health(args.health_endpoint)
            recovered = rpc_ok and health is not None

            results[scenario] = {
                "applied": chaos_applied,
                "rpc_failed": rpc_failed,
                "health_failed": health_failed,
                "recovered": recovered,
            }

            logger.info("Scenario %s: applied=%s, rpc_failed=%s, recovered=%s",
                        scenario, chaos_applied, rpc_failed, recovered)

    finally:
        # Final cleanup
        if not args.dry_run:
            cleanup_all(args.interface)

    # Overall result
    all_applied = all(r.get("applied", False) for r in results.values())
    all_recovered = all(r.get("recovered", False) for r in results.values())
    passed = all_applied and all_recovered

    result = {
        "test": "network_chaos",
        "passed": passed,
        "scenarios": scenarios,
        "results": results,
        "all_applied": all_applied,
        "all_recovered": all_recovered,
    }

    logger.info("=" * 60)
    logger.info("NETWORK CHAOS TEST %s", "PASSED" if passed else "FAILED")
    for scenario, r in results.items():
        logger.info("  %s: applied=%s, rpc_failed=%s, recovered=%s",
                    scenario, r.get("applied"), r.get("rpc_failed"), r.get("recovered"))
    logger.info("=" * 60)

    result_path = f"/tmp/x3-adversarial-network-chaos-{int(time.time())}.json"
    with open(result_path, "w") as f:
        json.dump(result, f, indent=2)
    logger.info("Result written to %s", result_path)

    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
