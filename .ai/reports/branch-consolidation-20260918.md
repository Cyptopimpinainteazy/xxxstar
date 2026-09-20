# Branch consolidation — 2026-09-18

Goal from the user: "get all branches on main / merge all work".

## Outcome

**Merged to `master` this pass (3 commits, `ea0294fb8`):**
- `57a69aae2` / `038cae139` — the two `x3lang-root-compiler-bridge` design docs
  (978 lines) from `design/x3lang-root-compiler-bridge`.
- `ea0294fb8` — `queue-drain.yml` from `ops/drain-actions-queue-20260911`,
  with `release-hardening.yml` kept at master's newer revision (the branch's
  rewrite of it conflicted; master had evolved past it).

Verified on the result: no conflict markers anywhere in the tree, worktree
clean, `cargo check -p x3-atomic-swap` exit 0, and master's
`release-hardening.yml` is master's own 205-line version.

## Why "all branches" is not a merge operation

~150 local/remote branches exist. Classified with
`git merge-base` + per-file `git diff <branch> <master>`:

**1. Cannot be merged at all — unrelated history (6 branches).** No merge base
with master, because master was rewritten (see
`archive/pr126-pre-master-rewrite-20260909`). `git merge` refuses these:
`archive/pr126-pre-master-rewrite-20260909` (218 commits),
`wip/consolidation-20260917/recovered-usb-clone` (179),
`docs/grant-readiness-truth-20260908` (179), `fix-x3lang-python` (69),
`your-task-branch` (14), `t5/fix-annotations-20260522-1458` (13).
Their *content* could be diffed onto master; their history cannot.

**2. Superseded — master already contains the work (4 branches).**
`ci/x3-local-runner-smoke`, `fix/private-mempool-real-shamir-threshold`,
`ci/consolidate-workflows-20260910`,
`preserve/20260918/pip/psycopg2-binary-gte-2.9.13`. Nothing to merge.

**3. Deliberately reverted — re-merging would undo a decision (~20 branches).**
The "missing" lines are mostly CI config that master intentionally changed:
- `cancel-in-progress: true` — master has the commit *"ci: stop cancelling
  in-flight default-branch gates"*, so re-adding it reverts that fix.
- `runs-on: ubuntu-latest` — master moved these to self-hosted runners.
- `feat/x3-lang-crosschain-integration-20260909` looked like it lost four
  assertions in `crates/x3-integration/tests/compiler_bridge.rs`; master has
  all four, written differently. The per-file line counter scored formatting
  as loss. **Check content, not line counts, before calling something lost.**

**4. Lockfile-only (~38 branches).** Dependabot and `preserve/20260918/cargo/*`
branches whose only change is `Cargo.toml`/`Cargo.lock` version bumps against
a lockfile 80+ commits older. These are a *dependency-update* job, not merges;
doing them one at a time would be 38 lockfile conflicts.

**5. Genuinely unmerged, large (~50 branches).** Real content not on master,
from 1 line to 19,206 (`wip/consolidation-20260917/main`). The substantial
ones, which need per-branch rebasing as real work:
- `finish/x3vm-live-transport` (56 commits) and `-fix` (20)
- `codex/x3-economic-safety-kernel` (45)
- `test/cross-domain-refund-recovery-20260911` (39)
- `wip/prompts-to-skills-20260918` (36)
- `fix/svm-htlc-native-custody` (34), `fix/production-gate-prerequisites` (33)
- `ci/master-lineage-gates-20260908` (31)
- `archive/stale-x3lang-trading-wip-20260918` (3, 12,762 lines) — already
  pushed to origin as a preservation branch; it holds the trading WIP that is
  still not on main (`opportunity.rs`, `fusion.rs`, `fixed.rs`).

## Process error worth recording

I ran `if git rebase ... | tail -2; then git push ...` — the `if` tested
`tail`'s exit status, not the rebase's, so the push ran while the rebase was
still conflicted. The pushed content turned out to be the intended result
(design docs + `queue-drain.yml`, master's workflow preserved) and I verified
it afterwards, but the push happened without the check it was supposed to
have. A pipeline's exit status is the last command's; never gate on it.

## Recommended next step

Not "merge all branches". Ordered by value:
1. Dependency bumps as one deliberate pass (class 4).
2. `archive/stale-x3lang-trading-wip-20260918` — decide whether the trading WIP
   (`opportunity.rs`, `fusion.rs`, `fixed.rs`) lands on main as real features.
3. Per-branch rebases for class 5, largest value first (`x3vm-live-transport`,
   `x3-economic-safety-kernel`).
4. Delete or explicitly retire classes 1–3 so future passes do not re-derive this.
