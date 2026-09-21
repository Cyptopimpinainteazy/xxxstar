# Stuck-work backups — 2026-09-18

Safety copies of uncommitted work that was sitting only in working trees,
taken because two of the affected worktrees live under `/tmp`, which this box
has a history of wiping mid-session.

These are **backups, not a resolution**. Nothing here has been committed,
merged, or pushed.

## What is here

| File | Source worktree | What it is |
| --- | --- | --- |
| `x3-mine-20260918-1053.patch` | `/tmp/x3-mine` (branch `master`) | tracked-file diff: custody-service HSM + service, external-chains adapter, cross-chain-position-manager accounting, `scripts/local-ci.sh`, `docs/reports/SECURITY_BLOCKERS.md`, Cargo.toml/lock |
| `x3-mine-untracked-20260918-1053/` | `/tmp/x3-mine` | the two untracked files: `scripts/check-workspace-membership.py`, `.ai/workspace-membership-baseline.txt` |
| `chatgpt-mainnet-20260918-1053.patch` | `~/Desktop/xxxstar-chatgpt` (branch `codex/chatgpt-mainnet-work`) | tracked-file diff: confidential-gpu attestation, x3-validator-attestation, x3-atomic-kernel, CI workflows, `MAINNET_READINESS.md` |
| `chatgpt-untracked-20260918-1053.list` | `~/Desktop/xxxstar-chatgpt` | paths of untracked files (contents not copied) |
| `pasted-text-processing-20260918-1053.patch` | `~/Desktop/xxxstar-main.worktrees/pasted-text-processing` | tracked-file diff (prompts → `.agents/skills` conversion, mostly deletions) |

## How to restore one of these

From the worktree it came from, with a clean tree:

```bash
git -C <worktree> apply --3way /home/lojak/Desktop/xxxstar-main/.ai/wip-backups/<file>.patch
```

Untracked files must be copied back by hand; they are plain files, not diffs.

## What was NOT backed up here

- The main worktree's own uncommitted changes. Those are on the durable
  filesystem already, not in `/tmp`.
- Any commit that exists only on a local branch. Those live in `.git`, which is
  durable; they need pushing, not copying.
