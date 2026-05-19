#!/usr/bin/env python3
from __future__ import annotations

import argparse
import fnmatch
import os
import re
import subprocess
import sys
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

CONSENSUS_MAP_FILE = Path(".codex/consensus_paths.txt")
COVERAGE_MAP_FILE = Path(".codex/coverage_paths.txt")
REQUIRED_ARTIFACTS = [
    Path(".codex/CONSENSUS_INVARIANTS.md"),
    Path(".codex/FUZZ_OR_PROPERTY_PLAN.md"),
    Path(".codex/X3_AUDIT_PASS.md"),
]

ALLOW_TEST_EDIT_TOKEN = "[ALLOW_TEST_EDIT]"

TEST_GLOBS = [
    "**/test/**",
    "**/tests/**",
    "**/__tests__/**",
    "**/*.test.*",
    "**/*.spec.*",
]

IGNORE_SUBSTRINGS = [
    "/node_modules/",
    "/.next/",
    "/dist/",
    "/build/",
    "/target/",
]

SKIP_PATTERNS = [
    r"\b(it|describe|test)\.skip\b",
    r"\b(xit|xdescribe)\s*\(",
    r"@pytest\.mark\.skip\b",
    r"\bpytest\.skip\s*\(",
    r"\bunittest\.skip\b",
]

# Strong->weak conversions (block)
JS_WEAKEN_REPLACEMENTS = [
    ("toStrictEqual", "toEqual", "toStrictEqual() → toEqual()"),
    ("toBe(", "toEqual(", "toBe() → toEqual()"),
    ("toMatchInlineSnapshot", "toMatchSnapshot", "inline snapshot → snapshot"),
    ("toBeTruthy", "toBeDefined", "toBeTruthy() → toBeDefined()"),
]

RUST_WEAKEN_REPLACEMENTS = [
    ("assert_eq!", "assert!", "assert_eq!() → assert!()"),
]

@dataclass
class Finding:
    level: str  # "BLOCK" or "WARN"
    msg: str

def run(cmd: list[str]) -> str:
    out = subprocess.check_output(cmd, cwd=REPO, stderr=subprocess.STDOUT)
    return out.decode("utf-8", errors="replace")

def is_ignored(path: str) -> bool:
    p = path.replace("\\", "/")
    return any(s in p for s in IGNORE_SUBSTRINGS)

def load_globs(path: Path) -> list[str]:
    fp = REPO / path
    if not fp.exists():
        return []
    globs: list[str] = []
    for line in fp.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        globs.append(line)
    return globs

def load_coverage_rules(path: Path) -> list[tuple[str, str, str]]:
    """Load coverage enforcement rules formatted as glob|spec_id|report_path."""
    rules: list[tuple[str, str, str]] = []
    fp = REPO / path
    if not fp.exists():
        return rules

    for line in fp.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("|")
        if len(parts) != 3:
            continue
        glob, spec_id, report = parts
        rules.append((glob.strip(), spec_id.strip(), report.strip()))
    return rules

def is_match_any(path: str, globs: Iterable[str]) -> bool:
    p = path.replace("\\", "/")
    return any(fnmatch.fnmatch(p, g) for g in globs)

def staged_files() -> list[str]:
    out = run(["git", "diff", "--cached", "--name-only"])
    return [f.strip() for f in out.splitlines() if f.strip() and not is_ignored(f.strip())]

def diff_files(base: str, head: str) -> list[str]:
    out = run(["git", "diff", "--name-only", f"{base}..{head}"])
    return [f.strip() for f in out.splitlines() if f.strip() and not is_ignored(f.strip())]

def diff_text_staged(path: str) -> str:
    return run(["git", "diff", "--cached", "--", path])

def diff_text_range(base: str, head: str, path: str) -> str:
    return run(["git", "diff", f"{base}..{head}", "--", path])

def last_commit_message() -> str:
    try:
        return run(["git", "log", "-1", "--pretty=%B"])
    except Exception:
        return ""

def has_allow_test_edit() -> bool:
    if os.getenv("X3_ALLOW_TEST_EDIT", "").strip() == "1":
        return True
    return (ALLOW_TEST_EDIT_TOKEN in last_commit_message())

def extract_added_removed(diff: str) -> tuple[str, str]:
    removed, added = [], []
    for ln in diff.splitlines():
        if ln.startswith("---") or ln.startswith("+++"):
            continue
        if ln.startswith("-"):
            removed.append(ln[1:])
        elif ln.startswith("+"):
            added.append(ln[1:])
    return ("\n".join(removed), "\n".join(added))

def detect_skips(added_text: str) -> list[str]:
    hits: list[str] = []
    for pat in SKIP_PATTERNS:
        if re.search(pat, added_text):
            hits.append(pat)
    return hits

