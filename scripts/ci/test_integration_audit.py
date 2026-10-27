"""Mutation-based tests: wiring audit must detect disconnected components."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("integration_audit", Path(__file__).with_name("integration_audit.py"))
auditmod = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(auditmod)


class IntegrationAuditTests(unittest.TestCase):
    def populate(self, root):
        for rel, markers in auditmod.CONTRACTS.values():
            p = root / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text("\n".join(markers), encoding="utf-8")

    def test_valid_fixture_does_not_claim_live_verification(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.populate(root)
            result = auditmod.audit(root)
            self.assertEqual(result["status"], "PASS")
            self.assertFalse(result["live_integration_verified"])
            self.assertFalse(result["release_authorized"])

    def test_missing_workflow_fails_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.populate(root)
            (root / ".github/workflows/pr-agent-auto.yml").unlink()
            result = auditmod.audit(root)
            self.assertEqual(result["status"], "FAIL")
            self.assertEqual(next(x for x in result["checks"] if x["contract"] == "automatic-review")["status"], "FAIL")

    def test_missing_fork_safety_marker_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.populate(root)
            p = root / ".github/workflows/pr-agent-auto.yml"
            p.write_text(p.read_text().replace(
                "pull_request.head.repo.full_name == github.repository", "true"))
            self.assertEqual(auditmod.audit(root)["status"], "FAIL")

    def test_missing_failure_classifier_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.populate(root)
            (root / "scripts/ci/autopilot_triage.py").unlink()
            self.assertEqual(auditmod.audit(root)["status"], "FAIL")

    def test_missing_runner_safety_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.populate(root)
            p = root / ".github/workflows/x3-ci-autopilot.yml"
            p.write_text(p.read_text().replace(
                "github.event.pull_request.head.repo.full_name == github.repository", "true"))
            self.assertEqual(auditmod.audit(root)["status"], "FAIL")


if __name__ == "__main__":
    unittest.main()
