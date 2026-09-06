#!/usr/bin/env python3
"""Local evidence-backed readiness tracking. No network service or automatic test execution."""
from contextlib import contextmanager
from datetime import datetime, timezone
import argparse
import copy
import csv
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import uuid

CRITERIA = ('implemented', 'wired', 'tested', 'executed', 'reproducible')
SEVERITIES = ('Critical', 'High', 'Medium', 'Low')
EXCLUDED = {'.git', 'target', 'node_modules', 'vendor', 'tauri-vendor', 'tauri-vendov',
            '.venv', '__pycache__', 'audit-artifacts', 'screenshots', '.pre-edit-snapshot',
            '.rc4-runtime-upgrade-work', 'keystore', '.pytest_cache'}
SCHEMA = 1


def now():
    return datetime.now(timezone.utc).isoformat()


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def file_hash(path):
    h = hashlib.sha256()
    with path.open('rb') as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def atomic_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix='.' + path.name, dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as handle:
            json.dump(value, handle, indent=2, ensure_ascii=False)
            handle.write('\n')
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def source_snapshot(repo):
    """Hash Git-visible source, including untracked work; never follow source symlinks."""
    output = subprocess.check_output(['git', 'ls-files', '-co', '--exclude-standard', '-z'], cwd=repo)
    hashes = {}
    for raw in sorted(set(output.split(b'\0'))):
        if not raw:
            continue
        name = os.fsdecode(raw)
        relative = Path(name)
        if EXCLUDED.intersection(relative.parts) or relative.name.startswith('.env'):
            continue
        if relative.suffix in {'.pem', '.key', '.pyc', '.tsbuildinfo'}:
            continue
        path = repo / relative
        if path.is_symlink():
            hashes[name] = digest({'symlink': os.readlink(path)})
        elif path.is_file():
            hashes[name] = digest({'sha256': file_hash(path), 'executable': bool(path.stat().st_mode & 0o111)})
        else:
            hashes[name] = 'MISSING'
    head = subprocess.run(['git', 'rev-parse', '--verify', 'HEAD'], cwd=repo,
                          text=True, capture_output=True, check=False)
    return {'fingerprint': digest(hashes), 'commit': head.stdout.strip() if head.returncode == 0 else None,
            'files': hashes, 'scope': 'Git-visible tracked and untracked files; documented generated/vendor/secret exclusions'}


def new_state(repo, store, tasks, features, subsystems):
    store.mkdir(parents=True, exist_ok=True)
    task_data = copy.deepcopy(tasks)
    for task in task_data:
        task.setdefault('requested_status', 'planned')
        task.setdefault('dependencies', [])
        task.setdefault('severity', task.get('risk', 'High'))
    return {'schema_version': SCHEMA, 'created_at': now(), 'max_evidence_age_days': 30,
            'tasks': task_data, 'features': copy.deepcopy(features), 'subsystems': copy.deepcopy(subsystems),
            'checks': {}, 'receipts': [], 'reviews': [], 'events': [], 'baseline': None}


def targets(state):
    return {x['id'] for x in state['tasks'] + state['features']}


def add_check(state, check_id, argv, bindings, timeout=600):
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.-]{0,79}', check_id):
        raise ValueError('Check ID must be a short safe identifier')
    if not argv or not all(isinstance(x, str) and x and '\0' not in x for x in argv):
        raise ValueError('A nonempty argument vector is required')
    if not bindings or not set(bindings) <= targets(state):
        raise ValueError('Bind checks to existing task/feature IDs')
    if not 0 < timeout <= 86400:
        raise ValueError('Timeout must be in (0, 86400] seconds')
    state['checks'][check_id] = {'id': check_id, 'argv': argv, 'targets': sorted(set(bindings)),
                                'timeout_seconds': timeout}


def safe_path(root, relative):
    path = (root / relative).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError('Evidence path escapes its store')
    return path


