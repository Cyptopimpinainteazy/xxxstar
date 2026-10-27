"""Regression suite for X3 Public Launch Guardian (stdlib only, zero network)."""
import copy
import datetime as dt
import io
import json
import pathlib
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import public_launch_guardian as g  # noqa: E402

HEAD = "a" * 40
OTHER = "b" * 40
NOW = dt.datetime(2026, 10, 26, 20, 0, tzinfo=dt.timezone.utc)
POLICY = {
    "schema_version": 1,
    "required_workflows": [
        {"gate": "local", "path": ".github/workflows/local-ci.yml"},
        {"gate": "trade", "path": ".github/workflows/x3-atomic-trade-engine.yml"},
    ],
    "min_validators": 7,
    "required_fleet_assertions": ["finality_progress_verified", "onchain_authority_set_verified"],
    "required_operator_assertions": ["rpc_rate_limits_verified", "rollback_rehearsed"],
}


def valid_runs():
    return [
        {"id": i + 1, "run_attempt": 1, "path": item["path"],
         "head_sha": HEAD, "created_at": "2026-10-26T19:00:00Z",
         "status": "completed", "conclusion": "success"}
        for i, item in enumerate(POLICY["required_workflows"])
    ]


def valid_fleet():
    return {
        "candidate_sha": HEAD,
        "genesis_hash": "0x" + "a" * 64,
        "evidence_sha256": "b" * 64,
        "validators": [
            {"peer_id": "peer-" + str(i), "authority_key": "0x" + format(i + 1, "064x"),
             "finalized_number": 150, "finalized_hash": "0x" + "f" * 64}
            for i in range(7)
        ],
        "finality_progress_verified": True,
        "onchain_authority_set_verified": True,
    }


def valid_operations():
    return {"candidate_sha": HEAD, "evidence_sha256": "d" * 64,
            "rpc_rate_limits_verified": True, "rollback_rehearsed": True}


def check(runs=None, fleet=None, operations=None, source="github_live"):
    return g.assess(HEAD,
                    valid_runs() if runs is None else runs,
                    POLICY,
                    valid_fleet() if fleet is None else fleet,
                    valid_operations() if operations is None else operations,
                    now=NOW, source=source)


