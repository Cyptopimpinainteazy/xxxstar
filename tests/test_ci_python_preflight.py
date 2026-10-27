"""Prove X3's CI test interpreter is an isolated, provisioned venv.

This suite is intentionally small; it must pass before the very expensive
Local CI gate starts, not replace any of the required release tests.
"""
from pathlib import Path
import os
import shutil
import subprocess
import sys
import unittest


class PythonPreflightTests(unittest.TestCase):
    def test_venv_python_not_system_interpreter(self):
        self.assertNotEqual(sys.prefix, sys.base_prefix)
        venv = Path(sys.prefix).resolve()
        runner_temp = os.environ.get("RUNNER_TEMP")
        self.assertTrue(runner_temp, "RUNNER_TEMP must be defined on Actions")
        self.assertTrue(
            venv.is_relative_to(Path(runner_temp).resolve()),
            "CI must not install packages in the shared runner Python",
        )

    def test_test_suite_runner_is_installed_inside_venv(self):
        pytest = shutil.which("pytest")
        self.assertIsNotNone(pytest)
        self.assertTrue(Path(pytest).resolve().is_relative_to(Path(sys.prefix).resolve()))
        result = subprocess.run(
            [sys.executable, "-m", "pytest", "--version"],
            check=False,
            capture_output=True,
            text=True,
            timeout=15,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_yaml_parser_is_available_and_parses_workflow(self):
        import yaml
        workflow = Path(__file__).resolve().parents[1] / ".github" / "workflows" / "local-ci.yml"
        data = yaml.safe_load(workflow.read_text(encoding="utf-8"))
        self.assertIsInstance(data, dict)
        self.assertIn("jobs", data)
        self.assertIn("local-ci", data["jobs"])


if __name__ == "__main__":
    unittest.main()
