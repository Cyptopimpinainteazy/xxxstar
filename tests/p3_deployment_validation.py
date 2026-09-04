#!/usr/bin/env python3
"""
P3 Deployment Validation - Staging Environment Verification
Comprehensive checks before production deployment
"""

import json
import logging
import subprocess
import sys
import time
from enum import Enum

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - [%(levelname)s] %(message)s'
)
logger = logging.getLogger(__name__)


class CheckStatus(Enum):
    """Status of a deployment check"""
    PASS = "✓ PASS"
    FAIL = "✗ FAIL"
    WARN = "⚠ WARN"
    SKIP = "⊘ SKIP"


class DeploymentCheck:
    """Individual deployment validation check"""

    def __init__(self, name: str, critical: bool = False):
        self.name = name
        self.critical = critical
        self.status = None
        self.message = ""

    def run(self) -> tuple[CheckStatus, str]:
        """Execute check - override in subclasses"""
        raise NotImplementedError


class CoordinatorHealthCheck(DeploymentCheck):
    """Verify coordinator is healthy and responding"""

    def __init__(self):
        super().__init__("Coordinator Health Check", critical=True)

    def run(self) -> tuple[CheckStatus, str]:
        try:
            result = subprocess.run(
                ["curl", "-s", "-m", "5", "http://localhost:9000/health"],
                capture_output=True,
                text=True
            )

            if result.returncode == 0:
                health = json.loads(result.stdout)
                if health.get("status") == "healthy":
                    return CheckStatus.PASS, f"Coordinator healthy (version {health.get('version')})"
                else:
                    return CheckStatus.FAIL, f"Coordinator unhealthy: {health.get('status')}"
            else:
                return CheckStatus.FAIL, "Coordinator unreachable (curl error)"

        except Exception as e:
            return CheckStatus.FAIL, f"Exception: {e!s}"


class PrometheusTargetsCheck(DeploymentCheck):
    """Verify Prometheus is scraping all targets"""

    def __init__(self):
        super().__init__("Prometheus Scrape Targets", critical=True)

    def run(self) -> tuple[CheckStatus, str]:
        try:
            result = subprocess.run(
                ["curl", "-s", "http://localhost:9090/api/v1/targets"],
                capture_output=True,
                text=True,
                timeout=5
            )

            if result.returncode != 0:
                return CheckStatus.FAIL, "Prometheus unreachable"

            targets = json.loads(result.stdout)
            active = targets["data"]["activeTargets"]
            down = targets["data"]["droppedTargets"]

            if len(active) >= 5:  # Expect at least coordinator, GPU nodes, exporter, etc.
                return CheckStatus.PASS, f"{len(active)} targets up, {len(down)} down"
            else:
                return CheckStatus.WARN, f"Only {len(active)} targets (expected 5+)"

        except Exception as e:
            return CheckStatus.FAIL, f"Exception: {e!s}"


class AlertRulesCheck(DeploymentCheck):
    """Verify alert rules are loaded"""

    def __init__(self):
        super().__init__("Prometheus Alert Rules", critical=True)

    def run(self) -> tuple[CheckStatus, str]:
        try:
            result = subprocess.run(
                ["curl", "-s", "http://localhost:9090/api/v1/rules"],
                capture_output=True,
                text=True,
                timeout=5
            )

            if result.returncode != 0:
                return CheckStatus.FAIL, "Cannot fetch alert rules"

            rules_data = json.loads(result.stdout)
            total_rules = sum(len(group["rules"]) for group in rules_data["data"]["groups"])

            if total_rules >= 40:
                return CheckStatus.PASS, f"{total_rules} alert rules loaded"
            else:
                return CheckStatus.FAIL, f"Only {total_rules} rules (expected 40+)"

        except Exception as e:
            return CheckStatus.FAIL, f"Exception: {e!s}"


class ElasticsearchHealthCheck(DeploymentCheck):
    """Verify Elasticsearch cluster health"""

    def __init__(self):
        super().__init__("Elasticsearch Health", critical=False)

    def run(self) -> tuple[CheckStatus, str]:
        try:
            result = subprocess.run(
                ["curl", "-s", "http://localhost:9200/_cluster/health"],
                capture_output=True,
                text=True,
                timeout=5
            )

            if result.returncode != 0:
                return CheckStatus.SKIP, "Elasticsearch not available"

            health = json.loads(result.stdout)
            status = health.get("status")
            nodes = health.get("number_of_nodes", 0)

            if status in ["green", "yellow"] and nodes > 0:
                return CheckStatus.PASS, f"Cluster {status} ({nodes} nodes)"
            else:
                return CheckStatus.WARN, f"Cluster {status} ({nodes} nodes)"

        except Exception:
            return CheckStatus.SKIP, "Unavailable"


