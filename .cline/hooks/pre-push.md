# Hook: Pre-Push

## When To Run
Before `git push`. Installed as `.git/hooks/pre-push` by `scripts/x3-install-git-hooks.sh`.

## Maps To
- `scripts/x3-proof-check.sh` (gating)

## What It Blocks
Pushing code that fails proof checks.

## Gate
BLOCKS push if proof commands fail.
Override with `SKIP_X3_CHECKS=1` in emergency (must be justified in proof ledger).