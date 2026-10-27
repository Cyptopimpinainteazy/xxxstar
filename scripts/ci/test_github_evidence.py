"""Fake-API integration tests: no network and no privileged token."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("github_evidence", Path(__file__).with_name("github_evidence.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
SHA = "a" * 40


def fake(path, token):
    if "/pulls/" in path:
        return {"head": {"sha": SHA}, "base": {"repo": {"full_name": "Cyptopimpinainteazy/xxxstar"}}}
    if "/actions/runs?" in path:
        return {"workflow_runs": [{"id": 123, "head_sha": SHA, "name": "Local CI", "run_attempt": 2}]}
    if "/jobs?" in path:
        return {"jobs": [{"id": 456, "name": "local-ci", "status": "completed", "conclusion": "failure"}]}
    raise AssertionError(path)


class EvidenceTests(unittest.TestCase):
    def test_exact_head_join(self):
        packet = module.gather("Cyptopimpinainteazy/xxxstar", 683, "fake", fake)
        self.assertEqual(packet["checks"][0]["head_sha"], SHA)
        self.assertEqual(packet["checks"][0]["run_attempt"], 2)
        self.assertEqual(packet["checks"][0]["conclusion"], "failure")
        self.assertFalse(packet["release_authorized"])
        self.assertFalse(packet["required_checks_verified"])
        self.assertFalse(packet["pr_agent_output_verified"])

    def test_no_runs_is_not_proof(self):
        def empty(path, token):
            return {"workflow_runs": []} if "/actions/runs?" in path else fake(path, token)
        packet = module.gather("Cyptopimpinainteazy/xxxstar", 683, "fake", empty)
        self.assertEqual(packet["checks"], [])
        self.assertFalse(packet["live_integration_verified"])

    def test_stale_run_rejected(self):
        def stale(path, token):
            if "/actions/runs?" in path:
                return {"workflow_runs": [{"id": 1, "head_sha": "b" * 40, "run_attempt": 1}]}
            return fake(path, token)
        with self.assertRaisesRegex(ValueError, "stale"):
            module.gather("Cyptopimpinainteazy/xxxstar", 683, "fake", stale)

    def test_jobless_run_is_unknown(self):
        def jobless(path, token):
            return {"jobs": []} if "/jobs?" in path else fake(path, token)
        packet = module.gather("Cyptopimpinainteazy/xxxstar", 683, "fake", jobless)
        self.assertEqual(packet["checks"][0]["evidence_state"], "NO_JOBS")

    def test_invalid_head_rejected(self):
        def invalid(path, token):
            return {"head": {"sha": "short"}} if "/pulls/" in path else fake(path, token)
        with self.assertRaisesRegex(ValueError, "head"):
            module.gather("Cyptopimpinainteazy/xxxstar", 683, "fake", invalid)

    def test_bounded_pagination_fails_closed(self):
        def full(path, token):
            if "/actions/runs?" in path:
                return {"workflow_runs": [{"id": i, "head_sha": SHA, "run_attempt": 1} for i in range(100)]}
            return fake(path, token)
        with self.assertRaisesRegex(ValueError, "pagination"):
            module.gather("Cyptopimpinainteazy/xxxstar", 683, "fake", full, max_pages=1)


    def test_head_moving_during_collection_is_rejected(self):
        calls = [0]
        def moved(path, token):
            if "/pulls/" in path:
                calls[0] += 1
                packet = fake(path, token)
                packet["head"]["sha"] = SHA if calls[0] == 1 else "b" * 40
                return packet
            return fake(path, token)
        with self.assertRaisesRegex(ValueError, "changed during collection"):
            module.gather("Cyptopimpinainteazy/xxxstar", 683, "fake", moved)

    def test_job_from_other_commit_is_rejected(self):
        def mismatched(path, token):
            result = fake(path, token)
            if "/jobs?" in path:
                result["jobs"][0]["head_sha"] = "b" * 40
            return result
        with self.assertRaisesRegex(ValueError, "job head"):
            module.gather("Cyptopimpinainteazy/xxxstar", 683, "fake", mismatched)

    def test_missing_base_repository_is_rejected(self):
        def no_base(path, token):
            if "/pulls/" in path:
                packet = fake(path, token)
                packet["base"]["repo"] = {}
                return packet
            return fake(path, token)
        with self.assertRaisesRegex(ValueError, "target requested repository"):
            module.gather("Cyptopimpinainteazy/xxxstar", 683, "fake", no_base)



    def test_event_head_is_immutable(self):
        with self.assertRaisesRegex(ValueError, "requested head"):
            module.gather("Cyptopimpinainteazy/xxxstar", 683, "fake", fake,
                          expected_head="b" * 40)
        packet = module.gather("Cyptopimpinainteazy/xxxstar", 683, "fake", fake,
                               expected_head=SHA)
        self.assertEqual(packet["pr"]["head_sha"], SHA)



if __name__ == "__main__":
    unittest.main()
