# Workflow: Start Task

## When To Use
Before beginning any coding task, feature, bugfix, or refactor.

## Steps
1. Run `scripts/x3-pre-task.sh` to get current state snapshot.
2. Read `docs/X3_COMPLETION_STATUS.md` for current area-level status.
3. Read `docs/X3_NEXT_TASKS.md` for prioritized next work.
4. Identify the language, test runner, and proof commands for the target area.
5. State the concrete goal: what will change, what proof will verify it.

## Required Checks
- Branch is clean or changes are intentional.
- No existing proof failures are being ignored.
- Target area is not blocked by missing dependencies.

## Proof Commands
- Run `scripts/x3-pre-task.sh` and include its output.

## Exit Criteria
- Pre-task snapshot captured.
- Goal stated with proof criteria.
- Known blockers identified.