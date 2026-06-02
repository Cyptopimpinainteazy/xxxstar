#!/usr/bin/env python3
import pathlib
import os
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
IGNORE_DIRS = {".git", "target", "node_modules", ".venv", "vendor", "vendov", "3d", "ChatGPT_files", "protoc_bin", "logs", ".toolchain"}
IGNORE_PREFIXES = (
    ".audit/",
    ".clinerules/",
    ".github/",
    ".kilo/",
    ".launchops/",
    ".roo/",
    ".repomix/",
    ".planning/",
    ".ai/",
    ".x3/graph/",
    "apps/x3-desktop/src-tauri/tauri-vendor/",
    "forge-std/",
    "packages/polkawallet-plugin/dist/",
    "proof/reports/",
    "tests/phase_core/security/lib/forge-std/",
)
TEXT_EXT = {".rs", ".py", ".ts", ".tsx", ".js", ".jsx", ".sol", ".go", ".sh"}
MAX_FILE_BYTES = 1_000_000

PATTERNS = [
    r"\bTODO\b",
    r"\bFIXME\b",
    r"\bstub\b",
    r"placeholder",
    r"panic!\(\s*\"stub",
    r"todo!\(",
    r"unimplemented!\(",
]
ALLOW_LINE_PATTERNS = [
    r"(?i)stub implementation for wasm32",
    r"(?i)Stub icu_properties to avoid rustc ICE",
    r"(?i)\bplaceholder\s*=",
    r"(?i)\bplaceholder-[a-z0-9-]+",
]


def git_lines(args: list[str]) -> list[str]:
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


def candidate_files() -> list[pathlib.Path]:
    names: set[str] = set()
    names.update(git_lines(["diff", "--cached", "--name-only", "--diff-filter=ACMRT"]))

    if os.environ.get("NO_STUB_FULL_PUSH_SCAN") == "1":
        upstream = git_lines(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"])
        if upstream:
            names.update(git_lines(["diff", "--name-only", "--diff-filter=ACMRT", f"{upstream[0]}..HEAD"]))

    if not names and os.environ.get("NO_STUB_FULL_REPO_SCAN") == "1":
        names.update(git_lines(["ls-files"]))

    return [ROOT / name for name in sorted(names)]


def should_scan(path: pathlib.Path) -> bool:
    rel = path.relative_to(ROOT).as_posix()
    if rel == "scripts/no_stub_guard.py":
        return False
    if any(part in IGNORE_DIRS for part in path.parts):
        return False
    if any(rel.startswith(prefix) for prefix in IGNORE_PREFIXES):
        return False
    try:
        if not path.exists():
            return False
        if path.stat().st_size > MAX_FILE_BYTES:
            return False
    except OSError:
        return False
    return path.suffix.lower() in TEXT_EXT


def main() -> int:
    issues = []
    rx = [re.compile(p, re.IGNORECASE) for p in PATTERNS]
    for path in candidate_files():
        if not path.is_file() or not should_scan(path):
            continue
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue
        for i, line in enumerate(text.splitlines(), 1):
            if any(re.search(allow, line) for allow in ALLOW_LINE_PATTERNS):
                continue
            if any(r.search(line) for r in rx):
                issues.append(f"{path.relative_to(ROOT)}:{i}: {line.strip()}")
    if issues:
        print("[no_stub_guard] blocked: stub/placeholder markers found")
        for item in issues[:200]:
            print(item)
        if len(issues) > 200:
            print(f"... and {len(issues)-200} more")
        return 1
    print("[no_stub_guard] ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
