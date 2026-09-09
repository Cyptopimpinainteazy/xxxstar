# Repository Recovery Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stabilize the active `master` lineage, land truthful grant-readiness documentation, protect the canonical branch, and retire only branches whose recovery decisions are recorded.

**Architecture:** Treat `master` as the sole active lineage and preserve the unrelated historical `main` head under `archive/main-pre-reconciliation-20260908`. Repair CI on PR #126 before rebasing PR #125, require evidence-producing checks before either merge, and make branch deletion the final review boundary.

**Tech Stack:** Git, GitHub Actions, Rust/Cargo, Node.js/npm, Python, OSV-Scanner, Trivy, CodeQL, Semgrep, Snyk.

**Spec:** `docs/BRANCH_RECONCILIATION_2026-09-08.md` and `docs/BRANCH_RECOVERY_INVENTORY_2026-09-08.md`

## Global Constraints

- Do not merge the unrelated `main` and `master` histories.
- Do not report configured workflows as passing evidence.
- Do not weaken a gate to hide a genuine product or dependency failure.
- External bridges remain disabled.
- Preserve `archive/main-pre-reconciliation-20260908`.
- Delete only exact branch names approved in the committed inventory.
- Every merge requires fresh results for its exact head commit.

---

### Task 1: Stabilize PR #126 CI

**Files:**
- Modify: `.github/workflows/*.yml`
- Modify only with log evidence: `Cargo.toml`, `Cargo.lock`, `patches/`, `scripts/`, `scripts_infrastructure/`
- Record evidence: PR #126 conversation

**Interfaces:**
- Consumes: workflow run IDs, failed job IDs, and complete GitHub logs
- Produces: a ledger mapping each failure to its root cause, fix commit, and rerun result

- [x] Confirm zero-step billing failures now execute normal steps.
- [x] Replace obsolete OSV installation and confirm OSV passes on commit `93122eb823e3ca26cb385173e0e7e900273b0285`.
- [x] Remove the unresolved `vendor/blst` path override.
- [x] Replace the missing no-op-in-CI integrity script with `pr_supervisor.py --base origin/master`.
- [x] Change Trivy from a nonexistent image scan to a filesystem scan.
- [x] Point dashboard CI at `apps/dashboard` and prove its npm 10.8.2 build.
- [x] Trace Solana v4 edges from `x3-svm-integration` and verify they belong to the official 3.0 runtime graph.
- [x] Replace the self-contradictory version grep with locked resolution plus the actual SVM test gate.
- [x] Normalize the registry-backed `blst` lock entry and prove `cargo test --locked -p x3-svm-integration --all-targets` (33 unit + 1 integration) passes.
- [x] Repair the two `idna_adapter` deprecation failures under `-D warnings` with narrowly scoped allowances.
- [x] Remove the unusable Snyk workflow after the owner directed this task to be resolved; retain OSV, Trivy, CodeQL, and Semgrep.
- [ ] Rerun all failed workflows and require zero failed required checks on the exact final head.

**Verification:**

```bash
python3 scripts/agent_guard.py
python3 scripts_infrastructure/pr_supervisor.py --base origin/master
python3 -m py_compile scripts/agent_guard.py scripts_infrastructure/pr_supervisor.py
git diff --check origin/master...HEAD
cd apps/dashboard && npm ci && npm run build
cargo test --workspace --all-targets --all-features
```

Expected: local commands exit 0 and the exact PR head has no failed required GitHub checks.

**Rollback:** Revert only the CI repair commit whose rerun regresses.

### Task 2: Review and squash-merge PR #126

**Files:**
- Review: every file returned by GitHub's PR changed-files API
- Update: PR #126 body and checklist

**Interfaces:**
- Consumes: green exact-head suite from Task 1
- Produces: one squash commit on `master`

- [ ] Confirm PR #126 is based on current `master` and mergeable.
- [ ] Run `git diff --check origin/master...HEAD` and parse every workflow YAML file.
- [ ] Verify active workflows contain no `origin/main`, `refs/heads/main`, `default: main`, or `main` trigger.
- [ ] Confirm no unrelated runtime or consensus changes.
- [ ] Convert from draft only after verification.
- [ ] Squash-merge and record the resulting `master` SHA.

