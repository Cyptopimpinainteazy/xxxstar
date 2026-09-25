#!/usr/bin/env python3
"""Measure which crates with tests no gate names. **A census, not a gate.**

Written on 2026-09-25 to answer a question `scripts/check-registry-tests-are-gated.py` only asks of
crates the readiness registry cites: of everything in this tree that has tests, what does no gate run?

What it reports, measured that day:

  * the root workspace has 194 crates, 183 with tests, and **22 of those in the fast set** — the
    other 161 are covered by `test workspace:` in `GATES_DEEP`, which is a choice about how long a
    push takes rather than a suite nobody runs;
  * outside the root workspace, 31 crates with tests, of which the largest are gated by
    whole-workspace commands (`x3-lang`'s six crates, `x3-cross-vm-coordinator`, `x3-sidecar`).

It is **not** wired into `scripts/local-ci.sh`, because deciding "is this crate gated" by reading
shell is not something this script does reliably. Four ways it was wrong before it was believed:

  1. `--all-features` matched a `--all\b` pattern and made every crate look workspace-gated;
  2. a workspace-level `--manifest-path` gate (the form `test x3-lang` uses) names no `-p`, so its
     six crates were reported ungated — 1,300 tests of false positive;
  3. the `nested workspaces` gate passed its manifest through a shell variable
     (`--manifest-path "$d/Cargo.toml"`), which a regex over the command text cannot follow. Both
     loop-shaped gates were split into one entry per workspace on 2026-09-25 for exactly this reason,
     so the hazard is historical — the loop expansion below is kept for anything that reintroduces
     the form, and a gate list with none needs none;
  4. `cargo check --all-targets` compiles a crate's tests without running them, and counts as a
     gate in any text-matching scheme even though nothing executes.

So treat `--list` as a starting point for a human, not as evidence. A sound version would have
`local-ci.sh` record the packages each gate tests (from `cargo metadata`, once, when the gate runs)
instead of a consumer re-deriving it from the command text.

    scripts/check-crate-tests-are-gated.py --list     # the census
    scripts/check-crate-tests-are-gated.py            # exits 1 on a crate outside KNOWN_UNGATED

"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[1]
LOCAL_CI = ROOT / "scripts" / "local-ci.sh"

# Test attributes worth counting. `#[test]`, `#[tokio::test]`, `#[async_std::test]` and the
# `rstest` / `test_case` procedural forms all mark a function a harness runs.
TEST_ATTR = re.compile(r"#\[(?:tokio::|async_std::|rstest|test_case|quickcheck)?test\b")
WORKSPACE_WIDE_GATE = re.compile(r"cargo test[^|\n]*--(?:workspace|all)(?![\w-])")

# Directories that are not this repository's own crates: build output, vendored sources (including the
# Tauri vendor tree, whose members carry their own tests), past audit snapshots and agent worktrees.
SKIP_PARTS = {"target", "node_modules", "vendor"}
# `patches/` holds vendored copies of upstream crates the workspace pins via `[patch.crates-io]`, and
# `x3-desktop` vendor trees hold more of the same: their tests are upstream's, run upstream, and
# naming them here would bury the repository's own suites under a hundred entries nobody will act on.
SKIP_NAMES = {"audit-artifacts", "forge-std", "patches"}


def skipped(relative: Path) -> bool:
    for part in relative.parts:
        if part in SKIP_PARTS or part in SKIP_NAMES:
            return True
        if part.startswith(".") or part.endswith("-vendor"):
            return True
    return False


# Crates outside the root workspace that have tests and **no gate**. Empty on purpose: with the
# detection hazards above unresolved, naming entries here would be asserting a fact the script cannot
# establish. The list stays as the shape a sound version would fill in.
KNOWN_UNGATED: dict[str, str] = {}


def metadata() -> dict:
    proc = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT, capture_output=True, text=True,
    )
    if proc.returncode != 0:
        print("check-crate-tests-are-gated: cargo metadata failed; nothing was verified",
              file=sys.stderr)
        print(proc.stderr.strip()[:400], file=sys.stderr)
        raise SystemExit(2)
    return json.loads(proc.stdout)


def _expand_loops(command: str) -> list[str]:
    """Expand the `for d in <workspaces>; do … --manifest-path "$d/Cargo.toml" …; done` form.

    `nested workspaces` is written that way, so the manifest a gate tests is a shell variable and a
    regex over the command text sees `"$d/Cargo.toml"` — which is how `x3-sidecar` (109 tests),
    `x3-swarm-core` (69) and `x3-solvency-sidecar` (14) were reported as ungated by the first version
    of this check. Each word of the loop's list becomes its own candidate command.
    """
    out = [command]
    for loop in re.finditer(r"for\s+(\w+)\s+in\s+([^;]+);\s*do(.*?)done", command, re.S):
        variable, words, body = loop.group(1), loop.group(2), loop.group(3)
        if f'"${variable}/Cargo.toml"' not in body and f"${variable}/Cargo.toml" not in body:
            continue
        for word in words.split():
            expanded = body.replace(f'"${variable}/Cargo.toml"', f"{word}/Cargo.toml")
            expanded = expanded.replace(f"${variable}/Cargo.toml", f"{word}/Cargo.toml")
            out.append(expanded)
    return out


def gate_commands() -> list[str]:
    """Every gate command in `scripts/local-ci.sh`, fast set and opt-in sets alike, loop-expanded.

    The question here is "does anything run this", so a `--deep` gate counts: it is a command a
    maintainer runs, and for a nested workspace it is the only thing that does.
    """
    raw = [m.group(1) for m in re.finditer(r'^\s*"[^"]+:(.*)"\s*$', LOCAL_CI.read_text(), re.M)]
    return [expanded for command in raw for expanded in _expand_loops(command)]


def fast_gate_commands() -> list[str]:
    text = LOCAL_CI.read_text()
    start = text.index("GATES_FAST=(")
    end = text.index("\n)", start)
    return [m.group(1) for m in re.finditer(r'^\s*"[^"]+:(.*)"\s*$', text[start:end], re.M)]


def is_gated(name: str, directory: Path, commands: list[str]) -> bool:
    """Does a `cargo test` gate run this crate?

    Three forms count, and the third is the one a naive `-p` scan misses:

      * `cargo test -p <name>` — the package is named directly;
      * `cargo test --manifest-path <crate>/Cargo.toml` — the crate's own manifest;
      * `cargo test --manifest-path <workspace>/Cargo.toml` with no `-p` — the crate is a member of
        that nested workspace, so the gate tests it. This is how the whole `x3-lang` tree (six crates,
        ~1,300 tests) and the `x3-cross-vm-coordinator` workspace are covered; reading them as
        ungated is how the first version of this check produced thirty false positives.

    A `-p` alongside a workspace manifest selects one package out of it, so only that package counts:
    `test x3-htlc` runs `-p x3_htlc` inside `X3-contracts/svm`, which does not test the workspace's
    other programs.
    """
    for command in commands:
        if "cargo test" not in command:
            continue
        selected = re.findall(r"-p\s+([A-Za-z0-9_-]+)", command)
        manifest = re.search(r"--manifest-path[=\s]+([^\s]+)", command)
        if manifest is None:
            if name in selected:
                return True
            continue
        workspace = (ROOT / manifest.group(1)).resolve().parent
        if directory != workspace and workspace not in directory.parents:
            continue
        if not selected or name in selected:
            return True
    return False


def test_count(directory: Path) -> int:
    total = 0
    for sub in ("src", "tests"):
        base = directory / sub
        if base.is_dir():
            for path in base.rglob("*.rs"):
                total += len(TEST_ATTR.findall(path.read_text(errors="replace")))
    return total


def nested_crates(root_names: set[str]) -> list[tuple[str, Path, int]]:
    """Crates with a `[package]` that the root workspace does not contain, and their test counts."""
    found = []
    for manifest in ROOT.rglob("Cargo.toml"):
        relative = manifest.relative_to(ROOT)
        if skipped(relative.parent):
            continue
        try:
            with manifest.open("rb") as handle:
                document = tomllib.load(handle)
        except (OSError, tomllib.TOMLDecodeError):
            continue
        package = document.get("package")
        if not isinstance(package, dict):
            continue
        name = package.get("name")
        if not isinstance(name, str) or name in root_names:
            continue
        count = test_count(manifest.parent)
        if count:
            found.append((name, manifest.parent, count))
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description="Check that crates with tests are run by some gate.")
    parser.add_argument("--list", action="store_true", help="print every nested crate and its verdict")
    args = parser.parse_args()

    commands = gate_commands()
    fast = fast_gate_commands()
    failures = []

    if not any(WORKSPACE_WIDE_GATE.search(c) for c in commands):
        failures.append(
            "no gate runs `cargo test --workspace` any more, so the root workspace's suites — "
            "183 crates with tests — are in no gate at all"
        )

    document = metadata()
    root_names = {p["name"] for p in document["packages"]}
    root_with_tests = [p for p in document["packages"] if test_count(Path(p["manifest_path"]).parent) > 0]
    root_in_fast = sum(
        1 for p in root_with_tests
        if is_gated(p["name"], Path(p["manifest_path"]).parent, fast)
    )

    nested = nested_crates(root_names)
    ungated = sorted(name for name, directory, _ in nested if not is_gated(name, directory, commands))

    if args.list:
        print(f"{'crate':<40}{'tests':>6}  gated")
        for name, directory, count in sorted(nested, key=lambda r: -r[2]):
            mark = "yes" if is_gated(name, directory, commands) else "NO"
            print(f"{name:<40}{count:>6}  {mark}")
        print()
        print(f"root workspace: {len(document['packages'])} crates, {len(root_with_tests)} with tests, "
              f"{root_in_fast} of those in the fast set (the rest via `test workspace`)")
        print(f"outside the root workspace: {len(nested)} crates with tests, {len(ungated)} ungated")
        return 0

    for name in ungated:
        if name not in KNOWN_UNGATED:
            count = next(c for n, _, c in nested if n == name)
            failures.append(
                f"{name} has {count} test attribute(s) outside the root workspace and no gate runs "
                f"them; add a `cargo test` gate naming it (or its workspace) to scripts/local-ci.sh"
            )
    for name in sorted(KNOWN_UNGATED):
        if name not in ungated:
            failures.append(
                f"{name} is on KNOWN_UNGATED but is gated now (or has no tests); remove it so the "
                f"list keeps describing the gap"
            )

    if failures:
        for failure in failures:
            print(f"check-crate-tests-are-gated: FAIL: {failure}", file=sys.stderr)
        return 1

    print(
        "check-crate-tests-are-gated: OK - %d root crate(s) with tests (%d in the fast set, the rest "
        "via `test workspace`), %d outside the root workspace, %d of those on the shrinking list"
        % (len(root_with_tests), root_in_fast, len(nested), len(KNOWN_UNGATED))
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
