#!/usr/bin/env python3
"""Inventory exact and structural source duplication without external tools.

The default scan follows tracked source files and excludes vendor dumps,
generated output, audit evidence, and frozen launch-gate source copies. A
"structural clone" is a window of normalized source lines that occurs in two
different files; whitespace and line comments do not defeat the comparison.

This is a technical-debt inventory, not an automatic refactor instruction.
Shared framework boilerplate can be legitimate. `--fail-on-findings` is meant
for a deliberate zero-duplication CI gate, not as a default for this repository.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable

SOURCE_SUFFIXES = {
    ".rs",
    ".sol",
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".mjs",
    ".cjs",
    ".py",
    ".sh",
    ".go",
    ".cu",
    ".cuh",
}

DEFAULT_EXCLUDED_PREFIXES = (
    ".ai/",
    ".git/",
    ".pre-edit-snapshot/",
    ".taskplane/",
    "_bmad/",
    "_bmad-output/",
    "apps/x3-desktop/src-tauri/tauri-vendor/",
    "audit-artifacts/",
    "forge-std/",
    "launch-gates/sources/",
    "memory/",
    "patches/",
    "proof/",
    "reports/",
    "target/",
    "tests/phase_core/security/lib/forge-std/",
    "tests_core/",
    "tests_phase4/",
    "X3-contracts/evm/cache/",
    "X3-contracts/evm/lib/",
    "X3-contracts/evm/out/",
)

EXCLUDED_SEGMENTS = {
    ".next",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "out",
    "vendor",
}

COMMENT_PREFIXES = ("//", "#", "/*", "*", "--")


def _is_excluded(rel: str) -> bool:
    if rel.startswith(DEFAULT_EXCLUDED_PREFIXES):
        return True
    parts = Path(rel).parts
    if any(part.startswith(".") for part in parts[:-1]):
        return True
    return any(part in EXCLUDED_SEGMENTS for part in parts)


def _tracked_files(root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.decode("utf-8", errors="replace").strip())
    return [
        item.decode("utf-8")
        for item in result.stdout.split(b"\0")
        if item
    ]


def _source_paths(root: Path, include_archives: bool) -> list[str]:
    paths = []
    for rel in _tracked_files(root):
        if Path(rel).suffix not in SOURCE_SUFFIXES:
            continue
        if not include_archives and _is_excluded(rel):
            continue
        if (root / rel).is_file():
            paths.append(rel)
    return sorted(paths)


def _normalized_lines(path: Path) -> list[str]:
    lines = []
    for raw in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        line = raw.strip()
        if not line or line.startswith(COMMENT_PREFIXES):
            continue
        line = re.sub(r"\s+//.*$", "", line)
        line = re.sub(r"\s+--.*$", "", line)
        line = re.sub(r"\s+", "", line)
        if line:
            lines.append(line)
    return lines


def _digest(values: Iterable[str]) -> str:
    payload = "\n".join(values).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _merge_clone_matches(
    matches: list[tuple[str, str, int, int]], minimum_lines: int
) -> list[dict[str, Any]]:
    merged: list[dict[str, Any]] = []
    for file_a, file_b, start_a, start_b in sorted(matches):
        current = merged[-1] if merged else None
        overlapping = (
            current is not None
            and current["file_a"] == file_a
            and current["file_b"] == file_b
            and start_a <= current["a_end"]
            and start_b <= current["b_end"]
        )
        if overlapping:
            current["a_end"] = max(current["a_end"], start_a + minimum_lines)
            current["b_end"] = max(current["b_end"], start_b + minimum_lines)
            current["lines"] = max(
                current["a_end"] - current["a_start"],
                current["b_end"] - current["b_start"],
            )
            continue
        merged.append(
            {
                "file_a": file_a,
                "file_b": file_b,
                "a_start": start_a,
                "a_end": start_a + minimum_lines,
                "b_start": start_b,
                "b_end": start_b + minimum_lines,
                "lines": minimum_lines,
            }
        )
    return merged


def scan(root: Path, minimum_lines: int = 12, include_archives: bool = False) -> dict[str, Any]:
    """Return exact duplicate files and structural clone windows."""
    if minimum_lines < 2:
        raise ValueError("minimum_lines must be at least 2")

    paths = _source_paths(root, include_archives)
    normalized_by_file: dict[str, list[str]] = {}
    exact_groups: dict[str, list[str]] = defaultdict(list)

    for rel in paths:
        lines = _normalized_lines(root / rel)
        if len(lines) < minimum_lines:
            continue
        normalized_by_file[rel] = lines
        exact_groups[_digest(lines)].append(rel)

    exact_duplicates = [
        {"sha256": digest, "files": sorted(files), "lines": len(normalized_by_file[files[0]])}
        for digest, files in exact_groups.items()
        if len(files) > 1
    ]
    exact_duplicates.sort(key=lambda item: (-item["lines"], item["files"]))

    windows: dict[str, list[tuple[str, int]]] = defaultdict(list)
    for rel, lines in normalized_by_file.items():
        for start in range(0, len(lines) - minimum_lines + 1):
            window_hash = _digest(lines[start : start + minimum_lines])
            windows[window_hash].append((rel, start))

    pair_matches: dict[tuple[str, str], list[tuple[str, str, int, int]]] = defaultdict(list)
    for occurrences in windows.values():
        by_file: dict[str, list[int]] = defaultdict(list)
        for rel, start in occurrences:
            by_file[rel].append(start)
        files = sorted(by_file)
        for index, file_a in enumerate(files):
            for file_b in files[index + 1 :]:
                for start_a in by_file[file_a]:
                    for start_b in by_file[file_b]:
                        pair_matches[(file_a, file_b)].append(
                            (file_a, file_b, start_a, start_b)
                        )

    structural_clones = []
    for matches in pair_matches.values():
        structural_clones.extend(_merge_clone_matches(matches, minimum_lines))
    structural_clones.sort(key=lambda item: (-item["lines"], item["file_a"], item["file_b"]))

    return {
        "schema": "x3-code-duplication-audit-v1",
        "root": str(root),
        "policy": {
            "source_suffixes": sorted(SOURCE_SUFFIXES),
            "excluded_prefixes": list(DEFAULT_EXCLUDED_PREFIXES),
            "excluded_segments": sorted(EXCLUDED_SEGMENTS),
            "include_archives": include_archives,
            "minimum_lines": minimum_lines,
        },
        "summary": {
            "source_files": len(paths),
            "compared_files": len(normalized_by_file),
            "exact_duplicate_groups": len(exact_duplicates),
            "exact_duplicate_files": sum(len(group["files"]) for group in exact_duplicates),
            "structural_clone_groups": len(structural_clones),
        },
        "exact_duplicate_groups": exact_duplicates,
        "structural_clone_groups": structural_clones,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--min-lines", type=int, default=12)
    parser.add_argument("--include-archives", action="store_true")
    parser.add_argument("--json", action="store_true", help="emit JSON instead of a summary")
    parser.add_argument("--output", type=Path, help="write the JSON report to this path")
    parser.add_argument(
        "--fail-on-findings",
        action="store_true",
        help="exit 1 when any exact or structural duplicate is present",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = scan(args.root.resolve(), args.min_lines, args.include_archives)
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")

    if args.json:
        sys.stdout.write(rendered)
    else:
        summary = report["summary"]
        print(
            "code duplication audit: "
            f"{summary['source_files']} source files, "
            f"{summary['exact_duplicate_groups']} exact duplicate group(s), "
            f"{summary['structural_clone_groups']} structural clone(s)"
        )
        for group in report["exact_duplicate_groups"][:10]:
            print(f"  exact {group['lines']} lines: {', '.join(group['files'])}")
        for group in report["structural_clone_groups"][:10]:
            print(
                "  clone "
                f"{group['file_a']}:{group['a_start']}-{group['a_end']} -> "
                f"{group['file_b']}:{group['b_start']}-{group['b_end']}"
            )

    has_findings = bool(
        report["exact_duplicate_groups"] or report["structural_clone_groups"]
    )
    return 1 if args.fail_on_findings and has_findings else 0


if __name__ == "__main__":
    raise SystemExit(main())
