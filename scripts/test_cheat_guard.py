#!/usr/bin/env python3
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]
IGNORE = {".git", "target", "node_modules", ".venv", "vendor", "vendov", "3d", "ChatGPT_files", "protoc_bin", "logs"}
TEST_EXT = {".py", ".rs", ".ts", ".tsx", ".js"}
SKIP_PATTERNS = [r"\.skip\(", r"pytest\.mark\.skip", r"#\[ignore\]", r"describe\.skip", r"xit\("]
WEAK_PATTERNS = [r"assert\s+true", r"assert\(true\)"]
MAX_FILE_BYTES = 1_000_000

def scan():
    issues = []
    for p in ROOT.rglob("*"):
        if not p.is_file() or "test" not in p.name.lower() or p.suffix.lower() not in TEST_EXT:
            continue
        if any(part in IGNORE for part in p.parts):
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
