#!/usr/bin/env python3
"""Tests for the physical preflight gate; no node or SSH daemon is required."""
from __future__ import annotations
import importlib.util
import json
import pathlib
import subprocess
import sys
import tempfile
import time
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCRIPT = ROOT / 'scripts/testnet/physical-seven-preflight.py'
spec = importlib.util.spec_from_file_location('physical_seven_preflight', SCRIPT)
pre = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = pre
spec.loader.exec_module(pre)

H1 = '0x' + 'a' * 64
H2 = '0x' + 'b' * 64
H3 = '0x' + 'c' * 64
F1 = 'f' * 64
F2 = 'e' * 64


def inventory():
    return {'validators': [{
        'name': f'x3star{i}', 'ssh_host': f'lojak@192.168.0.{65+i}',
        'rpc_port': 9944, 'binary_path': '/opt/x3/x3-chain-node',
        'chain_spec_path': '/etc/x3/testnet-raw.json',
        'data_dir': '/var/lib/x3/validator',
    } for i in range(1, 8)]}


def evidence():
    now = time.time()
    return {f'x3star{i}': {
        'machine_id_hash': f'{i:064x}', 'binary_sha256': F1,
        'spec_sha256': F2, 'genesis_hash': H1, 'system_chain': 'X3 Testnet',
        'local_peer_id': f'12D3Peer{i}', 'runtime': {
            'specName': 'x3', 'specVersion': 42, 'transactionVersion': 2,
            'stateVersion': 1},
        'best_height': 151, 'finalized_height': 148 + i % 2,
        'finalized_hash': H3, 'peers': 5, 'is_syncing': False,
        'free_bytes': 30 * 1024 ** 3, 'remote_time': now,
        'local_received_at': now,
    } for i in range(1, 8)}


def assess(snap=None, hashes=None, **kwargs):
    nodes = pre.validate_inventory(inventory())
    return pre.assess(nodes, snap or evidence(), hashes or {f'x3star{i}': H3 for i in range(1,8)}, **kwargs)


