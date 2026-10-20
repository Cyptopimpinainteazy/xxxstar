"""Exercise the actual verification playbook against controlled loopback RPC servers.

These are verifier integration tests, not physical-host or chain execution proof.
"""
import json
import os
from pathlib import Path
import random
import shutil
import subprocess
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import unittest

ROOT = Path(__file__).resolve().parents[1]
GENESIS = "0x" + "a" * 64
CANONICAL = "0x" + "b" * 64


class VerifyPlaybookTests(unittest.TestCase):
    def test_missing_inventory_is_an_error(self):
        executable = shutil.which("ansible-playbook")
        self.assertIsNotNone(executable)
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.run([executable, "--syntax-check", "-i",
                str(Path(directory) / "absent.yml"),
                str(ROOT / "ansible/playbooks/10-preflight.yml")], cwd=ROOT,
                env={**os.environ, "ANSIBLE_CONFIG": str(ROOT / "ansible/ansible.cfg")},
                capture_output=True, text=True, timeout=30)
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_accepts_agreement_and_rejects_forks_or_missing_genesis(self):
        executable = shutil.which("ansible-playbook")
        self.assertIsNotNone(executable, "ansible-playbook is required for verifier integration")
        scenario = {"fork": False, "genesis": GENESIS}
        class RPC(BaseHTTPRequestHandler):
            def do_POST(self):
                request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                method, params = request["method"], request.get("params", [])
                if method == "chain_getFinalizedHead":
                    self.server.height += 1
                    value = "0x" + f"{self.server.height:064x}"
                elif method == "chain_getHeader":
                    value = {"number": hex(int(params[0], 16) if params else 100)}
                elif method == "system_health":
                    value = {"peers": 6, "isSyncing": False}
                elif method == "chain_getBlockHash":
                    value = scenario["genesis"] if params == [0] else (
                        "0x" + "c" * 64 if scenario["fork"] and self.server.index == 7 else CANONICAL)
                else:
                    self.send_error(400)
                    return
                body = json.dumps({"jsonrpc": "2.0", "id": 1, "result": value}).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            def log_message(self, *_args):
                pass
        servers = []
        for _ in range(100):
            base = random.randrange(20000, 50000)
            try:
                for index in range(1, 8):
                    server = ThreadingHTTPServer(("127.0.0.1", base + index), RPC)
                    server.index, server.height = index, 10
                    servers.append(server)
                break
            except OSError:
                for server in servers:
                    server.server_close()
                servers = []
        self.assertEqual(len(servers), 7, "could not reserve seven adjacent RPC ports")
        workers = [threading.Thread(target=s.serve_forever, daemon=True) for s in servers]
        for worker in workers:
            worker.start()
        try:
            with tempfile.TemporaryDirectory() as directory:
                directory = Path(directory)
                inventory = {"all": {"children": {"validators": {"hosts": {
                    f"validator{i}": {"ansible_connection": "local", "ansible_host": "127.0.0.1",
                                      "ansible_python_interpreter": shutil.which("python3"),
                                      "x3_validators": [{"index": i}]}
                    for i in range(1, 8)}}}}}
                inv = directory / "inventory.json"
                inv.write_text(json.dumps(inventory))
                variables = directory / "vars.json"
                variables.write_text(json.dumps({"x3_rpc_base": base, "x3_p2p_base": base,
                    "x3_min_peers": 4, "x3_verify_timeout_sec": 6, "x3_validator_count": 7}))
                for fork, genesis, expected in ((False, GENESIS, 0), (True, GENESIS, 2), (False, None, 2)):
                    with self.subTest(fork=fork, genesis=genesis):
                        scenario.update(fork=fork, genesis=genesis)
                        result = subprocess.run([executable, "-i", str(inv), "-f", "7",
                            "-e", "@" + str(variables), str(ROOT / "ansible/playbooks/30-verify.yml")],
                            cwd=ROOT, env={**os.environ, "ANSIBLE_CONFIG": str(ROOT / "ansible/ansible.cfg")},
                            capture_output=True, text=True, timeout=180)
                        self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
                        if expected == 0:
                            self.assertIn("all 7 validators agree", result.stdout)
        finally:
            for server in servers:
                server.shutdown()
                server.server_close()
            for worker in workers:
                worker.join(timeout=2)


if __name__ == "__main__":
    unittest.main()
