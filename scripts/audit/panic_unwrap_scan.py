#!/usr/bin/env python3
"""Find panics that a production build can actually reach.

`scripts/mainnet/panic_unwrap_audit.sh` counted `unwrap()/expect()/panic!` with a
line-oriented grep and classified by file path. Because `#[cfg(test)] mod tests`
lives in the same file as the code it tests, that reported ~5,170 "production hot
path" findings, nearly all of them assertions in tests — a number too noisy to
act on, which is how the script came to always exit 0 while its own summary said
"gate: FAIL".

This scanner classifies the *enclosing code* instead:

  runtime-hook   the panic sits in `on_initialize`, `on_finalize` or
                 `offchain_worker` — a panic there stops block production
  pallet-call    the panic sits in a `#[pallet::call]` extrinsic body
  production     any other non-test code

Lines inside `#[cfg(test)]` items, and commented-out lines, are excluded: they do
not exist in the runtime or in a release node build. String literals and comments
are stripped before brace counting so `format!("{{}}")` cannot end a test module
early.

Output: JSON on stdout. Exit status is 0 when the scan itself succeeded.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys

PANIC_PATTERNS = (
    re.compile(r"\bpanic!\s*\("),
    re.compile(r"\.unwrap\s*\(\s*\)"),
    re.compile(r"\.expect\s*\("),
    re.compile(r"\bunreachable!\s*\("),
    re.compile(r"\btodo!\s*\("),
    re.compile(r"\bunimplemented!\s*\("),
)

RUNTIME_HOOKS = ("on_initialize", "on_finalize", "offchain_worker")

SKIP_DIR_PARTS = {
    "target",
    "node_modules",
    ".git",
    "vendor",
    "forge-std",
    "benchmarking",
    "fuzz",
    "mocks",
}


def strip_strings_and_comments(line: str, in_block_comment: bool) -> tuple[str, bool]:
    """Return the code-only part of `line` and the block-comment state.

    Braces inside string literals or comments must not move the depth counter, or
    a test module would appear to end early and its assertions would be counted
    as production panics (or, worse, real code would be skipped).
    """
    out: list[str] = []
    i = 0
    n = len(line)
    while i < n:
        if in_block_comment:
            end = line.find("*/", i)
            if end == -1:
                return "".join(out), True
            i = end + 2
            in_block_comment = False
            continue
        ch = line[i]
        if ch == "/" and i + 1 < n and line[i + 1] == "/":
            break
        if ch == "/" and i + 1 < n and line[i + 1] == "*":
            in_block_comment = True
            i += 2
            continue
        if ch == '"':
            i += 1
            while i < n:
                if line[i] == "\\":
                    i += 2
                    continue
                if line[i] == '"':
                    i += 1
                    break
                i += 1
            continue
        if ch == "'":
            # A char literal, not a lifetime: `'a'` is three characters.
            if i + 2 < n and line[i + 2] == "'":
                i += 3
                continue
            i += 1
            continue
        out.append(ch)
        i += 1
    return "".join(out), in_block_comment


def test_line_ranges(lines: list[str]) -> list[tuple[int, int]]:
    """1-based inclusive line ranges covered by `#[cfg(test)]` items."""
    ranges: list[tuple[int, int]] = []
    in_block_comment = False
    i = 0
    while i < len(lines):
        code, in_block_comment = strip_strings_and_comments(lines[i], in_block_comment)
        if "#[cfg(test)]" not in code:
            i += 1
            continue
        # Find the opening brace of the item this attribute belongs to.
        depth = 0
        started = False
        j = i
        block = in_block_comment
        while j < len(lines):
            text, block = strip_strings_and_comments(lines[j], block)
            for ch in text:
                if ch == "{":
                    depth += 1
                    started = True
                elif ch == "}":
                    depth -= 1
            if started and depth <= 0:
                ranges.append((i + 1, j + 1))
                break
            j += 1
        if not started:  # attribute without a body (e.g. on a `use`) — skip it
            i += 1
            continue
        i = j + 1
    return ranges


CFG_TEST_MODULE = re.compile(
    r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*mod\s+([A-Za-z0-9_]+)\s*;"
)


def test_only_files(roots: list[str]) -> set[str]:
    """Files that are only compiled under `cfg(test)`, by module declaration.

    A pallet declares its test support as `#[cfg(test)] mod tests;` /
    `mod mock;` in `lib.rs`, and those files' contents never exist in a
    production build. Line-level detection cannot see that — it only knows about
    `#[cfg(test)]` *items* inside a file — so `src/tests.rs` was scanned as
    production code and its assertions counted as panics reachable in a release
    node. That is the same mistake the line-oriented predecessor made, one level
    up, and it fired on this repository's own new tests.
    """
    out: set[str] = set()
    for root in roots:
        for dirpath, _dirnames, filenames in os.walk(root):
            if "Cargo.toml" not in filenames:
                continue
            for crate_root in ("lib.rs", "main.rs"):
                path = os.path.join(dirpath, "src", crate_root)
                if not os.path.exists(path):
                    continue
                with open(path, encoding="utf-8", errors="replace") as handle:
                    text = handle.read()
                for match in CFG_TEST_MODULE.finditer(text):
                    name = match.group(1)
                    out.add(os.path.normpath(os.path.join(dirpath, "src", f"{name}.rs")))
                    out.add(os.path.normpath(os.path.join(dirpath, "src", name)))
    return out


def scan_file(path: os.PathLike[str], test_only: set[str]) -> list[tuple[int, str]]:
    normalised = os.path.normpath(str(path))
    if normalised in test_only or any(
        normalised.startswith(prefix + os.sep) for prefix in test_only
    ):
        return []
    with open(path, encoding="utf-8", errors="replace") as handle:
        lines = handle.read().splitlines()
    ranges = test_line_ranges(lines)

    def in_test(line_no: int) -> bool:
        return any(start <= line_no <= end for start, end in ranges)

    hits: list[tuple[int, str]] = []
    for index, line in enumerate(lines):
        line_no = index + 1
        stripped = line.strip()
        if stripped.startswith("//"):
            continue
        if in_test(line_no):
            continue
        code, _ = strip_strings_and_comments(line, False)
        if not any(pattern.search(code) for pattern in PANIC_PATTERNS):
            continue
        hits.append((line_no, line.strip()))
    return hits


def enclosing_fn(lines: list[str], index: int) -> tuple[str, bool]:
    """Nearest `fn` declaration above `index`, and whether it is a pallet call."""
    for k in range(index, -1, -1):
        code, _ = strip_strings_and_comments(lines[k], False)
        match = re.search(r"\bfn\s+([A-Za-z0-9_]+)", code)
        if not match:
            continue
        is_call = False
        for attr in range(k, max(k - 12, -1), -1):
            attr_code, _ = strip_strings_and_comments(lines[attr], False)
            if "#[pallet::call" in attr_code:
                is_call = True
                break
            if re.search(r"\bfn\s+[A-Za-z0-9_]+", attr_code) and attr != k:
                break
        return match.group(1), is_call
    return "", False


def collect(roots: list[str]) -> dict:
    findings = {"runtime-hook": [], "pallet-call": [], "production": []}
    files = 0
    test_only = test_only_files(roots)
    for root in roots:
        for dirpath, dirnames, filenames in os.walk(root):
            dirnames[:] = [
                d
                for d in dirnames
                if d not in SKIP_DIR_PARTS and not d.startswith(".wt-")
            ]
            for name in sorted(filenames):
                if not name.endswith(".rs"):
                    continue
                path = os.path.join(dirpath, name)
                files += 1
                with open(path, encoding="utf-8", errors="replace") as handle:
                    lines = handle.read().splitlines()
                for line_no, text in scan_file(path, test_only):
                    fn_name, is_call = enclosing_fn(lines, line_no - 1)
                    if fn_name in RUNTIME_HOOKS:
                        kind = "runtime-hook"
                    elif is_call:
                        kind = "pallet-call"
                    else:
                        kind = "production"
                    findings[kind].append(
                        {"file": path, "line": line_no, "fn": fn_name, "text": text}
                    )
    return {
        "files_scanned": files,
        "counts": {kind: len(items) for kind, items in findings.items()},
        "runtime_hooks": sorted(
            f"{item['file']}:{item['line']}" for item in findings["runtime-hook"]
        ),
        "findings": findings,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "roots",
        nargs="*",
        default=["runtime", "node", "pallets", "crates"],
        help="directories to scan (default: runtime node pallets crates)",
    )
    parser.add_argument(
        "--no-findings",
        action="store_true",
        help="omit the per-line finding lists (smaller output)",
    )
    args = parser.parse_args()

    result = collect(args.roots)
    if args.no_findings:
        result.pop("findings")
    json.dump(result, sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
