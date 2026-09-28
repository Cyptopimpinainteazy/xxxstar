#!/usr/bin/env python3
"""Fail when a change can have altered the runtime WASM but the hash record did not move.

`scripts/mainnet_release_gate.py` stage 6b rebuilds the runtime inside the
pinned srtool image and fails when the result no longer matches
`docs/reports/runtime-wasm-hashes.json`. That check is the real proof, and it is
ten minutes into a release — so a runtime change that lands without the record
turns up as somebody else's red gate later. It has done that three times in one
day.

This is the cheap early warning: if the outgoing diff touches any package in the
runtime's dependency graph, the same change must also touch the record.

    ./scripts/update-runtime-hashes.sh   # rebuild twice, refuse unless they agree,
                                         # write the record, print the doc pairs

The dependency set comes from `cargo metadata` rather than from path prefixes,
so a change to a crate *outside* the runtime's graph (the sidecar, the mobile
SDK, a test-only crate) does not trigger it.

There are two ways a change is seen, and the second one is the load-bearing one:

* `--base` (default `origin/master`), for the pull-request shape where the
  branch under review is measured against its target.
* **the record's own `recorded_revision`**, for the shape this repository
  actually lands in. Commits here go straight to `master`, so `--base
  origin/master` and `HEAD` are the same commit and the first check compares a
  revision against itself: measured on 2026-09-26, sixty-four files in the
  runtime's dependency graph had changed since `recorded_revision 335a27d8c`
  while this gate reported nothing and only the ten-minute stage 6b could have
  caught it. A freshness check that cannot see the commits is not a freshness
  check, so the record's revision is a tripwire in its own right: anything in
  the graph that changed since the revision the record names, and the record
  itself not among them, fails.

Exit 0 → nothing to do, or the record moved with the change.
Exit 1 → the change can alter the runtime and the record did not move, or the
         record cannot be tied to a revision at all.
Exit 2 → the check could not run (no cargo, no metadata); nothing was verified.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
RECORD = "docs/reports/runtime-wasm-hashes.json"
RUNTIME_PACKAGE = "x3-chain-runtime"


def run(cmd: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)


def git_files(args: list[str]) -> set[str]:
    """Paths a `git` invocation prints, or nothing if it could not run."""
    result = run(["git", *args])
    if result.returncode != 0:
        return set()
    return {line for line in result.stdout.splitlines() if line.strip()}


def worktree_files() -> set[str]:
    """Tracked modifications plus untracked, unignored files."""
    return git_files(["diff", "--name-only", "HEAD"]) | git_files(
        ["ls-files", "--others", "--exclude-standard"]
    )


def recorded_revision() -> tuple[str | None, str | None]:
    """The revision the record was built at, or a reason it cannot be used.

    Returns `(revision, None)` when the record names a commit that exists here,
    and `(None, reason)` when it does not. An unattachable record is a defect,
    not a reason to skip the check.
    """
    path = ROOT / RECORD
    if not path.exists():
        return None, f"{RECORD} does not exist, so nothing ties the runtime source to an attested hash"
    try:
        record = json.loads(path.read_text())
    except (OSError, ValueError) as exc:
        return None, f"{RECORD} is not readable JSON: {exc}"
    revision = record.get("recorded_revision")
    if not isinstance(revision, str) or not revision.strip():
        return None, f"{RECORD} carries no recorded_revision"
    resolved = run(["git", "rev-parse", "--verify", "--quiet", f"{revision}^{{commit}}"])
    if resolved.returncode != 0:
        return None, (
            f"recorded_revision {revision!r} in {RECORD} is not a commit in this "
            f"repository, so the record cannot be tied to any source"
        )
    return revision, None


def runtime_graph_dirs() -> set[pathlib.Path]:
    """Workspace directories of every package `x3-chain-runtime` depends on."""
    result = run(["cargo", "metadata", "--format-version", "1"])
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip()[:400] or "cargo metadata failed")

    metadata = json.loads(result.stdout)
    packages = {pkg["id"]: pkg for pkg in metadata["packages"]}
    root = next(
        (pkg["id"] for pkg in metadata["packages"] if pkg["name"] == RUNTIME_PACKAGE),
        None,
    )
    if root is None:
        raise RuntimeError(f"{RUNTIME_PACKAGE} is not in this workspace")

    nodes = {node["id"]: node["deps"] for node in metadata["resolve"]["nodes"]}
    seen: set[str] = set()
    stack = [root]
    while stack:
        package_id = stack.pop()
        if package_id in seen or package_id not in nodes:
            continue
        seen.add(package_id)
        stack.extend(dep["pkg"] for dep in nodes[package_id])

    dirs: set[pathlib.Path] = set()
    for package_id in seen:
        package = packages.get(package_id)
        if package is None:
            continue
        manifest = pathlib.Path(package["manifest_path"])
        try:
            dirs.add(manifest.parent.relative_to(ROOT))
        except ValueError:
            continue  # a registry or git package; not our tree
    return dirs


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--base",
        default="origin/master",
        help="ref to diff against (default: origin/master)",
    )
    args = parser.parse_args()

    revision, unattached = recorded_revision()

    vs_base = git_files(["diff", "--name-only", f"{args.base}...HEAD"])
    worktree = worktree_files()
    # Committed work since the revision the record names. This is the check that
    # survives the direct-to-master flow, where `--base` is `HEAD` and the first
    # two sets would compare a revision against itself.
    since_record = git_files(["diff", "--name-only", f"{revision}..HEAD"]) if revision else set()
    changed = since_record | vs_base | worktree
    described = (
        f"since the record was taken at {revision}"
        if since_record
        else f"against {args.base} and since the record at {revision}"
    )

    if unattached is not None:
        print(f"[runtime-hash] {unattached}", file=sys.stderr)
        print(
            "\n    Run ./scripts/update-runtime-hashes.sh to rebuild and re-record it.",
            file=sys.stderr,
        )
        return 1

    if not changed:
        print(
            f"[runtime-hash] nothing changed {described} and the record's revision "
            f"is current; nothing to check"
        )
        return 0

    try:
        graph_dirs = runtime_graph_dirs()
    except RuntimeError as exc:
        print(f"[runtime-hash] could not compute the runtime dependency graph: {exc}", file=sys.stderr)
        return 2

    def in_graph(paths: set[str]) -> list[str]:
        return [
            path
            for path in sorted(paths)
            if any(parent in pathlib.Path(path).parents for parent in graph_dirs)
        ]

    # The record's revision is the decisive tripwire: if it was not moved, a
    # runtime change after it is stale, and touching the record's text later
    # does not fix that. Only the two change sets that a re-attestation *is*
    # allowed to accompany — the outgoing branch diff and the working tree —
    # are excused by a record edit in the same set.
    offenders = in_graph(since_record)
    excused: list[str] = []
    for files in (vs_base, worktree):
        found = in_graph(files)
        if found and RECORD in files:
            excused.extend(found)
        else:
            offenders.extend(found)

    if not offenders:
        detail = f"{len(graph_dirs)} packages"
        if excused:
            print(
                f"[runtime-hash] {RECORD} moved with {len(excused)} runtime-graph "
                f"file(s) in the working tree or the outgoing diff — the release gate's "
                f"rebuild is what proves the new hashes ({detail})"
            )
        else:
            print(
                f"[runtime-hash] nothing in the runtime's dependency graph changed "
                f"({detail}) — nothing to do"
            )
        return 0

    print(
        f"[runtime-hash] {len(offenders)} changed file(s) {described} can alter the "
        f"runtime that mainnet governance attests to, but {RECORD} did not move:",
        file=sys.stderr,
    )
    for path in offenders[:20]:
        print(f"    {path}", file=sys.stderr)
    if len(offenders) > 20:
        print(f"    … and {len(offenders) - 20} more", file=sys.stderr)
    print(
        "\n    Run ./scripts/update-runtime-hashes.sh and commit the record with this\n"
        "    change. It builds the runtime twice, refuses to write unless the two\n"
        "    builds agree, and records the revision. If the WASM turns out to be\n"
        "    unchanged (a code path the runtime never instantiates, for example), the\n"
        "    hashes stay the same and only the revision line moves.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
