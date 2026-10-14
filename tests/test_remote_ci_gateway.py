"""Reject unsafe SSH gateway arguments before any remote connection."""
from pathlib import Path
import os
import subprocess
import sys
import tempfile
import time
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

    def test_dispatch_result_survives_controller_script_update(self):
        # A test-only SSH process holds the call open while its controller is
        # updated. Real SSH and production worker files are never modified.
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            controller = root / 'controller.sh'
            original = SCRIPT.read_text()
            controller.write_text(original)
            started, release = root / 'started', root / 'release'
            transport = root / 'ssh'
            transport.write_text(
                '#!' + sys.executable + '\n'
                'import os,time\nfrom pathlib import Path\n'
                'Path(os.environ["GATEWAY_STARTED"]).touch()\n'
                'while not Path(os.environ["GATEWAY_RELEASE"]).exists(): time.sleep(.01)\n'
            )
            transport.chmod(0o700)
            env = dict(os.environ, PATH=str(root) + os.pathsep + os.environ['PATH'],
                       GATEWAY_STARTED=str(started), GATEWAY_RELEASE=str(release))
            process = subprocess.Popen(
                ['bash', str(controller), 'user@host', 'a' * 40, 'workspace-check'],
                env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                deadline = time.monotonic() + 3
                while not started.exists() and time.monotonic() < deadline:
                    time.sleep(.01)
                self.assertTrue(started.exists(), 'transport must actually start')
                # At the old EOF there is now invalid shell input. A waiting
                # wrapper must not parse a subsequent revision of itself.
                controller.write_text('#' + ' ' * (len(original.encode()) - 1) + "\n'\n")
            finally:
                release.touch()
                _, stderr = process.communicate(timeout=3)
            self.assertEqual(process.returncode, 0, stderr)

    def test_worker_rejects_invalid_requests_before_mutation(self):
        result = subprocess.run(['bash', str(WORKER), 'HEAD', '--publish-status'],
                                capture_output=True, text=True, timeout=3)
        self.assertEqual(result.returncode, 2)
        self.assertIn('Invalid commit or gate list', result.stderr)


if __name__ == '__main__':
    unittest.main()
