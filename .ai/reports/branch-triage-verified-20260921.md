# Archival branch re-verification (2026-09-21)

Question: does any remote branch hold real work that master does not have?

Prior triage (`branch-triage-20260921.md`) compared each branch's added lines against
**the same file** on master. That is wrong in both directions, so the question was
re-derived from scratch against `origin/master` = `ec241c258`.

## Method

1. **Patch-id equivalence.** 901 non-merge commits on master; 817 commits reachable from
   remote branches but not master; **432** carrying a patch-id that is not on master.
2. **Global content test.** master's whole tree reduced to 2,721,581 unique normalised
   lines; each branch's added lines checked against that set globally rather than per file.
3. **Read the code.** Patch-id inequality is *not* evidence of missing work.

## Result

Of the 432 novel patches, 375 distinct subjects. Almost all are CI/workflow rewrites, docs,
dependency bumps, or superseded May–June work. Two items were real and both are now on
master:

| item | evidence | landed |
| --- | --- | --- |
| `on_vote` counted votes with no verified proposal | new test FAILS on unmodified master (`cert2` was `Some`); 19 tests pass after | `#403` `eb7f36130` |
| `x3-foundry-core` could not deploy anything for real | `#399` only renamed the fabrication to `simulate_deploy_contracts`; real path replayed onto current master | `#404` `88020532a` |

## False positives worth naming (checked, already on master under other patches)

- `3873420c0` x3-pq "verification fails closed" — master's `x3-pq` already fails closed.
- `e72205188` private-mempool identity/duplicate share rejection — `git diff origin/master origin/fix/private-mempool-share-validation-hardening -- crates/private-mempool/src` is empty.
- `1cf5ec1e4`, `5ad4a6bd5` SVM HTLC native custody — master's `handle_claim_htlc` pays native SOL out of the PDA.
- `3ac8b960f` EVM claim/refund event + finality verification — master's `evm_live.rs` has `parse_receipt`/event checks.
- `fbc09542` x3-lang decimal overflow and null policies — master's `numeric.py`/`runner.py` already carry the newer form.
- `dc20f19a6`, `89c4e4ca8`, `95ff4c8e0`, `0c75dab06` trading-core features — `ReceiptReplayLedger`, `max_cumulative_loss`, `simulate_atomic`, oracle deviation are on master (`git grep -l` finds them).
- `30dfd6c33`, `3a41c8e19` workspace membership batches — master's `Cargo.toml` lists the crates.

## Deliberately off master

- x3-lang WIP branches: master's `arb.rs` is the newer PHASE 37 (it has `venue_standings`); the
  branch-only `arbitrage.rs` is a *second* PHASE 37, preserved on purpose by `1bfaa5243`
  ("kept off master on purpose").
- `codex/x3-economic-safety-kernel`: master removed those ceilings deliberately.
- `preserve/*`, `archive/local-20260920/*`: snapshots behind master. A `git diff` of `+0/-N`
  means the branch is master minus N lines.

## Commands

```bash
git log --no-merges -p --format='commit %H' origin/master | git patch-id --stable
git rev-list --no-merges $(cat remotes.txt) --not origin/master
git log --no-merges -p --format='commit %H' --no-walk=unsorted $(cat novel-commits.txt) | git patch-id --stable
LC_ALL=C comm -23 novel-pidset.txt master-pidset.txt
```

## Still unlanded, on purpose or by expense

- Local `master` in the main worktree: 8 commits ahead / 10 behind origin. The one code commit
  (`a4f63684c`) is now on origin as `1e5ef77e6`; the rest are doc/evidence commits plus the
  bip39 guard fix that landed as `#400`.
- 34 branches are patch-equivalent to master and safe to delete whenever the user wants.