def sanitize(text):
    text = re.sub(r'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----.*?-----END [^-]+-----',
                  '[REDACTED PRIVATE KEY]', text, flags=re.S)
    text = re.sub(r'(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{50,}|AKIA[A-Z0-9]{16}|xox[baprs]-[A-Za-z0-9-]{20,})',
                  '[REDACTED TOKEN]', text)
    return re.sub(r'(?im)^(.*\b(?:password|secret|access_token|private_key)\s*[=:]).*$',
                  r'\1 [REDACTED]', text)


def run_check(state, repo, store, check_id):
    if check_id not in state['checks']:
        raise ValueError('Unknown check: ' + check_id)
    check = copy.deepcopy(state['checks'][check_id])
    before = source_snapshot(repo)
    started = now()
    stamp = time.monotonic()
    environment = {key: os.environ[key] for key in ('PATH', 'HOME', 'CARGO_HOME', 'RUSTUP_HOME', 'TMPDIR') if key in os.environ}
    environment.update({'CARGO_NET_OFFLINE': 'true', 'CARGO_TARGET_DIR': str(store.resolve() / 'build'),
                        'PYTHONDONTWRITEBYTECODE': '1', 'NO_COLOR': '1', 'CI': '1'})
    error = None
    timed_out = False
    with tempfile.TemporaryFile() as output:
        try:
            process = subprocess.Popen(check['argv'], cwd=repo, env=environment, stdin=subprocess.DEVNULL,
                                       stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                code = process.wait(timeout=check['timeout_seconds'])
            except subprocess.TimeoutExpired:
                timed_out = True
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
                code = 124
        except OSError as exc:
            error = str(exc)
            code = 127
        output.seek(0)
        raw = output.read(16 * 1024 * 1024 + 1)
    truncated = len(raw) > 16 * 1024 * 1024
    content = sanitize(raw[:16 * 1024 * 1024].decode('utf-8', 'replace'))
    if error:
        content += '\n' + sanitize(error) + '\n'
    after = source_snapshot(repo)
    record_id = uuid.uuid4().hex
    log = 'evidence/' + record_id + '.log'
    destination = store / log
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(content)
    receipt = {'id': record_id, 'check_id': check_id, 'check_hash': digest(check), 'argv': check['argv'],
               'cwd': str(repo), 'started_at': started, 'finished_at': now(),
               'seconds': round(time.monotonic() - stamp, 3), 'exit_code': code, 'timed_out': timed_out,
               'log': log, 'log_sha256': file_hash(destination), 'truncated': truncated,
               'source_fingerprint': before['fingerprint'], 'source_changed': before['fingerprint'] != after['fingerprint'],
               'commit': before['commit'], 'environment_policy': 'Minimal inherited environment; offline Cargo; local disposable target directory'}
    atomic_json(store / 'evidence' / (record_id + '.json'), receipt)
    state['receipts'].append(receipt)
    return receipt


def check_results(state, snapshot, store):
    result = {}
    for check_id, definition in state['checks'].items():
        receipt = next((r for r in reversed(state['receipts']) if r['check_id'] == check_id), None)
        status, reason = 'not_run', 'No recorded execution'
        if receipt:
            try:
                log = safe_path(store, receipt['log'])
                recorded = json.loads(safe_path(store, 'evidence/' + receipt['id'] + '.json').read_text())
                if recorded != receipt or file_hash(log) != receipt['log_sha256'] or receipt['truncated']:
                    status, reason = 'invalid', 'Evidence integrity mismatch or truncated log'
                elif receipt['source_changed']:
                    status, reason = 'invalid', 'Source changed during execution'
                elif receipt['source_fingerprint'] != snapshot['fingerprint'] or receipt['check_hash'] != digest(definition):
                    status, reason = 'stale', 'Source or check definition changed'
                else:
                    age = (datetime.now(timezone.utc) - datetime.fromisoformat(receipt['finished_at'])).total_seconds()
                    if age < 0 or age > state['max_evidence_age_days'] * 86400:
                        status, reason = 'stale', 'Evidence outside allowed age'
                    elif receipt['exit_code'] != 0:
                        status, reason = 'failed', 'Command exited ' + str(receipt['exit_code'])
                    else:
                        status, reason = 'passed', 'Fresh execution and matching evidence hashes'
            except (OSError, ValueError, KeyError, TypeError):
                status, reason = 'invalid', 'Unreadable or malformed evidence'
        result[check_id] = {'id': check_id, 'targets': definition['targets'], 'argv': definition['argv'],
                            'status': status, 'reason': reason, 'receipt': receipt}
    return result


def target_contract(state, target):
    item = next(x for x in state['tasks'] + state['features'] if x['id'] == target)
    # Progress edits do not change acceptance. Everything else does.
    return digest({key: value for key, value in item.items() if key != 'requested_status'})


def review(state, repo, store, target, criterion, reviewer, note, checks):
    if target not in targets(state) or not reviewer.strip() or not note.strip():
        raise ValueError('Existing target, reviewer identity and acceptance rationale are required')
    is_task = any(t['id'] == target for t in state['tasks'])
    if criterion not in (('closure',) if is_task else CRITERIA):
        raise ValueError('Invalid review criterion for this target')
    snapshot = source_snapshot(repo)
    results = check_results(state, snapshot, store)
    if not checks or any(c not in results or results[c]['status'] != 'passed' or target not in results[c]['targets'] for c in checks):
        raise ValueError('Review requires fresh passing checks bound to this target')
    if is_task:
        required = {c for c, definition in state['checks'].items() if target in definition['targets']}
        if not required <= set(checks):
            raise ValueError('Closure must include every check bound to the task')
    record = {'id': uuid.uuid4().hex, 'target': target, 'criterion': criterion, 'reviewer': reviewer.strip(),
              'note': note.strip(), 'at': now(), 'source_fingerprint': snapshot['fingerprint'],
              'contract_hash': target_contract(state, target),
              'evidence': {c: results[c]['receipt']['id'] for c in sorted(set(checks))}}
    state['reviews'].append(record)
    if is_task:
        next(t for t in state['tasks'] if t['id'] == target)['requested_status'] = 'completed'
    return record


def valid_review(state, target, criterion, snapshot, checks):
    record = next((r for r in reversed(state['reviews']) if r['target'] == target and r['criterion'] == criterion), None)
    if not record or record['source_fingerprint'] != snapshot['fingerprint'] or record['contract_hash'] != target_contract(state, target):
        return False
    evidence = record['evidence']
    if not evidence or any(c not in checks or checks[c]['status'] != 'passed' or
                           target not in checks[c]['targets'] or checks[c]['receipt']['id'] != receipt
                           for c, receipt in evidence.items()):
        return False
    if criterion == 'closure':
        return {c for c, item in checks.items() if target in item['targets']} <= set(evidence)
    return True


def baseline_eligible(state, snapshot):
    baseline = state.get('baseline')
    if not baseline or not baseline.get('source_matched_at_import'):
        return False
    age = (datetime.now(timezone.utc) - datetime.fromisoformat(baseline['observed_at'])).total_seconds()
    return (baseline['source_fingerprint'] == snapshot['fingerprint'] and 0 <= age <= state['max_evidence_age_days'] * 86400)


def validate_state(state):
    if state['schema_version'] != SCHEMA:
        raise ValueError('Unsupported state schema')
    identifiers = [x['id'] for x in state['tasks'] + state['features']]
    if len(identifiers) != len(set(identifiers)):
        raise ValueError('Task and feature IDs must be unique')
    for task in state['tasks']:
        if task['severity'] not in SEVERITIES:
            raise ValueError('Unknown severity: ' + str(task['severity']))
        if task['requested_status'] not in ('planned', 'in_progress', 'awaiting_verification', 'completed'):
            raise ValueError('Unknown task status')
    weights = [s['weight'] for s in state['subsystems']]
    if any(not isinstance(w, (int, float)) or not math.isfinite(w) or w < 0 or w > 100 for w in weights) or sum(weights) != 100:
        raise ValueError('Finite nonnegative subsystem weights must sum to 100')
    names = [s['subsystem'] for s in state['subsystems']]
    if len(names) != len(set(names)) or any(f['subsystem'] not in names for f in state['features']):
        raise ValueError('Feature subsystem mapping is invalid')
    age = state['max_evidence_age_days']
    if not isinstance(age, (int, float)) or not math.isfinite(age) or age < 0:
        raise ValueError('Evidence age must be finite and nonnegative')


def evaluate(state, repo, store):
    validate_state(state)
    if state['schema_version'] != SCHEMA:
        raise ValueError('Unsupported state schema')
    snapshot = source_snapshot(repo)
    checks = check_results(state, snapshot, store)
    baseline_ok = baseline_eligible(state, snapshot)
    tasks = copy.deepcopy(state['tasks'])
    index = {t['id']: t for t in tasks}
    visiting, finished = set(), set()

    def resolve(task_id):
        if task_id in visiting:
            raise ValueError('Task dependency cycle at ' + task_id)
        if task_id not in index:
            raise ValueError('Unknown task dependency: ' + task_id)
        if task_id in finished:
            return
        visiting.add(task_id)
        task = index[task_id]
        for dependency in task['dependencies']:
            resolve(dependency)
        deps_ok = all(index[d]['status'] == 'completed' for d in task['dependencies'])
        reviewed = valid_review(state, task_id, 'closure', snapshot, checks)
        if task['requested_status'] == 'completed' and reviewed and deps_ok:
            task['status'] = 'completed'
        elif task['requested_status'] == 'completed':
            task['status'] = 'awaiting_verification'
        else:
            task['status'] = task['requested_status']
        task['closure_review_valid'] = reviewed
        task['dependencies_complete'] = deps_ok
        visiting.remove(task_id)
        finished.add(task_id)

    for task in tasks:
        resolve(task['id'])
    features = copy.deepcopy(state['features'])
    for feature in features:
        evidence_sources = {}
        for criterion in CRITERIA:
            reviewed = valid_review(state, feature['id'], criterion, snapshot, checks)
            inherited = baseline_ok and bool(feature.get('baseline_criteria', {}).get(criterion))
            # An explicit failing/stale/invalid bound check invalidates imported credit.
            regressions = any(feature['id'] in c['targets'] and c['status'] in ('failed', 'stale', 'invalid') for c in checks.values())
            feature[criterion] = int(reviewed or (inherited and not regressions))
            evidence_sources[criterion] = 'current review' if reviewed else 'historical audit, unchanged source' if inherited and not regressions else 'unverified'
        feature['completion_percent'] = 20 * sum(feature[k] for k in CRITERIA)
        feature['status'] = 'VERIFIED' if feature['completion_percent'] == 100 else 'PARTIAL' if feature['completion_percent'] else 'UNVERIFIED'
        feature['criterion_sources'] = evidence_sources
    subsystems = []
    if sum(s['weight'] for s in state['subsystems']) != 100:
        raise ValueError('Subsystem weights must sum to 100')
    raw = 0
    for subsystem in state['subsystems']:
        members = [f for f in features if f['subsystem'] == subsystem['subsystem']]
        if not members:
            raise ValueError('Empty subsystem: ' + subsystem['subsystem'])
        score = sum(f['completion_percent'] for f in members) / len(members)
        raw += score * subsystem['weight'] / 100
        subsystems.append({'subsystem': subsystem['subsystem'], 'weight': subsystem['weight'], 'score': round(score, 2)})
    counts = {s: sum(t['severity'] == s and t['status'] != 'completed' for t in tasks) for s in SEVERITIES}
    done = sum(t['status'] == 'completed' for t in tasks)
    score = min(raw, 20) if counts['Critical'] else raw
    return {'generated_at': now(), 'source': snapshot, 'baseline_eligible': baseline_ok,
            'baseline': state.get('baseline'), 'tasks': tasks, 'features': features, 'subsystems': subsystems,
            'checks': list(checks.values()), 'reviews': state['reviews'], 'open_findings': counts,
            'completed_tasks': done, 'task_count': len(tasks), 'task_progress_percent': round(100 * done / len(tasks), 2) if tasks else 0,
            'uncapped_score': round(raw, 2), 'readiness_score': round(score, 2),
            'release_decision': 'NO-GO' if any(counts.values()) else 'NOT ASSESSED',
            'score_formula': '20 points for each implemented/wired/tested/executed/reproducible criterion; weighted subsystem mean; cap 20 while a Critical finding remains open.',
            'review_trust': 'Named local operator attestations, not authenticated independent security certification. Local writers can alter state and evidence; hashes detect accidental tampering, not a hostile store owner.'}


def verify_snapshot(destination):
    manifest = json.loads((destination / 'manifest.json').read_text())
    actual = {str(p.relative_to(destination)) for p in destination.rglob('*') if p.is_file() and p.name != 'manifest.json'}
    if actual != set(manifest['files']):
        raise ValueError('Snapshot file set does not match manifest')
    for name, expected in manifest['files'].items():
        if file_hash(safe_path(destination, name)) != expected:
            raise ValueError('Snapshot checksum mismatch: ' + name)


def refresh(state, repo, store, pdf=True):
    import reporting
    report = evaluate(state, repo, store)
    identity = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S') + '-' + uuid.uuid4().hex[:8]
    snapshots = store / 'snapshots'
    snapshots.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix='.building-', dir=snapshots))
    destination = snapshots / identity
    try:
        atomic_json(staging / 'summary.json', report)
        atomic_json(staging / 'state.json', state)
        reporting.render(report, staging, pdf=pdf)
        manifest = {'created_at': now(), 'source_fingerprint': report['source']['fingerprint'],
                    'files': {str(p.relative_to(staging)): file_hash(p) for p in staging.rglob('*') if p.is_file()}}
        atomic_json(staging / 'manifest.json', manifest)
        verify_snapshot(staging)
        os.replace(staging, destination)
        # Only advance the pointer after every requested artifact exists and validates.
        atomic_json(store / 'current.json', {'snapshot': str(destination.relative_to(store)),
                                            'updated_at': now(), 'manifest_sha256': file_hash(destination / 'manifest.json')})
        (store / 'index.html').write_text(reporting.landing_html())
        return destination
    finally:
        if staging.exists():
            shutil.rmtree(staging)


