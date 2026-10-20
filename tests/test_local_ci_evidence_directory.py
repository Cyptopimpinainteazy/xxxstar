"""Exercise the real gate runner with a run-specific evidence directory."""
import json
import os
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
            result = subprocess.run(
                ["bash", "scripts/local-ci.sh", "--only", "guard-pattern-regressions",
                 "--jobs", "1", "--cargo-jobs", "1"],
                cwd=ROOT, env={**os.environ, "X3_LOCAL_CI_LOG_DIR": directory,
                               "PATH": str(tools) + os.pathsep + os.environ["PATH"]},
                capture_output=True, text=True, timeout=90,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("X3_DIAGNOSTIC_SENTINEL", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
