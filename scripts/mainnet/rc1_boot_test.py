#!/usr/bin/env python3
"""Boot a disposable local3 network from an RC1-feature binary and verify live finality."""
import argparse
import hashlib
import json
import pathlib
import os
import re
import shutil
import socket
import subprocess
import tempfile
import time
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=pathlib.Path, default=ROOT / 'target/debug/x3-chain-node')
    parser.add_argument('--evidence', type=pathlib.Path, required=True)
    parser.add_argument('--timeout', type=int, default=300)
    parser.add_argument('--expected-spec', type=int, help='expected runtime spec version (defaults to current source)')
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    expected_spec = args.expected_spec
    if expected_spec is None:
        expected_spec = int(re.search(r"^    spec_version: (\d+),$", (ROOT / "runtime/src/lib.rs").read_text(), re.MULTILINE).group(1))
    ports = [19974, 19975, 19976]
    p2p = [30404, 30405, 30406]
    for port in ports + p2p:
        with socket.socket() as probe:
            probe.bind(('127.0.0.1', port))
    children = []
    logs = []
    report = {'result': 'FAIL', 'command': sys.argv, 'commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(), 'dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT))}
    def rpc(port, method, params=None):
        data = json.dumps({'jsonrpc': '2.0', 'id': 1, 'method': method, 'params': params or []}).encode()
        request = urllib.request.Request(f'http://127.0.0.1:{port}', data=data, headers={'Content-Type': 'application/json'})
        with urllib.request.urlopen(request, timeout=5) as response:
            payload = json.load(response)
        if 'error' in payload:
            raise RuntimeError(f'{method}: {payload["error"]}')
        return payload['result']
    def wait(description, condition):
        deadline = time.monotonic() + args.timeout
        last_error = None
        while time.monotonic() < deadline:
            exited = [child.returncode for child in children if child.poll() is not None]
            if exited:
                raise RuntimeError(f"{description}: validator exited with {exited}; inspect evidence logs")
            try:
                if condition():
                    return
            except (OSError, ValueError, KeyError, RuntimeError) as error:
                last_error = str(error)
            time.sleep(1)
        raise RuntimeError(f'{description} timed out: {last_error}')
    temporary = tempfile.TemporaryDirectory(prefix='x3-rc1-')
    try:
        work = pathlib.Path(temporary.name)
        binary = work / 'x3-chain-node'
        shutil.copy2(args.binary, binary)
        with binary.open('rb') as artifact:
            digest = hashlib.file_digest(artifact, 'sha256').hexdigest()
        report['binary_sha256'] = digest
        bootnodes = []
        for index, name in enumerate(['alice', 'bob', 'charlie']):
            log = (args.evidence / f'{name}.log').open('w')
            logs.append(log)
            command = [str(binary), '--chain', 'local3', f'--{name}', '--base-path', str(work / name), '--rpc-port', str(ports[index]), '--port', str(p2p[index]), '--node-key', f'{index + 11:064x}', '--no-mdns', '--no-telemetry', '--no-prometheus', '--rpc-methods', 'unsafe']
            if bootnodes:
                command += ['--bootnodes', *bootnodes]
            children.append(subprocess.Popen(command, stdout=log, stderr=log))
            wait(f'{name} RPC', lambda: rpc(ports[index], 'system_health') is not None)
            bootnodes.append(f'/ip4/127.0.0.1/tcp/{p2p[index]}/p2p/{rpc(ports[index], "system_localPeerId")}')
        def finalized(port):
            head = rpc(port, 'chain_getFinalizedHead')
            return int(rpc(port, 'chain_getHeader', [head])['number'], 16)
        wait('all three validators finalize', lambda: all(finalized(port) >= 3 for port in ports))
        report['runtime_versions'] = [rpc(port, 'state_getRuntimeVersion') for port in ports]
        report['validators'] = [{'rpc_port': port, 'health': rpc(port, 'system_health'), 'finalized_height': finalized(port), 'genesis_hash': rpc(port, 'chain_getBlockHash', [0])} for port in ports]
        for port in ports:
            version = rpc(port, 'state_getRuntimeVersion')['specVersion']
            if version != expected_spec:
                raise RuntimeError(f'stale embedded WASM: spec {version}, expected {expected_spec}')
        metadata = bytes.fromhex(rpc(ports[0], 'state_getMetadata')[2:])
        for pallet in [b'X3SupplyLedger', b'X3CrossVmRouter']:
            if pallet not in metadata:
                raise RuntimeError(f'missing RC1 pallet {pallet.decode()}')
        # A default feature binary cannot pass as RC1 merely because it boots.
        for excluded in [b'EvolutionCore', b'DepinMarketplace', b'X3FlashLoan']:
            if excluded in metadata:
                raise RuntimeError(f'non-RC1 pallet present: {excluded.decode()} (build --features mainnet-rc1)')
        before = [finalized(port) for port in ports]
        wait('finality advances on every validator', lambda: all(finalized(port) > height for port, height in zip(ports, before)))
        height = min(finalized(port) for port in ports)
        hashes = [rpc(port, 'chain_getBlockHash', [height]) for port in ports]
        if len(set(hashes)) != 1:
            raise RuntimeError('validators disagree about finalized history')
        subprocess.run(['bash', str(ROOT / 'scripts/mainnet/rc1_smoke_test.sh')], env={**os.environ, 'RPC_URL': f'http://127.0.0.1:{ports[0]}'}, check=True)
        report.update(result='PASS', finalized_height=height, finalized_hash=hashes[0], runtime=rpc(ports[0], 'state_getRuntimeVersion'))
    except Exception as error:
        report['error'] = str(error)
        raise
    finally:
        for child in children:
            child.terminate()
        for child in children:
            try:
                child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
        for log in logs:
            log.close()
        temporary.cleanup()
        (args.evidence / 'result.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
