"""Regression tests for fail-closed advisory CI failure classification."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("autopilot_triage", Path(__file__).with_name("autopilot_triage.py"))
triage = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(triage)


class TriageTests(unittest.TestCase):
    def test_test_failure_is_not_infrastructure(self):
        value = triage.classify({"name": "tests", "status": "completed", "conclusion": "failure"}, "test result: FAILED. 1 failed")
        self.assertEqual(value["classification"], "TEST_FAILURE")
        self.assertFalse(value["safe_to_ignore"])

    def test_runner_disconnection_requires_evidence(self):
        value = triage.classify({"status": "completed", "conclusion": "failure"}, "Lost communication with the server. Runner connection lost")
        self.assertEqual(value["classification"], "INFRASTRUCTURE")
        self.assertFalse(value["safe_to_ignore"])

    def test_artifact_quota_is_evidence_storage_failure_not_code(self):
        value = triage.classify(
            {"status": "completed", "conclusion": "failure"},
            "Failed to CreateArtifact: Artifact storage quota has been hit. Unable to upload any new artifacts."
        )
        self.assertEqual(value["classification"], "EVIDENCE_STORAGE")
        self.assertFalse(value["safe_to_ignore"])

    def test_authentication_is_configuration_failure(self):
        value = triage.classify({"status": "completed", "conclusion": "failure"}, "Permission denied (publickey)")
        self.assertEqual(value["classification"], "CONFIGURATION")

    def test_missing_logs_are_unknown(self):
        value = triage.classify({"status": "completed", "conclusion": "failure"}, "")
        self.assertEqual(value["classification"], "UNKNOWN")

    def test_queued_job_not_finished(self):
        value = triage.classify({"status": "queued", "conclusion": None}, "")
        self.assertEqual(value["classification"], "NOT_FINISHED")

    def test_success_requires_success_conclusion(self):
        value = triage.classify({"status": "completed", "conclusion": "success"}, "warning: test failed previously")
        self.assertEqual(value["classification"], "PASSED")

    def test_raw_logs_are_not_included(self):
        secret = "password=supersecret"
        value = triage.classify({"status": "completed", "conclusion": "failure"}, "error: " + secret)
        self.assertNotIn(secret, str(value))

    def test_report_never_grants_production_ready(self):
        jobs = [{"id": 1, "status": "completed", "conclusion": "failure"}, {"id": 2, "status": "queued", "conclusion": None}]
        value = triage.report("abc123", jobs, {"1": "error: build failed"})
        self.assertEqual(value["overall"], "BLOCKED")
        self.assertEqual(len(value["jobs"]), 2)
        self.assertFalse(value["production_ready"])


if __name__ == "__main__":
    unittest.main()
