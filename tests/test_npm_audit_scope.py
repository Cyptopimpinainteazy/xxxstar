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
            report = {"vulnerabilities": findings}
            if manager == "pnpm":
                report = {"advisories": {} if omitted else {
                    "123": {"module_name": "braces", "severity": "high"},
                }}
            return subprocess.CompletedProcess(
                command, 0 if omitted else 1,
                stdout=json.dumps(report), stderr="",
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

    def test_invalid_audit_reports_fail_closed(self):
        reports = [
            {"error": {"code": "ENOAUDIT"}},
            {},
            [],
            {"vulnerabilities": []},
            {"vulnerabilities": {"braces": {"severity": "unknown"}}},
        ]
        for report in reports:
            with self.subTest(report=report), mock.patch.object(
                GATE.subprocess, "run", return_value=subprocess.CompletedProcess(
                    ["npm", "audit"], 1, stdout=json.dumps(report), stderr="",
                ),
            ):
                findings, error = GATE.audit("apps/explorer", "npm")
                self.assertIsNone(findings)
                self.assertTrue(error)

    def test_unexpected_tool_exit_fails_closed(self):
        with mock.patch.object(
            GATE.subprocess, "run", return_value=subprocess.CompletedProcess(
                ["npm", "audit"], 2,
                stdout=json.dumps({"vulnerabilities": {}}), stderr="",
            ),
        ):
            findings, error = GATE.audit("apps/explorer", "npm")
        self.assertIsNone(findings)
        self.assertTrue(error)

    def test_multiple_pnpm_advisories_preserve_highest_severity(self):
        report = {"advisories": {
            "1": {"module_name": "shell-quote", "severity": "critical"},
            "2": {"module_name": "shell-quote", "severity": "low"},
        }}
        with mock.patch.object(
            GATE.subprocess, "run", return_value=subprocess.CompletedProcess(
                ["pnpm", "audit"], 1, stdout=json.dumps(report), stderr="",
            ),
        ):
            findings, error = GATE.audit("apps/x3-studio", "pnpm")
        self.assertIsNone(error)
        self.assertEqual(findings, {"shell-quote": "critical"})


if __name__ == "__main__":
    unittest.main()
