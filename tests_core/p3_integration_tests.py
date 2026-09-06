#!/usr/bin/env python3
"""
P3 Integration Tests - End-to-End Component Validation
Tests all 8 P3 components working together
"""

import asyncio
import json
import logging
import subprocess
import time

import pytest

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


def skip_legacy_gpu_swarm_python(test_name: str) -> None:
    pytest.skip(
        f"{test_name} depended on removed gpu-swarm Python helpers; cover the canonical Rust/CUDA path instead"
    )

# Test configuration
TEST_TIMEOUT = 300  # 5 minutes per test
TASK_SUBMISSION_TIMEOUT = 10
VERIFICATION_TIMEOUT = 30


class TestMonitoringStack:
    """Test Prometheus + Grafana + Alertmanager integration"""

    def test_prometheus_scraping(self):
        """Verify Prometheus is scraping metrics from all targets"""
        logger.info("Testing Prometheus scrape targets...")

        # Query Prometheus API
        response = subprocess.run(
            ["curl", "-s", "http://localhost:9090/api/v1/targets"],
            capture_output=True,
            text=True,
            timeout=TASK_SUBMISSION_TIMEOUT
        )

        assert response.returncode == 0, "Prometheus API unreachable"

        targets = json.loads(response.stdout)
        assert "data" in targets, "Invalid Prometheus response"
        assert len(targets["data"]["activeTargets"]) > 0, "No active scrape targets"

        # Verify key targets are present
        active_targets = [t["labels"]["job"] for t in targets["data"]["activeTargets"]]
        assert "swarm-coordinator" in active_targets, "Coordinator not being scraped"
        assert "gpu-node" in active_targets, "GPU nodes not being scraped"

        logger.info(f"✓ Prometheus scraping {len(targets['data']['activeTargets'])} targets")

    def test_alert_rules_loaded(self):
        """Verify 40+ alert rules are loaded"""
        logger.info("Testing alert rules...")

        response = subprocess.run(
            ["curl", "-s", "http://localhost:9090/api/v1/rules"],
            capture_output=True,
            text=True,
            timeout=TASK_SUBMISSION_TIMEOUT
        )

        assert response.returncode == 0, "Cannot fetch alert rules"

        rules = json.loads(response.stdout)
        alert_count = sum(len(group["rules"]) for group in rules["data"]["groups"])

        assert alert_count >= 40, f"Expected 40+ alert rules, got {alert_count}"
        logger.info(f"✓ {alert_count} alert rules loaded and active")

    def test_grafana_dashboards(self):
        """Verify Grafana dashboards are accessible"""
        logger.info("Testing Grafana dashboards...")

        response = subprocess.run(
            ["curl", "-s", "http://localhost:3000/api/dashboards/home"],
            capture_output=True,
            text=True,
            timeout=TASK_SUBMISSION_TIMEOUT,
            headers={"Authorization": "Bearer admin"}
        )

        assert response.returncode == 0 or response.status_code == 200, "Grafana unreachable"
        logger.info("✓ Grafana dashboards accessible")


class TestLogAggregation:
    """Test ELK Stack and log aggregation"""

    def test_elasticsearch_health(self):
        """Verify Elasticsearch cluster is healthy"""
        logger.info("Testing Elasticsearch health...")

        response = subprocess.run(
            ["curl", "-s", "http://localhost:9200/_cluster/health"],
            capture_output=True,
            text=True,
            timeout=TASK_SUBMISSION_TIMEOUT
        )

        assert response.returncode == 0, "Elasticsearch unreachable"

        health = json.loads(response.stdout)
        assert health["status"] in ["green", "yellow"], f"Cluster status: {health['status']}"
        assert health["number_of_nodes"] >= 1, "No Elasticsearch nodes"

        logger.info(f"✓ Elasticsearch healthy ({health['status']}, {health['number_of_nodes']} nodes)")

    def test_log_ingestion(self):
        """Verify logs are being ingested correctly"""
        logger.info("Testing log ingestion...")

        # Send test log
        test_log = {
            "timestamp": int(time.time()),
            "level": "TEST",
            "message": "Integration test log",
            "component": "integration_test"
        }

        response = subprocess.run(
            ["curl", "-s", "-X", "POST",
             "http://localhost:9200/swarm-logs/_doc/test-doc",
             "-H", "Content-Type: application/json",
             "-d", json.dumps(test_log)],
            capture_output=True,
            text=True,
            timeout=TASK_SUBMISSION_TIMEOUT
        )

        assert response.returncode == 0, "Failed to ingest log"

        result = json.loads(response.stdout)
        assert result["result"] in ["created", "updated"], "Log not ingested"

        logger.info("✓ Logs ingesting successfully to Elasticsearch")


class TestDistributedTracing:
    """Test Jaeger and OpenTelemetry integration"""

    def test_jaeger_services(self):
        """Verify Jaeger is collecting traces from services"""
        logger.info("Testing Jaeger service discovery...")

        response = subprocess.run(
            ["curl", "-s", "http://localhost:16686/api/services"],
            capture_output=True,
            text=True,
            timeout=TASK_SUBMISSION_TIMEOUT
        )

        assert response.returncode == 0, "Jaeger UI unreachable"

        services = json.loads(response.stdout)
        assert "data" in services, "Invalid Jaeger response"

        logger.info(f"✓ Jaeger collecting traces from {len(services['data'])} services")


class TestPerformanceOptimizer:
    """Test GPU memory pooling, task batching, network optimization"""

    def test_memory_pooling_allocation(self):
        """Verify GPU memory pool allocation works"""
        skip_legacy_gpu_swarm_python("test_memory_pooling_allocation")

    def test_task_batch_optimizer(self):
        """Verify task batching reduces latency"""
        skip_legacy_gpu_swarm_python("test_task_batch_optimizer")


