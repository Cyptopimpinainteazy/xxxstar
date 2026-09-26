#!/usr/bin/env python3
"""Do the feature matrix's `required_tests` citations name tests that exist?

`scripts/check-readiness-consistency.sh` proves every `required_tests` name in `FEATURE_REGISTRY.toml`
exists as a real test function. The *matrix* fragments carry the same field and nothing checked them:
`X3-XVM-006` could cite `a_test_that_does_not_exist_anywhere` and every gate still passed — measured
2026-09-26, which is what prompted this script.

A citation is evidence, so it has to resolve. This reads every `feature-matrix/*.toml` fragment, takes
each row's `required_tests` and `paths`, and requires `fn <name>` to exist under one of those paths
(a file path means the directory containing it, so a row may cite `.../src/lib.rs` while its tests
live in `.../src/tests.rs`).

    scripts/ci/check-matrix-tests-exist.py           # check
    scripts/ci/check-matrix-tests-exist.py --list    # print every citation and where it resolved

Exit 0 -> every citation resolves. Exit 1 -> at least one names a test that does not exist.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
FRAGMENTS = sorted((ROOT / "feature-matrix").glob("*.toml"))

FN = re.compile(r"fn\s+([A-Za-z0-9_]+)\s*[(<]")


def corpus_for(paths: list[str]) -> str:
    """Concatenate every .rs file under the row's paths (a file path means its directory)."""
    text = []
    for raw in paths:
        target = ROOT / raw
        if target.is_file():
            target = target.parent
        if not target.exists():
            continue
        for rs in target.rglob("*.rs"):
            try:
                text.append(rs.read_text(encoding="utf-8", errors="replace"))
            except OSError:
                continue
    return "\n".join(text)


def names(text: str) -> set[str]:
    return set(FN.findall(text))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--list", action="store_true", help="print every citation and its resolution")
    args = parser.parse_args()

    missing: list[str] = []
    checked = 0
    for fragment in FRAGMENTS:
        try:
            data = tomllib.loads(fragment.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as err:
            print(f"check-matrix-tests-exist: {fragment.name} is not valid TOML: {err}", file=sys.stderr)
            return 2
        for feature in data.get("feature", []):
            citations = feature.get("required_tests") or []
            if not citations:
                continue
            defined = names(corpus_for(feature.get("paths") or []))
            for citation in citations:
                checked += 1
                # `module::name` cites the tail; the field is for fn names.
                wanted = citation.split("::")[-1]
                if wanted in defined:
                    if args.list:
                        print(f"ok    {feature.get('id')}  {citation}")
                else:
                    missing.append(f"{feature.get('id')}: required_tests cites {citation!r} but no fn {wanted} exists under {feature.get('paths')}")
                    if args.list:
                        print(f"MISS  {feature.get('id')}  {citation}")

    if args.list:
        print(f"\n{checked} citation(s) checked, {len(missing)} unresolved")
        return 0

    if missing:
        print("check-matrix-tests-exist: FAIL: citations that resolve to nothing:", file=sys.stderr)
        for line in missing:
            print(f"  {line}", file=sys.stderr)
        return 1

    print(f"check-matrix-tests-exist: OK - {checked} required_tests citation(s) resolve to real fn names")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
