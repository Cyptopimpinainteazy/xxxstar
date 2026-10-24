#!/usr/bin/env python3
"""Exercise the launcher's shell orchestration without starting chain processes.

Only the test harness replaces start_node/curl; these tests prove launch selection,
not block production, connectivity, or finality.
"""
import os
from pathlib import Path
import subprocess
import sys
import socket
import tempfile
import threading
import time
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/testnet/run-7-validators-local.sh"


class ValidatorLauncherTests(unittest.TestCase):
    def launch_selection(self, index, multi_host=False):
        source = SCRIPT.read_text()
        marker = "# X3_LAUNCH_SELECTION_BEGIN\n"
        self.assertEqual(source.count(marker), 1, "launcher selection marker must occur exactly once")
        footer = source.split(marker)[1]
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "starts"
            harness = '''set -euo pipefail
start_node() { echo "$1" >> "$START_LOG"; }
curl() {
  if [[ "$SKIP_BOOTNODE_MEMBERSHIP_CHECK" == "1" ]]; then
    echo 'Multi-host launch must not query a localhost bootnode' >&2
    return 99
  fi
  echo '{"result":"test-peer"}'
}
COUNT=7
RPC_BASE=9944
P2P_BASE=30333
LISTEN_IP=127.0.0.1
LOG_DIR=/unused
PID_DIR=/unused
'''
            result = subprocess.run(["bash", "-c", harness + footer],
                                    env={**os.environ, "ONLY_INDEX": str(index), "START_LOG": str(log),
                                         "SKIP_BOOTNODE_MEMBERSHIP_CHECK": "1" if multi_host else "0"},
                                    text=True, capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            return log.read_text().splitlines()

    def test_full_launch_starts_each_validator_once(self):
        self.assertEqual(self.launch_selection(0), [str(i) for i in range(1, 8)])

    def test_restart_other_validator_does_not_launch_bootnode(self):
        self.assertEqual(self.launch_selection(3), ["3"])

    def test_restart_bootnode_starts_it_only_once(self):
        self.assertEqual(self.launch_selection(1), ["1"])

    def test_multi_host_restart_uses_spec_without_local_bootnode(self):
        self.assertEqual(self.launch_selection(3, multi_host=True), ["3"])

    def test_restart_with_wipe_is_rejected_before_touching_data(self):
        with tempfile.TemporaryDirectory() as directory:
            sentinel = Path(directory) / "preserve"
            sentinel.write_text("existing state")
            result = subprocess.run(["bash", str(SCRIPT), "--only", "3", "--wipe",
                                     "--base-dir", directory], env={**os.environ, "COUNT": "7"},
                                    text=True, capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
            self.assertIn("cannot be combined", result.stderr)
            self.assertEqual(sentinel.read_text(), "existing state")

    def test_invalid_restart_index_is_rejected_before_creating_directories(self):
        for index in ("0", "8", "-1", "abc", "08"):
            with self.subTest(index=index), tempfile.TemporaryDirectory() as directory:
                base = Path(directory) / "untouched"
                result = subprocess.run(["bash", str(SCRIPT), "--only", index,
                                         "--base-dir", str(base)],
                                        env={**os.environ, "COUNT": "7"},
                                        text=True, capture_output=True, timeout=5)
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertFalse(base.exists())

    def test_explicit_zero_with_wipe_preserves_peer_data(self):
        with tempfile.TemporaryDirectory() as directory:
            sentinel = Path(directory) / "preserve"
            sentinel.write_text("peer state")
            result = subprocess.run(["bash", str(SCRIPT), "--only", "0", "--wipe",
                                     "--base-dir", directory],
                                    env={**os.environ, "COUNT": "7", "LOG_DIR": directory},
                                    text=True, capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
            self.assertEqual(sentinel.read_text(), "peer state")

    def test_restart_refuses_live_pid_without_changing_pid_file(self):
        with tempfile.TemporaryDirectory() as directory:
            pid_file = Path(directory) / "node-3.pid"
            process = subprocess.Popen(['x3-chain-node', '-c',
                'import time; print("ready", flush=True); time.sleep(30)',
                '--base-path', str(Path(directory) / 'node-3')], executable=sys.executable,
                stdout=subprocess.PIPE, text=True)
            try:
                self.assertEqual(process.stdout.readline().strip(), 'ready')
                pid_file.write_text(str(process.pid))
                result = subprocess.run(["bash", str(SCRIPT), "--only", "3", "--base-dir", directory],
                                        env={**os.environ, "COUNT": "7", "PID_DIR": directory},
                                        text=True, capture_output=True, timeout=5)
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertIn("still running", result.stderr)
                self.assertEqual(pid_file.read_text(), str(process.pid))
            finally:
                process.kill()
                process.wait(timeout=5)
                process.stdout.close()

    def test_reused_pid_does_not_block_restart(self):
        with tempfile.TemporaryDirectory() as directory, socket.socket() as port:
            port.bind(('127.0.0.1', 0))
            rpc_base = port.getsockname()[1]
            port.close()
            pid_file = Path(directory) / 'node-1.pid'
            pid_file.write_text(str(os.getpid()))
            result = subprocess.run(['bash', str(SCRIPT), '--only', '1', '--base-dir', directory,
                                     '--node-bin', str(Path(directory) / 'missing-node')],
                                    env={**os.environ, 'COUNT': '7', 'PID_DIR': directory,
                                         'RPC_BASE': str(rpc_base)}, capture_output=True, text=True, timeout=5)
            self.assertNotIn('still running', result.stderr)
            self.assertIn('binary', result.stdout + result.stderr)
            self.assertEqual(pid_file.read_text(), str(os.getpid()))

    def test_invalid_timeout_is_rejected_before_creating_directories(self):
        for value in ('0', '-1', 'abc'):
            with self.subTest(value=value), tempfile.TemporaryDirectory() as directory:
                base = Path(directory) / 'untouched'
                result = subprocess.run(['bash', str(SCRIPT), '--base-dir', str(base)],
                    env={**os.environ, 'COUNT': '7', 'RPC_READY_TIMEOUT_SECONDS': value},
                    capture_output=True, text=True, timeout=5)
                self.assertEqual(result.returncode, 2)
                self.assertIn('positive integer', result.stderr)
                self.assertFalse(base.exists())

    def test_restart_refuses_existing_rpc_listener(self):
        with tempfile.TemporaryDirectory() as directory, socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            listener.listen()
            result = subprocess.run(["bash", str(SCRIPT), "--only", "1", "--base-dir", directory],
                                    env={**os.environ, "COUNT": "7", "PID_DIR": directory,
                                         "RPC_BASE": str(listener.getsockname()[1])},
                                    text=True, capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 2)
            self.assertIn("RPC port", result.stderr)
            self.assertFalse((Path(directory) / "node-1.pid").exists())

    def rpc_function(self):
        source = SCRIPT.read_text()
        start, end = "wait_for_rpc() {\n", "\nstart_node() {\n"
        self.assertEqual(source.count(start), 1)
        self.assertEqual(source.count(end), 1)
        return start + source.split(start)[1].split(end)[0]

    def test_stalled_http_response_obeys_overall_deadline(self):
        release = threading.Event()

        class Stalled(BaseHTTPRequestHandler):
            def do_POST(self):
                release.wait(5)

            def log_message(self, *_):
                pass

        server = ThreadingHTTPServer(("127.0.0.1", 0), Stalled)
        server.daemon_threads = True
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        try:
            started = time.monotonic()
            result = subprocess.run(["bash", "-c", "set -euo pipefail\n" + self.rpc_function()
                                     + f"\nwait_for_rpc {server.server_port}"],
                                    env={**os.environ, "RPC_READY_TIMEOUT_SECONDS": "2"},
                                    text=True, capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 1)
            self.assertLess(time.monotonic() - started, 4)
        finally:
            release.set()
            server.shutdown()
            server.server_close()
            worker.join(timeout=2)

    def test_dead_node_cannot_be_marked_ready_by_another_rpc_process(self):
        child = subprocess.Popen(["true"])
        child.wait(timeout=2)
        result = subprocess.run(["bash", "-c", "set -euo pipefail\n" + self.rpc_function()
                                 + '\ncurl() { echo \'{"result":{"isSyncing":false}}\'; }\n'
                                 + f'wait_for_rpc 1234 "" {child.pid}'],
                                text=True, capture_output=True, timeout=5)
        self.assertEqual(result.returncode, 1)
        self.assertIn("exited", result.stderr)


if __name__ == "__main__":
    unittest.main()
