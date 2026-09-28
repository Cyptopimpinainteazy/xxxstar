"""Exercise the real wrapper with test-only commands in an isolated repository.

These tests prove orchestration and reporting, not blockchain readiness.
"""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = Path("scripts/mainnet/run_release_gates_rc6.sh")


class ReleaseSequenceTests(unittest.TestCase):
    def run_sequence(self, failure="", spaced=False, broken_reports=False):
        with tempfile.TemporaryDirectory(prefix="x3-rc6-test-") as temporary:
            parent = Path(temporary)
            repo = parent / ("repo with spaces" if spaced else "repo")
            (repo / SCRIPT.parent).mkdir(parents=True)
            shutil.copy2(ROOT / SCRIPT, repo / SCRIPT)
            for stage, relative in (
                ("e2e", "scripts/mainnet/rc2_mock_and_live_gate.sh"),
                ("security", "scripts/run-security-gates.sh"),
                ("readiness", "scripts/mainnet/rc6_public_testnet_readiness.sh"),
            ):
                (repo / relative).write_text(
                    f'echo {stage}\n'
                    f'if [[ "$RC6_TEST_FAILURE" == {stage} ]]; then exit 17; fi\n'
                )
            # BASH_ENV also reaches the original wrapper's login-shell commands.
            # No real cargo build or live gate is ever invoked by this fixture.
            environment = parent / "test-environment.sh"
            environment.write_text(
                'cargo() { echo build; '
                'if [[ "$RC6_TEST_FAILURE" == build ]]; then return 17; fi; }\n'
            )
            if broken_reports:
                (repo / "reports").write_text("not a directory\n")
            result = subprocess.run(
                ["bash", str(repo / SCRIPT)], cwd=parent,
                env={**os.environ, "BASH_ENV": str(environment),
                     "RC6_TEST_FAILURE": failure},
                capture_output=True, text=True, timeout=30,
            )
            reports = list((repo / "reports/rc6").glob("release_gate_sequence_*.md"))
            report = reports[0].read_text() if reports else ""
            logs = {p.name: p.read_text() for p in (repo / "reports/rc6").glob("*.log")}
            return result, report, logs, (parent / "reports").exists()

    def test_each_child_failure_fails_sequence_and_preserves_later_results(self):
        for stage in ("build", "e2e", "security", "readiness"):
            with self.subTest(stage=stage):
                result, report, logs, misplaced = self.run_sequence(stage)
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn("exit_code=17", report)
                self.assertIn("overall_status=FAIL", report)
                self.assertEqual(len(logs), 4)
                self.assertTrue(any("readiness" in log for log in logs.values()))
                self.assertFalse(misplaced)

    def test_successful_executed_stages_cannot_hide_missing_required_smoke(self):
        result, report, logs, misplaced = self.run_sequence()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertEqual(report.count("status=PASS\n"), 4)
        self.assertIn("status=SKIPPED", report)
        self.assertIn("overall_status=BLOCKED", report)
        self.assertEqual(report.count("exit_code=0"), 4)
        self.assertEqual(len(logs), 4)
        self.assertFalse(misplaced)

    def test_repository_path_with_spaces(self):
        result, report, _, _ = self.run_sequence(spaced=True)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertEqual(report.count("status=PASS\n"), 4)

    def test_unwritable_report_location_fails(self):
        result, _, _, misplaced = self.run_sequence(broken_reports=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(misplaced)


if __name__ == "__main__":
    unittest.main()
