# Hook: Pre-Task

## When To Run
Before starting any coding work.

## Maps To
`scripts/x3-pre-task.sh`

## What It Blocks
Starting work without understanding current state, dirty files, or existing proof failures.

## Required Output
- Current branch
- Dirty files list
- Detected languages
- Available proof commands
- Current status from `docs/X3_COMPLETION_STATUS.md`
- Current next 10 tasks from `docs/X3_NEXT_TASKS.md`
- Last proof result from `.x3/proof/latest-proof.log`

## Gate
Informational only. Does not block work, but must be run.