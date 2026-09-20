# Stuck work inventory — 2026-09-18

Taken because the repo has work that exists only in local commits and only in
working trees, spread across six worktrees.

## 1. Commits that exist nowhere on `origin`

`git fetch origin --prune` ran first, so these are real, not stale refs.
**197 commits** are reachable from local branches and from no origin ref.

| Branch | Unique commits | Tip | Subject |
| --- | --- | --- | --- |
| `finish/x3vm-live-transport` | 56 | `921f81d64` | test(x3vm): reject duplicate and split terminal operations |
| `codex/x3-economic-safety-kernel` | 45 | `0c75dab06` | fix(x3lang): enforce submission profile consistency |
| `feat/live-secret-release-firewall-20260911` | 26 | `a10ac597c` | ci: run x3vm-svm-live-lifecycle.yml on live firewall stack |
| `feat/live-feature-matrix-20260912` | 25 | `e6c6b09e8` | fix(matrix): separate master paths from secret-release PR evidence |
| `test/cross-domain-refund-recovery-20260911` | 20 | `29a3dcaff` | ci: exercise SVM lifecycle on stacked recovery PR |
| `feat/settlement-proofset-gate-20260911` | 12 | `c1d8ba790` | test(settlement): prove local claims cannot finalize alone |
| `codex/x3-trading-core-v1-hardening` | 11 | `5df7d70b9` | style(x3-lang): apply rustfmt to trading verifier |
| `feat/x3-lang-crosschain-integration-20260909` | 5 | `ef83d0566` | docs(grant): add reproducible cross-chain demo commands |
| `finish/x3vm-live-transport-fix` | 2 | `42f9a2d77` | merge: rebase X3VM transport on current master |
| `ci/harden-self-hosted-jobs` | 1 | `2043f6d16` | ci: add job timeouts to self-hosted jobs on x3star1 |
| `ci/route-more-workflows-self-hosted` | 1 | `fb9a640da` | ci: route more real-compute workflows to x3 self-hosted runner |
| ~24 `dependabot/*` branches | 24 | — | dependency bumps |

Counts overlap — several branches share ancestry, so the distinct total is 197,
not the sum of the column.

## 2. Uncommitted work in working trees

| Worktree | Branch | Dirty files | Risk |
| --- | --- | --- | --- |
| `/tmp/x3-mine` | `master` (3f1176bce) | 10 modified + 2 untracked | **HIGH — lives in `/tmp`** |
| `~/Desktop/xxxstar-chatgpt` | `codex/chatgpt-mainnet-work` | 22 modified | medium |
| `~/Desktop/xxxstar-main` | `chore/reqwest-0.12-rust-highs` | 64 modified + 100 untracked | medium (durable) |
| `~/Desktop/xxxstar-main.worktrees/pasted-text-processing` | `agents/pasted-text-processing` | 8 changes | low |
| `~/.kilo/worktrees/hyper-lamp` | detached `b1d0a4ba5` | 0 | clean |
| `/tmp/claude-1000/.../wf-audit` | `ci/harden-self-hosted-jobs` | 0 | clean, commit unpushed |

Contents of the three real piles are backed up as patches under
`.ai/wip-backups/` (see the README there).

## 3. Branch state

- Local `master` = `3f1176bce`: **15 behind `origin/master`, 0 ahead** → clean
  fast-forward available. The 15 commits include PRs #193, #269, #270.
- `origin/master` = `b1d0a4ba5` (PR #270, risk-scorer trading-core fix).
- Main worktree is parked on `chore/reqwest-0.12-rust-highs` (`d3db02250`),
  which is **444 commits behind** `origin/master`. Any work done there is built
  on a badly stale base.
- No stashes, no rebase/merge/cherry-pick in progress, no `index.lock`.

## 4. Root cause of "stuck"

1. The main worktree is on a stale feature branch, so its tree does not match
   the real mainline — everything built there looks like a huge diff.
2. `.git` is mounted **read-only** inside the agent sandbox, so no agent
   session could commit, branch, or check out. This is the mechanical reason
   nothing has landed from automation. It is writable unsandboxed.
3. `master` is held by the `/tmp/x3-mine` worktree, which carries uncommitted
   work — so `master` can be neither fast-forwarded nor checked out elsewhere
   until that work is committed or stashed.

## 5. Recommended order of operations

1. Commit the `/tmp/x3-mine` pile onto its own branch (it is the only pile at
   risk of deletion with the machine's `/tmp`), then push.
2. Commit the `xxxstar-chatgpt` and `pasted-text-processing` piles onto their
   branches, then push.
3. Push the 11 real local-only branches listed in section 1.
4. Fast-forward local `master` to `origin/master`.
5. Move the main worktree off `chore/reqwest-0.12-rust-highs` onto `master`
   (its only uncommitted content is the repo-wide reqwest bump plus the
   x3-lang work described below).

Steps 1-3 push to a shared remote and need explicit approval.
