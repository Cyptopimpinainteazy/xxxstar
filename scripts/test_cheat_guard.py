#!/usr/bin/env python3
import pathlib
import re
import subprocess

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
SKIP_PATTERNS = [r"\.skip\(", r"pytest\.mark\.skip", r"#\[ignore\]", r"describe\.skip", r"xit\("]
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


def candidate_files():
    names = set(git_lines(["diff", "--cached", "--name-only", "--diff-filter=ACMRT"]))
    return [ROOT / name for name in sorted(names)]

def scan():
    issues = []
    for p in candidate_files():
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
    found = scan()
    if found:
        print("[test_cheat_guard] blocked")
        print("\n".join(found[:200]))
        raise SystemExit(1)
    print("[test_cheat_guard] ok")
