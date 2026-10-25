"""Synthetic integration join mutation tests, no network or model calls."""
import importlib.util
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).parent))
spec = importlib.util.spec_from_file_location("integration_join", Path(__file__).with_name("integration_join.py"))
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
SHA = "a" * 40


def fixtures():
    github = {"source": "GITHUB_ACTIONS_METADATA",
              "pr": {"number": 683, "head_sha": SHA},
              "checks": [{"head_sha": SHA, "name": "local-ci",
                          "workflow_run_id": 9, "run_attempt": 1,
                          "status": "completed", "conclusion": "success",
                          "evidence_state": "OBSERVED"}]}
    review = {"source": "PR_AGENT", "pr_number": 683, "head_sha": SHA,
              "review_completed": True, "findings": []}
    return github, review


class IntegrationJoinTests(unittest.TestCase):
    def test_consistent_evidence_is_only_advisory(self):
        result = mod.join(*fixtures())
        self.assertEqual(result["integration"], "PASS")
        self.assertFalse(result["release_authorized"])
        self.assertFalse(result["live_integration_verified"])
        self.assertFalse(result["review_correctness_verified"])

    def test_stale_review_is_rejected(self):
        github, review = fixtures()
        review["head_sha"] = "b" * 40
        self.assertEqual(mod.join(github, review)["integration"], "FAIL")

    def test_missing_review_is_rejected(self):
        github, _ = fixtures()
        self.assertEqual(mod.join(github, {})["integration"], "FAIL")

    def test_unfinished_review_is_rejected(self):
        github, review = fixtures()
        review["review_completed"] = False
        self.assertEqual(mod.join(github, review)["integration"], "FAIL")

    def test_failing_job_is_rejected(self):
        github, review = fixtures()
        github["checks"][0]["conclusion"] = "failure"
        self.assertEqual(mod.join(github, review)["integration"], "FAIL")

    def test_missing_jobs_are_rejected(self):
        github, review = fixtures()
        github["checks"] = []
        self.assertEqual(mod.join(github, review)["integration"], "FAIL")

    def test_missing_attempt_is_rejected(self):
        github, review = fixtures()
        del github["checks"][0]["run_attempt"]
        self.assertEqual(mod.join(github, review)["integration"], "FAIL")

    def test_other_pr_review_is_rejected(self):
        github, review = fixtures()
        review["pr_number"] = 999
        self.assertEqual(mod.join(github, review)["integration"], "FAIL")

    def test_unproved_bug_reproduction_is_rejected(self):
        github, review = fixtures()
        review["findings"] = [{"head_sha": SHA, "path": "src/a.rs", "line": 3,
                               "severity": "high", "verification": "REPRODUCED"}]
        self.assertEqual(mod.join(github, review)["integration"], "FAIL")


if __name__ == "__main__":
    unittest.main()
