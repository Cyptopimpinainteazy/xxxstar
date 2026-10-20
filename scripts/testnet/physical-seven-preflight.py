#!/usr/bin/env python3
"""Fail-closed independent-hardware seven-validator preflight. Python stdlib only."""
from __future__ import annotations
import argparse
from dataclasses import dataclass
import ipaddress
import json
import math
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import time

class PreflightError(ValueError):
    pass


@dataclass(frozen=True)
class Node:
    name: str
    ssh_host: str
    rpc_port: int
    binary_path: str
    chain_spec_path: str
    data_dir: str


def validate_inventory(data):
    if not isinstance(data, dict) or not isinstance(data.get('validators'), list):
        raise PreflightError('inventory must have a validators array')
    entries = data['validators']
    if len(entries) != 7:
        raise PreflightError(f'physical testnet requires exactly 7 validators; got {len(entries)}')
    nodes = []
    seen_names, seen_ssh = set(), set()
    for idx, entry in enumerate(entries, 1):
        if not isinstance(entry, dict):
            raise PreflightError(f'validator {idx} must be an object')
        name, ssh_host = entry.get('name'), entry.get('ssh_host')
        if not isinstance(name, str) or not re.fullmatch(r'[A-Za-z][A-Za-z0-9_-]{0,62}', name):
            raise PreflightError(f'validator {idx} has invalid name')
        if not isinstance(ssh_host, str) or not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.@:-]{0,253}', ssh_host):
            raise PreflightError(f'{name}: invalid SSH target')
        host_only = ssh_host.rsplit('@', 1)[-1].lower()
        if host_only == 'localhost' or host_only.endswith('.localhost'):
            raise PreflightError(f'{name}: physical hosts cannot use loopback SSH')
        try:
            address = ipaddress.ip_address(host_only)
        except ValueError:
            address = None
        if address is not None and address.is_loopback:
            raise PreflightError(f'{name}: physical hosts cannot use loopback SSH')
        if name in seen_names or ssh_host in seen_ssh:
            raise PreflightError(f'{name}: duplicate name or SSH target')
        seen_names.add(name)
        seen_ssh.add(ssh_host)
        port = entry.get('rpc_port')
        if type(port) is not int or not (1 <= port <= 65535):
            raise PreflightError(f'{name}: rpc_port must be a valid TCP port')
        args = []
        for field in ('binary_path', 'chain_spec_path', 'data_dir'):
            value = entry.get(field)
            if not isinstance(value, str) or not value.startswith('/') or any(c in value for c in ('\0', '\n', '\r')):
                raise PreflightError(f'{name}: {field} must be an absolute single-line path')
            args.append(value)
        nodes.append(Node(name, ssh_host, port, *args))
    return nodes


def _hex_hash(value, prefix=False):
    return isinstance(value, str) and re.fullmatch(
        r'0x[0-9a-fA-F]{64}' if prefix else r'[0-9a-fA-F]{64}', value
    ) is not None


