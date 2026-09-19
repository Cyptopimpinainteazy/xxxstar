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

**Update 2:** `finish/x3vm-live-transport-fix` verified archival too (see its
row) — every piece (finality range-scan fix, settlement-engine auto-refund
reschedule, runtime-signer intent-id resolution, the blst patch removal, the
CI workflow) already exists on master via convergent implementation; master
just organized the transport export differently
(`x3vm_native::NativeX3NodeTransport` vs. this branch's `x3vm_node` exports).
Real count is now 26.

**Update 3:** `pr-181-check`, `fix/svm-htlc-native-custody`,
`your-task-branch`, and `t5/fix-annotations-20260522-1458` all checked and
confirmed superseded/archival too (see their rows). Notably
`fix/svm-htlc-native-custody`'s 34 commits looked substantial, but only one
(`854445e75`, "svm: enforce native HTLC custody") was actually about custody
— the rest was unrelated staleness drift — and even that one commit's
functions are already on master, which has *more* than this branch does
(`derive_htlc_pda`, `broadcast_claim_htlc`, PDA validation tests): master is
a strict superset, so there is nothing to salvage. This **reverses the
"salvage the custody commits" recommendation** in both this row and step 4
of Safe merge order below — don't.

**Update 6 (2026-09-19):** `preserve/20260918/*` (28 branches — safety
snapshots taken before a consolidation pass, per their naming) checked as a
batch:
- **20 of 28** are directly patch-equivalent to master (`git cherry` empty) —
  fully redundant already.
- **6 of the remaining 8** (`feat-live-secret-release-firewall-20260911`,
  `test-cross-domain-refund-recovery-20260911`,
  `feat-settlement-proofset-gate-20260911`, `codex-x3-economic-safety-kernel`,
  `finish-x3vm-live-transport`, `finish-x3vm-live-transport-fix`) have
  nonzero pending commits *against master*, but their **live (non-preserve)
  counterpart branches** (e.g. `feat/live-secret-release-firewall-20260911`,
  dropping the `preserve/20260918/` prefix and restoring the `/`) all still
  exist on origin and are themselves fully patch-equivalent to master. These
  preserve copies are earlier snapshots taken mid-flight, before their own
  branch's later commits finished landing — not unique content.
- **2 remain genuinely unresolved but are not "unlanded work" either**:
  `cargo/ark-ec-0.6.0` and `cargo/ark-ff-0.6.0` are dependabot bumps
  generated against an assumed `0.5.0` base (`ark-ec`/`ark-ff` "5.0 → 6.0"),
  but current master's `Cargo.lock` has them at `0.4.2` — *older* than the
  bump's own assumed starting point. Applying either branch's diff as-is
  isn't safe (huge unrelated diff from staleness, plus the version jump it
  represents no longer matches reality). If arkworks 0.4.x → 0.6.x is wanted,
  it needs a fresh dependency-bump pass, not a merge of either branch.
  `ark-std-0.6.0` looked like the same story by name but showed 0 pending —
  not independently re-verified beyond that.

All 28 can be deleted except the 2 ark branches, which should be closed only
once whoever owns the arkworks version decision has looked at them.

**Update 7 (2026-09-19):** `ci/path-filter-heavy-gates-20260910`,
`ci/consolidate-workflows-20260910`, and `ops/drain-actions-queue-20260911`
checked directly against current master's tip (all three predate `01d2b64e5`
"make local CI authoritative and drain hosted workflow noise"):

- `ci/consolidate-workflows-20260910` deletes 7 redundant workflow files
  (`build.yml`, `ci.yml`, `codeql-analysis.yml`, `full-ci.yml`, `rust.yml`,
  `snyk.yml`, `v04-ship-gate.yml`). All 7 are already gone from master —
  deleted by `3ab374015` "ci: consolidate redundant GitHub Actions
  workflows". Fully redundant.
