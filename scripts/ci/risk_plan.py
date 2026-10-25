#!/usr/bin/env python3
"""Fail-closed, advisory pre-push gate selection for low-risk X3 edits.

This never publishes x3/local-ci. Full exact-head CI remains required before merge.
"""

import argparse
import subprocess
import sys
from pathlib import PurePosixPath

COMMON_GATES = (
    "agent-guards",
    "workflow-schema",
    "script-syntax",
    "test-integrity-diff",
    "readiness-consistency",
)
DESKTOP_GATES = COMMON_GATES + ("test-js-x3-desktop",)


def _docs(path):
    if path == "README.md":
        return True
    if not path.startswith("docs/") or not path.endswith(".md"):
        return False
    # CI and security documentation describe release policy; keep their verification broad.
    return not (
        path == "docs/local-ci.md"
        or path.startswith("docs/reports/")
        or path.startswith("docs/security/")
        or path.startswith("docs/mainnet/")
    )


def _desktop_ui(path):
    return (
        path.startswith("apps/x3-desktop/src/")
        and PurePosixPath(path).suffix in (".ts", ".tsx", ".js", ".jsx", ".css", ".scss", ".html")
    )


def choose_profile(paths):
    """Return ('full' | 'focused', literal gate slugs) for changed paths.

    Unknown paths, gate code, policy files, money paths, or an empty diff default
    to the complete pre-push verification. This is not a merge decision.
    """
    paths = tuple(paths)
    if not paths or any(
        not path or path.startswith("/") or ".." in PurePosixPath(path).parts or "\\" in path
        for path in paths
    ):
        return "full", ()
    if all(_docs(path) for path in paths):
        return "focused", COMMON_GATES
    if all(_docs(path) or _desktop_ui(path) for path in paths):
        return "focused", DESKTOP_GATES
    return "full", ()


def _git_output(*args):
    return subprocess.run(
        ("git",) + args, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    ).stdout


def changed_paths(base):
    """Include committed, dirty and untracked changes; fail closed on git errors."""
    _git_output("rev-parse", "--verify", "--quiet", base + "^{commit}")
    results = set()
    for args in (
        ("diff", "--no-ext-diff", "--name-only", "-z", base + "...HEAD"),
        ("diff", "--no-ext-diff", "--name-only", "-z", "HEAD"),
        ("ls-files", "--others", "--exclude-standard", "-z"),
    ):
        raw = _git_output(*args)
        results.update(item.decode("utf-8") for item in raw.split(b"\0") if item)
    return sorted(results)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="origin/master")
    args = parser.parse_args()
    try:
        paths = changed_paths(args.base)
        mode, gates = choose_profile(paths)
    except (OSError, ValueError, UnicodeError, subprocess.CalledProcessError) as exc:
        print("risk-plan: git inspection unavailable; full verification required", file=sys.stderr)
        mode, gates = "full", ()
    if mode == "focused":
        print("risk-plan: focused pre-push only; required exact-head PR CI unchanged", file=sys.stderr)
        print("focused:" + ",".join(gates))
    else:
        print("full")
    return 0


if __name__ == "__main__":
    sys.exit(main())