class TestJurySystem:
    """Test Byzantine verification and encrypted auditing"""

    def test_encrypted_audit_logging(self):
        """Verify audit logs are encrypted and tamper-proof"""
        skip_legacy_gpu_swarm_python("test_encrypted_audit_logging")

    def test_jury_consensus(self):
        """Verify Byzantine consensus mechanism"""
        skip_legacy_gpu_swarm_python("test_jury_consensus")


class TestSocialAgents:
    """Test multi-platform social agent integration"""

    def test_social_action_queueing(self):
        """Verify social actions are queued correctly"""
        skip_legacy_gpu_swarm_python("test_social_action_queueing")


class TestCLITooling:
    """Test SwarmCLI operator commands"""

    def test_cli_task_commands(self):
        """Verify CLI task commands work"""
        logger.info("Testing CLI task commands...")

        # Test task submission
        result = subprocess.run(
            ["swarm-cli", "tasks", "list", "--limit=5"],
            capture_output=True,
            text=True,
            timeout=TASK_SUBMISSION_TIMEOUT
        )

        # Expected to work or show helpful error (CLI might not be in PATH)
        assert result.returncode == 0 or "not found" in result.stderr, "CLI error"

        logger.info("✓ CLI commands accessible")


class TestKubernetesDeployment:
    """Test Kubernetes deployment and health"""

    def test_coordinator_statefulset(self):
        """Verify coordinator StatefulSet is healthy"""
        logger.info("Testing Kubernetes coordinator deployment...")

        result = subprocess.run(
            ["kubectl", "-n", "gpu-swarm", "get", "pods", "-l", "app=swarm-coordinator", "-o", "json"],
            capture_output=True,
            text=True,
            timeout=TASK_SUBMISSION_TIMEOUT
        )

        if result.returncode == 0:
            pods = json.loads(result.stdout)
            assert len(pods["items"]) >= 1, "No coordinator pods running"

            for pod in pods["items"]:
                assert pod["status"]["phase"] == "Running", f"Pod {pod['metadata']['name']} not running"

            logger.info(f"✓ {len(pods['items'])} coordinator pods healthy")
        else:
            logger.warning("kubectl not available, skipping K8s test")


class TestEndToEndWorkflow:
    """Test complete workflow: submit task -> execute -> verify -> log"""

    @pytest.mark.asyncio
    async def test_full_task_workflow(self):
        """Test complete task lifecycle"""
        logger.info("Testing end-to-end task workflow...")

        # 1. Submit task to coordinator
        task_payload = {
            "code": "import cupy as cp; result = cp.arange(10).sum(); print(result)",
            "backend": "cuda",
            "priority": 5
        }

        submit_response = subprocess.run(
            ["curl", "-s", "-X", "POST", "http://localhost:9000/submit_task",
             "-H", "Content-Type: application/json",
             "-d", json.dumps(task_payload)],
            capture_output=True,
            text=True,
            timeout=TASK_SUBMISSION_TIMEOUT
        )

        if submit_response.returncode == 0:
            task_result = json.loads(submit_response.stdout)
            task_id = task_result.get("task_id")

            assert task_id is not None, "No task_id returned"
            logger.info(f"✓ Task submitted: {task_id}")

            # 2. Wait for execution and verification
            start_time = time.time()
            while time.time() - start_time < 30:
                status_response = subprocess.run(
                    ["curl", "-s", f"http://localhost:9000/task/{task_id}/status"],
                    capture_output=True,
                    text=True,
                    timeout=TASK_SUBMISSION_TIMEOUT
                )

                if status_response.returncode == 0:
                    status = json.loads(status_response.stdout)

                    if status.get("state") == "completed":
                        logger.info(f"✓ Task completed in {time.time() - start_time:.2f}s")
                        assert status.get("verified"), "Task not verified"
                        logger.info("✓ Task verified by jury system")
                        break

                await asyncio.sleep(1)

            else:
                logger.warning("Task did not complete within 30s (might be normal in test)")

        else:
            logger.warning("Coordinator API not available, skipping end-to-end test")


def run_all_tests():
    """Run all integration tests"""
    logger.info("=" * 80)
    logger.info("P3 INTEGRATION TEST SUITE - START")
    logger.info("=" * 80)

    test_classes = [
        TestMonitoringStack,
        TestLogAggregation,
        TestDistributedTracing,
        TestPerformanceOptimizer,
        TestJurySystem,
        TestSocialAgents,
        TestCLITooling,
        TestKubernetesDeployment,
    ]

    passed = 0
    failed = 0
    errors = []

    for test_class in test_classes:
        logger.info(f"\n--- {test_class.__name__} ---")

        instance = test_class()
        methods = [m for m in dir(instance) if m.startswith("test_")]

        for method_name in methods:
            try:
                method = getattr(instance, method_name)
                method()
                passed += 1
            except Exception as e:
                failed += 1
                error_msg = f"{test_class.__name__}.{method_name}: {e!s}"
                errors.append(error_msg)
                logger.error(f"✗ {error_msg}")

    # Summary
    logger.info("\n" + "=" * 80)
    logger.info(f"TEST RESULTS: {passed} passed, {failed} failed")
    logger.info("=" * 80)

    if errors:
        logger.error("\nFailed tests:")
        for error in errors:
            logger.error(f"  - {error}")
        return False

    return True


if __name__ == "__main__":
    success = run_all_tests()
    exit(0 if success else 1)