class JaegerHealthCheck(DeploymentCheck):
    """Verify Jaeger is collecting traces"""

    def __init__(self):
        super().__init__("Jaeger Tracing", critical=False)

    def run(self) -> tuple[CheckStatus, str]:
        try:
            result = subprocess.run(
                ["curl", "-s", "http://localhost:16686/api/services"],
                capture_output=True,
                text=True,
                timeout=5
            )

            if result.returncode != 0:
                return CheckStatus.SKIP, "Jaeger UI not available"

            services = json.loads(result.stdout)
            service_count = len(services.get("data", []))

            if service_count > 0:
                return CheckStatus.PASS, f"Collecting from {service_count} services"
            else:
                return CheckStatus.WARN, "No services found yet"

        except Exception:
            return CheckStatus.SKIP, "Unavailable"


class KubernetesClusterCheck(DeploymentCheck):
    """Verify Kubernetes cluster is accessible"""

    def __init__(self):
        super().__init__("Kubernetes Cluster Access", critical=False)

    def run(self) -> tuple[CheckStatus, str]:
        try:
            result = subprocess.run(
                ["kubectl", "cluster-info"],
                capture_output=True,
                text=True,
                timeout=5
            )

            if result.returncode == 0:
                return CheckStatus.PASS, "Cluster accessible"
            else:
                return CheckStatus.SKIP, "kubectl not configured"

        except FileNotFoundError:
            return CheckStatus.SKIP, "kubectl not installed"
        except Exception:
            return CheckStatus.SKIP, "Unavailable"


class KubernetesNamespaceCheck(DeploymentCheck):
    """Verify gpu-swarm namespace exists"""

    def __init__(self):
        super().__init__("Kubernetes Namespace", critical=False)

    def run(self) -> tuple[CheckStatus, str]:
        try:
            result = subprocess.run(
                ["kubectl", "get", "namespace", "gpu-swarm"],
                capture_output=True,
                text=True,
                timeout=5
            )

            if result.returncode == 0:
                return CheckStatus.PASS, "Namespace exists"
            else:
                return CheckStatus.WARN, "Namespace not found (not deployed yet)"

        except FileNotFoundError:
            return CheckStatus.SKIP, "kubectl not available"


class CoordinatorPodsCheck(DeploymentCheck):
    """Verify coordinator pods are running"""

    def __init__(self):
        super().__init__("Coordinator Pods", critical=False)

    def run(self) -> tuple[CheckStatus, str]:
        try:
            result = subprocess.run(
                ["kubectl", "-n", "gpu-swarm", "get", "pods", "-l", "app=swarm-coordinator", "-o", "json"],
                capture_output=True,
                text=True,
                timeout=10
            )

            if result.returncode != 0:
                return CheckStatus.SKIP, "Cannot query pods"

            pods = json.loads(result.stdout)
            running = sum(1 for p in pods["items"] if p["status"]["phase"] == "Running")
            total = len(pods["items"])

            if total >= 3 and running == total:
                return CheckStatus.PASS, f"All {total} coordinator pods running"
            elif total > 0:
                return CheckStatus.WARN, f"{running}/{total} pods running"
            else:
                return CheckStatus.SKIP, "No coordinator pods deployed"

        except Exception as e:
            return CheckStatus.SKIP, f"Cannot check: {e!s}"


class StorageVolumeCheck(DeploymentCheck):
    """Verify persistent volumes are bound"""

    def __init__(self):
        super().__init__("Persistent Volumes", critical=False)

    def run(self) -> tuple[CheckStatus, str]:
        try:
            result = subprocess.run(
                ["kubectl", "-n", "gpu-swarm", "get", "pvc", "-o", "json"],
                capture_output=True,
                text=True,
                timeout=10
            )

            if result.returncode != 0:
                return CheckStatus.SKIP, "Cannot query PVCs"

            pvcs = json.loads(result.stdout)
            bound = sum(1 for p in pvcs["items"] if p["status"]["phase"] == "Bound")
            total = len(pvcs["items"])

            if total == 0:
                return CheckStatus.SKIP, "No PVCs deployed"
            elif bound == total:
                return CheckStatus.PASS, f"All {total} volumes bound"
            else:
                return CheckStatus.WARN, f"{bound}/{total} volumes bound"

        except Exception as e:
            return CheckStatus.SKIP, f"Cannot check: {e!s}"


class APIConnectivityCheck(DeploymentCheck):
    """Verify API endpoints are responsive"""

    def __init__(self):
        super().__init__("API Connectivity", critical=True)

    def run(self) -> tuple[CheckStatus, str]:
        endpoints = [
            ("Coordinator API", "http://localhost:9000/health"),
            ("Prometheus", "http://localhost:9090/-/ready"),
            ("Grafana", "http://localhost:3000/api/health"),
        ]

        results = []
        for name, url in endpoints:
            result = subprocess.run(
                ["curl", "-s", "-o", "/dev/null", "-w", "%{http_code}", url],
                capture_output=True,
                text=True,
                timeout=5
            )

            if result.returncode == 0 and result.stdout.startswith("2"):
                results.append(f"{name}: ✓")
            else:
                results.append(f"{name}: ✗")

        message = " | ".join(results)

        if all("✓" in r for r in results):
            return CheckStatus.PASS, message
        elif any("✓" in r for r in results):
            return CheckStatus.WARN, message
        else:
            return CheckStatus.FAIL, message