def detect_weakening(removed: str, added: str) -> list[str]:
    hits: list[str] = []

    for strict, loose, desc in JS_WEAKEN_REPLACEMENTS:
        if strict in removed and loose in added and strict not in added:
            hits.append(desc)

    for strict, loose, desc in RUST_WEAKEN_REPLACEMENTS:
        if strict in removed and loose in added and strict not in added:
            hits.append(desc)

    if "===" in removed and "==" in added and "===" not in added:
        hits.append("=== → ==")
    if "!==" in removed and "!=" in added and "!==" not in added:
        hits.append("!== → !=")

    return hits

def artifacts_missing() -> list[Path]:
    missing = []
    for p in REQUIRED_ARTIFACTS:
        fp = REPO / p
        if (not fp.exists()) or fp.stat().st_size == 0:
            missing.append(p)
    return missing

def enforce(files: list[str], diff_fn) -> list[Finding]:
    findings: list[Finding] = []

    consensus_globs = load_globs(CONSENSUS_MAP_FILE)
    touched_consensus = [f for f in files if is_match_any(f, consensus_globs)] if consensus_globs else []
    touched_tests = [f for f in files if is_match_any(f, TEST_GLOBS)]
    coverage_rules = load_coverage_rules(COVERAGE_MAP_FILE)

    if touched_tests and not has_allow_test_edit():
        findings.append(Finding(
            "BLOCK",
            "Test files changed but no explicit allowance token found.\n"
            f"- If intentional, commit with message containing {ALLOW_TEST_EDIT_TOKEN}\n"
            "  e.g. git commit -m \"[ALLOW_TEST_EDIT] <why test was wrong>\"\n"
            "- Or set env var X3_ALLOW_TEST_EDIT=1 for one commit."
        ))

    for f in touched_tests:
        diff = diff_fn(f)
        removed, added = extract_added_removed(diff)

        skips = detect_skips(added)
        if skips:
            findings.append(Finding("BLOCK", f"Test disabling detected in {f}: added {', '.join(skips)}"))

        weak = detect_weakening(removed, added)
        if weak:
            findings.append(Finding("BLOCK", f"Possible assertion weakening in {f}: " + "; ".join(weak)))

        removed_assert = bool(re.search(r"\b(assert|expect|require)\b", removed))
        added_assert = bool(re.search(r"\b(assert|expect|require)\b", added))
        if removed_assert and not added_assert:
            findings.append(Finding("BLOCK", f"{f}: assertions removed without comparable replacements (heuristic)."))

    if touched_consensus:
        missing = artifacts_missing()
        if missing:
            findings.append(Finding(
                "BLOCK",
                "Consensus/state-touching changes detected, but required artifacts are missing/empty:\n"
                + "\n".join(f"- {p.as_posix()}" for p in missing)
                + "\n\nFill them in before merge."
            ))
        findings.append(Finding(
            "WARN",
            "Consensus/state-touching change detected. Run audit profile:\n  codex --profile x3-audit"
        ))

    # Enforce coverage artifacts for critical paths when enabled
    if os.getenv("X3_ENFORCE_COVERAGE", "").strip() == "1" and coverage_rules:
        touched_specs: list[tuple[str, str]] = []
        for f in files:
            for glob, spec_id, report in coverage_rules:
                if fnmatch.fnmatch(f.replace("\\", "/"), glob):
                    touched_specs.append((spec_id, report))

        missing_reports = []
        for spec_id, report in touched_specs:
            report_path = REPO / report
            if (not report_path.exists()) or report_path.stat().st_size == 0:
                missing_reports.append((spec_id, report))

        if missing_reports:
            msg_lines = [
                "Coverage reports missing for critical paths touched in this change (X3_ENFORCE_COVERAGE=1):"
            ]
            for spec_id, report in missing_reports:
                msg_lines.append(f"- {spec_id}: expected coverage artifact at {report}")
            msg_lines.append("Run the coverage job (tarpaulin/grcov) and ensure reports are generated before merging.")
            findings.append(Finding("BLOCK", "\n".join(msg_lines)))

    return findings

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--mode", choices=["staged", "ci"], default="staged")
    ap.add_argument("--base", default=None, help="Base ref for ci diff (e.g. origin/main)")
    ap.add_argument("--head", default="HEAD", help="Head ref for ci diff (default HEAD)")
    args = ap.parse_args()

    if args.mode == "staged":
        files = staged_files()
        def diff_fn(p: str) -> str:
            return diff_text_staged(p)
    else:
        if not args.base:
            print("ci mode requires --base (e.g. --base origin/main)", file=sys.stderr)
            return 2
        files = diff_files(args.base, args.head)
        def diff_fn(p: str) -> str:
            return diff_text_range(args.base, args.head, p)

    if not files:
        return 0

    findings = enforce(files, diff_fn)
    blocks = [f for f in findings if f.level == "BLOCK"]
    warns = [f for f in findings if f.level == "WARN"]

    if warns:
        print("\nWARNINGS:")
        for w in warns:
            print(f"- {w.msg}")

    if blocks:
        print("\nBLOCKED BY X3 ENFORCEMENT:\n")
        for b in blocks:
            print(f"- {b.msg}\n")
        return 1

    return 0

if __name__ == "__main__":
    raise SystemExit(main())
