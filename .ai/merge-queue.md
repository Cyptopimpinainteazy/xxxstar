# Merge queue — remote branch consolidation

**Question this answers:** of the ~191 branches on `origin`, which ones still hold
work that is not on `master`, and in what order should they land?

**How the buckets were computed** (all four are needed; each one alone lies):

Commands below use `origin/<b>` throughout — these are remote branch names, and
`<b>` alone only resolves if a same-named local branch happens to exist (true in
the working tree this census was run from, not guaranteed in a fresh clone).
Run `git fetch origin` first.

| Signal | Command | What it proves |
| --- | --- | --- |
| In master | `git merge-base --is-ancestor origin/<b> origin/master` | the tip is literally in master's history |
| Nothing to land | `git cherry origin/master origin/<b>` → every commit prefixed `-` | each commit's *patch* is already in master (covers squash/cherry-pick/rebase landings). Merging such a branch is **not** a no-op — its tree can be *older* than master's, so it reverts work |
| Content delta | `git diff --shortstat origin/master...origin/<b>` | how much content the branch still adds |
| Real merge | `git merge --no-commit --no-ff origin/<b>` in a scratch worktree | which files actually conflict |

**Precondition:** all of this assumes a full-history clone. In a shallow clone,
`git merge-base` returns empty when the shared history simply wasn't fetched,
not only when history is genuinely unrelated — the same empty result this
census reads as "archival, safe to delete" (see the 12-branch bucket below).
Run `git fetch --unshallow` (or check `git rev-parse --is-shallow-repository`
is `false`) before trusting an empty merge-base enough to delete a branch on
its strength.

## Census

`origin` had **191** branches at the time of writing.

- **101** are ancestors of `master` — done, delete candidates.
- **90** are not. Of those:
  - **29** are patch-equivalent to `master` (`git cherry` shows no `+`): *nothing to
    land.* This is the dangerous bucket: the tree diff is non-empty, so a blind
    merge reverts the newer implementation that replaced it. `fix/private-mempool-real-shamir-threshold`
    (PR #288) is the worked example — it is the pre-#287, pre-#291 state of
    `crates/private-mempool`; merging it would undo the Ristretto threshold work.
  - **21** are dependabot heads, each with its own PR (see "Dependency heads").
  - **12** have unrelated/rewritten history (`git merge-base` is empty) — they are
    archival snapshots, not mergeable branches.
  - **28** carry commits that are not in `master` in any form. Those are the queue.

## Landed in this pass

| Work | Landed as |
| --- | --- |
| Batch 5: `x3-dns-server` + `x3-marketplace` into the workspace (38 tests that ran nowhere), `x3-pq` fails closed, stale coordinator lockfile, local-CI `WASM_BUILD_WORKSPACE_HINT` fix | #290 → `eb08d8b1a` |
| Batch 6: SVM native HTLC custody (`programs/svm/x3_atomic_swap`), X3VM recovery matrix (`crates/x3-atomic-swap`), x3-lang `intent_bridge` + python tooling (`.serena`, `x3-lang/*.py`) | `batch/20260918T2015Z` |
| `ops(ci): drain stale GitHub Actions PR runs` | `ea0294fb8` |
| `docs: design canonical X3Lang root compiler bridge` (+ implementation plan) | `57a69aae2`, `038cae139` |

## Queue — unlanded work (28)

`pending` = commits whose patch is not in master (`git cherry` `+`).
`conflict` = files that conflict in a real merge against current `master`.

The table below lists 34 rows for context, not 28: 3 are marked "landed in
batch 6" (already merged since this census was taken) and 3 are marked "no
merge base" (they belong to the 12-branch archival bucket above, not this
queue). 34 − 3 − 3 = 28, the branches that actually still need a disposition.

**Update (2026-09-18, post-census):** `fix/master-trading-core-compile-break`
was verified archival (see its row) — `git cherry`/conflict-file signals alone
made it look like real work, but the code converged independently, not by
patch-equivalence, so the automated signals didn't catch it. The true "real"
count is 27, not 28; the other 27 rows have not been individually re-verified
this way, so treat "real" as provisional per-row, not just per-count, until
each one gets the same direct-file-check treatment before merging.

