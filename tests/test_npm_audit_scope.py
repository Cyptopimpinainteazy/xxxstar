"""Security gate regressions for advisories outside shipped dependencies."""

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import unittest
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "check-npm-audit.py"
SPEC = importlib.util.spec_from_file_location("npm_audit_gate", SCRIPT)
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


class AuditScopeTests(unittest.TestCase):
    def assert_high_advisory_rejected(self, project, manager):
        def audit_process(command, **kwargs):
            # A real audit may be clean only when development dependencies are omitted.
            omitted = "--omit=dev" in command or "--prod" in command
            findings = {} if omitted else {"braces": {"severity": "high"}}
            return subprocess.CompletedProcess(
                command, 0 if omitted else 1,
                stdout=json.dumps({"vulnerabilities": findings}), stderr="",
            )

        errors = io.StringIO()
        with (
            mock.patch.object(GATE, "projects", return_value=[(project, manager)]),
            mock.patch.object(GATE, "load_baseline", return_value={"allowed": {}}),
            mock.patch.object(GATE.subprocess, "run", side_effect=audit_process),
            mock.patch("sys.argv", [str(SCRIPT)]),
            contextlib.redirect_stdout(io.StringIO()),
            contextlib.redirect_stderr(errors),
        ):
            result = GATE.main()
        self.assertEqual(result, 1, errors.getvalue())
        self.assertIn("braces is high", errors.getvalue())

    def test_development_advisories_fail_for_both_package_managers(self):
        for manager in ("npm", "pnpm"):
            with self.subTest(manager=manager):
                self.assert_high_advisory_rejected("apps/explorer", manager)

    def test_test_harness_advisories_fail(self):
        for directory in ("tests", "tests_core"):
            with self.subTest(directory=directory):
                self.assert_high_advisory_rejected(f"{directory}/security-harness", "npm")


if __name__ == "__main__":
    unittest.main()
