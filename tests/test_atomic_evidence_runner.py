"""Test evidence orchestration only; Cargo output here is a test fixture."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]


class AtomicEvidenceRunnerTests(unittest.TestCase):
    def run_runner(self, output, code=0):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            cargo = base / "cargo"
            cargo.write_text(
                "#!/usr/bin/env python3\n"
                "import os, sys\n"
                "if sys.argv[1:] == ['--version']:\n"
                "    print('cargo orchestration test fixture')\n"
                "    sys.exit(0)\n"
                "print(os.environ['CARGO_TEST_OUTPUT'])\n"
                "sys.exit(int(os.environ['CARGO_TEST_EXIT']))\n"
            )
            cargo.chmod(0o755)
            evidence = base / "evidence"
            env = dict(os.environ, PATH=f"{base}:{os.environ['PATH']}",
                       CARGO_TEST_OUTPUT=output, CARGO_TEST_EXIT=str(code),
                       X3_ATOMIC_EVIDENCE_DIR=str(evidence))
            result = subprocess.run(
                ["bash", "scripts/testnet/atomic-trade-evidence.sh"],
                cwd=ROOT, env=env, text=True, capture_output=True,
            )
            files = {p.name: p.read_text() for p in evidence.iterdir() if p.is_file()}
            return result, files

    def test_zero_selected_tests_is_failure(self):
        result, files = self.run_runner(
            "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out"
        )
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("vm-trading", files["status.txt"])

    def test_ignored_required_tests_is_failure(self):
        result, _ = self.run_runner(
            "test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out"
        )
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_missing_summary_is_failure(self):
        result, _ = self.run_runner("build finished without running a test binary")
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_command_failure_preserves_exit_code_and_continues(self):
        result, files = self.run_runner("compiler failure", code=17)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("vm-all 17", files["status.txt"])
        self.assertIn("atomic-engine-tests 17", files["status.txt"])

    def test_success_records_reproducible_metadata(self):
        result, files = self.run_runner(
            "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("cargo test -p pallet-atomic-trade-engine --locked", files["commands.txt"])
        self.assertIn("rustc", files["toolchain.txt"])
        self.assertEqual(len(files["head.txt"].strip()), 40)

    def test_empty_doctest_binary_does_not_hide_executed_tests(self):
        result, _ = self.run_runner(
            "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n"
            "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
        )
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
