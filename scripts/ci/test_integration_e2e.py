"""Offline contract mutation regression tests."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("integration_e2e", Path(__file__).with_name("integration_e2e.py"))
mod = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(mod)


class IntegrationEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.sha = "a" * 40
        self.good = {"pr": {"head_sha": self.sha}, "checks": [{"head_sha": self.sha, "status": "completed", "conclusion": "success"}], "findings": []}

    def test_synthetic_faults_caught(self):
        self.assertTrue(mod.synthetic_suite()["passed"])
        self.assertFalse(mod.synthetic_suite()["live_integration_verified"])

    def test_stale_review_head_fails(self):
        self.good["findings"] = [{"head_sha": "b" * 40, "path": "a.rs", "line": 2, "severity": "high", "verification": "UNVERIFIED"}]
        self.assertEqual(mod.assess(self.good)["integration"], "FAIL")

    def test_claimed_reproduction_needs_evidence(self):
        self.good["findings"] = [{"head_sha": self.sha, "path": "a.rs", "line": 2, "severity": "high", "verification": "REPRODUCED"}]
        self.assertEqual(mod.assess(self.good)["integration"], "FAIL")

    def test_proved_finding_keeps_release_off(self):
        self.good["findings"] = [{"head_sha": self.sha, "path": "a.rs", "line": 2, "severity": "high", "verification": "REPRODUCED", "reproducer": {"command": "cargo test", "exit_code": 1}}]
        result = mod.assess(self.good)
        self.assertEqual(result["integration"], "PASS")
        self.assertFalse(result["release_authorized"])

    def test_missing_checks_fails(self):
        self.good["checks"] = []
        self.assertEqual(mod.assess(self.good)["integration"], "FAIL")

    def test_invalid_relative_path_fails(self):
        self.good["findings"] = [{"head_sha": self.sha, "path": "../bad.rs", "line": 2, "severity": "high", "verification": "UNVERIFIED"}]
        self.assertEqual(mod.assess(self.good)["integration"], "FAIL")


if __name__ == "__main__":
    unittest.main()