**Rollback:** Revert the squash commit; never move `master` backward.

### Task 3: Rebase and verify PR #125

**Files:**
- Rebase/recreate: `docs/grant-readiness-truth-20260908`
- Verify: all Markdown changed by PR #125

**Interfaces:**
- Consumes: Task 2 squash commit
- Produces: a documentation-only grant-readiness PR

- [ ] Record the current PR #125 head SHA.
- [ ] Recreate the branch from updated `master` and replay only documented changes.
- [ ] Keep Task 2 workflow versions when resolving overlap.
- [ ] Recalculate the 15-entry registry score and eight-gate cross-chain score.
- [ ] Scan for obsolete billing-lock, 36%, 100%-wallet, and production-ready claims.
- [ ] Require green documentation checks on the exact rebased head.

**Rollback:** Restore the recorded pre-rebase branch SHA.

### Task 4: Protect `master`

**Files:**
- Verify/update: `.github/BRANCH_PROTECTION.md`
- External configuration: GitHub branch protection or ruleset

**Interfaces:**
- Consumes: aggregate job name from `.github/workflows/ci.yml`
- Produces: enforced pull-request and required-check policy

- [ ] Confirm `master` is the default branch.
- [ ] Read the existing protection rule or ruleset.
- [ ] Require pull requests and up-to-date branches.
- [ ] Require `x3 / critical-path-all-pass`.
- [ ] Block force pushes and branch deletion.
- [ ] Re-read saved settings and capture evidence.

**Stop condition:** Without authenticated protection read/write access, request an owner browser/API action. Documentation is not enforcement.

**Rollback:** Restore the captured prior protection configuration.

### Task 5: Review and merge PR #125

- [ ] Confirm every required check passes on the exact head.
- [ ] Confirm no runtime, consensus, deployment, or bridge-enablement changes.
- [ ] Confirm branch and cross-chain ledgers match repository state.
- [ ] Merge PR #125 and verify the documents from `master`.

**Rollback:** Revert the PR merge commit.

### Task 6: Finalize held-branch decisions

**Files:**
- Modify: `docs/BRANCH_RECOVERY_INVENTORY_2026-09-08.md`
- Modify: `docs/BRANCH_RECONCILIATION_2026-09-08.md`

- [ ] Refresh every remote branch tip.
- [ ] Mark contained branches `DELETE_APPROVED_CONTAINED`.
- [ ] Mark rejected obsolete dependency branches `DELETE_APPROVED_REJECTED`.
- [ ] Keep unique feature/recovery branches `HOLD` unless individually ported or rejected with reasons.
- [ ] Record replacement PR/commit SHAs for selectively ported work.
- [ ] Commit the ledger before deleting any ref.

Expected: every branch appears exactly once with its tip SHA and disposition.

### Task 7: Delete only approved stale branches

- [ ] Present exact branch names and tip SHAs for owner confirmation.
- [ ] Exclude `master`, the archive branch, and every `HOLD` branch.
- [ ] Delete approved refs one at a time.
- [ ] Refresh refs and compare remaining branches against the ledger.
- [ ] Post a deletion receipt.

**Stop condition:** No deletion capability or no exact owner confirmation means zero deletions.

**Rollback:** Recreate a mistakenly deleted branch at its recorded tip SHA.

### Task 8: Publish final completion evidence

**Files:**
- Update: `FINAL_REPORT.md`
- Update: `GRANT_READINESS.md`
- Update if scores changed: `docs/CROSS_CHAIN_READINESS.md`

- [ ] Record merge SHAs and workflow runs.
- [ ] Record protection evidence.
- [ ] Record deleted and retained branches.
- [ ] Recompute completion percentages.
- [ ] State every remaining blocker and disabled feature.
- [ ] Run final Markdown, YAML, Python, npm, and applicable Cargo verification.

**Completion gate:** Recovery is complete only when PRs #126 and #125 are merged, `master` protection is enforced, the final ledger is committed, and every approved deletion is verified.
