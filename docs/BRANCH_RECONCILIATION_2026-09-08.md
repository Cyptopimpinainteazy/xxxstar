# Branch Reconciliation Record

**Reviewed:** 2026-09-08
**Repository:** `Cyptopimpinainteazy/xxxstar`
**Current default branch:** `master`

## Decision

Do not merge `main` into `master`. GitHub reports that the two branches have no common ancestor. They are separate histories, so a normal merge would combine unrelated lineages and produce a large, difficult-to-audit change.

Use `master` as the active lineage unless repository ownership deliberately chooses another history. It contains the newest security, runtime, governance, SVM HTLC, validator-drill, and audit-refresh commits observed during this review.

## Inventory

The repository currently exposes 57 branches. They include:

- the active `master` lineage;
- a disconnected `main` lineage;
- historical recovery and work-in-progress branches;
- stale feature and documentation branches;
- numerous automated dependency branches.

Two pull requests remain open against the disconnected `main` lineage:

| PR | Branch | State on 2026-09-08 | Action |
|---|---|---|---|
| #109 | `docs/x3-deployment-runbook` | Draft; 4 commits ahead and 42 behind `main` | Review the four-file delta and port useful content onto a fresh branch from `master` |
| #46 | `wip/recover-local-changes-20260523` | 2 commits ahead and 166 behind `main`; reported non-mergeable | Do not merge; inspect individual commits/files and selectively port verified SVM changes only if absent from `master` |

## Branches with unique old-lineage commits

These comparisons are against `main`, not `master`.

| Branch | Ahead | Behind | Recommendation |
|---|---:|---:|---|
| `design/x3-funding-os-2026-09-03` | 3 | 15 | Port only if the funding design is still current |
| `docs/x3-deployment-runbook` | 4 | 42 | Review and rewrite against current commands before porting |
| `fix/sidecar-router-e2e-gate-20260523` | 8 | 167 | Inspect test changes; do not merge wholesale |
| `fix/x3-lang-production-gate` | 1 | 42 | Check whether the workflow fix already exists on `master` |
| `recovery/git-corruption-salvage-20260524-031437` | 2 | 153 | Archive; the diff contains broad snapshots and generated state |
| `t5/fix-annotations-20260522-1458` | 3 | 210 | Inspect only the RFC/service delta |
| `t5/fix-t5-blockers-v2` | 19 | 204 | Do not bulk merge; selectively compare sidecar and node changes |
| `wip/recover-local-changes-20260523` | 2 | 166 | Do not bulk merge |
| `your-task-branch` | 4 | 210 | Broad unrelated delta; inspect by component only |

Branches reported as fully behind `main` contain no unique commits relative to that lineage and are candidates for deletion after a backup tag or exported inventory.

## Active-lineage CI defect

The default branch is `master`, but `.github/workflows/ci.yml` says its push and pull-request gates target `main`. The newest `master` commit had no associated workflow runs when checked. Fix CI triggers and branch-protection rules before presenting CI as an enforced merge gate.

## Safe reconciliation procedure

1. Freeze direct merges to both `main` and `master`.
2. Confirm `master` as the canonical branch in repository settings and public docs.
3. Update critical workflows and branch protection to target `master`.
4. Close or retarget PRs that still use `main`.
5. For every ahead branch, compare files and cherry-pick only a small, coherent change onto a new branch from `master`.
6. Run the full required checks on each ported change.
7. Record accepted and rejected commits in this file or a follow-up ledger.
8. Tag the old `main` head before deletion or archival.
9. Delete stale branches only after the inventory is reviewed and the useful deltas are preserved.

## Stop conditions

Stop a port or merge if it:

- requires `--allow-unrelated-histories`;
- changes generated files, lockfiles, runtime code, and docs without one coherent purpose;
- reintroduces mocks, no-op validation, placeholder proofs, hardcoded keys, or unsupported completion claims;
- cannot pass the active critical-path checks;
- changes consensus, supply, bridge, or signing behavior without focused tests and rollback notes.

## Recovery decisions recorded on 2026-09-08

| Source branch or group | Decision | Evidence |
|---|---|---|
| `main` | Archived, do not merge | Preserved at `archive/main-pre-reconciliation-20260908` pointing to `5d9deab1138f0f40fd16f74bf8ea8d1c17955a85` |
| `docs/x3-deployment-runbook` | Selective port | Deployment runbook and missing PR supervisor moved to fresh `ci/master-lineage-gates-20260908`; old toolchain and vendoring changes rejected |
| `design/x3-funding-os-2026-09-03` | Do not port | 1,643 lines of internal design/planning material; no implementation evidence and poor grant-facing signal |
| T5/PoH branches | Superseded | `master` contains `PoHVerifyBlockImport`, shadow-mode tests, and current node wiring |
| Sidecar recovery branches | Superseded; no bulk port | `master` contains fail-closed signing, 32-slot confirmation logic, runtime metadata refresh, submission tests, and current operator code |
| `fix/x3-lang-production-gate` | Superseded | `master` contains the combined X3 language/VM/EVM/SVM readiness workflow |
| Git-corruption salvage branches | Preserve only through old-main archive | Broad snapshots and generated state are unsuitable for selective merge |
| Old Dependabot branches based on `main` | Reject | Dependency state on active `master` has moved through later security and dependency commits |
| Branches fully contained in `master` | Delete candidates | No unique commits relative to `master` |

No unresolved code branch is approved for deletion until its tip SHA and disposition are present in this record.
