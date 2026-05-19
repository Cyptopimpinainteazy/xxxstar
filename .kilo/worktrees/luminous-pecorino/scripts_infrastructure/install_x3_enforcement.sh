#!/usr/bin/env bash
set -euo pipefail

# X3 Anti-Agent-Cheat Installer (idempotent)
# Writes/rewrites:
#  - .codex/AGENT_RULES.md
#  - .codex/consensus_paths.txt   (surgical for your current repo)
#  - .codex/{CONSENSUS_INVARIANTS,FUZZ_OR_PROPERTY_PLAN,X3_AUDIT_PASS}.md templates
#  - scripts/x3_enforce.py        (diff scanner + assertion weakening detector)
#  - .git/hooks/pre-commit        (local enforcement)
#  - .github/CODEOWNERS
#  - .github/labeler.yml
#  - .github/workflows/{x3-enforce,labeler}.yml
#
# Run from repo root:
#   cd /home/lojak/Desktop/x3-chain-master
#   bash ./scripts/install_x3_enforcement.sh
#
# NOTE: Replace @YOUR_GITHUB_USERNAME_OR_TEAM after install.

REPO_ROOT="$(pwd)"

if [[ ! -d ".git" ]]; then
  echo "ERROR: Run this from the repo root (where .git exists)."
  exit 1
fi

mkdir -p .codex scripts .github/workflows .github

############################################
# 1) AGENT RULES
############################################
cat > .codex/AGENT_RULES.md <<'MD'
# X3 Chain — Codex Agent Rules

These rules are mandatory for any automated code modification.

Violation of these rules invalidates the change.

---

## 1. Test Integrity Policy

- Tests must not be modified to make failing builds pass unless explicitly instructed.
- If a test fails:
  - Fix production code first.
  - Only modify a test if it is demonstrably incorrect.
  - When modifying a test:
    - Provide a written explanation of why it is wrong.
    - Reference specification, invariant, or intended behavior.
    - Include before/after reasoning.

Prohibited:
- Weakening assertions.
- Removing assertions.
- Converting strict equality to loose checks without justification.
- Skipping or ignoring failing tests.

---

## 2. Consensus / State Transition Safety Policy

Any change affecting:

- Block validation
- Transaction validation
- State transition logic
- Fork choice
- Finality
- Gas accounting
- Fee logic
- Slashing logic
- VM execution semantics
- Cross-VM atomicity

Requires all of the following:

### (1) Invariant List

Provide a structured list of invariants impacted, including:

- Safety invariants (nothing invalid is accepted)
- Liveness invariants (valid tx eventually included)
- Determinism invariants (identical input → identical state)
- Economic invariants (no unintended mint/burn)

### (2) Property or Fuzz Plan

Provide either:

- Property-based tests covering invariant preservation  
or  
- A fuzzing plan describing:
  - Input domain
  - Mutation strategy
  - Expected invariant preservation checks

### (3) Audit Mode Review

The change must pass review under:

    codex --profile x3-audit

No merge allowed without audit-mode reasoning pass.

---

## 3. Performance Optimization Guardrails

Performance improvements must:

- Preserve consensus determinism.
- Preserve ordering semantics.
- Preserve state root correctness.
- Not introduce race conditions across shards/threads.
- Document whether changes alter:
  - Execution order
  - Batching semantics
  - Mempool priority
  - Validator incentives

If parallelism is introduced:
- Provide race-condition analysis.
- Provide deterministic replay validation strategy.

---

## 4. Prohibited Shortcuts

The agent must NOT:

- Silence errors without root cause.
- Add catch-all try/catch without justification.
- Introduce hidden side effects.
- Disable validation checks.
- Reduce cryptographic verification strictness.
- Modify golden files or snapshots to match incorrect output.

---

## 5. Required Change Summary Template

Every structural change must include:

### Change Type
- Bug fix / Refactor / Optimization / Feature

### Modules Affected

### Risk Level
- Low / Medium / High / Consensus-Critical

### Invariants Touched

### Validation Strategy
- Unit tests
- Property tests
- Fuzz tests
- Replay tests
- Simulation

---

## 6. Determinism Rule

For consensus code:

- No nondeterministic sources:
  - Wall-clock time
  - Randomness without seeded determinism
  - Unordered map iteration
- All hashing inputs must be explicit.
- Serialization must be canonical.

---

## 7. Cross-VM Atomicity Rule (X3 Specific)

Any change affecting:

- EVM ↔ SVM interaction
- BTC atomic bridging
- X3VM execution engine

Must include:

- Atomic commit validation logic
- Rollback scenario analysis
- Partial-failure state handling
- Cross-chain replay protection review

---

## 8. If Uncertain

If unsure whether a change affects consensus:

Assume it does.

Switch to audit mode.

---

## 9. Merge Blockers

Changes are blocked if:

- Invariants not listed.
- Tests weakened without justification.
- Consensus code altered without audit pass.
- Determinism not verified.

---

