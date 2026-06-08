# Hook: Post-Task

## When To Run
After completing any coding work, before claiming completion.

## Maps To
`scripts/x3-post-task.sh`

## What It Blocks
Claiming completion without running proof, stub detection, test-cheat detection, and updating docs.

## Required Output
1. `scripts/x3-proof-check.sh` output
2. `scripts/x3-status-report.sh` output
3. `scripts/x3-update-proof-ledger.sh` output

## Gate
Blocks completion claim. If proof fails, status is FAIL/PARTIAL — agent must NOT claim success.