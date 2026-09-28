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

Exit 0 → nothing to do, or the record moved with the change.
Exit 1 → the change can alter the runtime and the record did not move.
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


def changed_files(base: str) -> list[str]:
    """Files the outgoing change touches, committed and not."""
    files: set[str] = set()
    for args in (["git", "diff", "--name-only", f"{base}...HEAD"], ["git", "diff", "--name-only", "HEAD"]):
        result = run(args)
        if result.returncode == 0:
            files.update(line for line in result.stdout.splitlines() if line.strip())
    return sorted(files)


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

    changed = changed_files(args.base)
    if not changed:
        print(f"[runtime-hash] nothing changed against {args.base}; nothing to check")
        return 0

    if RECORD in changed:
        print(f"[runtime-hash] the change touches {RECORD}")
        return 0

    try:
        graph_dirs = runtime_graph_dirs()
    except RuntimeError as exc:
        print(f"[runtime-hash] could not compute the runtime dependency graph: {exc}", file=sys.stderr)
        return 2

    offenders = [
        path
        for path in changed
        if any(parent in pathlib.Path(path).parents for parent in graph_dirs)
    ]
    if not offenders:
        print(
            f"[runtime-hash] {len(changed)} file(s) changed, none in the runtime's "
            f"dependency graph ({len(graph_dirs)} packages) — nothing to do"
        )
        return 0

    print(
        f"[runtime-hash] {len(offenders)} changed file(s) can alter the runtime that "
        f"mainnet governance attests to, but {RECORD} did not move:",
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
