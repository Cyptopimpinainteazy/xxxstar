#!/usr/bin/env python3
"""Lightweight PR integrity gate used by GitHub Actions.

The gate is intentionally deterministic and dependency-free. It validates that the
requested base exists, inspects the PR diff for credential leakage, and rejects
obvious repository-wide churn that is unsafe for a focused PR.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys

SECRET_PATTERNS = [
    re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH |PRIVATE )?PRIVATE KEY-----"),
    re.compile(r"(?:X3_SIGNER_SEED_HEX|PRIVATE_KEY|SECRET_KEY|API_KEY|BEARER_TOKEN)\s*=\s*['\"][^'\"]{16,}['\"]"),
    re.compile(r"(?:mnemonic|seed_phrase|seed phrase)\s*[:=]\s*['\"][^'\"]{12,}['\"]", re.I),
]


def run(*args: str) -> str:
    proc = subprocess.run(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or "command failed")
    return proc.stdout


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

    for pattern in SECRET_PATTERNS:
        if pattern.search(diff):
            print("PR Supervisor: possible credential/private-key material detected in diff", file=sys.stderr)
            return 1

    # A focused PR should not silently become a repository-wide rewrite.
    if len(names) > 1000:
        print("PR Supervisor: refusing >1000 changed files; split the change into smaller PRs", file=sys.stderr)
        return 1

    # Verify changed Cargo manifests remain parseable by Cargo when Cargo is present.
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
