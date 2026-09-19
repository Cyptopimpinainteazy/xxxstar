#!/usr/bin/env python3
"""Fail when an invariant in `tests/invariants/registry.toml` is referenced by nothing.

This used to be `tests_core/invariant_registry_check.rs`, a `#[test]` that no
crate, script or Makefile ever compiled — the same class as the orphan
`runtime/src/tests.rs`. Ported here so the CI of record enforces it: a registry
entry with no test mentioning it is an invariant nobody checks.

Exit status is 1 when any id is unreferenced, 0 otherwise (including when the
registry itself is missing, which is reported as a failure too — a
silently-passing meta test is how this file went unnoticed).
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "tests" / "invariants" / "registry.toml"

# First-party trees a reference can live in. Vendored third-party code is not
# evidence that *we* test an invariant.
SEARCH_ROOTS = ("tests", "tests_core", "pallets", "crates", "runtime", "x3-lang", "programs")
SEARCH_SUFFIXES = (".rs", ".ts", ".tsx", ".py", ".sh", ".toml", ".json", ".md")


def registry_ids() -> list[str]:
    text = REGISTRY.read_text()
    return re.findall(r'^id\s*=\s*"([^"]+)"', text, re.M)


def candidate_texts() -> list[tuple[pathlib.Path, str]]:
    """Read each candidate file once (one pass, not one per invariant)."""
    texts: list[tuple[pathlib.Path, str]] = []
    for top in SEARCH_ROOTS:
        base = ROOT / top
        if not base.is_dir():
            continue
        for path in base.rglob("*"):
            if not path.is_file() or path.suffix not in SEARCH_SUFFIXES:
                continue
            if path == REGISTRY:
                continue
            try:
                texts.append((path.relative_to(ROOT), path.read_text(errors="ignore")))
            except OSError:
                continue
    return texts


def main() -> int:
    if not REGISTRY.is_file():
        print(f"invariant registry: {REGISTRY.relative_to(ROOT)} is missing", file=sys.stderr)
        return 1

    ids = registry_ids()
    if not ids:
        print("invariant registry: no invariant ids parsed from the registry", file=sys.stderr)
        return 1

    texts = candidate_texts()
    unreferenced = [inv for inv in ids if not any(inv in body for _, body in texts)]

    print("invariant registry check")
    print(f"  invariants:     {len(ids)} (searched {len(texts)} files)")
    print(f"  unreferenced:   {len(unreferenced)}")

    if unreferenced:
        print(
            "\nThese invariants are registered but no test references them — either "
            "write the test or drop the entry:",
            file=sys.stderr,
        )
        for inv in unreferenced:
            print(f"  x {inv}", file=sys.stderr)
        return 1

    print("\nOK - every registered invariant is referenced by a test")
    return 0


if __name__ == "__main__":
    sys.exit(main())
