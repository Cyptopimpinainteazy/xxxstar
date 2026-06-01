#!/usr/bin/env python3
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
IGNORE_DIRS = {".git", "target", "node_modules", ".venv", "vendor", "vendov", "3d", "ChatGPT_files", "protoc_bin", "logs"}
TEXT_EXT = {".rs", ".py", ".ts", ".tsx", ".js", ".jsx", ".sol", ".go", ".md", ".toml", ".yml", ".yaml", ".json", ".sh"}
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


def should_scan(path: pathlib.Path) -> bool:
    if any(part in IGNORE_DIRS for part in path.parts):
        return False
    try:
        if path.stat().st_size > MAX_FILE_BYTES:
            return False
    except OSError:
        return False
    return path.suffix.lower() in TEXT_EXT


def main() -> int:
    issues = []
    rx = [re.compile(p, re.IGNORECASE) for p in PATTERNS]
    for path in ROOT.rglob("*"):
        if not path.is_file() or not should_scan(path):
            continue
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue
        for i, line in enumerate(text.splitlines(), 1):
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
