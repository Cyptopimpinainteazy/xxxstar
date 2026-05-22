#!/usr/bin/env python3
"""
Repo scanner: finds duplicates, stale files, TODO/FIXME/unimplemented markers,
and common build/tooling artifacts that appear unused. Produces checklist JSON and CHECKLIST.md.
Safe interactive deletion available.
"""
import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
from collections import defaultdict
from datetime import datetime, timezone, timedelta

# Configurable default thresholds
DEFAULT_STALE_DAYS = 365
DEFAULT_ARTIFACT_DAYS = 180
EXCLUDE_DIRS = {'.git', '.venv', 'venv', '__pycache__', '.kilo'}
ARTIFACT_DIR_NAMES = {'target', 'node_modules', 'dist', 'build', '.cache', 'out', '.next', '.turbo', 'coverage', 'artifacts'}
# Skip these extensions when hashing (compiled objects, large binaries)
SKIP_EXTENSIONS = {'.o', '.a', '.so', '.dylib', '.dll', '.exe', '.pdb', '.wasm', '.rlib', '.rmeta', '.d', '.bc'}

TODO_PATTERNS = re.compile(r"\bTODO\b|\bFIXME\b|unimplemented!\(|todo!\(|panic!\(|TODO:\s|FIXME:", re.I)

# Language-specific half-done heuristics
HALF_DONE_RULES = {
    '.rs': [
        (re.compile(r'unimplemented!\('), 'Rust stub: unimplemented!()'),
        (re.compile(r'todo!\('), 'Rust stub: todo!()'),
        (re.compile(r'panic!\("not yet'), 'Rust stub: panic!(not yet)'),
        (re.compile(r'#\[allow\(dead_code\)\]'), 'Dead code allowed (suppressed warning)'),
        (re.compile(r'fn .+\{[\s\n]*\}'), 'Empty function body'),
    ],
    '.ts': [
        (re.compile(r'throw new Error\(["\']not implemented', re.I), 'TS stub: not implemented throw'),
        (re.compile(r'return null; ?//'), 'TS suspicious null return with comment'),
        (re.compile(r'^\s*//\s*TODO', re.M), 'TS TODO comment'),
    ],
    '.tsx': [
        (re.compile(r'throw new Error\(["\']not implemented', re.I), 'TSX stub: not implemented throw'),
        (re.compile(r'^\s*//\s*TODO', re.M), 'TSX TODO comment'),
    ],
    '.js': [
        (re.compile(r'throw new Error\(["\']not implemented', re.I), 'JS stub: not implemented throw'),
        (re.compile(r'^\s*//\s*TODO', re.M), 'JS TODO comment'),
    ],
    '.py': [
        (re.compile(r'raise NotImplementedError'), 'Python stub: NotImplementedError'),
        (re.compile(r'pass\s*$', re.M), 'Python pass statement (possible stub)'),
        (re.compile(r'^\s*#\s*TODO', re.M), 'Python TODO comment'),
    ],
    '.sol': [
        (re.compile(r'revert\(["\']not implemented', re.I), 'Solidity stub: not implemented revert'),
        (re.compile(r'TODO'), 'Solidity TODO'),
    ],
}

def detect_half_done(path, text):
    """Return list of half-done signals for a file based on its extension."""
    ext = os.path.splitext(path)[1].lower()
    signals = []
    rules = HALF_DONE_RULES.get(ext, [])
    for pattern, label in rules:
        if pattern.search(text):
            signals.append(label)
    return signals


def is_text_file(path, blocksize=512):
    try:
        with open(path, 'rb') as f:
            chunk = f.read(blocksize)
            if b"\0" in chunk:
                return False
            # try decode
            try:
                chunk.decode('utf-8')
                return True
            except Exception:
                return False
    except Exception:
        return False


