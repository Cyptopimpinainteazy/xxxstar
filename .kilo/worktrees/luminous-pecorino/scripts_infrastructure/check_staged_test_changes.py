#!/usr/bin/env python3
"""Pre-commit hook: prevent commits that only change tests without any code changes."""
import subprocess
import sys


def staged_files():
    out = subprocess.check_output(["git", "diff", "--cached", "--name-only"]).decode().strip()
    return [line for line in out.splitlines() if line]

def is_test_file(path: str) -> bool:
    p = path.replace('\\\\', '/').lower()
    patterns = [
        p.startswith('tests/'),
        '/test/' in p or p.startswith('test/'),
        '/e2e/' in p,
        '/__tests__/' in p,
        p.endswith('.spec.js') or p.endswith('.spec.ts') or p.endswith('.spec.py') or p.endswith('.spec.rs'),
        p.endswith('.test.js') or p.endswith('.test.ts') or p.endswith('.test.py') or p.endswith('.test.rs'),
        '_test.' in p,
    ]
    return any(patterns)


def main() -> int:
    files = staged_files()
    if not files:
        return 0
    only_tests = True
    for f in files:
        if not is_test_file(f):
            only_tests = False
            break
    if only_tests:
        print('\n🚨 EXECUTION BANNER: Human overseer online.\n')
        print('Policy: no test-mangling. This commit modifies only test files without source changes.')
        print('If this change is valid, include a non-test fix or a clear justification in the commit/PR.')
        print('\nCommit aborted.')
        return 1
    return 0

if __name__ == '__main__':
    sys.exit(main())
