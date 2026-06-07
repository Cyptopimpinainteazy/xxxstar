"""Tests for x3_operator.health"""

from x3_operator.health import HealthStatus, run_health_check


def test_health_check_passes():
    report = run_health_check(min_disk_gb=1, min_ram_gb=1, min_cpu_cores=1)
    assert report.overall == HealthStatus.PASS


def test_health_check_disk_fail():
    report = run_health_check(min_disk_gb=999999)
    disk_check = next(c for c in report.checks if c.name == "disk_space")
    assert disk_check.status == HealthStatus.FAIL
    assert report.overall == HealthStatus.FAIL


def test_health_check_ram_fail():
    report = run_health_check(min_ram_gb=999999)
    ram_check = next(c for c in report.checks if c.name == "ram")
    assert ram_check.status == HealthStatus.FAIL


def test_health_check_includes_os():
    report = run_health_check()
    os_check = next(c for c in report.checks if c.name == "os")
    assert os_check.status == HealthStatus.PASS
    assert "Linux" in os_check.value or "Darwin" in os_check.value or "Windows" in os_check.value


def test_recommended_roles():
    report = run_health_check(min_disk_gb=1, min_ram_gb=1, min_cpu_cores=1)
    assert "validator" in report.recommended_roles
    assert "relayer" in report.recommended_roles


def test_gpu_optional():
    """When not requiring GPU, GPU check should be pass/absent."""
    report = run_health_check(require_gpu=False)
    gpu_checks = [c for c in report.checks if c.name == "gpu"]
    for g in gpu_checks:
        assert g.status in (HealthStatus.PASS, HealthStatus.WARN)
