# Hook: Test-Cheat Detector

## When To Run
Before committing or claiming tests pass.

## Maps To
`scripts/x3-detect-test-cheats.sh`

## What It Blocks
Committing or claiming green test suite when tests have been weakened, skipped, deleted, or disabled.

## Gate
HARD GATE. If suspicious test changes detected, agent must explain each one or revert.
Override only with documented justification in proof ledger.