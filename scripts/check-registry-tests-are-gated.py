#!/usr/bin/env python3
"""Fail when a feature the registry calls ready has no gate that runs its tests.

`FEATURE_REGISTRY.toml` lists `required_tests` for every feature, and
`scripts/check-readiness-consistency.sh` proves those *names exist* as functions. Neither asks whether
anything ever **runs** them. Measured on 2026-09-25, five suites sat in that gap — the kernel pallet,
the supply ledger, the cross-VM router, `x3-x3-integration` and the runtime crate itself — each cited
as evidence, each executing only in the opt-in `--deep` workspace run. That is the shape of a green
claim built on tests nobody ran.

This checker closes the loop for registry-cited crates:

  * resolve the crate `FEATURE_REGISTRY.toml` points at (via `cargo metadata`, not by guessing
    directory names);
  * read the fast gate list out of `scripts/local-ci.sh` and collect the packages those gates test,
    including the `--manifest-path` form the nested `x3-lang/` workspace uses;
  * a registry feature whose crate no gate tests is a finding — unless it is on `KNOWN_UNGATED`,
    which may only shrink.

A registry target that is not a workspace crate (a shell gate, a Solana program, a Tauri app) has no
`cargo test -p` to give it; those are listed separately so the decision is visible rather than implied.

    scripts/check-registry-tests-are-gated.py            # check
    scripts/check-registry-tests-are-gated.py --list     # show the full mapping

Exit 0 -> every registry-cited crate is tested by a gate, or is on the shrinking list.
Exit 1 -> a crate gained a registry citation without a gate, or a listed crate is now gated (so the
          list must shrink).
Exit 2 -> the check could not run (no cargo metadata); nothing was verified.
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
REGISTRY = ROOT / "FEATURE_REGISTRY.toml"
LOCAL_CI = ROOT / "scripts" / "local-ci.sh"

# Registry features whose crate no fast gate tests *today*. This list may only shrink: adding an entry
# is a decision to ship a registry claim whose tests nothing runs, and the gate says so out loud.
#
# Each one is recorded with the reason it is not gated yet, so the next reader inherits the decision
# rather than the silence.
KNOWN_UNGATED = {
    # Measured 2026-09-25 with this script's `--list`. Each entry is a registry feature whose crate
    # has a test suite and no fast gate: the registry cites it as evidence, and nothing runs it.
    "atomic_lock": "pallets/x3-lp-locker has no gate yet",
    "atomic_trade_engine": "pallets/atomic-trade-engine has no gate yet",
    "axe": "pallets/x3-dex has no gate yet",
    "x3_forge": "pallets/x3-token-factory has no gate yet",
    "x3_htlc": "X3-contracts/svm/programs/x3_htlc is an orphan tree (the registry row says so); the SVM HTLC on the live path is programs/svm/x3_atomic_swap, which its own gate exercises",
    "x3_reactor": "crates/x3-bench has no gate yet",
    "x3_sentinel": "pallets/x3-sentinel has no gate yet",
    "x3_wallet_pallet": "pallets/x3-wallet-pallet has no gate yet",
    "x3_wrapped": "pallets/x3-wrapped has no gate yet",
}


def _metadata() -> dict:
    proc = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT, capture_output=True, text=True,
    )
    if proc.returncode != 0:
        print("check-registry-tests-are-gated: cargo metadata failed; nothing was verified",
              file=sys.stderr)
        print(proc.stderr.strip()[:400], file=sys.stderr)
        raise SystemExit(2)
    return json.loads(proc.stdout)


def _gate_commands() -> list[str]:
    """Every gate command in `scripts/local-ci.sh`, as written."""
    return [m.group(1) for m in re.finditer(r'^\s*"[^"]+:(.*)"\s*$', LOCAL_CI.read_text(), re.M)]


def _manifest_is_tested(manifest: Path) -> bool:
    """Is there a gate whose command is a `cargo test` for this manifest?"""
    relative = str(manifest.relative_to(ROOT))
    for command in _gate_commands():
        if "cargo test" in command and relative in command:
            return True
    return False


def _gate_text() -> str:
    """Every gate command in one blob, for the "does a gate run something in this tree?" question."""
    return "\n".join(_gate_commands())


def _tested_targets(commands: list[str]) -> tuple[set[str], set[Path]]:
    """Packages named with `-p`, and manifests named with `--manifest-path`.

    `--manifest-path` is collected from *any* cargo invocation, not only `cargo test`: a nested
    workspace or a program crate can be exercised by a build-and-run gate (`cargo build-sbf` plus a
    lifecycle script) rather than by cargo's own test harness, and that is still a gate.
    """
    packages: set[str] = set()
    manifests: set[Path] = set()
    for command in commands:
        if "cargo" not in command:
            continue
        packages.update(re.findall(r"cargo test[^|]*?-p\s+([A-Za-z0-9_-]+)", command))
        for raw in re.findall(r"--manifest-path[=\s]+([^\s]+)", command):
            manifests.add((ROOT / raw).resolve())
    return packages, manifests


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--list", action="store_true", help="print the full mapping and exit")
    args = parser.parse_args(argv)

    registry = tomllib.loads(REGISTRY.read_text())
    meta = _metadata()
    packages, manifests = _tested_targets(_gate_commands())
    member_by_dir = {Path(p["manifest_path"]).parent.resolve(): p["name"] for p in meta["packages"]}

    ungated: dict[str, str] = {}
    not_a_crate: list[tuple[str, str]] = []
    no_tests: list[tuple[str, str]] = []
    covered: list[tuple[str, str]] = []
    for feature, body in sorted(registry.items()):
        if not isinstance(body, dict):
            continue
        target = str(body.get("crate_or_service", "")).strip()
        if not target:
            continue
        directory = (ROOT / target).resolve()
        manifest = directory / "Cargo.toml"
        required = body.get("required_tests", []) or []
        member = member_by_dir.get(directory)
        if member is None and not manifest.is_file():
            # A shell gate, a Solana program, a Tauri app: no `cargo test -p` can name it.
            not_a_crate.append((feature, target))
            continue
        # A nested workspace (`x3-lang/`) is gated through `--manifest-path`, which is a real gate.
        # A gate that *runs* something in the target tree exercises it even without cargo's test
        # harness: the SVM program is covered by
        # `programs/svm/x3_atomic_swap/test-live-lifecycle.sh`, which builds and drives it against a
        # validator. Containment alone is not enough — `cargo check --all-targets --manifest-path
        # <tree>` also names the tree and only *compiles* its tests, so a gate that merely mentions
        # the directory must not count as running it.
        runs_a_script_in_tree = re.search(rf"{re.escape(target)}/[\w./-]+\.sh", _gate_text()) is not None
        tested_by_manifest = manifest in manifests and _manifest_is_tested(manifest)
        if runs_a_script_in_tree or tested_by_manifest:
            covered.append((feature, member or f"{target} (gate runs this tree)"))
            continue
        if not required:
            # Nothing for a gate to run: the registry lists no test names for this feature.
            no_tests.append((feature, member or target))
            continue
        if member is not None and member in packages:
            covered.append((feature, member))
        else:
            ungated[feature] = member or target

    if args.list:
        for feature, member in covered:
            print(f"  gated   {feature:22} {member}")
        for feature, member in sorted(ungated.items()):
            print(f"  UNGATED {feature:22} {member}")
        for feature, who in no_tests:
            print(f"  no tests {feature:21} {who} (registry lists no test names)")
        for feature, target in not_a_crate:
            print(f"  n/a     {feature:22} {target} (not a cargo crate)")
        return 0

    new = {f: m for f, m in ungated.items() if f not in KNOWN_UNGATED}
    stale = [f for f in KNOWN_UNGATED if f not in ungated]

    for feature, member in sorted(new.items()):
        print(f"ERROR: registry feature '{feature}' cites {member}, and no gate runs its tests",
              file=sys.stderr)
        print(f"       add a gate for {member}, or add '{feature}' to KNOWN_UNGATED with a reason",
              file=sys.stderr)
    for feature in sorted(stale):
        print(f"ERROR: '{feature}' is on KNOWN_UNGATED but its crate is now gated — shrink the list",
              file=sys.stderr)
    if new or stale:
        return 1

    print(
        f"check-registry-tests-are-gated: {len(covered)} registry feature(s) cite a crate a gate tests, "
        f"{len(ungated)} are on the shrinking KNOWN_UNGATED list, {len(no_tests)} list no tests, "
        f"{len(not_a_crate)} are not cargo crates"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
