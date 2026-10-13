"""Reject unsafe SSH gateway arguments before any remote connection."""
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / 'scripts/ci/run-remote-checks.sh'
WORKER = ROOT / 'scripts/ci/remote-worker.sh'


class RemoteGatewayArguments(unittest.TestCase):
    def invoke(self, args):
        return subprocess.run(['bash', str(SCRIPT), *args], capture_output=True,
                              text=True, timeout=3)

    def test_help_needs_no_worker(self):
        result = self.invoke(['--help'])
        self.assertEqual(result.returncode, 0)
        self.assertIn('Usage:', result.stdout)

    def test_missing_arguments_fail(self):
        result = self.invoke([])
        self.assertEqual(result.returncode, 2)
        self.assertIn('Expected host', result.stderr)

    def test_host_option_or_shell_injection_rejected(self):
        for host in ['-oProxyCommand=bad', 'user@host;id', 'user@$(id)', 'user@host bad']:
            result = self.invoke([host, 'a' * 40, 'workspace-check'])
            self.assertEqual(result.returncode, 2)
            self.assertIn('Invalid worker host', result.stderr)

    def test_short_or_shell_commit_rejected(self):
        for sha in ['a' * 12, 'a' * 40 + ';id', 'HEAD']:
            result = self.invoke(['user@host', sha, 'workspace-check'])
            self.assertEqual(result.returncode, 2)
            self.assertIn('Invalid full commit SHA', result.stderr)

    def test_gate_options_and_shell_injection_rejected(self):
        for gates in ['--publish-status', 'workspace-check,--publish-status', 'workspace-check;id', 'workspace-check --skip all', '']:
            result = self.invoke(['user@host', 'a' * 40, gates])
            self.assertEqual(result.returncode, 2)
            self.assertIn('Invalid gate list', result.stderr)

    def test_worker_rejects_invalid_requests_before_mutation(self):
        result = subprocess.run(['bash', str(WORKER), 'HEAD', '--publish-status'],
                                capture_output=True, text=True, timeout=3)
        self.assertEqual(result.returncode, 2)
        self.assertIn('Invalid commit or gate list', result.stderr)


if __name__ == '__main__':
    unittest.main()
