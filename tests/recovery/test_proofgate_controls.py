from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]


class ProofGateControlTests(unittest.TestCase):
    def test_runner_is_portable_and_hard_fails(self):
        runner = (ROOT / "launch-gates/run-proof-commands.sh").read_text()
        self.assertIn('REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)', runner)
        self.assertNotIn('/home/lojak/Desktop/X3_ATOMIC_STAR', runner)
        self.assertIn('exit 1', runner)
        self.assertIn('cargo check -p x3-chain-runtime', runner)
        self.assertIn('cargo run --release -p x3-chain-node -- build-spec --chain production', runner)

    def test_deployment_workflow_preserves_gate_exit_codes(self):
        workflow = (ROOT / ".github/workflows/proof-gates.yml").read_text()
        self.assertIn("EXIT_CODE=${PIPESTATUS[0]}", workflow)
        self.assertIn("exit $EXIT_CODE", workflow)
        self.assertIn("s1-security-gate:", workflow)
        self.assertNotIn("continue-on-error: true", workflow)

    def test_deployment_verifier_does_not_abort_on_first_counter_increment(self):
        verifier = (ROOT / "scripts/verify-deployment.sh").read_text()
        self.assertNotIn("CHECKS_PASSED++", verifier)
        self.assertNotIn("CHECKS_WARNINGS++", verifier)
        self.assertNotIn("CHECKS_FAILED++", verifier)
        self.assertIn("print_summary", verifier)


if __name__ == "__main__":
    unittest.main()