| Branch | pending | conflict files | disposition |
| --- | --- | --- | --- |
| `agents/setup-instructions-request` | 1 | — (clean) | landed in batch 6 |
| `fix/svm-htlc-native-custody-master` | 2 | `.github/workflows/x3vm-svm-live-lifecycle.yml` | landed in batch 6, workflow hunk resolved in master's favour |
| `test/cross-domain-recovery-matrix-20260911` | 10 | `.github/workflows/mainnet-readiness.yml` | landed in batch 6, workflow hunk resolved in master's favour |
| `fix/svm-htlc-native-custody` | 33 | 169 files | same work as the `-master` branch plus a much older base; salvage the SVM custody commits onto master rather than merging |
| `fix/production-gate-prerequisites` | 32 | 163 files | predates the local-CI-of-record work; most of it is superseded — diff the 33 commits against `scripts/local-ci.sh` before merging |
| `wip/prompts-to-skills-20260918`, `wip/consolidation-20260917/pasted-text-processing`, `agents/pasted-text-processing`, `pr132-work` | 33–34 each | 86–90 files | one work item mirrored on four branches — pick the newest, rebase, land once |
| `ci/master-lineage-gates-20260908` | 31 | 47 files | superseded by the local CI of record |
| `docs/grant-readiness-truth-20260908` | 31 | no merge base | archival; lift the document if it is still wanted |
| `wip/consolidation-20260917/recovered-usb-clone` | 149 | no merge base | salvage review only |
| `archive/pr126-pre-master-rewrite-20260909` | 66 | no merge base | archival |
| `your-task-branch` (14), `t5/fix-annotations-20260522-1458` (13), `pr-181-check` (12) | 12–14 | real | May/September snapshots; triage individually |
| `fix/master-trading-core-compile-break` | 4 | 20 files | archival — verified 2026-09-18: every function/struct/test this branch adds (including exact test names) already exists on master verbatim, landed independently via #133/#212/#216/#223. The `git cherry`/`conflict-files` signals alone made this look like real unlanded work; a direct file check (not a triple-dot diff against the branch's own stale base) showed 100% overlap. Same pattern as `fix/private-mempool-real-shamir-threshold` (PR #288) above — do not merge, it would revert to an older tree shape |
| `finish/x3vm-live-transport-fix` | 4 | 16 files | real, but `finish/x3vm-live-transport` already landed |
| `feat/x3vm-durable-recovery-20260911` | 4 | 2 files | newer versions of the same files landed — verify before merging |
| `feat/secret-release-firewall-20260911` | 3 | `crates/x3-atomic-swap/src/secret_release.rs` | superseded by `feat/live-secret-release-firewall-20260911` (in master) |
| `feat/canonical-cross-domain-proof-bundle-20260911-pre-rebase-20260917` | 3 | `crates/x3-atomic-swap/{lib,proof_bundle}.rs` | pre-rebase copy; the rebased branch is in master |
| `feat/idempotent-cross-domain-coordinator-20260911-pre-rebase-20260917` | 1 | 2 files | same |
| `wip/chatgpt-mainnet-attestation-20260918`, `wip/consolidation-20260917/chatgpt-mainnet` | 1 each | 22 files | snapshot of uncommitted work |
| `archive/stale-x3lang-trading-wip-20260918` | 3 | 68 files | archival |
| `ci/path-filter-heavy-gates-20260910` (9), `ci/consolidate-workflows-20260910` (7), `ops/drain-actions-queue-20260911` (1) | 1–9 | workflow files | the CI routing they implement has moved on (`01d2b64e5` and successors) |
| `preserve/20260918/*` (6 with pending commits) | 2–13 | mixed | preservation copies; their content is either in master or in a live branch |

## Dependency heads (21)

`dependabot/cargo/*` — one commit each, each with an open PR. They conflict with
each other because every one rewrites `Cargo.lock`; they have to land one at a
time with the lock regenerated, or batched by a single `cargo update` (which is
what `deps/batch-low-risk` (#292) did for the five low-risk ones). The high-risk
ones are `syn 2 → 3`, `toml 0.5 → 1.1.5`, `jsonrpsee 0.22 → 0.26`,
`solana-program/sdk 3 → 5`, `primitive-types 0.12 → 0.13`, `k256 0.13 → 0.14`,
`thiserror 1 → 2`, `solana-program-test 3.0.14` — each needs a compile pass, not
a merge.

## Safe merge order

1. Any branch that merges clean today (batch 6 shape: `agents/setup-instructions-request`,
   `fix/svm-htlc-native-custody-master`, `test/cross-domain-recovery-matrix-20260911`).
2. ~~`fix/master-trading-core-compile-break`~~ — verified archival, not real
   unlanded work (see Queue table). Delete rather than merge.
3. The `pasted-text-processing` family — pick one branch, rebase, land.
4. `fix/svm-htlc-native-custody` — salvage the custody commits (its `-master`
   sibling already carries the same work with a newer base).
5. `fix/production-gate-prerequisites` — after diffing against the CI of record.
6. Dependency heads, one `cargo update` batch at a time.
7. Everything else: archival or superseded → delete the branch, do not merge.

## Post-merge validation

```bash
./scripts/local-ci.sh --jobs 4                      # 21 fast gates, the CI of record
CARGO_TARGET_DIR=/tmp/x3-verify WASM_BUILD_WORKSPACE_HINT="$PWD" \
  ./scripts/local-ci.sh --only workspace-check,clippy-workspace,clippy-runtime-rc1
gh workflow run production-gate.yml --ref <batch-branch>
```

`git cherry` is the check that stops a "merge everything" pass from silently
reverting newer work — run it against `origin/master` for every branch before
merging it.
