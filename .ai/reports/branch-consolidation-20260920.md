# Branch consolidation — are the branches merged? (2026-09-20)

Question asked: "did you push and merge" / "lets get all branches on main / merge all work".

Answer, measured: **master is pushed** (`origin/master` = local `master` = `3ab992eb3`, confirmed with
`git ls-remote`). There is **no `main` branch** — GitHub's default for this repo is `master`
(`git ls-remote --symref origin HEAD` -> `ref: refs/heads/master`). Most of the work already *is* on
master: of the refs that still exist, 193 are contained in master outright and the branch commits that
remain are, for the ones inspected, stale re-implementations of code master already carries.

## Method

```bash
# refs considered: every origin/* ref plus local branches with no upstream
git for-each-ref --format='%(refname:short)' refs/remotes/origin | grep -v '/HEAD$'
git for-each-ref --format='%(refname:short) %(upstream:short)' refs/heads | awk '$2==""{print $1}'
# per ref
git rev-list --count master..<ref>          # commits not on master
git diff --name-only master...<ref>         # files that differ
git cherry master <tip>                     # which patches are absent (- = already equivalent)
# content-landed metric: of the lines the branch adds, how many appear verbatim in master's copy of
# the same file (lock/generated files skipped; see /tmp/landed.py for the exact filter)
```

## Numbers

- 343 distinct refs (origin + local-only).
- **193 refs are already contained in master** (`git rev-list --count master..<ref>` == 0). Their work is
  on master; the ref is a leftover.
- 150 refs carry commits master does not have, but they collapse to **90 distinct commits** (the rest are
  `preserve/2026…`, `archive/local-20260920/…` and local copies of the same tip).
- Of those 90 tips: **47 have >=90% of their added lines already present verbatim in master**,
  24 are partial, 19 have nothing attributable (lock/generated files only). 20 are
  dependabot version bumps.
- **No branch has master as an ancestor** — every remaining ref was cut before master's tip and moved on,
  so not one of them merges as a fast-forward. `git apply --check` on the three x3-lang candidates:
  every x3-lang file conflicts.

## The x3-lang candidates, verified by file content (not by the score alone)

| branch | landed | what master already has |
|---|---|---|
| `origin/salvage/x3lang-intent-bridge` | 87.5% | `numeric.py` rejects a Decimal that narrows to inf, and `runner.py` reads `intent.get('requires') or []` — master carries both: `numeric.py:26-38` narrows and checks `isfinite`, `runner.py:144` uses `or []` with the typechecker's null normalization above it. |
| `origin/wip/x3lang-arb-graph-filter-20260919` | 33.8% | master's `compiler/src/arb.rs` documents and implements the same feature — bounds judged against the opportunity graph via `venue_standings`, 'a declaration no venue survives is refused with every venue and the bound'. The branch is the pre-rewrite text of it. |
| `add-slippage` | 93.3% | master's `vm/src/trading.rs` carries the measured-slippage path (`X3_SLIPPAGE_ABOVE_CEILING`, `report_outcome(measured_profit_bps, measured_slippage_bps, ...)`); `x3c run --measured-slippage-bps` discharges it. |

A low percentage here means *the branch's text is not master's text*, not *the feature is missing*:
`arb.rs` on the branch is the pre-rewrite spelling of master's graph-judged bounds, so almost none of its
lines match while the capability is present.

## What merging the rest would take

The 150 refs are pre-rewrite lineage (`behind master` by 830-1010 commits each). A wholesale merge would
drag rewritten-away history back into master and re-open the conflicts that the rewrite resolved. The
honest unit of work is one branch at a time: diff the branch against master, keep the hunks whose
capability master lacks, drop the rest, run the workspace needle. That is filed as TICKET-126 with the
per-branch table below as its work list.

## Appendix — all 90 distinct unmerged tips, least-landed first

`landed%` = share of the branch's added lines present verbatim in master's copy of the same files
(-1 = nothing attributable: lock/generated files only). `ahead` = commits not on master.

