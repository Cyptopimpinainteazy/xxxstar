#!/usr/bin/env python3
"""Fail when a fuzz target is a placeholder, unless it is on the shrinking list.

This is a **ratchet, not a clean bill of health.** All it checks is the marker this repository uses
for unfinished work: a target that contains `TODO`, or whose `fuzz/Cargo.toml` does not exist and so
can never be built. A target with neither marker is *not* thereby a good fuzzer — it may exercise
nothing, as most of this suite does; see `.ai/reports/fuzz-suite-20260925.md`.

What the ratchet buys: the suite cannot grow while it is already nominal. A new target that arrives
with a `TODO` in it — which is how all thirty of the original ones arrived — fails the build.

  scripts/check-fuzz-targets.py            # check, non-zero on drift
  scripts/check-fuzz-targets.py --list     # print every target and its verdict
"""
from __future__ import annotations

import argparse
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
BASELINE = ROOT / "security" / "fuzz-placeholder-baseline.txt"


def targets():
    found = []
    for path in ROOT.rglob("fuzz/fuzz_targets/*.rs"):
        relative = path.relative_to(ROOT)
        if any(part in {"target", "node_modules"} or part.startswith(".") for part in relative.parts[:-1]):
            continue
        found.append(path)
    return sorted(found)


def is_placeholder(path: pathlib.Path) -> str | None:
    """Why this target is unfinished, or None."""
    if "TODO" in path.read_text(errors="replace"):
        return "contains TODO"
    if not (path.parent.parent / "Cargo.toml").exists():
        return "no fuzz/Cargo.toml"
    return None


def load_baseline():
    if not BASELINE.exists():
        sys.exit("check-fuzz-targets: missing %s" % BASELINE.relative_to(ROOT))
    return {
        line.strip()
        for line in BASELINE.read_text().splitlines()
        if line.strip() and not line.startswith("#")
    }


def main():
    parser = argparse.ArgumentParser(description="Check fuzz targets against the placeholder ratchet.")
    parser.add_argument("--list", action="store_true", help="print every target and its verdict")
    args = parser.parse_args()

    baseline = load_baseline()
    found = []
    for path in targets():
        relative = str(path.relative_to(ROOT))
        reason = is_placeholder(path)
        found.append((relative, reason))

    if args.list:
        for relative, reason in found:
            mark = "placeholder" if reason else "            "
            print(f"{mark}  {relative}{('  (' + reason + ')') if reason else ''}")
        print()
        print("targets: %d, placeholders: %d, on the baseline: %d" % (
            len(found), sum(1 for _, r in found if r), len(baseline)))
        return 0

    failures = []
    for relative, reason in found:
        if reason and relative not in baseline:
            failures.append(
                "%s is a placeholder (%s) and is not on the baseline; finish it or remove it"
                % (relative, reason)
            )
    current = {relative for relative, reason in found if reason}
    for relative in sorted(baseline - current):
        failures.append(
            "%s is on the baseline but is no longer a placeholder (or no longer exists); remove it "
            "so the list keeps describing the tree" % relative
        )

    if failures:
        for failure in failures:
            print("check-fuzz-targets: FAIL: %s" % failure, file=sys.stderr)
        return 1

    print(
        "check-fuzz-targets: OK - %d target(s), %d placeholder(s), all on the shrinking list"
        % (len(found), len(current))
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
