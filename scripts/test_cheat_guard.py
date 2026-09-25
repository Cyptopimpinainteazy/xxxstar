#!/usr/bin/env python3
"""Reject weakened, skipped, or stubbed-out tests before they land.

Default (no arguments) scopes the scan to the *staged* files, which is what the
pre-commit hook wants. `--base <ref>` scopes it to everything this branch adds
or modifies relative to that ref, which is what a CI job on a PR wants - there
is nothing staged in a fresh checkout, so a scan that only looks at the index
would pass vacuously there.
"""

import argparse
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
IGNORE = {".git", "target", "node_modules", ".venv", "vendor", "vendov", "3d", "ChatGPT_files", "protoc_bin", "logs"}
IGNORE_PREFIXES = (
    ".kilo/",
    "apps/x3-desktop/src-tauri/tauri-vendor/",
    "forge-std/",
    "packages/polkawallet-plugin/dist/",
    "tests/phase_core/security/lib/forge-std/",
)
TEST_EXT = {".py", ".rs", ".ts", ".tsx", ".js"}
SKIP_PATTERNS = [
    r"\.skip\(",
    r"pytest\.mark\.skip",
    r"#\[ignore\]",
    r"describe\.skip",
    # Jasmine's "x it". The bare `xit\(` this replaced also matched `sys.exit(main())`
    # and `sys.exit(0)`, because an unanchored regex finds `xit(` inside `exit(` - so every
    # Python script that exits through `sys.exit` was reported as a skipped test. The
    # lookbehind keeps the real pattern (a standalone `xit(` call) and drops the false one:
    # a preceding word character or a dot means this is `exit(`, not `xit(`.
    r"(?<![\w.])xit\(",
]
WEAK_PATTERNS = [r"assert\s+true", r"assert\(true\)"]
MAX_FILE_BYTES = 1_000_000


def git_lines(args):
    result = subprocess.run(
        ["git", *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        return []
    return [line for line in result.stdout.splitlines() if line]


def ref_exists(ref: str) -> bool:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}"],
        cwd=ROOT,
        capture_output=True,
        check=False,
    )
    return result.returncode == 0


def candidate_files(base: str | None = None):
    names = set()
    if base:
        names.update(git_lines(["diff", "--name-only", "--diff-filter=ACMRT", f"{base}...HEAD"]))
        names.update(git_lines(["diff", "--name-only", "--diff-filter=ACMRT"]))
    else:
        names.update(git_lines(["diff", "--cached", "--name-only", "--diff-filter=ACMRT"]))
    return [ROOT / name for name in sorted(names)]

def scan(base: str | None = None):
    issues = []
    for p in candidate_files(base):
        if not p.is_file() or "test" not in p.name.lower() or p.suffix.lower() not in TEST_EXT:
            continue
        rel = p.relative_to(ROOT).as_posix()
        if rel == "scripts/test_cheat_guard.py":
            continue
        if any(part in IGNORE for part in p.parts):
            continue
        if any(rel.startswith(prefix) for prefix in IGNORE_PREFIXES):
            continue
        try:
            if p.stat().st_size > MAX_FILE_BYTES:
                continue
        except OSError:
            continue
        txt = p.read_text(encoding="utf-8", errors="ignore")
        for i, line in enumerate(txt.splitlines(), 1):
            if any(re.search(rx, line) for rx in SKIP_PATTERNS + WEAK_PATTERNS):
                issues.append(f"{p.relative_to(ROOT)}:{i}: {line.strip()}")
    return issues

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--base",
        help="scan everything this branch changes relative to this ref "
        "(e.g. origin/master) instead of the staged files",
    )
    args = parser.parse_args()

    if args.base and not ref_exists(args.base):
        # A silently empty diff would make this gate pass for the wrong reason.
        print(f"[test_cheat_guard] base ref not found: {args.base}", file=sys.stderr)
        raise SystemExit(2)

    found = scan(args.base)
    if found:
        print("[test_cheat_guard] blocked")
        print("\n".join(found[:200]))
        raise SystemExit(1)
    print("[test_cheat_guard] ok")
