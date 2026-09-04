# Hook: Proof Gate

## When To Run
Before claiming any feature complete.

## Maps To
`scripts/x3-proof-check.sh`

## What It Blocks
Claiming completion when proof commands fail.

## Gate
HARD GATE. Proof must PASS (exit 0) for a completion claim.
If proof fails, agent must report PARTIAL or FAILED.
No override permitted — fix the failures.