def sha256_of_file(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def git_last_commit_date(path, repo_root):
    try:
        out = subprocess.check_output(['git', '--no-pager', 'log', '-1', '--pretty=format:%cI', '--', path], cwd=repo_root, stderr=subprocess.DEVNULL)
        s = out.decode().strip()
        if s:
            return datetime.fromisoformat(s)
    except subprocess.CalledProcessError:
        return None
    except Exception:
        return None
    return None


def git_bulk_commit_dates(repo_root):
    """Return {relpath: datetime} for tracked files via a single bounded git log call."""
    dates = {}
    try:
        # Limit to last 3 years to keep output manageable; files older than that fall back to mtime
        out = subprocess.check_output(
            ['git', '--no-pager', 'log', '--name-only', '--pretty=format:%cI', '--since=3 years ago'],
            cwd=repo_root, stderr=subprocess.DEVNULL, timeout=30
        )
        text = out.decode('utf-8', errors='replace')
        current_date = None
        for line in text.splitlines():
            line = line.strip()
            if not line:
                continue
            if re.match(r'\d{4}-\d{2}-\d{2}T', line):
                try:
                    current_date = datetime.fromisoformat(line)
                except Exception:
                    pass
            elif current_date:
                # file path — store only the most-recent (first seen) date
                if line not in dates:
                    dates[line] = current_date
    except subprocess.TimeoutExpired:
        pass
    except Exception:
        pass
    return dates


def human_dt(dt):
    if dt is None:
        return 'never'
    return dt.astimezone(timezone.utc).isoformat()


def gather(repo_root, stale_days, artifact_days):
    file_hashes = defaultdict(list)
    todos = []
    files_info = []

    # scanner output files and self — skip to avoid false positives
    self_dir = os.path.relpath(os.path.dirname(__file__), repo_root)

    # Load all git commit dates in one shot (fast)
    git_dates = git_bulk_commit_dates(repo_root)

    for dirpath, dirnames, filenames in os.walk(repo_root):
        # skip .git and typical excludes
        rel = os.path.relpath(dirpath, repo_root)
        if rel == '.':
            rel = ''
        # skip scanner's own directory
        if rel == self_dir or rel.startswith(self_dir + os.sep):
            dirnames[:] = []
            continue
        parts = set(rel.split(os.sep)) if rel else set()
        # Skip excluded dirs and artifact dirs at ANY depth
        skip_set = EXCLUDE_DIRS | ARTIFACT_DIR_NAMES
        if parts & skip_set:
            dirnames[:] = []
            continue
        # Also prune matching subdirs before descending
        for d in list(dirnames):
            if d in skip_set:
                dirnames.remove(d)

        for fname in filenames:
            path = os.path.join(dirpath, fname)
            try:
                st = os.lstat(path)
            except Exception:
                continue
            # skip symlinks to outside etc
            if not os.path.isfile(path):
                continue
            info = {
                'path': os.path.relpath(path, repo_root),
                'size': st.st_size,
                'mtime': datetime.fromtimestamp(st.st_mtime, timezone.utc),
                'hash': None,
                'git_last': None,
                'todo_matches': [],
            }
            # compute git last commit from bulk map (no per-file subprocess)
            info['git_last'] = git_dates.get(info['path'])
            # only hash files under 10MB and not compiled object extensions
            ext = os.path.splitext(path)[1].lower()
            if st.st_size < 10 * 1024 * 1024 and ext not in SKIP_EXTENSIONS:
                try:
                    info['hash'] = sha256_of_file(path)
                    file_hashes[info['hash']].append(info['path'])
                except Exception:
                    info['hash'] = None

            # scan for TODO markers and half-done signals only in text files
            if is_text_file(path):
                try:
                    with open(path, 'r', encoding='utf-8', errors='ignore') as fh:
                        txt = fh.read()
                        for m in TODO_PATTERNS.finditer(txt):
                            snippet = txt[max(0, m.start()-40):m.end()+40].splitlines()[0]
                            info['todo_matches'].append({'marker': m.group(0), 'snippet': snippet})
                        # language-specific half-done detection
                        info['half_done'] = detect_half_done(info['path'], txt)
                except Exception:
                    pass

            files_info.append(info)

    # duplicates
    duplicates = []
    for h, paths in file_hashes.items():
        if len(paths) > 1:
            duplicates.append({'hash': h, 'paths': sorted(paths)})

    # stale files and artifact dirs
    stale_cutoff = datetime.now(timezone.utc) - timedelta(days=stale_days)
    artifact_cutoff = datetime.now(timezone.utc) - timedelta(days=artifact_days)

    stale = []
    artifacts = []
    todo_items = []
    half_done_items = []

    for f in files_info:
        last = f['git_last'] or f['mtime']
        if f['todo_matches']:
            todo_items.append({'path': f['path'], 'matches': f['todo_matches'], 'last_used': human_dt(last)})
        if last < stale_cutoff:
            stale.append({'path': f['path'], 'last_used': human_dt(last), 'size': f['size']})
        if f.get('half_done'):
            half_done_items.append({'path': f['path'], 'signals': f['half_done'], 'last_used': human_dt(last)})

    # artifact directories detection (by name)
    for dname in ARTIFACT_DIR_NAMES:
        dpath = os.path.join(repo_root, dname)
        if os.path.exists(dpath):
            # find last git touch inside dir
            last = git_last_commit_date(dname, repo_root)
            # fallback to mtime of dir
            try:
                mtime = datetime.fromtimestamp(os.path.getmtime(dpath), timezone.utc)
            except Exception:
                mtime = None
            if last is None:
                last = mtime
            if last is None or last < artifact_cutoff:
                artifacts.append({'path': dname, 'last_used': human_dt(last)})

    checklist = []
    # add duplicates
    for i, d in enumerate(duplicates, 1):
        checklist.append({
            'id': f'dup-{i}',
            'type': 'duplicate',
            'summary': f'{len(d["paths"])} files with same content',
            'paths': d['paths'],
            'evidence': f'hash:{d["hash"]}'
        })
    # stale
    for i, s in enumerate(stale, 1):
        checklist.append({
            'id': f'stale-{i}',
            'type': 'stale-file',
            'summary': os.path.basename(s['path']),
            'path': s['path'],
            'last_used': s['last_used'],
            'size': s['size']
        })
    # TODOs
    for i, t in enumerate(todo_items, 1):
        checklist.append({
            'id': f'todo-{i}',
            'type': 'todo-marker',
            'summary': os.path.basename(t['path']),
            'path': t['path'],
            'last_used': t['last_used'],
            'matches': t['matches']
        })
    # artifacts
    for i, a in enumerate(artifacts, 1):
        checklist.append({
            'id': f'artifact-{i}',
            'type': 'artifact-dir',
            'summary': os.path.basename(a['path']),
            'path': a['path'],
            'last_used': a['last_used']
        })
    # half-done (language-specific stubs / not-implemented patterns)
    for i, h in enumerate(half_done_items, 1):
        checklist.append({
            'id': f'halfdone-{i}',
            'type': 'half-done',
            'summary': os.path.basename(h['path']),
            'path': h['path'],
            'last_used': h['last_used'],
            'signals': h['signals'],
        })

    return checklist


def write_outputs(checklist, out_json, out_md, repo_root):
    with open(out_json, 'w', encoding='utf-8') as f:
        json.dump({'generated': human_dt(datetime.now(timezone.utc)), 'items': checklist}, f, indent=2)

    with open(out_md, 'w', encoding='utf-8') as f:
        f.write('# Repository Cleanup Checklist\n\n')
        f.write('Generated: %s\n\n' % human_dt(datetime.now(timezone.utc)))
        for it in checklist:
            if it['type'] == 'duplicate':
                f.write(f"- [ ] {it['id']} DUPLICATES: {it['summary']}\n")
                for p in it['paths']:
                    f.write(f"    - {p}\n")
            elif it['type'] == 'stale-file':
                f.write(f"- [ ] {it['id']} STALE: {it['path']} (last used: {it.get('last_used')})\n")
            elif it['type'] == 'todo-marker':
                f.write(f"- [ ] {it['id']} TODOS: {it['path']} (last used: {it.get('last_used')})\n")
                for m in it.get('matches', [])[:3]:
                    f.write(f"    - {m.get('marker')}: {m.get('snippet')}\n")
            elif it['type'] == 'artifact-dir':
                f.write(f"- [ ] {it['id']} ARTIFACT DIR: {it['path']} (last used: {it.get('last_used')})\n")
            elif it['type'] == 'half-done':
                f.write(f"- [ ] {it['id']} HALF-DONE: {it['path']} (last used: {it.get('last_used')})\n")
                for sig in it.get('signals', []):
                    f.write(f"    - {sig}\n")
        f.write('\n')
        f.write('To remove selected items, run: python3 tools/agent_scanner/scan.py --delete ids.txt --yes\n')


def delete_items(ids_to_delete, checklist, repo_root, assume_yes=False):
    idmap = {it['id']: it for it in checklist}
    for iid in ids_to_delete:
        it = idmap.get(iid)
        if not it:
            print(f'Unknown id: {iid}')
            continue
        # confirm
        if not assume_yes:
            ans = input(f"Delete {iid} ({it.get('path', it.get('paths', 'multiple'))})? [y/N]: ").strip().lower()
            if ans not in ('y', 'yes'):
                print('skip')
                continue
        # perform deletion
        try:
            if it['type'] == 'duplicate':
                # by default keep first, delete rest
                paths = it['paths'][1:]
                for p in paths:
                    ap = os.path.join(repo_root, p)
                    if os.path.isfile(ap):
                        os.remove(ap)
                        print('removed', p)
            else:
                p = it.get('path')
                ap = os.path.join(repo_root, p)
                if os.path.isdir(ap):
                    shutil.rmtree(ap)
                    print('removed dir', p)
                elif os.path.isfile(ap):
                    os.remove(ap)
                    print('removed file', p)
                else:
                    print('not found', p)
        except Exception as e:
            print('error deleting', iid, e)


def main():
    parser = argparse.ArgumentParser(description='Scan repository and produce cleanup checklist')
    parser.add_argument('--repo', default='.', help='Repository root')
    parser.add_argument('--stale-days', type=int, default=DEFAULT_STALE_DAYS)
    parser.add_argument('--artifact-days', type=int, default=DEFAULT_ARTIFACT_DAYS)
    parser.add_argument('--out-json', default='tools/agent_scanner/checklist.json')
    parser.add_argument('--out-md', default='tools/agent_scanner/CHECKLIST.md')
    parser.add_argument('--delete', help='Path to file with newline-separated ids to delete')
    parser.add_argument('--yes', action='store_true', help='Assume yes when deleting')
    args = parser.parse_args()

    repo_root = os.path.abspath(args.repo)

    checklist = gather(repo_root, args.stale_days, args.artifact_days)
    write_outputs(checklist, args.out_json, args.out_md, repo_root)

    print(f'Checklist written to {args.out_json} and {args.out_md} ({len(checklist)} items)')

    if args.delete:
        try:
            with open(args.delete, 'r', encoding='utf-8') as f:
                ids = [l.strip() for l in f if l.strip()]
        except Exception as e:
            print('cannot read ids file', e); sys.exit(1)
        delete_items(ids, checklist, repo_root, assume_yes=args.yes)


if __name__ == '__main__':
    main()