def assess(nodes, snapshots, common_hashes, *, min_peers=3, min_free_gib=5.0,
           max_clock_skew_sec=3.0, expected_genesis=None, expected_binary_sha256=None,
           expected_spec_sha256=None):
    """Return every violation; empty list is the only successful result.

    This verifies reported node state, NOT independent GRANDPA signatures,
    validator authoring keys, SSH host authenticity or binary-to-commit mapping.
    """
    problems = []
    if not isinstance(snapshots, dict) or not isinstance(common_hashes, dict):
        return ['snapshot or canonical hash evidence is missing']
    validated = {}
    for node in nodes:
        name = node.name
        state = snapshots.get(name)
        if not isinstance(state, dict):
            problems.append(f'{name}: missing valid SSH/RPC snapshot')
            continue
        valid = True
        for field, prefixed in (('machine_id_hash', False), ('binary_sha256', False),
                                ('spec_sha256', False), ('genesis_hash', True),
                                ('finalized_hash', True)):
            if not _hex_hash(state.get(field), prefixed):
                problems.append(f'{name}: invalid or missing {field}')
                valid = False
        for field in ('best_height', 'finalized_height', 'peers', 'free_bytes'):
            value = state.get(field)
            if type(value) is not int or value < 0:
                problems.append(f'{name}: {field} must be a non-negative integer')
                valid = False
        if type(state.get('is_syncing')) is not bool:
            problems.append(f'{name}: is_syncing is missing or not boolean')
            valid = False
        if not isinstance(state.get('local_peer_id'), str) or not state['local_peer_id'].strip():
            problems.append(f'{name}: missing libp2p peer identity')
            valid = False
        if not isinstance(state.get('system_chain'), str) or not state['system_chain'].strip():
            problems.append(f'{name}: missing chain identity')
            valid = False
        runtime = state.get('runtime')
        if not isinstance(runtime, dict) or not isinstance(runtime.get('specName'), str) or not runtime['specName']:
            problems.append(f'{name}: missing runtime specName')
            valid = False
        else:
            for field in ('specVersion', 'transactionVersion', 'stateVersion'):
                if type(runtime.get(field)) is not int or runtime[field] < 0:
                    problems.append(f'{name}: invalid runtime {field}')
                    valid = False
        for field in ('remote_time', 'local_received_at'):
            value = state.get(field)
            if type(value) not in (int, float) or not math.isfinite(value):
                problems.append(f'{name}: invalid timestamp {field}')
                valid = False
        if not valid:
            continue
        validated[name] = state
        if state['is_syncing']:
            problems.append(f'{name}: RPC says node is still syncing')
        if state['peers'] < min_peers:
            problems.append(f'{name}: {state["peers"]} peers is below {min_peers}')
        if state['finalized_height'] < 1:
            problems.append(f'{name}: no non-genesis finality evidence')
        if state['finalized_height'] > state['best_height']:
            problems.append(f'{name}: finalized height exceeds best height')
        if state['free_bytes'] < min_free_gib * 1024 ** 3:
            problems.append(f'{name}: free disk is below {min_free_gib} GiB')
        if abs(state['remote_time'] - state['local_received_at']) > max_clock_skew_sec:
            problems.append(f'{name}: remote clock skew exceeds {max_clock_skew_sec} sec')
        for field, expected in (('genesis_hash', expected_genesis),
                                ('binary_sha256', expected_binary_sha256),
                                ('spec_sha256', expected_spec_sha256)):
            if expected is not None and state[field].lower() != expected.lower():
                problems.append(f'{name}: {field} does not match expected artifact')

    if len(validated) != 7:
        problems.append(f'only {len(validated)} of 7 validators supplied valid evidence')
    for field, label in (('machine_id_hash', 'machine identity'),
                         ('local_peer_id', 'libp2p peer identity')):
        values = [state[field] for state in validated.values()]
        if len(values) != len(set(values)):
            problems.append(f'duplicate {label}: two validators are not independent')
    for field in ('binary_sha256', 'spec_sha256', 'genesis_hash', 'system_chain'):
        values = {state[field].lower() for state in validated.values()}
        if len(values) > 1:
            problems.append(f'{field}: validators disagree')
    version_tuples = {
        (r['specName'], r['specVersion'], r['transactionVersion'], r['stateVersion'])
        for state in validated.values() for r in [state['runtime']]
    }
    if len(version_tuples) > 1:
        problems.append('runtime versions differ across physical validators')

    canonical = []
    for node in nodes:
        value = common_hashes.get(node.name)
        if not _hex_hash(value, True):
            problems.append(f'{node.name}: missing canonical hash at shared finalized height')
        else:
            canonical.append(value.lower())
    if len(set(canonical)) > 1:
        problems.append('canonical block hashes diverge at shared finalized height')
    if len(validated) == 7:
        shared_height = min(state['finalized_height'] for state in validated.values())
        for node in nodes:
            state = validated[node.name]
            at_height = common_hashes.get(node.name)
            if state['finalized_height'] == shared_height and _hex_hash(at_height, True):
                if state['finalized_hash'].lower() != at_height.lower():
                    problems.append(f'{node.name}: finalized head disagrees with canonical shared-height block')
    return problems


def assess_progress(initial, later):
    """Require two complete, stable seven-host samples and advancing finality."""
    if not isinstance(initial, dict) or not isinstance(later, dict) or len(initial) != 7 or set(initial) != set(later):
        return ['two complete samples from the same seven validators are required']
    errors = []
    for name, first in initial.items():
        second = later[name]
        if not isinstance(first, dict) or not isinstance(second, dict):
            errors.append(f'{name}: invalid progress sample')
            continue
        for field in ('machine_id_hash', 'binary_sha256', 'spec_sha256',
                      'genesis_hash', 'local_peer_id', 'runtime'):
            if first.get(field) is None or first.get(field) != second.get(field):
                errors.append(f'{name}: {field} changed during observation')
        a, b = first.get('finalized_height'), second.get('finalized_height')
        if type(a) is not int or type(b) is not int or a <= 0 or b <= a:
            errors.append(f'{name}: finality did not advance during observation')
        if not _hex_hash(first.get('finalized_hash'), True) or not _hex_hash(second.get('finalized_hash'), True) or first['finalized_hash'].lower() == second['finalized_hash'].lower():
            errors.append(f'{name}: finalized block hash did not advance')
    return errors


