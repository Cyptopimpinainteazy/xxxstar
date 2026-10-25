"""Advisory verification reuse: never authorize CI skipping from fixture data."""
import json
import tempfile
import unittest
from pathlib import Path

from verification_reuse import decide, main


def target(**changes):
    item = {
        "gate_id": "script-syntax",
        "profile": "fast",
        "head_sha": "a" * 40,
        "base_sha": "b" * 40,
        "gate_spec_sha256": "1" * 64,
        "toolchain_sha256": "2" * 64,
        "dependencies_sha256": "3" * 64,
        "environment_sha256": "4" * 64,
    }
    item.update(changes)
    return item


def receipt(**changes):
    item = {
        **target(),
        "status": "completed",
        "conclusion": "success",
        "exit_code": 0,
        "run_id": 51,
        "run_attempt": 1,
        "log_sha256": "5" * 64,
    }
    item.update(changes)
    return item


class ReuseTests(unittest.TestCase):
    def test_exact_success_is_only_an_untrusted_candidate(self):
        result = decide(target(), [receipt()])
        self.assertEqual(result["decision"], "EVIDENCE_CANDIDATE")
        self.assertEqual(result["candidate_run_ids"], [51])
        self.assertFalse(result["skip_authorized"])
        self.assertFalse(result["trusted_provenance_verified"])
        self.assertFalse(result["production_ready"])

    def test_head_or_base_must_match(self):
        for delta in (dict(head_sha="e" * 40), dict(base_sha="d" * 40)):
            with self.subTest(delta=delta):
                self.assertEqual(decide(target(**delta), [receipt()])["decision"], "RUN_REQUIRED")

    def test_toolchain_dependencies_gate_and_environment_are_identity(self):
        for field in ("gate_spec_sha256", "toolchain_sha256", "dependencies_sha256", "environment_sha256"):
            with self.subTest(field=field):
                self.assertEqual(decide(target(**{field: "f" * 64}), [receipt()])["decision"], "RUN_REQUIRED")

    def test_incomplete_identity_requires_run(self):
        t = target()
        t.pop("dependencies_sha256")
        self.assertEqual(decide(t, [receipt()])["decision"], "RUN_REQUIRED")

    def test_failed_cancelled_or_pending_do_not_match(self):
        for changes in (dict(conclusion="failure"), dict(status="queued", conclusion=None),
                        dict(conclusion="cancelled"), dict(exit_code=1), dict(exit_code=False)):
            with self.subTest(changes=changes):
                self.assertEqual(decide(target(), [receipt(**changes)])["decision"], "RUN_REQUIRED")

    def test_missing_log_or_run_provenance_is_not_evidence(self):
        for changes in (dict(log_sha256=None), dict(log_sha256="abc"), dict(run_id=None),
                        dict(run_attempt=0), dict(run_id=True)):
            with self.subTest(changes=changes):
                self.assertEqual(decide(target(), [receipt(**changes)])["decision"], "RUN_REQUIRED")

    def test_scoped_or_live_gates_are_not_reused(self):
        for changes in (dict(profile="scoped"), dict(gate_id="consensus-soak"),
                        dict(gate_id="release-gate-mainnet-check")):
            with self.subTest(changes=changes):
                t = target(**changes)
                self.assertEqual(decide(t, [receipt(**changes)])["decision"], "RUN_REQUIRED")

    def test_contradictory_receipts_require_new_run(self):
        self.assertEqual(decide(target(), [receipt(), receipt(run_id=52, conclusion="failure")])["decision"], "RUN_REQUIRED")

    def test_malformed_collection_fails_closed(self):
        self.assertEqual(decide(target(), {"bad": "data"})["decision"], "RUN_REQUIRED")
        self.assertEqual(decide(target(), ["fake"])["decision"], "RUN_REQUIRED")
        self.assertEqual(decide(target(), [receipt()] * 257)["decision"], "RUN_REQUIRED")

    def test_malformed_gate_id_fails_closed(self):
        self.assertEqual(decide(target(gate_id=[]), [receipt()])["decision"], "RUN_REQUIRED")

    def test_unrelated_receipts_do_not_count(self):
        self.assertEqual(decide(target(), [receipt(gate_id="format-check")])["decision"], "RUN_REQUIRED")

    def test_cli_offline_never_returns_authorization(self):
        with tempfile.TemporaryDirectory() as d:
            t, r = Path(d) / "target.json", Path(d) / "receipts.json"
            t.write_text(json.dumps(target()))
            r.write_text(json.dumps([receipt()]))
            self.assertEqual(main(["--target", str(t), "--receipts", str(r)]), 0)


if __name__ == "__main__":
    unittest.main()