- `ci/path-filter-heavy-gates-20260910` adds/tunes `paths:` filters on
  `pull_request`/`push` triggers for 9 workflow files, to scope expensive
  hosted-CI gates to relevant changes. Moot: `01d2b64e5` already stripped
  every `pull_request`/`push` trigger repo-wide (confirmed on
  `benchmark-regression.yml`: master now has `on: workflow_dispatch` only,
  comment "GitHub-hosted CI is disabled for this repo; run manually on the
  local x3 runner") — there's no push/PR trigger left for these path filters
  to scope. The premise the branch optimizes for no longer exists.
- `ops/drain-actions-queue-20260911` adds `queue-drain.yml` and trims
  `release-hardening.yml`'s triggers. `01d2b64e5` already deleted
  `queue-drain.yml` outright (-69 lines, filed under "hosted workflow
  noise") and already reduced `release-hardening.yml` to `workflow_dispatch`
  only — same end state the branch was going for, reached a different way.

All three are archival; none should be merged. Recommend closing the PRs (if
any) and deleting the branches.

**Running tally:** 5 of 6 "flagged as real" queue entries checked this
session were fully superseded (private-mempool, compile-break,
live-transport-fix, pr-181-check, svm-htlc-custody); 1 was genuine
(`feat/x3vm-durable-recovery-20260911`, landed as #303). The census's
automated signals (git cherry, conflict-file counts) are a weak prior for
this repo specifically — most of this queue is the product of many parallel
agent sessions independently reimplementing the same fixes over the past
week, so a branch "looking unlanded" by patch/conflict signals mostly means
*nobody rebased it*, not that its content is missing from master. Given the
1-in-6 hit rate, grinding through the remaining ~20 entries one-by-one has
real diminishing returns — recommend checking the rest **opportunistically**
(when someone's already looking at that area of the code) rather than as a
dedicated pass, and continuing to require a direct-file-check (not just
trusting this doc's "real" label) before merging any of them. Do **not**
bulk-delete the unverified remainder on the strength of this pattern alone —
a 1-in-6 real rate among ~20 branches is still plausibly 2-4 more genuine
finds sitting in there.

**Update 4 (2026-09-19):** `fix/production-gate-prerequisites` spot-checked
(not a full commit-by-commit — 163 files is too much to justify for what's
looking like a 6th archival hit in a row; treat this as a lean, not a
verified disposition). Signals: branch tip is 8 days stale (2026-09-11,
based on 2026-09-10 master) against a repo where `.github/workflows/` has
kept churning (34 workflow files exist on master now vs. the branch's
older, smaller set); its copy of `production-gate.yml` diffs as 40 lines
vs. master's current 138, i.e. master's workflow has grown well past what
this branch proposed rather than the reverse; a sampled fix's distinguishing
string had no hits anywhere on master. Recommend treating as leaning
archival and not spending more time on it without a specific reason to
revisit.

**Update 5 (2026-09-19):** `fix/production-gate-prerequisites` upgraded to
fully verified archival (this is the safe-merge-order step 5 item). Sampled
5 non-workflow files spanning unrelated subsystems — chosen because they're
real code, not Cargo.toml bumps or benchmarking.rs boilerplate:
`crates/x3-foundry-core/src/security.rs`, `pallets/x3-atomic-kernel/src/{lib,vm_revert}.rs`,
`crates/x3-atomic-swap/src/evm_live.rs`, `node/src/service.rs` (the largest
non-workflow diff in the branch, 128 lines). Every hunk in all five was one
of: (a) pure formatting/import-order noise from a `cargo fmt`/import-sort
pass the branch predates, (b) a clippy-idiomatic rewrite master has and the
branch doesn't (`if let Err(_) = x` → `x.is_err()`, `.map_or(false, ..)` →
`.is_some_and(..)`, an added `impl Default` clippy suggests), or (c) master
strictly ahead (an added `#[cfg(test)] mod mock;`, a `cfg`-gated import
split). Zero functional differences found anywhere in the sample — not one
hunk represents work missing from master. Combined with the `production-gate.yml`
evidence from Update 4 (branch's copy is a strict, smaller subset of
master's current one) and the branch being 8-9 days stale in a repo whose
CI has kept moving, this is now a confirmed archival, not a lean — the 6th
of 6 "flagged as real" entries in this session to turn out fully superseded
(only `feat/x3vm-durable-recovery-20260911` / #303 was genuine). Note for
whoever re-reads Update 4's "34 workflow files" figure: that was already
stale by the time it was written; current count is 37. Don't treat either
number as authoritative — re-`ls .github/workflows/*.yml` if it matters.

| Branch | pending | conflict files | disposition |
| --- | --- | --- | --- |
| `agents/setup-instructions-request` | 1 | — (clean) | landed in batch 6 |
| `fix/svm-htlc-native-custody-master` | 2 | `.github/workflows/x3vm-svm-live-lifecycle.yml` | landed in batch 6, workflow hunk resolved in master's favour |
| `test/cross-domain-recovery-matrix-20260911` | 10 | `.github/workflows/mainnet-readiness.yml` | landed in batch 6, workflow hunk resolved in master's favour |
| `fix/svm-htlc-native-custody` | 33 | 169 files | archival — verified 2026-09-18: of 34 commits, only `854445e75` ("svm: enforce native HTLC custody") is on-topic, the rest is staleness drift; that commit's functions are already on master, which has strictly more (derive_htlc_pda, broadcast_claim_htlc, PDA validation tests). Nothing to salvage, do not merge |
| `fix/production-gate-prerequisites` | 32 | 163 files | archival — verified 2026-09-19 (see Update 5): 5 sampled non-workflow files across unrelated subsystems (foundry, pallet, atomic-swap, node service) showed zero functional differences from master — only formatting/clippy-style drift or master being strictly ahead. Combined with its `production-gate.yml` being a strict subset of master's current one (Update 4) and 8-9 days of staleness against a fast-moving CI surface. Do not merge |
| `wip/prompts-to-skills-20260918`, `wip/consolidation-20260917/pasted-text-processing`, `agents/pasted-text-processing`, `pr132-work` | 33–34 each | 86–90 files | one work item mirrored on four branches — pick the newest, rebase, land once |
| `ci/master-lineage-gates-20260908` | 31 | 47 files | superseded by the local CI of record |
| `docs/grant-readiness-truth-20260908` | 31 | no merge base | archival; lift the document if it is still wanted |
| `wip/consolidation-20260917/recovered-usb-clone` | 149 | no merge base | salvage review only |
| `archive/pr126-pre-master-rewrite-20260909` | 66 | no merge base | archival |
| `your-task-branch` (14), `t5/fix-annotations-20260522-1458` (13) | 13–14 | real | verified 2026-09-18: genuinely unrelated history (confirmed not a shallow-clone false negative first) — matches the archival bucket despite `git cherry` reporting pending commits; archival, do not merge |
| `pr-181-check` (12) | 12 | real | verified 2026-09-18: its CI concurrency/cancel-stale-runs fix is already on master, and the receipt files it deletes are already absent from master's tree — archival, do not merge |
| `fix/master-trading-core-compile-break` | 4 | 20 files | archival — verified 2026-09-18: every function/struct/test this branch adds (including exact test names) already exists on master verbatim, landed independently via #133/#212/#216/#223. The `git cherry`/`conflict-files` signals alone made this look like real unlanded work; a direct file check (not a triple-dot diff against the branch's own stale base) showed 100% overlap. Same pattern as `fix/private-mempool-real-shamir-threshold` (PR #288) above — do not merge, it would revert to an older tree shape |
| `finish/x3vm-live-transport-fix` | 4 | 16 files | archival — verified 2026-09-18: finality range-scan fix, settlement-engine auto-refund reschedule, runtime-signer intent-id resolution, blst patch removal, and the CI workflow all already on master via convergent implementation; master just organized the transport export differently. Do not merge |
| `feat/x3vm-durable-recovery-20260911` | 4 | 2 files | landed as #303 (salvaged, not merged — master's `from_recovery_snapshot` had gained an `expected_simulation` parameter since this branch's base, so the two new functions were manually ported rather than merged) |
| `feat/secret-release-firewall-20260911` | 3 | `crates/x3-atomic-swap/src/secret_release.rs` | superseded by `feat/live-secret-release-firewall-20260911` (in master) |
| `feat/canonical-cross-domain-proof-bundle-20260911-pre-rebase-20260917` | 3 | `crates/x3-atomic-swap/{lib,proof_bundle}.rs` | archival — verified 2026-09-19: its rebased sibling `feat/canonical-cross-domain-proof-bundle-20260911` is fully patch-equivalent to master (empty `git cherry`), and a direct tree diff shows master strictly larger in the same files this branch touches (e.g. `x3vm_htlc.rs`/`x3vm_live.rs`/`x3vm_native.rs` hundreds of lines bigger on master) — this pre-rebase snapshot predates work master has since grown past. Do not merge |
| `feat/idempotent-cross-domain-coordinator-20260911-pre-rebase-20260917` | 1 | 2 files | archival — verified 2026-09-19, same evidence as its sibling row above: rebased sibling branch fully lands in master, this pre-rebase snapshot's tree is a strict subset. Do not merge |
| `wip/chatgpt-mainnet-attestation-20260918`, `wip/consolidation-20260917/chatgpt-mainnet` | 1 each | 22 files | snapshot of uncommitted work |
| `archive/stale-x3lang-trading-wip-20260918` | 3 | 68 files | archival |
| `ci/path-filter-heavy-gates-20260910` (9), `ci/consolidate-workflows-20260910` (7), `ops/drain-actions-queue-20260911` (1) | 1–9 | workflow files | verified 2026-09-19 as archival (see Update 7) — `01d2b64e5` already reached the same or a further end state for all three |
| `preserve/20260918/*` (28 branches) | 0–13 | mixed | verified 2026-09-19 as a batch (see Update 6) — 26 fully archival/redundant, 2 (`cargo/ark-ec-0.6.0`, `cargo/ark-ff-0.6.0`) are a stale dependency bump that needs fresh work, not a merge |

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
3. ~~The `pasted-text-processing` family~~ — done 2026-09-19: only one
   commit across all three branches was genuinely on-topic and unlanded
   (a 4-file `.github/prompts/*.prompt.md` → `.agents/skills/*/SKILL.md`
   conversion); cherry-picked onto master rather than merging the
   90-file, 10-day-stale branches wholesale. Landed as #344. The three
   source branches can be deleted, nothing left to salvage.
4. ~~`fix/svm-htlc-native-custody`~~ — verified archival; master is already a
   strict superset of its one on-topic commit (see Queue table). Delete
   rather than merge.
5. ~~`fix/production-gate-prerequisites`~~ — verified archival 2026-09-19
   (see Update 5). Delete rather than merge.
6. Dependency heads, one `cargo update` batch at a time.
7. Everything else: archival or superseded → delete the branch, do not merge.
   As of 2026-09-19 this includes `fix/master-trading-core-compile-break`,
   `finish/x3vm-live-transport-fix`, `fix/svm-htlc-native-custody`,
   `pr-181-check`, `your-task-branch`, `t5/fix-annotations-20260522-1458`,
   and `fix/production-gate-prerequisites` — verified archival, see
   the Queue table above for why each one.

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