# Runs on the validator itself over an authenticated SSH session. No LAN RPC
# exposure, remote temp files, sudo, shell pipelines, or SSH host-key acceptance.
REMOTE_CODE = r'''
import hashlib
import json
import os
import sys
import time
import urllib.request

mode, port, binary, spec, data_dir, height = sys.argv[1:]
url = 'http://127.0.0.1:' + port


def rpc(method, params=None):
    request = urllib.request.Request(url, data=json.dumps({
        'jsonrpc': '2.0', 'id': 1, 'method': method,
        'params': [] if params is None else params,
    }).encode('utf-8'), headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(request, timeout=5) as response:
        value = json.load(response)
    if not isinstance(value, dict) or value.get('jsonrpc') != '2.0' or value.get('id') != 1 or 'error' in value or 'result' not in value:
        raise ValueError('invalid JSON-RPC response for ' + method)
    return value['result']


def filehash(path):
    digest = hashlib.sha256()
    with open(path, 'rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def height_of(header):
    if not isinstance(header, dict):
        raise ValueError('missing block header')
    value = header.get('number')
    if not isinstance(value, str) or not value.startswith('0x'):
        raise ValueError('invalid block number')
    return int(value, 16)

if mode == 'hash':
    result = {'canonical_hash': rpc('chain_getBlockHash', [int(height)])}
elif mode == 'inspect':
    machine_id = open('/etc/machine-id', encoding='utf-8').read().strip()
    if not machine_id:
        raise ValueError('machine-id unavailable')
    stat = os.statvfs(data_dir)
    finalized_hash = rpc('chain_getFinalizedHead')
    health = rpc('system_health')
    result = {
        'machine_id_hash': hashlib.sha256(machine_id.encode('utf-8')).hexdigest(),
        'binary_sha256': filehash(binary),
        'spec_sha256': filehash(spec),
        'genesis_hash': rpc('chain_getBlockHash', [0]),
        'system_chain': rpc('system_chain'),
        'local_peer_id': rpc('system_localPeerId'),
        'runtime': rpc('state_getRuntimeVersion'),
        'best_height': height_of(rpc('chain_getHeader')),
        'finalized_height': height_of(rpc('chain_getHeader', [finalized_hash])),
        'finalized_hash': finalized_hash,
        'peers': health.get('peers') if isinstance(health, dict) else None,
        'is_syncing': health.get('isSyncing') if isinstance(health, dict) else None,
        'free_bytes': stat.f_bavail * stat.f_frsize,
        'remote_time': time.time(),
    }
else:
    raise ValueError('unsupported mode')
print(json.dumps(result, separators=(',', ':')))
'''


