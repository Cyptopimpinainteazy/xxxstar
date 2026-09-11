#!/usr/bin/env python3
"""Deterministic PR integrity gate used by GitHub Actions."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys

SECRET_PATTERNS = [
    re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH |PRIVATE )?PRIVATE KEY-----"),
    re.compile(
        r"(?:[A-Za-z][A-Za-z0-9_]*_)?"
        r"(?:X3(?:VM)?_SIGNER_(?:SURI|SEED_HEX)|PRIVATE_KEY|SECRET_KEY|API_KEY|BEARER_TOKEN)"
        r"\s*[:=]\s*(?:['\"](?!\$(?:\{|[A-Za-z_])|(?:env|process\.env|secrets)\.)[^'\"]{16,}['\"]"
        r"|(?!\$(?:\{|[A-Za-z_])|(?:env|process\.env|secrets)\.)[^\s#'\"`]{16,})",
        re.I,
    ),
    re.compile(r"(?:mnemonic|seed_phrase|seed phrase)\s*[:=]\s*['\"][^'\"]{12,}['\"]", re.I),
]


def run(*args: str) -> str:
    proc = subprocess.run(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or "command failed")
    return proc.stdout


def added_hunks_with_context(diff: str) -> str:
    """Return additions and unchanged hunk context, never removed file content."""
    hunks: list[str] = []
    current_hunk: list[str] | None = None
    for line in diff.splitlines():
        if line.startswith("@@"):
            if current_hunk is not None:
                hunks.append("\n".join(current_hunk))
            current_hunk = []
        elif line.startswith("diff --git "):
            if current_hunk is not None:
                hunks.append("\n".join(current_hunk))
            current_hunk = None
        elif current_hunk is not None:
            if line.startswith("+") or line.startswith(" "):
                current_hunk.append(line[1:])
    if current_hunk is not None:
        hunks.append("\n".join(current_hunk))
    return "\n".join(hunks)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default="origin/main")
    args = parser.parse_args()

    try:
        run("git", "rev-parse", "--verify", args.base)
        diff = run("git", "diff", "--no-ext-diff", "--unified=0", f"{args.base}...HEAD")
        names = run("git", "diff", "--name-only", f"{args.base}...HEAD").splitlines()
    except RuntimeError as exc:
        print(f"PR Supervisor: unable to inspect diff: {exc}", file=sys.stderr)
        return 2

    print(f"PR Supervisor: {len(names)} changed file(s)")
    changed_hunks = added_hunks_with_context(diff)
    if any(pattern.search(changed_hunks) for pattern in SECRET_PATTERNS):
        print("PR Supervisor: possible credential/private-key material detected in diff", file=sys.stderr)
        return 1
    if len(names) > 1000:
        print("PR Supervisor: refusing >1000 changed files; split the change into smaller PRs", file=sys.stderr)
        return 1
    if any(name.endswith("Cargo.toml") for name in names):
        try:
            run("cargo", "metadata", "--no-deps", "--format-version", "1")
        except RuntimeError as exc:
            print(f"PR Supervisor: Cargo metadata check failed: {exc}", file=sys.stderr)
            return 1

    print("PR Supervisor: integrity checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