# Enforcement Principle

Correctness > Speed  
Determinism > Convenience  
Security > Throughput  
MD

############################################
# 2) Consensus path map (surgical: current reality)
############################################
cat > .codex/consensus_paths.txt <<'TXT'
# X3 consensus/state-touching map (surgical, current repo)

apps/blockchain-adapter/src/index.ts
apps/blockchain-adapter/src/worker.ts
apps/blockchain-adapter/scripts/deploy_provenance.ts
TXT

############################################
# 3) Required artifacts (templates)
############################################
cat > .codex/CONSENSUS_INVARIANTS.md <<'MD'
# Consensus Invariants (X3)

## Safety invariants
- [ ] Determinism: identical inputs → identical state root
- [ ] Validity: invalid txs/blocks never accepted
- [ ] No unintended mint/burn

## Liveness invariants
- [ ] Valid tx eventually eligible for inclusion
- [ ] No deadlocks in proposer/validator flow

## Economic invariants
- [ ] Fee accounting correct
- [ ] Gas metering correct
MD

cat > .codex/FUZZ_OR_PROPERTY_PLAN.md <<'MD'
# Fuzz / Property Plan (X3)

## Target area
- (modules touched)

## Input domain
- (tx/block/state ranges)

## Mutation strategy
- (how inputs are perturbed)

## Invariant checks
- (which invariants are asserted)
MD

cat > .codex/X3_AUDIT_PASS.md <<'MD'
# X3 Audit Pass

Run:
- codex --profile x3-audit

Paste:
- Summary of reasoning
- Risks found / mitigations
- What was validated (tests, replay, fuzz, etc.)
MD

############################################
# 4) Enforcement script (diff scanner + assertion strength detector)
############################################
cat > scripts/x3_enforce.py <<'PY'
#!/usr/bin/env python3
from __future__ import annotations

import argparse
import fnmatch
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Tuple

REPO = Path(__file__).resolve().parents[1]

CONSENSUS_MAP_FILE = Path(".codex/consensus_paths.txt")
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

SKIP_TOKENS = [
    "it.skip", "describe.skip", "test.skip", "xit(", "xdescribe(",
    "@pytest.mark.skip", "pytest.skip(", "unittest.skip",
]

JS_WEAKEN_REPLACEMENTS = [
    ("toStrictEqual", "toEqual", "toStrictEqual() → toEqual()"),
    ("toBe(", "toEqual(", "toBe() → toEqual()"),
    ("toBeTruthy", "toBeDefined", "toBeTruthy() → toBeDefined()"),
]

RUST_WEAKEN_REPLACEMENTS = [
    ("assert_eq!", "assert!", "assert_eq!() → assert!()"),
]

@dataclass
class Finding:
    level: str  # "BLOCK" or "WARN"
    msg: str

def run(cmd: List[str]) -> str:
    out = subprocess.check_output(cmd, cwd=REPO, stderr=subprocess.STDOUT)
    return out.decode("utf-8", errors="replace")

def is_ignored(path: str) -> bool:
    p = path.replace("\\", "/")
    return any(s in p for s in IGNORE_SUBSTRINGS)

def load_globs(path: Path) -> List[str]:
    fp = REPO / path
    if not fp.exists():
        return []
    globs: List[str] = []
    for line in fp.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        globs.append(line)
    return globs

def is_match_any(path: str, globs: Iterable[str]) -> bool:
    p = path.replace("\\", "/")
    for g in globs:
        if fnmatch.fnmatch(p, g):
            return True
    return False

def staged_files() -> List[str]:
    out = run(["git", "diff", "--cached", "--name-only"])
    return [f.strip() for f in out.splitlines() if f.strip() and not is_ignored(f.strip())]

def diff_files(base: str, head: str) -> List[str]:
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

def extract_added_removed(diff: str) -> Tuple[str, str]:
    removed, added = [], []
    for ln in diff.splitlines():
        if ln.startswith("---") or ln.startswith("+++"):
            continue
        if ln.startswith("-"):
            removed.append(ln[1:])
        elif ln.startswith("+"):
            added.append(ln[1:])
    return ("\n".join(removed), "\n".join(added))

def detect_skips(added_text: str) -> List[str]:
    return [tok for tok in SKIP_TOKENS if tok in added_text]

def detect_weakening(removed: str, added: str) -> List[str]:
    hits: List[str] = []

    # JS style replacements
    for strict, loose, desc in JS_WEAKEN_REPLACEMENTS:
        if strict in removed and loose in added:
            hits.append(desc)

    # Rust style replacements
    for strict, loose, desc in RUST_WEAKEN_REPLACEMENTS:
        if strict in removed and loose in added:
            hits.append(desc)

    # Equality operator weakening
    if "===" in removed and "==" in added:
        hits.append("=== → ==")
    if "!==" in removed and "!=" in added:
        hits.append("!== → !=")

    return hits

