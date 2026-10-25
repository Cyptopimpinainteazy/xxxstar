#!/usr/bin/env python3
"""Read-only diagnostics. Private evidence is never written into the checkout."""
import argparse
import datetime
import glob
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

STATE = Path.home() / '.local/state/x3-network'
# Candidates only: confirm these through help on the installed AOS release first.
STATUS_COMMANDS = (
    'show system', 'show chassis', 'show microcode', 'show running-directory',
    'show vlan', 'show interfaces', 'show lldp remote-system', 'show spantree',
)


def private_directory(label):
    STATE.mkdir(mode=0o700, parents=True, exist_ok=True)
    if STATE.is_symlink() or STATE.stat().st_uid != os.getuid():
        raise ValueError('Private state root must be owned by the current user, without a symlink')
    STATE.chmod(0o700)
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')
    return Path(tempfile.mkdtemp(prefix=f'{label}-{stamp}-', dir=STATE))


def run(argv):
    try:
        result = subprocess.run(argv, capture_output=True, text=True, timeout=20, check=False)
        return {'argv': argv, 'exit_code': result.returncode,
                'stdout': result.stdout, 'stderr': result.stderr}
    except FileNotFoundError:
        return {'argv': argv, 'exit_code': 127, 'stdout': '', 'stderr': 'tool unavailable'}
    except subprocess.TimeoutExpired:
        # Avoid emitting partial potentially sensitive device output into ordinary logs.
        return {'argv': argv, 'exit_code': 124, 'stdout': '', 'stderr': '20-second timeout'}


def save(directory, name, data):
    path = directory / name
    with path.open('x', encoding='utf-8') as output:
        path.chmod(0o600)
        json.dump(data, output, indent=2)
        output.write('\n')
    return path


def discover():
    directory = private_directory('console')
    commands = [['hostname'], ['id'], ['lsusb'], ['dmesg'], ['sudo', '-n', 'dmesg'],
                ['ip', '-json', 'address'], ['ip', '-json', 'route', 'show', 'table', 'all'],
                ['ip', '-json', 'neigh']]
    evidence = [run(cmd) for cmd in commands]
    devices = sorted(set(glob.glob('/dev/ttyUSB*') + glob.glob('/dev/ttyACM*')))
    serial = [{'path': d, 'read_write_access': os.access(d, os.R_OK | os.W_OK),
               'udev': run(['udevadm', 'info', '--query=property', '--name=' + d])}
              for d in devices]
    tools = {t: shutil.which(t) for t in ('picocom', 'minicom', 'screen', 'lldpcli', 'ethtool')}
    path = save(directory, 'discovery.json', {'serial_devices': serial, 'tools': tools, 'commands': evidence})
    ready = any(d['read_write_access'] for d in serial) and any(tools[t] for t in ('picocom', 'minicom', 'screen'))
    print(json.dumps({'evidence': str(path), 'serial_devices': [{k: v for k, v in d.items() if k != 'udev'} for d in serial], 'tools': tools,
                      'console_access': 'NOT RUN' if ready else 'BLOCKED'}))
    # Device presence is not proof of a switch login. Never open a tty automatically.
    return 0 if ready else 2


