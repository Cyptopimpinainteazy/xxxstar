#!/usr/bin/env python3
"""Do the names a matrix `test_evidence` note cites resolve to anything real?

`check-matrix-tests-exist.py` (added the same day) validates the structured `required_tests` field.
The free-text `test_evidence` notes cite tests and files too — "`node/tests/x3vm_live_lifecycle.rs`
runs a_compiled_x3_program_is_finalized_and_its_receipt_is_readable" — and nothing checked those. A
note is evidence, so the names in it have to resolve.

The rule, chosen so that ordinary prose does not trip it:

    an identifier in a test_evidence note counts as a citation when it starts with `test_` or
    contains at least two underscores, and it must resolve to

      * a `fn <name>` anywhere in the tree, or
      * a file name, file stem, or directory name anywhere in the tree

Lowercase English does not carry two underscores, so the check stays quiet on paragraphs while
catching an invented test name or a path that moved. Identifiers that resolve are ignored even if
they are also words, which is why `event_proof_extraction` and `six_internal_routes_strict_invariants_and_replay_guards`
both pass — the first names a field that exists, the second a test.

    scripts/ci/check-matrix-test-evidence.py           # check
    scripts/ci/check-matrix-test-evidence.py --list    # print every citation it resolved
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
SKIP = {"target", ".git", "node_modules"}
FN = re.compile(r"fn\s+([A-Za-z0-9_]+)\s*[(<]")
IDENT = re.compile(r"\b([a-z][a-z0-9_]{5,})\b")


def prune(dirnames: list[str]) -> None:
    """Never descend into build output or other agents' worktrees."""
    dirnames[:] = [d for d in dirnames if d not in SKIP and not d.startswith(".wt-")]


def resolvable() -> set[str]:
    names: set[str] = set()
    for dirpath, dirnames, filenames in os.walk(ROOT):
        # A directory name is a legitimate citation ("programs/svm/x3_atomic_swap/").
        names.update(dirnames)
        prune(dirnames)
        for filename in filenames:
            names.add(filename)
            names.add(filename.rsplit(".", 1)[0])
            if filename.endswith(".rs"):
                try:
                    text = Path(dirpath, filename).read_text(encoding="utf-8", errors="replace")
                except OSError:
                    continue
                names |= set(FN.findall(text))
    return names


def is_citation(name: str) -> bool:
    return name.startswith("test_") or name.count("_") >= 2


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--list", action="store_true", help="print every citation and its resolution")
    args = parser.parse_args()

    known = resolvable()
    missing: list[str] = []
    checked = 0

    for fragment in sorted((ROOT / "feature-matrix").glob("*.toml")):
        try:
            data = tomllib.loads(fragment.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as err:
            print(f"check-matrix-test-evidence: {fragment.name} is not valid TOML: {err}", file=sys.stderr)
            return 2
        for feature in data.get("feature", []):
            for note in feature.get("test_evidence") or []:
                for name in IDENT.findall(note):
                    if not is_citation(name):
                        continue
                    checked += 1
                    if name in known:
                        if args.list:
                            print(f"ok    {feature.get('id')}  {name}")
                    else:
                        missing.append(
                            f"{feature.get('id')}: test_evidence cites {name!r}, which is not a fn name, "
                            f"file, stem or directory anywhere in the tree"
                        )
                        if args.list:
                            print(f"MISS  {feature.get('id')}  {name}")

    if args.list:
        print(f"\n{checked} citation(s) checked, {len(missing)} unresolved")
        return 0

    if missing:
        print("check-matrix-test-evidence: FAIL: citations that resolve to nothing:", file=sys.stderr)
        for line in missing:
            print(f"  {line}", file=sys.stderr)
        return 1

    print(f"check-matrix-test-evidence: OK - {checked} citation(s) in test_evidence notes resolve")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
