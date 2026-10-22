"""Exercise the real gate runner with a run-specific evidence directory."""
import json
import os
import re
import shlex
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]


class EvidenceDirectory(unittest.TestCase):
    def test_scoped_gate_writes_summary_and_logs_to_requested_directory(self):
        with tempfile.TemporaryDirectory(prefix="x3 CI evidence ") as directory:
            result = subprocess.run(
                ["bash", "scripts/local-ci.sh", "--only", "economic-model",
                 "--jobs", "1", "--cargo-jobs", "1"],
                cwd=ROOT, env={**os.environ, "X3_LOCAL_CI_LOG_DIR": directory},
                capture_output=True, text=True, timeout=90,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            summaries = list(Path(directory).glob("local-ci-*-summary.json"))
            self.assertEqual(len(summaries), 1, result.stdout + result.stderr)
            summary = json.loads(summaries[0].read_text())
            self.assertEqual(summary["result"], "PASS")
            self.assertEqual(len(summary["gates"]), 1)
            gate = summary["gates"][0]
            self.assertEqual(gate["name"], "economic model")
            self.assertEqual(gate["result"], "PASS")
            self.assertTrue((Path(directory) / gate["log"]).is_file())

    def test_failed_gate_diagnostics_survive_without_artifacts(self):
        with tempfile.TemporaryDirectory(prefix="x3-failed-evidence-") as directory:
            tools = Path(directory) / "bin"
            tools.mkdir()
            wrapper = tools / "python3"
            wrapper.write_text(
                "#!/bin/sh\nif [ \"$1\" = -m ] && [ \"$2\" = pytest ]; then\n"
                "echo X3_DIAGNOSTIC_SENTINEL >&2\nexit 42\nfi\n"
                + "exec " + shlex.quote(sys.executable) + ' "$@"\n'
            )
            wrapper.chmod(0o755)
            pytest_wrapper = tools / "pytest"
            pytest_wrapper.write_text("#!/bin/sh\necho X3_DIAGNOSTIC_SENTINEL >&2\nexit 42\n")
            pytest_wrapper.chmod(0o755)
            result = subprocess.run(
                ["bash", "scripts/local-ci.sh", "--only", "guard-pattern-regressions",
                 "--jobs", "1", "--cargo-jobs", "1"],
                cwd=ROOT, env={**os.environ, "X3_LOCAL_CI_LOG_DIR": directory,
                               "PATH": str(tools) + os.pathsep + os.environ["PATH"]},
                capture_output=True, text=True, timeout=90,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("X3_DIAGNOSTIC_SENTINEL", result.stdout + result.stderr)


    def test_guard_gate_uses_installed_pytest_without_python_module(self):
        with tempfile.TemporaryDirectory(prefix="x3-pytest-entrypoint-") as directory:
            tools = Path(directory) / "bin"
            tools.mkdir()
            wrapper = tools / "python3"
            wrapper.write_text(
                "#!/bin/sh\nif [ \"$1\" = -m ] && [ \"$2\" = pytest ]; then\n"
                "echo 'No module named pytest' >&2\nexit 1\nfi\n"
                + "exec " + shlex.quote(sys.executable) + ' "$@"\n'
            )
            wrapper.chmod(0o755)
            pytest_wrapper = tools / "pytest"
            pytest_wrapper.write_text("#!/bin/sh\nexec " + shlex.quote(sys.executable)
                                      + ' -m pytest "$@"\n')
            pytest_wrapper.chmod(0o755)
            result = subprocess.run(
                ["bash", "scripts/local-ci.sh", "--only", "guard-pattern-regressions",
                 "--jobs", "1", "--cargo-jobs", "1"],
                cwd=ROOT, env={**os.environ, "X3_LOCAL_CI_LOG_DIR": directory,
                               "PATH": str(tools) + os.pathsep + os.environ["PATH"]},
                capture_output=True, text=True, timeout=90,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


    def test_nested_gates_keep_builds_within_each_callers_target(self):
        with tempfile.TemporaryDirectory(prefix="x3-nested-isolation-") as directory:
            work = Path(directory)
            tools = work / "bin"
            tools.mkdir()
            cargo = tools / "cargo"
            cargo.write_text('#!/bin/sh\nprintf \"%s\\n\" "$CARGO_TARGET_DIR" >> "$X3_TEST_TARGET_LOG"\n')
            cargo.chmod(0o755)
            for caller in ("first target", "second target"):
                target = work / caller
                capture = work / (caller + ".log")
                environment = {**os.environ, "X3_LOCAL_CI_LOG_DIR": str(work / (caller + " evidence")),
                    "CARGO_TARGET_DIR": str(target), "X3_TEST_TARGET_LOG": str(capture),
                    "PATH": str(tools) + os.pathsep + os.environ["PATH"]}
                result = subprocess.run(
                    ["bash", "scripts/local-ci.sh", "--only",
                     "test-svm-counter,test-x3-sidecar,test-x3-svm-client", "--dry-run"],
                    cwd=ROOT, env=environment, capture_output=True, text=True, timeout=30,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                commands = [match.group(1) for line in result.stdout.splitlines()
                    if (match := re.match(r"^would run .{34} (.*CARGO_TARGET_DIR=.*)$", line))]
                self.assertEqual(len(commands), 3, result.stdout)
                for command in commands:
                    subprocess.run(["bash", "-c", command], cwd=ROOT, env=environment,
                                   check=True, capture_output=True, text=True, timeout=10)
                observed = [Path(line) for line in capture.read_text().splitlines()]
                self.assertEqual(len(observed), 3)
                for path in observed:
                    self.assertTrue(path.is_relative_to(target), f"{caller}: unexpected shared target {path}")


if __name__ == "__main__":
    unittest.main()