def artifacts_missing() -> List[Path]:
    missing = []
    for p in REQUIRED_ARTIFACTS:
        fp = REPO / p
        if (not fp.exists()) or fp.stat().st_size == 0:
            missing.append(p)
    return missing

def enforce(files: List[str], diff_fn) -> List[Finding]:
    findings: List[Finding] = []

    consensus_globs = load_globs(CONSENSUS_MAP_FILE)
    touched_consensus = [f for f in files if is_match_any(f, consensus_globs)] if consensus_globs else []

    touched_tests = [f for f in files if is_match_any(f, TEST_GLOBS)]

    # Rule: tests changed require explicit allowance
    if touched_tests and not has_allow_test_edit():
        findings.append(Finding(
            "BLOCK",
            "Test files changed but no explicit allowance token found.\n"
            f"- If intentional, commit with message containing {ALLOW_TEST_EDIT_TOKEN}\n"
            "  e.g. git commit -m \"[ALLOW_TEST_EDIT] <why test was wrong>\"\n"
            "- Or set env var X3_ALLOW_TEST_EDIT=1 for one commit."
        ))

    # Rule: reject weakening/disabling tests
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

    # Rule: consensus-touching requires artifacts
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
PY

chmod +x scripts/x3_enforce.py

############################################
# 5) Pre-commit hook
############################################
cat > .git/hooks/pre-commit <<'SH'
#!/usr/bin/env bash
set -euo pipefail
python3 scripts/x3_enforce.py --mode staged
SH
chmod +x .git/hooks/pre-commit

############################################
# 6) GitHub workflows: enforcement + labeler
############################################
cat > .github/workflows/x3-enforce.yml <<'YAML'
name: X3 Enforcement

on:
  pull_request:
  push:
    branches: [ main, master, develop ]

jobs:
  enforce:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: "3.11"

      - name: Determine base ref
        id: base
        run: |
          if [ "${{ github.event_name }}" = "pull_request" ]; then
            echo "base=origin/${{ github.base_ref }}" >> $GITHUB_OUTPUT
          else
            echo "base=${{ github.sha }}~1" >> $GITHUB_OUTPUT
          fi

      - name: Run X3 enforcement
        run: |
          python3 scripts/x3_enforce.py --mode ci --base "${{ steps.base.outputs.base }}" --head "${{ github.sha }}"
YAML

cat > .github/labeler.yml <<'YAML'
risk:consensus:
  - changed-files:
      - any-glob-to-any-file:
          - 'apps/blockchain-adapter/src/index.ts'
          - 'apps/blockchain-adapter/src/worker.ts'
          - 'apps/blockchain-adapter/scripts/deploy_provenance.ts'

risk:tests:
  - changed-files:
      - any-glob-to-any-file:
          - '**/test/**'
          - '**/tests/**'
          - '**/__tests__/**'
          - '**/*.test.*'
          - '**/*.spec.*'
YAML

cat > .github/workflows/labeler.yml <<'YAML'
name: PR Labeler

on:
  pull_request_target:

jobs:
  label:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
    steps:
      - uses: actions/labeler@v5
        with:
          configuration-path: .github/labeler.yml
YAML

############################################
# 7) CODEOWNERS (merge policy foundation)
############################################
cat > .github/CODEOWNERS <<'TXT'
# Replace this with your actual GitHub username or team, e.g. @lojak or @org/core
* @YOUR_GITHUB_USERNAME_OR_TEAM

# Consensus-touching surfaces (current repo)
apps/blockchain-adapter/src/index.ts @YOUR_GITHUB_USERNAME_OR_TEAM
apps/blockchain-adapter/src/worker.ts @YOUR_GITHUB_USERNAME_OR_TEAM
apps/blockchain-adapter/scripts/deploy_provenance.ts @YOUR_GITHUB_USERNAME_OR_TEAM
TXT

############################################
# 8) Installer saved for reuse
############################################
INSTALLER_PATH="scripts/install_x3_enforcement.sh"
if [[ "${0##*/}" != "install_x3_enforcement.sh" ]]; then
  # If user ran this as a one-liner, also drop a copy into scripts/
  cat > "${INSTALLER_PATH}" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
bash -c "$(cat <<'INNER'
REPO_ROOT="$(pwd)"
if [[ ! -d ".git" ]]; then
  echo "ERROR: Run from repo root."
  exit 1
fi
# Re-run the same installer by reusing the committed version of this script.
# If you saved it as scripts/install_x3_enforcement.sh, just run it directly next time.
INNER
)"
echo "This copy is just a placeholder. Use the installer content you ran originally."
SH
  chmod +x "${INSTALLER_PATH}"
fi

echo "OK: X3 enforcement stack written."
echo "Next steps:"
echo "  1) Edit .github/CODEOWNERS and replace @YOUR_GITHUB_USERNAME_OR_TEAM"
echo "  2) In GitHub Branch Protection: require 2 approvals, require CODEOWNERS, require status check 'X3 Enforcement'"
echo "  3) Commit these files."
