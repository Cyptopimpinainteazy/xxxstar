#!/usr/bin/env python3
"""Do the feature matrix's `required_tests` citations name tests that exist?

`scripts/check-readiness-consistency.sh` proves every `required_tests` name in `FEATURE_REGISTRY.toml`
exists as a real test function. The *matrix* fragments carry the same field and nothing checked them:
`X3-XVM-006` could cite `a_test_that_does_not_exist_anywhere` and every gate still passed — measured
2026-09-26, which is what prompted this script.

A citation is evidence, so it has to resolve. This reads every `feature-matrix/*.toml` fragment, takes
each row's `required_tests`, `paths` and `test_paths`, and requires `fn <name>` to exist under one of
those paths (a file path means the directory containing it, so a row may cite `.../src/lib.rs`
while its tests live in `.../src/tests.rs`).

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


def search_paths(feature: dict) -> list[str]:
    paths = feature.get("paths") or []
    test_paths = feature.get("test_paths") or []
    if not isinstance(paths, list) or not isinstance(test_paths, list):
        return []
    return [
        value
        for value in paths + test_paths
        if isinstance(value, str) and value.strip()
    ]


def citation_missing(feature: dict, citation: str) -> str | None:
    paths = search_paths(feature)
    wanted = citation.split("::")[-1]
    if wanted in names(corpus_for(paths)):
        return None
    return (
        f"{feature.get('id')}: required_tests cites {citation!r} "
        f"but no fn {wanted} exists under {paths}"
    )


def unresolved_for(feature: dict) -> list[str]:
    citations = feature.get("required_tests") or []
    if not citations:
        return []
    return [
        problem
        for citation in citations
        if (problem := citation_missing(feature, citation)) is not None
    ]


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
            feature_missing = unresolved_for(feature)
            checked += len(feature.get("required_tests") or [])
            missing.extend(feature_missing)
            if args.list:
                for citation in feature.get("required_tests") or []:
                    status = (
                        "MISS"
                        if citation_missing(feature, citation) is not None
                        else "ok  "
                    )
                    print(f"{status}  {feature.get('id')}  {citation}")

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
