# Push / merge state — 2026-09-20

Question asked: "did you push and merge on GitHub?"

## Answer

Yes. GitHub `refs/heads/master` = `dffabc32f`. Every commit from this line of
work is on the remote, and the local checkout on this machine is now identical
to it.

Verified with `git ls-remote origin refs/heads/master` (not with a
remote-tracking ref, which can be stale — it was).

## What was actually wrong before this pass

| thing | state found | state now |
|---|---|---|
| GitHub master | `2419bc2fe` | `dffabc32f` |
| user's checkout `/home/lojak/Desktop/xxxstar-main` | `920a44775`, **144 commits behind** | `dffabc32f`, identical to remote |
| `.ai/merge-queue.md` round-72 section | modified, uncommitted | committed and pushed |
| `.ai/reports`, `.ai/memory`, `.ai/runlogs`, `.ai/wip-backups` | **102 files, never committed**, no gitignore rule | committed and pushed |
| 14 branches whose tips were ahead of their GitHub copy | local-only commits | pushed as `archive/local-20260920/<branch>` |
| uncommitted work in 3 `/tmp` worktrees | would die with `/tmp` | patches committed under `.ai/wip-backups/20260920-*.patch` |

The stale remote-tracking ref is worth calling out: any check that reads
`origin/master` locally can report the wrong answer, because the canonical
checkout had not fetched since `93d762b5c` while GitHub was at `2419bc2fe`.
`git ls-remote` is the only honest source.

## The 3 worktrees that held uncommitted work

| worktree | branch | tip | captured as |
|---|---|---|---|
| `/tmp/x3-ext` | `fix/external-chains-honest-adapters` | `f2ff29841` | `20260920-fix-external-chains-honest-adapters.patch` |
| `/tmp/x3-gov` | `mainnet/block-hook-panics` | `f3bc2f2b4` | `20260920-mainnet-block-hook-panics.patch` (+ 19 untracked listed) |
| `/tmp/x3-kernel` | `fix/kernel-authority-bounds` | `af6710c7e` | `20260920-fix-kernel-authority-bounds.patch` (275 KB) |

`git stash create` fails in all three (exit 1), so `git diff HEAD --binary` was
used instead. Patches are additive records; the worktrees were not modified.

## The "unapplied patches" backlog — measured, not guessed

`git cherry -v origin/master <branch>` counts patches whose change is *not*
represented on master. 8 branches carry 51 such patches:

| branch | unapplied | patch-equivalent |
|---|---|---|
| `feat/live-secret-release-firewall-20260911` | 13 | 31 |
| `test/cross-domain-refund-recovery-20260911` | 12 | 26 |
| `finish/x3vm-live-transport` | 9 | 47 |
| `feat/settlement-proofset-gate-20260911` | 7 | 5 |
| `finish/x3vm-live-transport-fix` | 4 | 16 |
| `feat/canonical-cross-domain-proof-bundle-20260911` | 3 | 0 |
| `codex/x3-economic-safety-kernel` | 2 | 42 |
| `feat/idempotent-cross-domain-coordinator-20260911` | 1 | 7 |

They are **not** merged, deliberately. The merge-queue doc states the rule:
`git cherry` is what stops a "merge everything" pass from reverting newer work.
All 8 are September 11–12 lineages; master has moved 144 commits past them.

### Spot-check: the headline feature of each is on master already

| branch | headline | evidence on master |
|---|---|---|
| `feat/settlement-proofset-gate-20260911` | canonical cross-domain proof bundle | `crates/x3-atomic-swap/src/proof_bundle.rs` exists |
| `feat/live-secret-release-firewall-20260911` | secret release firewall | `crates/x3-atomic-swap/src/secret_release.rs` exists |
| `finish/x3vm-live-transport` | export native X3 node transport | `crates/x3-atomic-swap/src/lib.rs:106,112` exports `NativeX3NodeTransport`, `X3ExtrinsicSigner`, `X3FinalizedInclusionProof`, `X3NodeTransportConfig` |
| `codex/x3-economic-safety-kernel` | versioned economic commitments | `x3-lang/vm/src/economic.rs` (10.8 KB) + `vm/tests/economic_types.rs` |

That is 4 of 8 branches checked at their headline, not 51 patches checked
individually. The remaining 4 branches are recorded as unverified, not as
cleared.

## Commands

```
git ls-remote origin refs/heads/master                       # 2419bc2fe -> then dffabc32f
git fetch origin --prune
git stash push -m "..." -- .ai/merge-queue.md
git merge --ff-only origin/master                            # 920a44775 -> 2419bc2fe
git stash pop                                                # clean
git rev-list --left-right --count origin/<b>...<b>           # divergence per branch
git cherry -v origin/master <b>                              # unapplied vs patch-equivalent
git push origin refs/heads/<b>:refs/heads/archive/local-20260920/<b>
git diff HEAD --binary > .ai/wip-backups/20260920-<name>.patch
```

## Not done, on purpose

- **No force pushes.** 14 branch pushes were rejected as non-fast-forward. The
  remote copies had diverged. Force-pushing would have destroyed remote commits
  (`finish/x3vm-live-transport-fix` had 20 commits not in master). The local tips
  were archived under `archive/local-20260920/` instead — nothing lost, nothing
  overwritten.
- **No bulk merge of the 61 archival branches** listed in `.ai/merge-queue.md`.