class InventoryContract(unittest.TestCase):
    def test_exactly_seven_unique_machine_targets(self):
        nodes = pre.validate_inventory(inventory())
        self.assertEqual(len(nodes), 7)
        self.assertEqual(nodes[0].rpc_port, 9944)

    def test_rejects_under_or_over_count(self):
        for count in (0, 6, 8):
            with self.subTest(count=count), self.assertRaises(pre.PreflightError):
                pre.validate_inventory({'validators': inventory()['validators'][:count] if count < 7 else inventory()['validators'] + [inventory()['validators'][0]]})

    def test_rejects_duplicate_names_or_ssh_hosts(self):
        for field in ('name', 'ssh_host'):
            data = inventory()
            data['validators'][-1][field] = data['validators'][0][field]
            with self.subTest(field=field), self.assertRaises(pre.PreflightError):
                pre.validate_inventory(data)

    def test_rejects_injection_and_untrustworthy_paths(self):
        for field, value in (('ssh_host', '-oProxyCommand=evil'), ('ssh_host', 'lojak@host;id'), ('ssh_host', '127.0.0.1'),
                             ('binary_path', '../node'), ('data_dir', '/tmp/d\ninjection'),
                             ('rpc_port', 70000), ('rpc_port', True)):
            data = inventory()
            data['validators'][0][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(pre.PreflightError):
                pre.validate_inventory(data)

    def test_rejects_loopback_physical_targets(self):
        data = inventory()
        data['validators'][0]['ssh_host'] = 'localhost'
        with self.assertRaises(pre.PreflightError):
            pre.validate_inventory(data)


class ObservationContract(unittest.TestCase):
    def test_all_seven_honest_observations_pass(self):
        self.assertEqual(assess(), [])

    def test_missing_remote_evidence_fails_closed(self):
        s = evidence()
        del s['x3star2']
        self.assertTrue(assess(snap=s))

    def test_cloned_machine_identity_is_rejected(self):
        s = evidence()
        s['x3star7']['machine_id_hash'] = s['x3star1']['machine_id_hash']
        self.assertTrue(any('machine' in e.lower() for e in assess(snap=s)))

    def test_binary_chain_spec_and_genesis_divergence_rejected(self):
        for field, value in (('binary_sha256', '0' * 64), ('spec_sha256', '0' * 64),
                             ('genesis_hash', '0x' + '0' * 64), ('system_chain', 'EvilNet')):
            s = evidence()
            s['x3star3'][field] = value
            with self.subTest(field=field):
                self.assertTrue(assess(snap=s), field)

    def test_runtime_mismatch_rejected(self):
        s = evidence()
        s['x3star4']['runtime']['specVersion'] = 99
        self.assertTrue(assess(snap=s))

    def test_bad_rpc_health_and_finality_rejected(self):
        cases = [('is_syncing', True), ('peers', 0), ('best_height', 1),
                 ('finalized_height', 0), ('finalized_hash', None),
                 ('local_peer_id', '')]
        for field, value in cases:
            s = evidence()
            s['x3star4'][field] = value
            with self.subTest(field=field):
                self.assertTrue(assess(snap=s), field)

    def test_fork_at_shared_finalized_height_rejected(self):
        hashes = {f'x3star{i}': H3 for i in range(1, 8)}
        hashes['x3star3'] = H2
        self.assertTrue(any('canonical' in e.lower() for e in assess(hashes=hashes)))

    def test_shared_height_canonical_must_equal_the_finalized_head(self):
        s = evidence()
        # x3star2 is at 148 and claims to have finalized H2, even though the
        # canonical height-148 hash returned by that same RPC is H3.
        s['x3star2']['finalized_hash'] = H2
        problems = assess(snap=s)
        self.assertTrue(any('finalized head' in p.lower() for p in problems), problems)

    def test_missing_canonical_response_rejected(self):
        hashes = {f'x3star{i}': H3 for i in range(1, 7)}
        self.assertTrue(assess(hashes=hashes))

    def test_duplicate_libp2p_identity_rejected(self):
        s = evidence()
        s['x3star7']['local_peer_id'] = s['x3star1']['local_peer_id']
        self.assertTrue(assess(snap=s))

    def test_low_disk_and_clock_skew_rejected(self):
        for field, value in (('free_bytes', 1), ('remote_time', time.time() - 100)):
            s = evidence()
            s['x3star1'][field] = value
            with self.subTest(field=field):
                self.assertTrue(assess(snap=s))

    def test_explicit_expected_artifact_hashes_are_enforced(self):
        self.assertEqual(assess(expected_genesis=H1, expected_binary_sha256=F1,
                                expected_spec_sha256=F2), [])
        self.assertTrue(assess(expected_binary_sha256='0'*64))

    def test_malformed_evidence_cannot_crash_to_a_pass(self):
        for field, value in (('peers', '5'), ('best_height', True), ('runtime', {}),
                             ('machine_id_hash', ''), ('remote_time', 'nope')):
            s = evidence()
            s['x3star2'][field] = value
            with self.subTest(field=field):
                self.assertTrue(assess(snap=s))


class RemoteProbeIntegration(unittest.TestCase):
    def test_actual_remote_python_uses_loopback_rpc_and_file_hashes(self):
        answers = {
            'chain_getFinalizedHead': H2,
            'chain_getBlockHash': H1,
            'chain_getHeader': {'number': '0x96'},
            'system_health': {'peers': 5, 'isSyncing': False},
            'system_chain': 'X3 Testnet',
            'system_localPeerId': '12D3Peer1',
            'state_getRuntimeVersion': {'specName': 'x3', 'specVersion': 1,
                                        'transactionVersion': 1, 'stateVersion': 1},
        }
        seen = []
        class RPC(BaseHTTPRequestHandler):
            def do_POST(self):
                raw = self.rfile.read(int(self.headers['Content-Length']))
                msg = json.loads(raw)
                seen.append((msg['method'], msg.get('params')))
                value = answers[msg['method']]
                if msg['method'] == 'chain_getBlockHash' and msg['params'] == [148]:
                    value = H3
                if msg['method'] == 'chain_getHeader' and msg['params'] == [H2]:
                    value = {'number': '0x94'}
                body = json.dumps({'jsonrpc': '2.0', 'id': msg['id'], 'result': value}).encode()
                self.send_response(200)
                self.send_header('Content-Length', str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            def log_message(self, *_args):
                pass
        server = ThreadingHTTPServer(('127.0.0.1', 0), RPC)
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        try:
            with tempfile.TemporaryDirectory() as temp:
                root = pathlib.Path(temp)
                binary = root / 'x3-chain-node'
                spec_file = root / 'testnet.json'
                binary.write_bytes(b'fictional node binary')
                spec_file.write_bytes(b'{"spec":true}')
                args = ['python3', '-c', pre.REMOTE_CODE, 'inspect', str(server.server_port),
                        str(binary), str(spec_file), str(root), '0']
                result = subprocess.run(args, capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, 0, result.stderr)
                value = json.loads(result.stdout)
                self.assertEqual(value['finalized_height'], 148)
                self.assertEqual(value['best_height'], 150)
                self.assertEqual(value['peers'], 5)
                self.assertTrue(pre._hex_hash(value['binary_sha256']))
                args[3] = 'hash'
                args[-1] = '148'
                result = subprocess.run(args, capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(json.loads(result.stdout)['canonical_hash'], H3)
                self.assertIn(('chain_getBlockHash', [148]), seen)
        finally:
            server.shutdown()
            server.server_close()
            worker.join(timeout=2)


class CLIContract(unittest.TestCase):
    def test_invalid_inventory_writes_failure_artifact(self):
        with tempfile.TemporaryDirectory() as temp:
            base = pathlib.Path(temp)
            inp, out = base/'inventory.json', base/'report.json'
            inp.write_text(json.dumps({'validators': []}))
            rc = pre.main(['--inventory', str(inp), '--out', str(out), '--commit', 'a'*40])
            self.assertEqual(rc, 1)
            report = json.loads(out.read_text())
            self.assertEqual(report['status'], 'FAIL')
            self.assertEqual(report['commit'], 'a'*40)
            self.assertTrue(report['errors'])

    def test_cli_requires_real_remote_snapshots_then_common_height_hashes(self):
        with tempfile.TemporaryDirectory() as temp:
            base = pathlib.Path(temp)
            inp, out = base/'inventory.json', base/'report.json'
            inp.write_text(json.dumps(inventory()))
            seen = []
            facts = evidence()
            def fake_remote(node, *, mode='inspect', height=0):
                seen.append((node.name, mode, height))
                return dict(facts[node.name]) if mode == 'inspect' else {'canonical_hash': H3}
            with mock.patch.object(pre, 'run_remote', side_effect=fake_remote):
                rc = pre.main(['--inventory', str(inp), '--out', str(out), '--commit', 'b'*40,
                               '--expected-genesis', H1, '--expected-binary-sha256', F1,
                               '--expected-spec-sha256', F2])
            self.assertEqual(rc, 0)
            report = json.loads(out.read_text())
            self.assertEqual(report['status'], 'PASS')
            self.assertEqual(report['shared_finalized_height'], 148)
            self.assertEqual(len(seen), 14)
            self.assertTrue(all(mode == 'hash' and height == 148 for _, mode, height in seen[7:]))

    def test_cli_fails_if_any_common_height_query_fails(self):
        with tempfile.TemporaryDirectory() as temp:
            base = pathlib.Path(temp)
            inp, out = base/'inventory.json', base/'report.json'
            inp.write_text(json.dumps(inventory()))
            facts = evidence()
            def fake_remote(node, *, mode='inspect', height=0):
                if mode == 'hash' and node.name == 'x3star3':
                    raise pre.PreflightError('x3star3: remote hash query timed out')
                return dict(facts[node.name]) if mode == 'inspect' else {'canonical_hash': H3}
            with mock.patch.object(pre, 'run_remote', side_effect=fake_remote):
                rc = pre.main(['--inventory', str(inp), '--out', str(out), '--commit', 'b'*40])
            self.assertEqual(rc, 1)
            report = json.loads(out.read_text())
            self.assertEqual(report['status'], 'FAIL')
            self.assertIn('timed out', str(report['errors']))

    def test_ssh_call_is_strict_and_no_shell(self):
        node = pre.validate_inventory(inventory())[0]
        captured = {}
        def run(args, **kwargs):
            captured['args'] = args
            captured['kwargs'] = kwargs
            return subprocess.CompletedProcess(args, 1, '', 'host key verification failed')
        with mock.patch.object(pre.subprocess, 'run', side_effect=run):
            with self.assertRaises(pre.PreflightError):
                pre.run_remote(node, mode='inspect')
        self.assertIn('StrictHostKeyChecking=yes', captured['args'])
        self.assertIn('BatchMode=yes', captured['args'])
        self.assertFalse(captured['kwargs'].get('shell', False))


if __name__ == '__main__':
    unittest.main()
