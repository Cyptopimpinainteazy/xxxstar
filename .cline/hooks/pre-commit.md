# Hook: Pre-Commit

## When To Run
Before `git commit`. Installed as `.git/hooks/pre-commit` by `scripts/x3-install-git-hooks.sh`.

## Maps To
- `scripts/x3-detect-stubs.sh` (gating)
- `scripts/x3-detect-test-cheats.sh` (gating)

## What It Blocks
Committing code with stubs in critical paths or test-cheating patterns.

## Gate
BLOCKS commit if critical-path stubs found or test-cheat patterns detected.
Override with `SKIP_X3_CHECKS=1` in emergency (must be justified in proof ledger).