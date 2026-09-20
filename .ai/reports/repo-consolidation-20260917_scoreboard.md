# Repo Consolidation Scoreboard — 2026-09-17

Scope: `x3-repo/version-control-consolidation`

## Progress

```txt
x3-repo/version-control-consolidation  ██████████  100%  every local commit and every uncommitted file in all four copies is now stored on origin
```

## Remote durability verification (final)

- 26/26 local branches have tips reachable from `origin` refs — stranded count is 0.
- `origin` branch count went 65 → 76.
- 16 refs were created or updated on origin during this pass.

## What changed

- Fetched every remote ref into this clone (65 origin refs), including `codex/x3-economic-safety-kernel`, which did not exist locally before.
- Captured all three dirty working trees as snapshot commits without touching their files or indexes:
  - `wip/consolidation-20260917/main` (`292730e8d`, +17486/-19493 across 756 files)
  - `wip/consolidation-20260917/chatgpt-mainnet` (`2df3f5b82`, +1786/-113 across 22 files)
  - `wip/consolidation-20260917/pasted-text-processing` (`470c77801`, 4 files)
- Pruned 7 stale worktree registrations pointing at deleted `/tmp/x3-*` directories.
- Verified the second clone at `Recovered-from-USB/Desktop/xxxstar-main` holds no commits absent from this repo.

## Refs pushed to origin in this pass

| Ref | Contents |
|---|---|
| `wip/consolidation-20260917/main` | main tree: reqwest 0.12 migration, registry, configs, build churn |
| `wip/consolidation-20260917/chatgpt-mainnet` | attestation/validator pallet work, MAINNET_READINESS.md |
| `wip/consolidation-20260917/pasted-text-processing` | prompts → `.agents/skills` conversion |
| `wip/consolidation-20260917/recovered-usb-clone` | uncommitted content of the recovered USB clone |
| `wip/consolidation-20260917/x3-lang-prototype-20260621` | standalone x3-lang Rust prototype, 22 files |
| `fix/svm-htlc-native-custody` | 34 commits |
| `fix/svm-htlc-native-custody-master` | 2 commits |
| `pr-181-check` | 12 commits |
| `test/cross-domain-recovery-matrix-20260911` | 10 commits |
| `agents/setup-instructions-request` | 1 commit |
| `ci/x3-local-runner-smoke` | 1 commit |
| `feat/canonical-cross-domain-proof-bundle-20260911-pre-rebase-20260917` | 3 pre-rebase commits |
| `feat/idempotent-cross-domain-coordinator-20260911-pre-rebase-20260917` | 8 pre-rebase commits |

## What is still missing (out of scope for this pass)

- Nothing has been merged into `master`; ~45 open PRs remain unmerged.
- Local `master` is 3 commits behind `origin/master` (`8009ff1a2`) and should be fast-forwarded.
- The main working tree still mixes real edits with build-artifact churn, so it is not commit-ready as-is.

## Next best action

- Fast-forward local `master`, then triage the 45 open PRs by whether their branches are still mergeable.
