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

    def test_rust_format_diagnostics_are_read_only(self):
        output = triage.classify(
            {"id": 70, "status": "completed", "conclusion": "failure"},
            "Diff in /runner/xxxstar/pallets/x3-kernel/src/native_trading.rs:28:")
        self.assertEqual(output["classification"], "RUST_FORMAT")
        self.assertEqual(output["suggested_diagnostic"], "cargo fmt --all -- --check")

    def test_action_download_timeout_is_not_misidentified_as_code_failure(self):
        log = (
            "Failed to resolve action download info. Error: "
            "The HTTP request timed out after 00:01:40. "
            "Resource temporarily unavailable (launch.actions.githubusercontent.com:443)"
        )
        result = triage.classify(
            {"id": 121599738535, "status": "completed", "conclusion": "failure"}, log
        )
        self.assertEqual(result["classification"], "ACTION_DOWNLOAD_FAILURE")
        self.assertFalse(result["safe_to_ignore"])
        self.assertIn("do not auto-retry", result["suggested_diagnostic"])
        self.assertNotIn("launch.actions.githubusercontent.com", str(result))

    def test_python_dependency_not_misreported_as_build(self):
        output = triage.classify(
            {"status": "completed", "conclusion": "failure"},
            "check_ci_workflow_refs: PyYAML is required (pip install pyyaml)")
        self.assertEqual(output["classification"], "PYTHON_DEPENDENCY")

    def test_locked_cargo_resolution_failure(self):
        output = triage.classify(
            {"status": "completed", "conclusion": "failure"},
            "the lock file needs to be updated but --locked was passed")
        self.assertEqual(output["classification"], "LOCKFILE_DRIFT")

    def test_artifact_storage_quota_is_not_green(self):
        output = triage.classify(
            {"status": "completed", "conclusion": "failure"},
            "Artifact storage quota has been hit.")
        self.assertEqual(output["classification"], "ARTIFACT_STORAGE")
        self.assertFalse(output["safe_to_ignore"])

    def test_no_jobs_is_missing_evidence_not_success(self):
        output = triage.classify(
            {"name": "compiler", "status": "unknown", "conclusion": None,
             "evidence_state": "NO_JOBS"})
        self.assertEqual(output["classification"], "MISSING_EVIDENCE")

    def test_head_bound_github_evidence_input(self):
        import json
        import tempfile
        head = "a" * 40
        with tempfile.TemporaryDirectory() as directory:
            packet = Path(directory) / "evidence.json"
            output = Path(directory) / "triage.json"
            packet.write_text(json.dumps({
                "source": "GITHUB_ACTIONS_METADATA",
                "pr": {"number": 683, "head_sha": head},
                "checks": [{"head_sha": head, "name": "local-ci", "job_id": 11,
                            "status": "completed", "conclusion": "failure",
                            "evidence_state": "OBSERVED"}]}))
            self.assertEqual(triage.main([
                "--github-evidence", str(packet), "--head-sha", head,
                "--expected-head", head, "--output", str(output)]), 0)
            record = json.loads(output.read_text())
            self.assertEqual(record["overall"], "BLOCKED")
            self.assertEqual(record["jobs"][0]["id"], 11)
            self.assertFalse(record["production_ready"])
            with self.assertRaises(SystemExit) as raised:
                triage.main(["--github-evidence", str(packet), "--head-sha", "b" * 40])
            self.assertEqual(raised.exception.code, 2)

    def test_report_never_grants_production_ready(self):
        jobs = [{"id": 1, "status": "completed", "conclusion": "failure"}, {"id": 2, "status": "queued", "conclusion": None}]
        value = triage.report("abc123", jobs, {"1": "error: build failed"})
        self.assertEqual(value["overall"], "BLOCKED")
        self.assertEqual(len(value["jobs"]), 2)
        self.assertFalse(value["production_ready"])


if __name__ == "__main__":
    unittest.main()
