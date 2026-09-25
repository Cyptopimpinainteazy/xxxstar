#!/usr/bin/env python3
"""Fail when a resolved dependency version falls inside a verified-vulnerable window.

GitHub's advisory metadata is sometimes coarser than the regression that actually
produced the bug. `GHSA-vxx9-2994-q338` (yamux) publishes `<0.13.10`, which also
matches the whole 0.12 line; the panic it describes was introduced in 0.13.9 and
fixed in 0.13.10, so no 0.12.x release ever carried it. Dependabot reports the
published range, cargo-audit has no RustSec id for that advisory at all, and
nothing in this repository would have noticed that the resolved versions sit
outside the window. That is the gap this check closes.

`security/advisory-scope.toml` records, per advisory, the window that is really
vulnerable, the window GitHub publishes, the versions this repository resolves
today, and the evidence for the distinction. This script fails when:

  * any resolved version of the package falls inside `vulnerable_range`; or
  * the resolved set drifts from `expected_resolved`, so that a lockfile bump
    cannot silently turn a recorded judgement into a stale one.

Usage:
  scripts/check-advisory-scope.py            # check, non-zero on drift
  scripts/check-advisory-scope.py --list     # print the records and exit
"""

from __future__ import annotations

import argparse
import os
import pathlib
import re
import sys

try:  # Python 3.11+
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - 3.10 and older
    try:
        import tomli as tomllib  # type: ignore
    except ModuleNotFoundError:
        sys.exit(
            "check-advisory-scope needs a TOML parser: Python 3.11+ (tomllib) "
            "or the `tomli` backport"
        )

ROOT = pathlib.Path(__file__).resolve().parent.parent
RECORDS = ROOT / "security" / "advisory-scope.toml"
# Directories that hold build output, vendored crate sources, or another
# agent's checkout rather than this tree's own manifests. Hidden directories are
# skipped wholesale: `.git`, `.wt-*` agent worktrees, `.kilo/worktrees`, and
# `.pre-edit-snapshot` all live there, and none of them is part of what a
# release builds.
SKIP_DIRS = {"target", "node_modules", "vendor"}


def _is_skipped(name):
    return name.startswith(".") or name in SKIP_DIRS or name.endswith("-vendor")

_COMPARATOR = re.compile(r"^(>=|<=|>|<|=)?\s*(\d+(?:\.\d+)*)$")


def lockfiles():
    """Every Cargo.lock in the tree except build output and vendored sources.

    A plain `rglob` would descend into `target/` and `node_modules/`, which is
    tens of thousands of stat calls for a gate that wants to be instant, so the
    excluded directories are pruned during the walk instead.
    """
    found = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = sorted(name for name in dirnames if not _is_skipped(name))
        if "Cargo.lock" in filenames:
            found.append(pathlib.Path(dirpath) / "Cargo.lock")
    return sorted(found)


def parse_lock(text):
    """Minimal [[package]] reader: (name, version) pairs, in file order."""
    packages = []
    name = None
    for raw in text.splitlines():
        line = raw.strip()
        if line.startswith("["):
            name = None
        elif line.startswith("name = "):
            name = line[len("name = "):].strip().strip('"')
        elif line.startswith("version = ") and name is not None:
            packages.append((name, line[len("version = "):].strip().strip('"')))
            name = None
    return packages


def parse_version(raw):
    core = raw.split("+")[0].split("-")[0]
    return tuple(int(part) for part in core.split("."))


def _padded(a, b):
    width = max(len(a), len(b))
    return a + (0,) * (width - len(a)), b + (0,) * (width - len(b))


def satisfies(version, spec):
    """True when `version` meets every comma-separated comparator in `spec`."""
    for part in spec.split(","):
        part = part.strip()
        match = _COMPARATOR.match(part)
        if not match:
            raise ValueError("unsupported version range component: %r" % (part,))
        operator = match.group(1) or "="
        left, right = _padded(version, parse_version(match.group(2)))
        ok = {
            ">=": left >= right,
            ">": left > right,
            "<=": left <= right,
            "<": left < right,
            "=": left == right,
        }[operator]
        if not ok:
            return False
    return True


def load_records():
    if not RECORDS.exists():
        sys.exit("check-advisory-scope: missing %s" % RECORDS.relative_to(ROOT))
    with RECORDS.open("rb") as handle:
        document = tomllib.load(handle)
    records = document.get("advisory", [])
    if not records:
        sys.exit("check-advisory-scope: security/advisory-scope.toml declares no [[advisory]] records")
    return records


def main():
    parser = argparse.ArgumentParser(description="Check resolved versions against verified advisory windows.")
    parser.add_argument("--list", action="store_true", help="print the records and exit")
    args = parser.parse_args()

    records = load_records()
    if args.list:
        for record in records:
            print(
                "%s  %s  published=%s  vulnerable=%s  resolved=%s" % (
                    record["id"],
                    record["package"],
                    record["published_range"],
                    record["vulnerable_range"],
                    sorted(record["expected_resolved"], key=parse_version),
                )
            )
        return 0

    locks = lockfiles()
    if not locks:
        print("check-advisory-scope: no Cargo.lock found in the tree", file=sys.stderr)
        return 1

    resolved = {}
    for path in locks:
        for name, version in parse_lock(path.read_text()):
            resolved.setdefault(name, {}).setdefault(version, []).append(str(path.relative_to(ROOT)))

    failures = []
    for record in records:
        package = record["package"]
        versions = sorted(resolved.get(package, {}), key=parse_version)
        expected = sorted(set(record["expected_resolved"]), key=parse_version)

        for version in versions:
            if satisfies(parse_version(version), record["vulnerable_range"]):
                where = ", ".join(resolved[package][version])
                failures.append(
                    "%s: resolved %s %s (%s) is inside the vulnerable window %s - see %s" % (
                        record["id"], package, version, where,
                        record["vulnerable_range"], record["evidence"],
                    )
                )

        if versions != expected:
            failures.append(
                "%s: %s resolves to %s but the record expects %s; re-verify the window and update "
                "%s (evidence: %s)" % (
                    record["id"], package, versions or ["<absent>"], expected,
                    RECORDS.relative_to(ROOT), record["evidence"],
                )
            )

        if not [v for v in versions if satisfies(parse_version(v), record["published_range"])]:
            print(
                "check-advisory-scope: note - no resolved %s version is inside the published range "
                "%s any more; the record is kept as the reason the alert could not be acted on, "
                "but it may now be obsolete." % (package, record["published_range"]),
                file=sys.stderr,
            )

    if failures:
        for failure in failures:
            print("check-advisory-scope: FAIL: %s" % failure, file=sys.stderr)
        return 1

    print(
        "check-advisory-scope: OK - %d advisory record(s) verified against %d lockfile(s)" % (
            len(records), len(locks),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
