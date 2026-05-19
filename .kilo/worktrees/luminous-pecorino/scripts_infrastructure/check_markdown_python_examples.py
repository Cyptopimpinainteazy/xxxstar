#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

from mktestdocs import check_md_file

EXCLUDED_PARTS = {
    "node_modules",
    "target",
    "patches",
    "forge-std",
    "tests/security/lib",
    "botchain-tri-vm-genesis",
    "Plan_v1___Boost_Blockchain_Speed",
    "Plan_v2___Boost_Blockchain_Speed",
    "Plan_v3___Boost_Blockchain_Speed",
    "Plan_v4___Boost_Blockchain_Speed",
    "Plan_v5___Boost_Blockchain_Speed",
}

PYTHON_BLOCK_RE = re.compile(r"^```python\\b", re.IGNORECASE | re.MULTILINE)


def should_skip(path: Path) -> bool:
    text = path.as_posix()
    return any(part in text for part in EXCLUDED_PARTS)


def has_python_blocks(path: Path) -> bool:
    try:
        content = path.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        return False
    return PYTHON_BLOCK_RE.search(content) is not None


def iter_markdown_files(root: Path):
    for file_path in root.rglob("*.md"):
        if should_skip(file_path):
            continue
        if has_python_blocks(file_path):
            yield file_path


def main() -> int:
    root = Path.cwd()
    markdown_files = sorted(iter_markdown_files(root))

    if not markdown_files:
        print("No markdown files with python code blocks found.")
        return 0

    failures = []
    for file_path in markdown_files:
        try:
            check_md_file(fpath=file_path, lang="python", memory=True)
            print(f"PASS {file_path}")
        except Exception as exc:
            failures.append((file_path, exc))
            print(f"FAIL {file_path}: {exc}")

    if failures:
        print(f"\\nDetected {len(failures)} markdown python example failure(s).")
        return 1

    print(f"\\nValidated python markdown examples in {len(markdown_files)} file(s).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
