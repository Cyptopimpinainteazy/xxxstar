#!/usr/bin/env python3
import os
import re
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

# --- Tune these for your repo structure ---
CONSENSUS_PATH_HINTS = [
    "consensus", "state", "state_transition", "forkchoice", "finality",
    "block", "header", "txpool", "mempool", "slashing", "fees", "gas",
    "vm", "x3vm", "evm", "svm", "atomic", "bridge", "btc"
]

TEST_PATH_HINTS = ["test", "tests", "__tests__", "spec", "specs"]

# File extensions we care about for assertion weakening heuristics
CODE_EXTS = {".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".sol"}

# Simple “weakening” heuristics (intentionally conservative)
WEAKEN_PATTERNS: list[tuple[re.Pattern, str]] = [
    (re.compile(r"\bassert(?:_eq|Eq)?\b"), "assert* changed (manual review)"),
    (re.compile(r"\bexpect\("), "expect() changed (manual review)"),
    (re.compile(r"\brequire\("), "require() changed (manual review)"),
    (re.compile(r"\bshould\."), "should.* changed (manual review)"),
    (re.compile(r"\btoBe\("), "toBe() changed (manual review)"),
    (re.compile(r"\btoEqual\("), "toEqual() changed (manual review)"),
    (re.compile(r"\btoStrictEqual\("), "toStrictEqual() changed (manual review)"),
    (re.compile(r"\btoMatchSnapshot\("), "snapshot changed (manual review)"),
]

def run(cmd: list[str]) -> str:
    out = subprocess.check_output(cmd, cwd=REPO, stderr=subprocess.STDOUT)
    return out.decode("utf-8", errors="replace")

def staged_files() -> list[str]:
    out = run(["git", "diff", "--cached", "--name-only"])
    return [f.strip() for f in out.splitlines() if f.strip()]

def is_test_path(p: str) -> bool:
    lp = p.lower()
    return any(h in lp.split("/") for h in TEST_PATH_HINTS) or "/tests/" in lp or lp.endswith((".spec.ts", ".spec.tsx", ".test.ts", ".test.tsx", ".test.js"))

def is_consensus_path(p: str) -> bool:
    lp = p.lower()
    parts = lp.split("/")
    return any(h in parts or h in lp for h in CONSENSUS_PATH_HINTS)

def read_staged_diff(path: str) -> str:
    return run(["git", "diff", "--cached", "--", path])

def file_ext_ok(path: str) -> bool:
    return Path(path).suffix in CODE_EXTS

def has_override_token() -> bool:
    # Allow explicit override via commit message containing token
    # Token is checked in the staged commit message (if exists), else env var.
    # Use: git commit -m "[ALLOW_TEST_EDIT] reason..."
    msg = ""
    try:
        msg = run(["git", "log", "-1", "--pretty=%B"])
    except Exception:
        msg = ""
    env = os.getenv("X3_ALLOW_TEST_EDIT", "")
    return ("[ALLOW_TEST_EDIT]" in msg) or (env.strip() == "1")

def require_file_exists(relpath: str, err: str) -> list[str]:
    p = REPO / relpath
    if not p.exists():
        return [err]
    if p.is_file() and p.stat().st_size == 0:
        return [f"{err} (file exists but is empty)"]
    return []

def main() -> int:
    files = staged_files()
    if not files:
        return 0

    errors: list[str] = []
    warnings: list[str] = []

    touched_tests = [f for f in files if is_test_path(f)]
    touched_consensus = [f for f in files if is_consensus_path(f)]

    # Rule 1: Tests can’t be modified unless explicit override token is present
    if touched_tests and not has_override_token():
        errors.append(
            "Test files staged but no explicit override token found.\n"
            "If you truly intend to modify tests, commit with:\n"
            "  git commit -m \"[ALLOW_TEST_EDIT] <why the test is wrong>\"\n"
            "Or set env var X3_ALLOW_TEST_EDIT=1 for a single commit."
        )

    # Rule 2: If consensus/state transition touched, require invariant + audit artifacts
    if touched_consensus:
        errors += require_file_exists(
            ".codex/CONSENSUS_INVARIANTS.md",
            "Consensus/state-related code staged but missing .codex/CONSENSUS_INVARIANTS.md"
        )
        errors += require_file_exists(
            ".codex/FUZZ_OR_PROPERTY_PLAN.md",
            "Consensus/state-related code staged but missing .codex/FUZZ_OR_PROPERTY_PLAN.md"
        )
        errors += require_file_exists(
            ".codex/X3_AUDIT_PASS.md",
            "Consensus/state-related code staged but missing .codex/X3_AUDIT_PASS.md (paste audit output/summary)"
        )

    # Rule 3: Heuristic detection of weakened assertions (warn/block if tests touched)
    for f in touched_tests:
        if not file_ext_ok(f):
            continue
        diff = read_staged_diff(f)
        # Look for removed assertions or replaced strict with loose patterns
        removed_lines = "\n".join([ln[1:] for ln in diff.splitlines() if ln.startswith("-") and not ln.startswith("---")])
        added_lines = "\n".join([ln[1:] for ln in diff.splitlines() if ln.startswith("+") and not ln.startswith("+++")])

        # If assertions appear in removed but not in added, flag
        removed_hits = sum(bool(p.search(removed_lines)) for p, _ in WEAKEN_PATTERNS)
        added_hits = sum(bool(p.search(added_lines)) for p, _ in WEAKEN_PATTERNS)

        if removed_hits > 0 and added_hits == 0:
            errors.append(
                f"Possible assertion weakening in {f}: assertions removed without comparable replacements.\n"
                "If this is intentional, use [ALLOW_TEST_EDIT] and explain clearly."
            )

    # Output
    if warnings:
        print("\nWARNINGS:")
        for w in warnings:
            print(f" - {w}")

    if errors:
        print("\nBLOCKED BY X3 ENFORCEMENT:\n")
        for e in errors:
            print(f" - {e}\n")
        return 1

    return 0

if __name__ == "__main__":
    raise SystemExit(main())