class AdmissionTests(unittest.TestCase):
    def test_only_review_required_even_with_all_observed_checks(self):
        report = check()
        self.assertEqual(report["decision"], "INDEPENDENT_REVIEW_REQUIRED")
        self.assertEqual([r["status"] for r in report["checks"]], ["PASS", "PASS"])
        self.assertFalse(report["release_authorized"])
        self.assertFalse(report["physical_fleet_verified_by_guardian"])
        self.assertFalse(report["independent_security_review_verified"])

    def test_missing_gate_is_blocking(self):
        report = check(runs=valid_runs()[:1])
        self.assertEqual(report["decision"], "BLOCKED")
        self.assertEqual(report["checks"][1]["status"], "MISSING")

    def test_later_queued_run_overrides_prior_green(self):
        runs = valid_runs()
        runs.append({**runs[0], "id": 90, "created_at": "2026-10-26T19:30:00Z",
                     "status": "queued", "conclusion": None})
        report = check(runs=runs)
        self.assertEqual(report["checks"][0]["status"], "BLOCKED")

    def test_later_failed_run_overrides_prior_green(self):
        runs = valid_runs()
        runs.append({**runs[0], "id": 90, "created_at": "2026-10-26T19:30:00Z",
                     "status": "completed", "conclusion": "failure"})
        self.assertEqual(check(runs=runs)["checks"][0]["status"], "BLOCKED")

    def test_wrong_sha_never_passes(self):
        runs = valid_runs()
        runs[0]["head_sha"] = OTHER
        self.assertEqual(check(runs=runs)["checks"][0]["status"], "BLOCKED")

    def test_malformed_identity_does_not_count_as_green(self):
        runs = valid_runs()
        runs[0]["run_attempt"] = None
        self.assertEqual(check(runs=runs)["checks"][0]["status"], "INVALID")

    def test_stale_green_is_refused(self):
        runs = valid_runs()
        runs[0]["created_at"] = "2026-08-26T19:00:00Z"
        self.assertEqual(check(runs=runs)["checks"][0]["status"], "STALE")

    def test_future_green_is_refused(self):
        runs = valid_runs()
        runs[0]["created_at"] = "2027-10-26T19:00:00Z"
        self.assertEqual(check(runs=runs)["checks"][0]["status"], "STALE")

    def test_skipped_never_counts_as_green(self):
        runs = valid_runs()
        runs[0]["conclusion"] = "skipped"
        self.assertEqual(check(runs=runs)["checks"][0]["status"], "BLOCKED")

    def test_like_named_workflow_cannot_spoof_gate(self):
        runs = valid_runs()
        runs[0]["path"] = "fork/.github/workflows/local-ci.yml"
        self.assertEqual(check(runs=runs)["checks"][0]["status"], "MISSING")

    def test_valid_github_repo_prefixed_workflow_path(self):
        runs = valid_runs()
        runs[0]["path"] = g.DEFAULT_REPO + "/.github/workflows/local-ci.yml@refs/heads/master"
        self.assertEqual(check(runs=runs)["checks"][0]["status"], "PASS")

    def test_offline_fixture_never_admits(self):
        report = check(source="fixture")
        self.assertEqual(report["decision"], "BLOCKED")
        self.assertTrue(any("unverified" in reason for reason in report["blockers"]))

    def test_missing_physical_evidence_never_admits(self):
        report = g.assess(HEAD, valid_runs(), POLICY, fleet=None,
                          operations=valid_operations(), now=NOW, source="github_live")
        self.assertEqual(report["decision"], "BLOCKED")

    def test_duplicate_authority_key_is_blocking(self):
        fleet = valid_fleet()
        fleet["validators"][2]["authority_key"] = fleet["validators"][0]["authority_key"]
        self.assertTrue(any("duplicate" in x for x in
                            check(fleet=fleet)["blockers"]))

    def test_duplicate_peer_identity_is_blocking(self):
        fleet = valid_fleet()
        fleet["validators"][2]["peer_id"] = fleet["validators"][0]["peer_id"]
        self.assertEqual(check(fleet=fleet)["decision"], "BLOCKED")

    def test_divergent_finality_is_blocking(self):
        fleet = valid_fleet()
        fleet["validators"][3]["finalized_hash"] = "0x" + "0" * 64
        self.assertEqual(check(fleet=fleet)["decision"], "BLOCKED")

    def test_missing_onchain_authorities_is_blocking(self):
        fleet = valid_fleet()
        fleet["onchain_authority_set_verified"] = False
        self.assertEqual(check(fleet=fleet)["decision"], "BLOCKED")

    def test_operations_claim_must_be_bound_to_candidate(self):
        operations = valid_operations()
        operations["candidate_sha"] = OTHER
        self.assertEqual(check(operations=operations)["decision"], "BLOCKED")

    def test_no_evidence_hash_is_blocking(self):
        operations = valid_operations()
        operations.pop("evidence_sha256")
        self.assertEqual(check(operations=operations)["decision"], "BLOCKED")

    def test_policy_rejects_duplicate_workflow(self):
        policy = copy.deepcopy(POLICY)
        policy["required_workflows"].append(copy.deepcopy(policy["required_workflows"][0]))
        with self.assertRaises(ValueError):
            g.require_policy(policy)

    def test_policy_does_not_allow_lower_validator_minimum(self):
        policy = copy.deepcopy(POLICY)
        policy["min_validators"] = 3
        with self.assertRaises(ValueError):
            g.require_policy(policy)

    def test_reject_malformed_candidate(self):
        with self.assertRaises(ValueError):
            g.assess("not-a-git-sha", valid_runs(), POLICY)

    def test_fail_closed_when_github_pagination_exceeds_bound(self):
        repeated = [{"head_sha": HEAD}] * 100

        class Response:
            def __enter__(self): return io.BytesIO(json.dumps({"workflow_runs": repeated}).encode())
            def __exit__(self, *args): return False

        with patch.object(g.urllib.request, "urlopen", return_value=Response()):
            with self.assertRaisesRegex(ValueError, "pagination limit"):
                g.fetch_github_runs(g.DEFAULT_REPO, HEAD, "fake", max_pages=2)

    def test_fail_closed_when_github_returns_other_head(self):
        payload = {"workflow_runs": [{"head_sha": OTHER}]}

        class Response:
            def __enter__(self): return io.BytesIO(json.dumps(payload).encode())
            def __exit__(self, *args): return False

        with patch.object(g.urllib.request, "urlopen", return_value=Response()):
            with self.assertRaisesRegex(ValueError, "non-candidate"):
                g.fetch_github_runs(g.DEFAULT_REPO, HEAD, "fake")

    def test_offline_cli_returns_nonzero_even_with_all_green_runs(self):
        with tempfile.TemporaryDirectory() as scratch:
            folder = pathlib.Path(scratch)
            policy = folder / "policy.json"
            runs = folder / "runs.json"
            policy.write_text(json.dumps(POLICY))
            runs.write_text(json.dumps(valid_runs()))
            code = g.main(["--candidate-sha", HEAD, "--policy", str(policy),
                           "--runs-json", str(runs), "--output", str(folder / "report.json")])
            self.assertEqual(code, 1)
            report = json.loads((folder / "report.json").read_text())
            self.assertEqual(report["decision"], "BLOCKED")
            self.assertFalse(report["release_authorized"])


if __name__ == "__main__":
    unittest.main()