@contextmanager
def locked(store):
    store.mkdir(parents=True, exist_ok=True)
    with (store / '.lock').open('a') as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        yield


def load_state(store):
    path = store / 'state.json'
    if not path.exists():
        raise ValueError('Run init before using this store')
    return json.loads(path.read_text())


def import_baseline(repo, store, baseline):
    manifest = json.loads((baseline / 'manifest.json').read_text())
    for item in manifest['files']:
        if file_hash(safe_path(baseline, item['path'])) != item['sha256']:
            raise ValueError('Baseline integrity mismatch: ' + item['path'])
    score = json.loads((baseline / 'scorecard.json').read_text())
    recovery = json.loads((baseline / 'recovery-plan.json').read_text())
    provenance = json.loads((baseline / 'provenance.json').read_text())
    source_hashes = json.loads((baseline / 'evidence/source-hashes.json').read_text())
    tasks = recovery['items']
    for task in tasks:
        task['severity'] = task['risk']
        task['dependencies'] = [d if d.startswith('FIX-') else 'FIX-' + d for d in task['dependencies']]
    # Historical dependencies sometimes contain prose; only explicit task IDs are graph edges.
    task_ids = {t['id'] for t in tasks}
    for task in tasks:
        task['historical_dependency_notes'] = task['dependencies']
        task['dependencies'] = [d for d in task['dependencies'] if d in task_ids]
    for feature in score['features']:
        feature['baseline_criteria'] = {c: feature[c] for c in CRITERIA}
    state = new_state(repo, store, tasks, score['features'], score['subsystems'])
    matches = all((repo / x['path']).is_file() and file_hash(repo / x['path']) == x['sha256'] for x in source_hashes)
    matches = matches and all(file_hash(repo / name) == value for name, value in provenance['lockfile_hashes'].items())
    state['baseline'] = {'path': str(baseline), 'manifest_sha256': file_hash(baseline / 'manifest.json'),
                         'readiness_score': score['readiness_score'], 'observed_at': provenance['created_at'],
                         'source_matched_at_import': matches, 'source_fingerprint': source_snapshot(repo)['fingerprint'],
                         'commit': provenance['commit']}
    return state


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--store', type=Path, default=None)
    subs = parser.add_subparsers(dest='action', required=True)
    init = subs.add_parser('init'); init.add_argument('--baseline', type=Path, required=True)
    add = subs.add_parser('check-add'); add.add_argument('id'); add.add_argument('--targets', nargs='+', required=True)
    add.add_argument('--timeout', type=float, default=600); add.add_argument('--command', nargs=argparse.REMAINDER, required=True)
    run = subs.add_parser('run'); run.add_argument('id')
    task = subs.add_parser('task'); task.add_argument('id'); task.add_argument('status', choices=['planned', 'in_progress', 'awaiting_verification', 'completed'])
    rev = subs.add_parser('review'); rev.add_argument('target'); rev.add_argument('criterion', choices=['closure', *CRITERIA])
    rev.add_argument('--reviewer', required=True); rev.add_argument('--note', required=True); rev.add_argument('--checks', nargs='+', required=True)
    for action in ['refresh', 'watch']:
        command = subs.add_parser(action); command.add_argument('--no-pdf', action='store_true')
        if action == 'watch':
            command.add_argument('--interval', type=float, default=10); command.add_argument('--iterations', type=int, default=0)
    subs.add_parser('status')
    verify = subs.add_parser('verify'); verify.add_argument('snapshot', type=Path)
    args = parser.parse_args(argv)
    repo = args.repo.resolve(); store = (args.store or repo / 'audit-artifacts/mainnet-readiness/live').resolve()
    if not store.is_relative_to(repo / 'audit-artifacts'):
        raise ValueError('Store must be beneath repository audit-artifacts to exclude output from source fingerprints')
    if args.action == 'verify':
        verify_snapshot(args.snapshot.resolve()); print('Snapshot hashes verified'); return 0
    if args.action == 'watch':
        if args.interval < 1 or args.iterations < 0:
            raise ValueError('Watch interval must be >= 1 second; iterations must be nonnegative')
        previous = None
        iteration = 0
        while args.iterations == 0 or iteration < args.iterations:
            with locked(store):
                state = load_state(store)
                report = evaluate(state, repo, store)
                # Include evidence integrity/expiry, not just state file modification times.
                signature = digest({k: v for k, v in report.items() if k != 'generated_at'})
                if signature != previous:
                    destination = refresh(state, repo, store, pdf=not args.no_pdf)
                    print(str(destination), flush=True)
                    previous = signature
            iteration += 1
            if args.iterations == 0 or iteration < args.iterations:
                time.sleep(args.interval)
        return 0
    with locked(store):
        if args.action == 'init':
            if (store / 'state.json').exists():
                raise ValueError('Store already initialized; refusing to overwrite history')
            state = import_baseline(repo, store, args.baseline.resolve())
        else:
            state = load_state(store)
        return_code = 0
        if args.action == 'check-add':
            add_check(state, args.id, args.command, args.targets, args.timeout)
        elif args.action == 'run':
            receipt = run_check(state, repo, store, args.id)
            print(json.dumps(receipt, indent=2)); return_code = 0 if receipt['exit_code'] == 0 else 1
        elif args.action == 'task':
            match = next((t for t in state['tasks'] if t['id'] == args.id), None)
            if not match:
                raise ValueError('Unknown task')
            match['requested_status'] = args.status
        elif args.action == 'review':
            review(state, repo, store, args.target, args.criterion, args.reviewer, args.note, args.checks)
        elif args.action == 'status':
            report = evaluate(state, repo, store)
            print(json.dumps({k: report[k] for k in ['readiness_score', 'uncapped_score', 'completed_tasks', 'task_count', 'open_findings', 'baseline_eligible', 'release_decision']}, indent=2)); return 0
        if args.action not in ['refresh', 'status']:
            state['events'].append({'at': now(), 'action': args.action, 'state_digest_before_event': digest(state)})
            atomic_json(store / 'state.json', state)
        destination = refresh(state, repo, store, pdf=not getattr(args, 'no_pdf', False))
        print('Updated ' + str(destination))
        return return_code


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError) as exc:
        print('Readiness error: ' + str(exc), file=sys.stderr)
        sys.exit(2)
    except KeyboardInterrupt:
        sys.exit(130)
