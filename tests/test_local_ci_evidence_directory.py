"""Exercise the real gate runner with a run-specific evidence directory."""
import json
import os
from pathlib import Path
import subprocess
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


if __name__ == "__main__":
    unittest.main()