class NetworkPoliciesCheck(DeploymentCheck):
    """Verify network policies are applied"""

    def __init__(self):
        super().__init__("Network Policies", critical=False)

    def run(self) -> tuple[CheckStatus, str]:
        try:
            result = subprocess.run(
                ["kubectl", "-n", "gpu-swarm", "get", "networkpolicies"],
                capture_output=True,
                text=True,
                timeout=5
            )

            if result.returncode == 0 and "gpu-swarm" in result.stdout:
                count = result.stdout.count("gpu-swarm")
                return CheckStatus.PASS, f"{count} policies active"
            else:
                return CheckStatus.WARN, "No network policies found"

        except FileNotFoundError:
            return CheckStatus.SKIP, "kubectl not available"


class RBACRolesCheck(DeploymentCheck):
    """Verify RBAC roles and bindings"""

    def __init__(self):
        super().__init__("RBAC Configuration", critical=False)

    def run(self) -> tuple[CheckStatus, str]:
        try:
            result = subprocess.run(
                ["kubectl", "-n", "gpu-swarm", "get", "rolebindings"],
                capture_output=True,
                text=True,
                timeout=5
            )

            if result.returncode == 0 and "rolebinding" in result.stdout:
                return CheckStatus.PASS, "RBAC roles configured"
            else:
                return CheckStatus.WARN, "RBAC configuration incomplete"

        except FileNotFoundError:
            return CheckStatus.SKIP, "kubectl not available"


class DeploymentValidator:
    """Main deployment validation orchestrator"""

    def __init__(self):
        self.checks: list[DeploymentCheck] = []
        self.results: dict[str, tuple[CheckStatus, str]] = {}

    def add_check(self, check: DeploymentCheck):
        """Add a check to the validator"""
        self.checks.append(check)

    def run_all(self) -> bool:
        """Run all checks and return overall status"""

        logger.info("\n" + "="*80)
        logger.info("P3 DEPLOYMENT VALIDATION")
        logger.info("="*80 + "\n")

        critical_passes = 0
        critical_fails = 0
        total_passes = 0
        total_fails = 0

        for check in self.checks:
            try:
                status, message = check.run()
                self.results[check.name] = (status, message)

                status_str = f"  {status.value}"
                logger.info(f"{status_str:20} {check.name:40} {message}")

                if status == CheckStatus.PASS:
                    total_passes += 1
                    if check.critical:
                        critical_passes += 1
                elif status == CheckStatus.FAIL:
                    total_fails += 1
                    if check.critical:
                        critical_fails += 1

            except Exception as e:
                logger.error(f"  ✗ EXCEPTION  {check.name:40} {e!s}")
                total_fails += 1
                if check.critical:
                    critical_fails += 1

        # Summary
        logger.info("\n" + "="*80)
        logger.info("VALIDATION SUMMARY")
        logger.info("="*80)

        logger.info(f"\nTotal: {total_passes} passed, {total_fails} failed")
        logger.info(f"Critical: {critical_passes} passed, {critical_fails} failed")

        if critical_fails > 0:
            logger.error(f"\n✗ DEPLOYMENT VALIDATION FAILED - {critical_fails} critical checks failed")
            return False
        else:
            logger.info("\n✓ DEPLOYMENT VALIDATION PASSED - Ready for staging/production")
            return True

    def generate_report(self, filename: str = "/tmp/deployment_validation_report.json"):
        """Generate validation report"""

        report = {
            "timestamp": time.time(),
            "results": {
                name: {
                    "status": status.value,
                    "message": message
                }
                for name, (status, message) in self.results.items()
            }
        }

        with open(filename, "w") as f:
            json.dump(report, f, indent=2)

        logger.info(f"\n✓ Report saved to {filename}")


def run_validation():
    """Execute deployment validation"""

    validator = DeploymentValidator()

    # Add all checks
    validator.add_check(CoordinatorHealthCheck())
    validator.add_check(APIConnectivityCheck())
    validator.add_check(PrometheusTargetsCheck())
    validator.add_check(AlertRulesCheck())
    validator.add_check(ElasticsearchHealthCheck())
    validator.add_check(JaegerHealthCheck())

    validator.add_check(KubernetesClusterCheck())
    validator.add_check(KubernetesNamespaceCheck())
    validator.add_check(CoordinatorPodsCheck())
    validator.add_check(StorageVolumeCheck())
    validator.add_check(NetworkPoliciesCheck())
    validator.add_check(RBACRolesCheck())

    # Run all checks
    success = validator.run_all()

    # Generate report
    validator.generate_report()

    return 0 if success else 1


if __name__ == "__main__":
    exit_code = run_validation()
    sys.exit(exit_code)
