"""Regression tests for capacity-aware, read-only CI placement."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("autopilot_capacity", Path(__file__).with_name("autopilot_capacity.py"))
capacity = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(capacity)


def worker(**changes):
    base = dict(name="x3build1", role="build", ci="IDLE", fresh=True,
                primary_workload_active=False, cpu_threads=16, load_1m=3,
                disk_free_gb=80)
    base.update(changes)
    return base


class CapacityTests(unittest.TestCase):
    def test_recommends_only_confirmed_eligible_worker(self):
        result = capacity.plan([worker(), worker(name="x3build2", disk_free_gb=100, load_1m=6)], "build")
        self.assertEqual(result["decision"], "RECOMMEND")
        self.assertEqual(result["recommended_worker"], "x3build1")
        self.assertFalse(result["execution_authorized"])

    def test_unknown_freshness_defers(self):
        self.assertEqual(capacity.plan([worker(fresh=None)], "build")["decision"], "DEFER")

    def test_protect_primary_workloads(self):
        self.assertEqual(capacity.plan([worker(primary_workload_active=True)], "build")["decision"], "DEFER")

    def test_busy_and_offline_runners_rejected(self):
        for state in ("BUSY", "OFFLINE", "UNKNOWN", "UNREGISTERED"):
            with self.subTest(state=state):
                self.assertEqual(capacity.plan([worker(ci=state)], "build")["decision"], "DEFER")

    def test_disk_and_cpu_thresholds(self):
        for change in ({"disk_free_gb": 2}, {"load_1m": 15}, {"cpu_threads": 1}):
            with self.subTest(change=change):
                self.assertEqual(capacity.plan([worker(**change)], "build")["decision"], "DEFER")

    def test_missing_data_fails_closed(self):
        for field in ("cpu_threads", "load_1m", "disk_free_gb"):
            x = worker()
            del x[field]
            self.assertEqual(capacity.plan([x], "build")["decision"], "DEFER")

    def test_nonfinite_capacity_is_rejected(self):
        for value in (float('nan'), float('inf'), float('-inf')):
            for key in ('cpu_threads', 'load_1m', 'disk_free_gb'):
                with self.subTest(value=value, key=key):
                    self.assertEqual(capacity.plan([worker(**{key: value})], 'build')['decision'], 'DEFER')

    def test_gpu_work_remains_on_gpu_role(self):
        self.assertEqual(capacity.plan([worker()], "inference")["decision"], "DEFER")
        self.assertEqual(capacity.plan([worker(name="x3gpu1", role="gpu")], "inference")["decision"], "RECOMMEND")

    def test_unknown_class_rejected(self):
        self.assertEqual(capacity.plan([worker()], "release")["decision"], "REJECT")

    def test_empty_workers_defers(self):
        self.assertEqual(capacity.plan([], "build")["decision"], "DEFER")


if __name__ == "__main__":
    unittest.main()