def validate():
    directory = private_directory('address-check')
    commands = [run(['ip', '-json', 'address']),
                run(['ip', '-json', '-4', 'route', 'show', 'table', 'all']),
                run(['ip', '-json', 'rule']),
                run(['docker', 'network', 'ls', '--format', '{{.ID}}'])]
    networks = set()
    errors = []
    for result in commands[:2]:
        if result['exit_code']:
            errors.append(result['argv'])
            continue
        try:
            rows = json.loads(result['stdout'])
            for row in rows:
                dst = row.get('dst')
                # The default route is not a locally allocated subnet.
                if dst and dst != 'default':
                    networks.add(ipaddress.ip_network(dst, strict=False))
                for addr in row.get('addr_info', []):
                    if addr.get('family') == 'inet':
                        networks.add(ipaddress.ip_network(f"{addr['local']}/{addr['prefixlen']}", strict=False))
        except (ValueError, KeyError, TypeError):
            errors.append(result['argv'])
    if commands[2]['exit_code']:
        errors.append(commands[2]['argv'])
    docker = commands[3]
    if docker['exit_code']:
        errors.append(docker['argv'])
    else:
        for network_id in docker['stdout'].splitlines():
            result = run(['docker', 'network', 'inspect', network_id, '--format', '{{json .IPAM.Config}}'])
            commands.append(result)
            try:
                if result['exit_code']:
                    raise ValueError('docker inspection failed')
                for entry in json.loads(result['stdout']) or []:
                    if entry.get('Subnet'):
                        networks.add(ipaddress.ip_network(entry['Subnet'], strict=False))
            except (ValueError, TypeError, KeyError):
                errors.append(result['argv'])
    proposals = [ipaddress.ip_network(f'10.30.{vlan}.0/24') for vlan in range(10, 100, 10)]
    overlaps = [{'proposed': str(p), 'observed': str(n)} for p in proposals for n in networks
                if p.version == n.version and p.overlaps(n)]
    result = {'local_address_check': 'BLOCKED' if errors else ('FAIL' if overlaps else 'PASS'),
              'scope': 'local addresses, IPv4 routes in all tables, and Docker IPAM only',
              'deployment_approval': 'BLOCKED: upstream/VPN/Kubernetes allocations and firewall policy unverified',
              'observed_networks': sorted(map(str, networks)), 'overlaps': overlaps,
              'failed_commands': errors, 'commands': commands}
    path = save(directory, 'address-check.json', result)
    print(json.dumps({k: v for k, v in result.items() if k != 'commands'}))
    print(f'Private evidence: {path}')
    return 2 if errors else (1 if overlaps else 0)


def backup(source, switch_id):
    # Archive an operator-exported configuration; no unverified device CLI or credentials.
    source = Path(source)
    if source.is_symlink() or not source.is_file() or source.stat().st_size == 0:
        raise ValueError('Source must be a nonempty regular file, not a symlink')
    directory = private_directory(f'backup-{switch_id}')
    destination = directory / 'configuration.raw'
    with source.open('rb') as src, destination.open('xb') as dst:
        destination.chmod(0o600)
        shutil.copyfileobj(src, dst)
    digest = hashlib.sha256(destination.read_bytes()).hexdigest()
    save(directory, 'manifest.json', {'switch_id': switch_id, 'sha256': digest,
         'bytes': destination.stat().st_size, 'capture': 'operator-supplied export',
         'restore_test': 'NOT RUN', 'completeness': 'NOT VERIFIED',
         'note': 'Running export alone does not back up saved config, licenses, certificates, or VC state.'})
    print(f'Private archive: {directory}; sha256={digest}; completeness NOT VERIFIED')
    return 0


def collect(host, username, reviewed):
    if not reviewed:
        raise ValueError('Confirm STATUS_COMMANDS against actual CLI help first; then use --reviewed-cli')
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9.-]{0,252}', host):
        raise ValueError('Use a literal IPv4 address or DNS hostname; no SSH aliases/options')
    if not re.fullmatch(r'[A-Za-z0-9_][A-Za-z0-9_.-]{0,63}', username):
        raise ValueError('Invalid SSH username')
    directory = private_directory('switch-status')
    results = []
    for cmd in STATUS_COMMANDS:
        result = run(['ssh', '-F', '/dev/null', '-o', 'BatchMode=yes', '-o', 'StrictHostKeyChecking=yes',
                      '-o', 'ConnectTimeout=5', '-o', 'ClearAllForwardings=yes',
                      '-o', 'RequestTTY=no', '-l', username, host, cmd])
        results.append(result)
        if result['exit_code']:
            break
    path = save(directory, 'status.json', results)
    print(f'Private capture: {path}; device responses require review; hardware health NOT VERIFIED')
    # SSH success does not prove AOS accepted a command or that the device is healthy.
    return 2 if any(r['exit_code'] for r in results) else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    sub.add_parser('discover')
    sub.add_parser('validate')
    b = sub.add_parser('backup', help='Archive a previously exported file outside Git')
    b.add_argument('source')
    b.add_argument('switch_id', choices=['x3-switch1', 'x3-switch2', 'x3-switch3'])
    c = sub.add_parser('collect', help='Capture reviewed read-only AOS commands using existing SSH trust')
    c.add_argument('host')
    c.add_argument('username')
    c.add_argument('--reviewed-cli', action='store_true')
    args = parser.parse_args()
    os.umask(0o077)
    try:
        if args.action == 'discover':
            return discover()
        if args.action == 'validate':
            return validate()
        if args.action == 'backup':
            return backup(args.source, args.switch_id)
        return collect(args.host, args.username, args.reviewed_cli)
    except (ValueError, OSError) as exc:
        print(f'BLOCKED: {exc}')
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
