#!/usr/bin/env python3
"""Fail when a crate is in neither the workspace `members` nor its `exclude`.

That gap is a trap: the crate inherits `workspace = true` dependencies, so cargo
refuses to build it standalone ("current package believes it's in a workspace
when it's not") and nothing in the workspace pulls it in either. Its code and its
tests are then never compiled anywhere — on 2026-09-18 that was true of 46 crates
under `crates/`, including `custody-service`, which did not compile at all.

The gate is a ratchet: `.ai/workspace-membership-baseline.txt` lists the crates
known to be in the gap today. A crate that is not in that file must be a member
or excluded, and a baseline entry that has been fixed must be removed (otherwise
the file stops meaning anything).

Usage:
  scripts/check-workspace-membership.py            # check, non-zero on drift
  scripts/check-workspace-membership.py --list     # print the baseline and exit
  scripts/check-workspace-membership.py --update   # rewrite the baseline
"""

from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys

try:  # Python 3.11+
    import tomllib
except ModuleNotFoundError:  # Python 3.10 and older
    # The gate is run by whatever python3 the box has. On 3.10 that is the
    # `tomli` backport; without either there is no TOML parser and the check
    # cannot mean anything, so say so rather than dying on an import. This is
    # the second script in the tree with this problem — feature_matrix.py had
    # the same one, and `local-ci.sh` reported this gate as a bare FAIL until
    # now.
    try:
        import tomli as tomllib
    except ModuleNotFoundError as exc:  # pragma: no cover - environment guard
        raise SystemExit(
            "check-workspace-membership needs a TOML parser: Python 3.11+ (tomllib) or the `tomli` backport"
        ) from exc

ROOT = pathlib.Path(__file__).resolve().parents[1]
BASELINE = ROOT / ".ai" / "workspace-membership-baseline.txt"
SEARCH_ROOTS = ("crates", "pallets", "services", "apps", "tools", "node", "runtime", "tests")


def workspace_lists() -> tuple[set[str], set[str]]:
    doc = tomllib.loads((ROOT / "Cargo.toml").read_text())
    workspace = doc.get("workspace", {})
    members = {entry.rstrip("/") for entry in workspace.get("members", [])}
    exclude = {entry.rstrip("/") for entry in workspace.get("exclude", [])}
    return members, exclude


def actual_members() -> set[str]:
    """Workspace members as cargo sees them, including path-dependency members.

    A crate that is a path dependency of a member is a member itself even though
    it never appears in `workspace.members`. That is how `x3-evolution` and
    `custody-service` joined during the 2026-09-18 burn-down, and reading only
    the manifest arrays would have missed it.
    """
    try:
        result = subprocess.run(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError:
        return set()
    if result.returncode != 0:
        return set()
    try:
        doc = json.loads(result.stdout)
    except json.JSONDecodeError:
        return set()
    paths: set[str] = set()
    for package in doc.get("packages", []):
        manifest = pathlib.Path(package["manifest_path"]).parent
        try:
            paths.add(manifest.relative_to(ROOT).as_posix())
        except ValueError:
            continue
    return paths


def candidate_manifests() -> list[pathlib.Path]:
    """Depth-1 manifests under each root, matching the workspace's own layout.

    Nested crates (e.g. `crates/x3-swarm-core/services/x3-swarm-worker`) are
    deliberately separate builds and are not part of this check.
    """
    found: list[pathlib.Path] = []
    for top in SEARCH_ROOTS:
        base = ROOT / top
        if not base.is_dir():
            continue
        direct = base / "Cargo.toml"
        if direct.is_file():
            found.append(direct)
        for manifest in sorted(base.glob("*/Cargo.toml")):
            found.append(manifest)
    return found


def load_baseline() -> set[str]:
    if not BASELINE.exists():
        return set()
    lines = BASELINE.read_text().splitlines()
    return {line.strip() for line in lines if line.strip() and not line.startswith("#")}


def write_baseline(entries: set[str]) -> None:
    BASELINE.parent.mkdir(parents=True, exist_ok=True)
    header = [
        "# Crates that are neither workspace `members` nor `exclude`d.",
        "#",
        "# Each entry is a crate whose code and tests are never compiled: it uses",
        "# `workspace = true` dependencies, so cargo will not build it standalone",
        "# either. Burn the list down by moving each crate to `members` (it joins",
        "# `cargo test --workspace`) or to `exclude` with a self-contained manifest",
        "# plus its own gate. `scripts/check-workspace-membership.py` fails when a",
        "# crate appears in neither list, or when a fixed entry stays here.",
        "",
    ]
    BASELINE.write_text("\n".join(header + sorted(entries)) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--list", action="store_true", help="print the baseline")
    parser.add_argument("--update", action="store_true", help="rewrite the baseline")
    args = parser.parse_args()

    listed_members, exclude = workspace_lists()
    resolved_members = actual_members()
    if resolved_members:
        members = resolved_members
    else:
        print("workspace membership check: cargo metadata unavailable; using the members array")
        members = listed_members
    in_limbo: dict[str, str] = {}
    for manifest in candidate_manifests():
        try:
            doc = tomllib.loads(manifest.read_text())
        except (OSError, tomllib.TOMLDecodeError):
            continue
        name = doc.get("package", {}).get("name")
        if not name:
            continue  # virtual manifest
        rel = manifest.parent.relative_to(ROOT).as_posix()
        if rel in members or rel in exclude:
            continue
        in_limbo[name] = rel

    current = set(in_limbo)
    baseline = load_baseline()

    if args.list:
        for name in sorted(baseline):
            print(f"{name}: {in_limbo.get(name, '(no longer in limbo)')}")
        return 0

    if args.update:
        write_baseline(current)
        print(f"baseline rewritten with {len(current)} crate(s)")
        return 0

    new_debt = sorted(current - baseline)
    fixed = sorted(baseline - current)

    print("workspace membership check")
    print(f"  crates in the gap:  {len(current)} (baseline {len(baseline)})")
    print(f"  new:                {len(new_debt)}")
    print(f"  fixed since:        {len(fixed)}")

    if not new_debt and not fixed:
        print("\nOK - no crate is in the members/exclude gap")
        return 0

    for name in new_debt:
        print(f"\n  x {name} ({in_limbo[name]}) is neither a workspace member nor excluded")
        print("    add it to `members` (then it is built and tested by cargo test --workspace)")
        print("    or to `exclude` with a self-contained manifest and its own gate")
    for name in fixed:
        print(f"\n  x baseline entry '{name}' is no longer in the gap")
        print("    remove it from .ai/workspace-membership-baseline.txt (run --update)")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