| landed% | ahead | adds | same | files | branch |
|---|---|---|---|---|---|
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/sp-keystore-54f11b1` |
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/sc-rpc-api-54f11b1` |
| n/a | 179 | 0 | 0 | 0 | `origin/wip/consolidation-20260917/recovered-usb-clone` |
| n/a | 218 | 0 | 0 | 0 | `origin/archive/pr126-pre-master-rewrite-20260909` |
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/frame-support-54f11b1` |
| n/a | 69 | 0 | 0 | 0 | `origin/fix-x3lang-python` |
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/pallet-scheduler-54f11b1` |
| n/a | 7 | 0 | 0 | 7 | `origin/ci/consolidate-workflows-20260910` |
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/sc-rpc-54f11b1` |
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/sc-consensus-aura-54f11b1` |
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/sc-basic-authorship-54f11b1` |
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/sp-version-54f11b1` |
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/pallet-sudo-54f11b1` |
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/sc-cli-54f11b1` |
| n/a | 14 | 0 | 0 | 0 | `origin/your-task-branch` |
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/frame-benchmarking-cli-54f11b1` |
| n/a | 1 | 0 | 0 | 1 | `origin/dependabot/cargo/pallet-staking-54f11b1` |
| n/a | 13 | 0 | 0 | 0 | `origin/t5/fix-annotations-20260522-1458` |
| n/a | 179 | 0 | 0 | 0 | `origin/docs/grant-readiness-truth-20260908` |
| 0.0% | 1 | 1 | 0 | 2 | `origin/fix/agent-guard-bip39-allow` |
| 0.0% | 14 | 7 | 0 | 8 | `deps-mod-test` |
| 0.0% | 3 | 6 | 0 | 3 | `origin/docs/readiness-score-reconciliation` |
| 0.0% | 1 | 87 | 0 | 22505 | `origin/wip/consolidation-20260917/x3-lang-prototype-20260621` |
| 0.9% | 9 | 116 | 1 | 9 | `origin/ci/path-filter-heavy-gates-20260910` |
| 6.4% | 3 | 109 | 7 | 2 | `origin/ops/drain-actions-queue-20260911` |
| 17.3% | 1 | 13963 | 2414 | 756 | `origin/wip/consolidation-20260917/main` |
| 28.6% | 1 | 448 | 128 | 6 | `origin/fix/foundry-real-evm-deploy` |
| 31.8% | 31 | 453 | 144 | 47 | `origin/ci/master-lineage-gates-20260908` |
| 33.8% | 1 | 237 | 80 | 6 | `origin/wip/x3lang-arb-graph-filter-20260919` |
| 51.5% | 1 | 66 | 34 | 1 | `origin/fix/fake-metrics-honesty` |
| 53.1% | 1 | 382 | 203 | 22 | `origin/wip/consolidation-20260917/chatgpt-mainnet` |
| 53.1% | 1 | 382 | 203 | 22 | `origin/wip/chatgpt-mainnet-attestation-20260918` |
| 56.1% | 12 | 173 | 97 | 34 | `origin/pr-181-check` |
| 59.6% | 1 | 2456 | 1465 | 17 | `origin/wip/x3lang-preserve-packets-and-arbitrage-20260919` |
| 67.6% | 5 | 108 | 73 | 16 | `origin/fix/workspace-membership-batch-5` |
| 68.4% | 35 | 1603 | 1097 | 86 | `origin/agents/pasted-text-processing` |
| 68.6% | 36 | 1612 | 1106 | 90 | `origin/wip/consolidation-20260917/pasted-text-processing` |
| 68.6% | 36 | 1612 | 1106 | 90 | `origin/wip/prompts-to-skills-20260918` |
| 80.0% | 1 | 5 | 4 | 5 | `origin/ci/harden-self-hosted-jobs` |
| 82.5% | 1 | 143 | 118 | 9 | `origin/fix/x3lang-frame-classification` |
| 84.8% | 4 | 348 | 295 | 2 | `origin/feat/x3vm-durable-recovery-20260911` |
| 87.5% | 1 | 8 | 7 | 5 | `origin/ci/route-more-workflows-self-hosted` |
| 87.5% | 1 | 8 | 7 | 2 | `origin/salvage/x3lang-intent-bridge` |
| 90.0% | 11 | 528 | 475 | 8 | `origin/archive/local-20260920/codex/x3-trading-core-v1-hardening` |
| 91.8% | 7 | 716 | 657 | 3 | `origin/merge/all-work-c3095b883` |
| 92.1% | 45 | 2972 | 2738 | 26 | `origin/archive/local-20260920/codex/x3-economic-safety-kernel` |
| 92.3% | 1 | 13 | 12 | 1 | `origin/docs/merge-queue-production-gate-lean` |
| 92.4% | 1 | 937 | 866 | 17 | `pr132-work` |
| 93.3% | 1 | 105 | 98 | 4 | `add-slippage` |
| 93.3% | 33 | 2230 | 2081 | 163 | `origin/fix/production-gate-prerequisites` |
| 93.5% | 39 | 3591 | 3356 | 17 | `origin/archive/local-20260920/test/cross-domain-refund-recovery-20260911` |
| 93.5% | 34 | 2431 | 2274 | 169 | `fix/svm-htlc-native-custody` |
| 93.6% | 45 | 3821 | 3575 | 18 | `origin/archive/local-20260920/feat/live-secret-release-firewall-20260911` |
| 93.8% | 20 | 2678 | 2512 | 16 | `origin/finish/x3vm-live-transport-fix` |
| 94.1% | 3 | 10376 | 9761 | 68 | `origin/archive/stale-x3lang-trading-wip-20260918` |

## Refresh — local heads only, 2026-09-21

Re-ran the patch-equivalence check against current `master` for every local `refs/heads/*`:

```text
patch_equivalent=214
novel=51
```

This is a smaller, local-head snapshot than the origin-wide table above. The 214 patch-equivalent
branches carry no `git cherry master <branch>` `+` patches; the 51 novel branches still need the
one-at-a-time `diff -> keep missing capability -> drop the rest` review TICKET-126 prescribes.

### x3-lang-specific novel-head verdicts

- `salvage/x3lang-intent-bridge` — already on master. The branch's `numeric.py` float-narrowing guard
  and `runner.py` `or []` / `or {}` normalization are both present verbatim in the current tree.
- `wip/x3lang-arb-graph-filter-20260919` — pre-rewrite spelling. Current `compiler/src/arb.rs` already
  implements graph-judged venue bounds with the same tests (`test_arb.rs`,
  `test_opportunity_graph.rs`).
- `archive/stale-x3lang-trading-wip-20260918` — stale. Trading Core v1 was later implemented and merged
  through `codex/x3-trading-core-v1`; this branch is the earlier WIP and should remain archived.
- `fix-x3lang-python` — unrelated lineage with no merge base against master; archive rather than merge.
- `wip/x3lang-preserve-packets-and-arbitrage-20260919` — pre-rewrite spelling. Its new
  `compiler/src/arbitrage.rs` is the older `arb` plan contract; master's `compiler/src/arb.rs`
  already implements the same PHASE 37 `ArbDecl`/`ArbPlan` machinery. Its VM
  `opportunity_packet.rs` and tests already exist on master too. The branch should be archived rather
  than merged.
| 94.2% | 56 | 3917 | 3691 | 25 | `origin/archive/local-20260920/finish/x3vm-live-transport` |
| 94.5% | 5 | 73 | 69 | 5 | `origin/archive/local-20260920/feat/x3-lang-crosschain-integration-20260909` |
| 94.9% | 21 | 2604 | 2470 | 14 | `origin/archive/local-20260920/finish/x3vm-live-transport-fix` |
| 95.4% | 1 | 151 | 144 | 4 | `origin/fix/x3lang-proof-vocabulary` |
| 95.9% | 12 | 733 | 703 | 7 | `origin/archive/local-20260920/feat/settlement-proofset-gate-20260911` |
| 96.8% | 8 | 409 | 396 | 3 | `origin/archive/local-20260920/feat/idempotent-cross-domain-coordinator-20260911` |
| 96.9% | 3 | 450 | 436 | 2 | `origin/archive/local-20260920/feat/canonical-cross-domain-proof-bundle-20260911` |
| 97.2% | 1 | 720 | 700 | 19 | `agents/setup-instructions-request` |
| 97.2% | 18 | 719 | 699 | 18 | `origin/batch/20260918T2015Z` |
| 97.5% | 4 | 609 | 594 | 20 | `origin/codex/x3-trading-core-v1-hardening` |
| 98.3% | 3 | 58 | 57 | 14 | `origin/rebase-310` |
| 98.4% | 7 | 1237 | 1217 | 20 | `origin/fix/master-trading-core-compile-break` |
| 99.0% | 2 | 193 | 191 | 7 | `origin/fix/svm-htlc-native-custody-master` |
| 99.0% | 1 | 209 | 207 | 5 | `origin/fix/private-mempool-real-shamir-threshold` |
| 99.7% | 10 | 694 | 692 | 12 | `origin/test/cross-domain-recovery-matrix-20260911` |
| 99.7% | 25 | 2959 | 2951 | 21 | `origin/archive/local-20260920/feat/live-feature-matrix-20260912` |
| 100.0% | 1 | 34 | 34 | 1 | `origin/docs/merge-queue-production-gate-prereq-verified` |
| 100.0% | 1 | 1 | 1 | 1 | `origin/dependabot/pip/psycopg2-binary-gte-2.9.13` |
| 100.0% | 1 | 48 | 48 | 1 | `origin/fix/relayer-real-solana-verifier-351` |
| 100.0% | 1 | 123 | 123 | 5 | `origin/fix/foundry-core-creator-fee-remainder` |
| 100.0% | 1 | 1 | 1 | 2 | `origin/dependabot/cargo/ark-ec-0.6.0` |
| 100.0% | 1 | 2 | 2 | 3 | `origin/dependabot/cargo/hex-literal-1.1.0` |
| 100.0% | 1 | 5 | 5 | 2 | `origin/ci/foundry-crate-test-gate-110` |
| 100.0% | 2 | 657 | 657 | 2 | `origin/design/x3lang-root-compiler-bridge` |
| 100.0% | 1 | 1 | 1 | 2 | `origin/dependabot/cargo/redis-1.7.0` |
| 100.0% | 11 | 3 | 3 | 4 | `deps-batch-test` |
| 100.0% | 1 | 1 | 1 | 2 | `origin/dependabot/cargo/ark-std-0.6.0` |
| 100.0% | 1 | 1 | 1 | 2 | `origin/dependabot/cargo/ark-ff-0.6.0` |
| 100.0% | 3 | 600 | 600 | 2 | `origin/feat/secret-release-firewall-20260911` |
| 100.0% | 1 | 2 | 2 | 3 | `origin/dependabot/cargo/libloading-0.9.0` |
| 100.0% | 1 | 1 | 1 | 2 | `origin/dependabot/cargo/minicbor-2.3.0` |
| 100.0% | 1 | 39 | 39 | 1 | `origin/ci/x3-local-runner-smoke` |
| 100.0% | 1 | 67 | 67 | 2 | `origin/fix/private-mempool-share-validation-hardening` |
| 100.0% | 2 | 149 | 149 | 1 | `origin/land-durable-recovery-v2` |
| 100.0% | 1 | 391 | 391 | 10 | `origin/retarget-batch8` |

## The 51 local novel heads

```text
add-slippage
agents/pasted-text-processing
agents/setup-instructions-request
archive/pr126-pre-master-rewrite-20260909
archive/stale-x3lang-trading-wip-20260918
batch/20260918T2015Z
ci/consolidate-workflows-20260910
ci/master-lineage-gates-20260908
ci/path-filter-heavy-gates-20260910
codex/x3-economic-safety-kernel
dependabot/cargo/ark-ec-0.6.0
dependabot/cargo/ark-ff-0.6.0
deps-batch-test
deps-mod-test
deps/batch-low-risk
docs/grant-readiness-truth-20260908
feat/canonical-cross-domain-proof-bundle-20260911
feat/canonical-cross-domain-proof-bundle-20260911-pre-rebase-20260917
feat/idempotent-cross-domain-coordinator-20260911
feat/idempotent-cross-domain-coordinator-20260911-pre-rebase-20260917
feat/live-secret-release-firewall-20260911
feat/secret-release-firewall-20260911
feat/settlement-proofset-gate-20260911
feat/x3vm-durable-recovery-20260911
finish/x3vm-live-transport
finish/x3vm-live-transport-fix
fix-x3lang-python
fix/agent-guard-bip39-allow
fix/foundry-real-evm-deploy
fix/production-gate-prerequisites
fix/svm-htlc-native-custody
fix/svm-htlc-native-custody-master
fix/workspace-membership-batch-5
merge/all-work-c3095b883
ops/drain-actions-queue-20260911
pr-181-check
rebase-310
salvage/x3lang-intent-bridge
t5/fix-annotations-20260522-1458
test/cross-domain-recovery-matrix-20260911
test/cross-domain-refund-recovery-20260911
wip/chatgpt-mainnet-attestation-20260918
wip/consolidation-20260917/chatgpt-mainnet
wip/consolidation-20260917/main
wip/consolidation-20260917/pasted-text-processing
wip/consolidation-20260917/recovered-usb-clone
wip/consolidation-20260917/x3-lang-prototype-20260621
wip/prompts-to-skills-20260918
wip/x3lang-arb-graph-filter-20260919
wip/x3lang-preserve-packets-and-arbitrage-20260919
your-task-branch
```

### Dependency-bump novel heads

The following are dependency-version branches, not capability branches. They modify `Cargo.toml` and
`Cargo.lock` only:

- `dependabot/cargo/ark-ec-0.6.0`
- `dependabot/cargo/ark-ff-0.6.0`
- `dependabot/cargo/ark-std-0.6.0`
- `deps-batch-test`
- `deps-mod-test`

Disposition: review against the current dependency policy and archive unless a specific upgrade is
still needed. They add no product capability to master.

### Runtime/settlement novel heads

These branches touch live-transport, proof-bundle, coordinator, or settlement files whose key files
already exist on current `master`:

- `feat/x3vm-durable-recovery-20260911`
- `finish/x3vm-live-transport`
- `finish/x3vm-live-transport-fix`
- `feat/settlement-proofset-gate-20260911`
- `feat/idempotent-cross-domain-coordinator-20260911`
- `feat/canonical-cross-domain-proof-bundle-20260911`

Disposition: pre-rewrite or partial lineages; review individual hunks only if a named capability is
missing from master, otherwise archive.

`fix/agent-guard-bip39-allow` is already represented on master: the mobile SDK already uses the bip39
2.x `parse_in`/`to_seed` API, and `scripts/agent_guard.py` already has the corresponding
`Mnemonic::parse_in` pattern. The branch's only addition is comment wording; archive it.

`add-slippage` is already represented on master: `vm/src/executor.rs` and `vm/src/x3_lang_vm.rs`
carry the measured-slippage state and `X3_SLIPPAGE_ABOVE_CEILING` enforcement. The branch is a
pre-rewrite/partial spelling of that path and should be archived.

### Correction: not archive material

`codex/x3-economic-safety-kernel` carries real x3-lang work that is **not all on master**. A bulk
cherry-pick attempted on a clean master worktree conflicts in `lib.rs` and the economic test files,
and taking the whole tree would revert newer master work such as bridge `min_receive`. This branch is
**merge-worthy** and must be landed with a focused rebase that keeps the newer master hunks.

Measured blocker 2026-09-21: the branch's `compiler/src/ir.rs` is an older IR tree that removes the
master re-exports (`ReleaseAct`, `ComparisonOp`, `ChoiceCriterion`) and newer operation variants.
Cherry-picking the top economic commits with `-X theirs` therefore fails the full `x3-lang` build.
The focused merge must first reconcile `ir.rs`, then apply the economic policy/commitment changes on
top of master's current IR.

Second measured blocker: the branch's `EconomicPolicy` still expects `max_total_cost`,
`max_price_impact_bps`, `max_mev_leakage_bps`, and a bare `quote_freshness_blocks` field. Master's
`CompiledTradingPolicy` intentionally removed those unenforced aliases and now carries
`minimum_net_profit_asset`, `max_oracle_deviation_bps`, and `max_cumulative_loss`. The branch's
economic layer is therefore stale relative to master and must be rewritten against the current
policy schema rather than merged directly.

Resolution: current master already has the newer `x3-lang/vm/src/economic.rs` and
`x3-lang/vm/tests/economic_types.rs`. The branch's economic-safety-kernel layer is superseded by
master's evolved schema; no merge is needed for this branch.

### Already-represented runtime branches

- `feat/secret-release-firewall-20260911`
- `feat/live-secret-release-firewall-20260911`
- `finish/x3vm-live-transport`
- `finish/x3vm-live-transport-fix`
- `feat/settlement-proofset-gate-20260911`
- `feat/canonical-cross-domain-proof-bundle-20260911`
- `test/cross-domain-recovery-matrix-20260911`
- `test/cross-domain-refund-recovery-20260911`

Their key files (`secret_release.rs`, `x3vm_htlc.rs`, `x3vm_live.rs`, `x3vm_native.rs`, and the
settlement/coordinator files) already exist on master. These branches are pre-rewrite or partial
lineages; archive unless a named capability is missing from master.

### Remaining novel heads by first-file subsystem

The remaining heads group as:

- CI/workflow-only (`.github/*`, `.dockerignore`, `.ai/*`, `.serena/*`): configuration and process
  changes, not product capability.
- Runtime/crate (`crates/*`, `pallets/*`): needs targeted hunk review.
- `docs/grant-readiness-truth-20260908` and `fix-x3lang-python` have no merge base against current
  master and are archive material.

`agents/pasted-text-processing` is **merge-worthy**, not archive: it updates CI/workflows and SVM
programs plus `scripts/agent_guard.py` allow-list patterns and a PR-supervisor test. It needs a
focused review to keep only the changes master lacks.
