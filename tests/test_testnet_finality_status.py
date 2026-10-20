"""Component tests for the actual shell monitor using test-only RPC fixtures.

These check monitor rejection semantics, not live consensus readiness.
"""
import contextlib
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / 'scripts/testnet/status-7-validators.sh'


@contextlib.contextmanager
def rpc_nodes(chains, pretty=False, stagnant=False, best=21, rpc_error=False, same_genesis=False):
    servers = []
    for _ in range(100):
        first = ThreadingHTTPServer(('127.0.0.1', 0), BaseHTTPRequestHandler)
        port = first.server_port
        first.server_close()
        try:
            for i in range(len(chains)):
                servers.append(ThreadingHTTPServer(('127.0.0.1', port + i), BaseHTTPRequestHandler))
            break
        except OSError:
            for server in servers:
                server.server_close()
            servers = []
    if not servers:
        raise RuntimeError('unable to reserve contiguous fixture RPC ports')
    threads = []
    for server, chain in zip(servers, chains):
        def handler_factory(chain):
            class Handler(BaseHTTPRequestHandler):
                final_calls = 0

                def log_message(self, *args):
                    pass

                def do_POST(self):
                    request = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                    method, params = request['method'], request['params']
                    if method == 'system_health':
                        result = {'peers': 6, 'isSyncing': False}
                    elif method == 'chain_getFinalizedHead':
                        type(self).final_calls += 1
                        n = 10 if stagnant or type(self).final_calls == 1 else 20
                        result = '0x' + chain * 2 + f'{n:062x}'
                    elif method == 'chain_getHeader':
                        n = int(params[0][4:], 16) if params else best
                        result = {'number': hex(n)}
                    elif method == 'chain_getBlockHash':
                        n = params[0] if params else best
                        prefix = 'c' if same_genesis and n == 0 else chain
                        result = '0x' + prefix * 2 + f'{n:062x}'
                    else:
                        raise AssertionError(method)
                    response = {'jsonrpc': '2.0', 'id': request['id'], 'result': result}
                    if rpc_error:
                        response = {'jsonrpc': '2.0', 'id': request['id'],
                                    'error': {'code': -32603, 'message': 'fixture outage'}}
                    data = json.dumps(response, indent=2 if pretty else None,
                                      separators=None if pretty else (',', ':')).encode()
                    self.send_response(200)
                    self.send_header('Content-Type', 'application/json')
                    self.send_header('Content-Length', str(len(data)))
                    self.end_headers()
                    self.wfile.write(data)
            return Handler
        server.RequestHandlerClass = handler_factory(chain)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        threads.append(thread)
    try:
        yield port
    finally:
        for server in servers:
            server.shutdown()
            server.server_close()
        for thread in threads:
            thread.join(timeout=2)


class FinalityStatusTests(unittest.TestCase):
    def monitor(self, chains=('a',), **kwargs):
        with rpc_nodes(chains, **kwargs) as port:
            env = dict(os.environ, BASE_RPC_PORT=str(port), COUNT=str(len(chains)),
                       FINALITY_WINDOW_SEC='0', MAX_FINALITY_LAG_MULT='2')
            return subprocess.run(['bash', str(SCRIPT), '--check-finality'],
                                  env=env, capture_output=True, text=True, timeout=15)

    def test_status_and_both_checks_work_without_python_alias(self):
        with rpc_nodes(('a',)) as port, tempfile.TemporaryDirectory() as directory:
            for tool in ('bash', 'curl', 'seq', 'date', 'sleep'):
                executable = shutil.which(tool)
                self.assertIsNotNone(executable)
                (Path(directory) / tool).symlink_to(executable)
            (Path(directory) / 'python3').symlink_to(sys.executable)
            result = subprocess.run([str(Path(directory) / 'bash'), str(SCRIPT),
                '--check-peers', '--timeout-sec', '1', '--check-finality', '--finality-window-sec', '0'],
                env={**os.environ, 'PATH': directory, 'COUNT': '1',
                     'BASE_RPC_PORT': str(port), 'NO_PROXY': '127.0.0.1'},
                capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn('Peer check OK', result.stdout)
            self.assertIn('Finality check OK', result.stdout)

    def test_compact_json_progress_passes(self):
        result = self.monitor()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_pretty_json_progress_passes(self):
        result = self.monitor(pretty=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_divergent_chains_rejected(self):
        result = self.monitor(chains=('a', 'b'))
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_fork_with_same_genesis_rejected(self):
        result = self.monitor(chains=('a', 'b'), same_genesis=True)
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_stagnant_finality_rejected(self):
        self.assertNotEqual(self.monitor(stagnant=True).returncode, 0)

    def test_finalized_ahead_of_best_rejected(self):
        self.assertNotEqual(self.monitor(best=19).returncode, 0)

    def test_rpc_error_rejected(self):
        self.assertNotEqual(self.monitor(rpc_error=True).returncode, 0)

    def test_same_chain_multiple_nodes_pass(self):
        result = self.monitor(chains=('a', 'a'))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