def run_remote(node, *, mode='inspect', height=0):
    """Read remote evidence through strict, non-interactive host-key-checked SSH."""
    if mode not in ('inspect', 'hash') or type(height) is not int or height < 0:
        raise PreflightError('invalid remote probe mode or height')
    command = shlex.join(['python3', '-c', REMOTE_CODE, mode, str(node.rpc_port),
                          node.binary_path, node.chain_spec_path, node.data_dir,
                          str(height)])
    argv = ['ssh', '-o', 'BatchMode=yes', '-o', 'StrictHostKeyChecking=yes',
            '-o', 'NumberOfPasswordPrompts=0', '-o', 'ConnectTimeout=5', '-T',
            '--', node.ssh_host, command]
    try:
        result = subprocess.run(argv, capture_output=True, text=True, timeout=120,
                                check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise PreflightError(f'{node.name}: SSH unavailable: {exc.__class__.__name__}') from exc
    if result.returncode != 0:
        raise PreflightError(f'{node.name}: SSH/RPC probe failed (exit {result.returncode}): '
                             f'{result.stderr.strip()[-300:]}')
    try:
        output = json.loads(result.stdout)
    except (ValueError, TypeError) as exc:
        raise PreflightError(f'{node.name}: remote probe did not return JSON') from exc
    if not isinstance(output, dict):
        raise PreflightError(f'{node.name}: remote probe JSON is not an object')
    if mode == 'inspect':
        output['local_received_at'] = time.time()
    return output


def main(argv=None):
    parser = argparse.ArgumentParser(description='Physical seven-validator SSH/RPC preflight')
    parser.add_argument('--inventory', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--commit', required=True, help='Exact 40-hex git commit being tested')
    parser.add_argument('--observation-seconds', type=float, default=12.0)
    parser.add_argument('--min-peers', type=int, default=4)
    parser.add_argument('--min-free-gib', type=float, default=20.0)
    parser.add_argument('--max-clock-skew-sec', type=float, default=3.0)
    parser.add_argument('--expected-genesis')
    parser.add_argument('--expected-binary-sha256')
    parser.add_argument('--expected-spec-sha256')
    args = parser.parse_args(argv)
    errors, snapshots, canonical, shared_height = [], {}, {}, None
    initial, previous_canonical = {}, {}
    try:
        if not re.fullmatch(r'[0-9a-fA-F]{40}', args.commit):
            raise PreflightError('--commit must be a 40-digit SHA-1 git commit')
        if type(args.min_peers) is not int or args.min_peers < 1:
            raise PreflightError('--min-peers must be positive')
        if not math.isfinite(args.min_free_gib) or args.min_free_gib < 0:
            raise PreflightError('--min-free-gib must be nonnegative and finite')
        if not math.isfinite(args.max_clock_skew_sec) or args.max_clock_skew_sec < 0:
            raise PreflightError('--max-clock-skew-sec must be nonnegative and finite')
        for name, value, prefixed in (
            ('genesis', args.expected_genesis, True),
            ('binary', args.expected_binary_sha256, False),
            ('spec', args.expected_spec_sha256, False),
        ):
            if value is not None and not _hex_hash(value, prefixed):
                raise PreflightError(f'--expected-{name} has invalid hash format')
        if not math.isfinite(args.observation_seconds) or not 0 < args.observation_seconds <= 300:
            raise PreflightError('--observation-seconds must be finite and in (0, 300]')
        nodes = validate_inventory(json.loads(args.inventory.read_text(encoding='utf-8')))
        for node in nodes:
            try:
                snapshots[node.name] = run_remote(node)
            except PreflightError as exc:
                errors.append(str(exc))
        initial = snapshots
        snapshots = {}
        if len(initial) == 7:
            time.sleep(args.observation_seconds)
            for node in nodes:
                try:
                    snapshots[node.name] = run_remote(node)
                except PreflightError as exc:
                    errors.append(str(exc))
        errors.extend(assess_progress(initial, snapshots))
        # Increasing heights can still hide a rewrite of an already-finalized block.
        # Query each original height again after the second sample, on that node.
        if len(initial) == 7 and len(snapshots) == 7:
            for node in nodes:
                first = initial[node.name]
                height, expected = first.get('finalized_height'), first.get('finalized_hash')
                if type(height) is not int or height <= 0 or not _hex_hash(expected, True):
                    errors.append(f'{node.name}: invalid previous finalized block evidence')
                    continue
                try:
                    actual = run_remote(node, mode='hash', height=height).get('canonical_hash')
                    previous_canonical[node.name] = actual
                    if not _hex_hash(actual, True) or actual.lower() != expected.lower():
                        errors.append(f'{node.name}: previous finalized block changed or is unavailable')
                except PreflightError as exc:
                    errors.append(f'{node.name}: previous finalized block query failed: {exc}')
        if len(snapshots) == 7:
            heights = [v.get('finalized_height') for v in snapshots.values()]
            if all(type(h) is int and h > 0 for h in heights):
                shared_height = min(heights)
                for node in nodes:
                    try:
                        canonical[node.name] = run_remote(node, mode='hash',
                                                          height=shared_height).get('canonical_hash')
                    except PreflightError as exc:
                        errors.append(str(exc))
        errors.extend(assess(nodes, snapshots, canonical, min_peers=args.min_peers,
                             min_free_gib=args.min_free_gib,
                             max_clock_skew_sec=args.max_clock_skew_sec,
                             expected_genesis=args.expected_genesis,
                             expected_binary_sha256=args.expected_binary_sha256,
                             expected_spec_sha256=args.expected_spec_sha256))
    except (PreflightError, OSError, ValueError, TypeError) as exc:
        errors.append(f'preflight configuration failed: {exc}')
    report = {
        'schema': 'x3-physical-seven-preflight-v1',
        'commit': args.commit,
        'timestamp_utc': __import__('datetime').datetime.now(__import__('datetime').timezone.utc).isoformat(),
        'status': 'FAIL' if errors else 'PASS',
        'shared_finalized_height': shared_height,
        'initial_snapshots': initial,
        'snapshots': snapshots,
        'canonical_hashes_at_shared_height': canonical,
        'canonical_hashes_at_initial_finalized_heights': previous_canonical,
        'errors': errors,
        'limits': {'observation_seconds': args.observation_seconds, 'min_peers': args.min_peers, 'min_free_gib': args.min_free_gib,
                   'max_clock_skew_sec': args.max_clock_skew_sec},
    }
    try:
        import tempfile
        args.out.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile('w', encoding='utf-8', dir=args.out.parent,
                                         prefix='.x3-preflight-', suffix='.json', delete=False) as handle:
            temp_path = handle.name
            json.dump(report, handle, indent=2, sort_keys=True)
            handle.write('\n')
        os.replace(temp_path, args.out)
    except OSError as exc:
        print(f'[physical-seven] FAILED to write evidence: {exc}', file=sys.stderr)
        return 1
    print(f'[physical-seven] {report["status"]}: {len(errors)} issue(s), evidence: {args.out}')
    for problem in errors:
        print(f'  - {problem}', file=sys.stderr)
    return 1 if errors else 0


if __name__ == '__main__':
    raise SystemExit(main())
