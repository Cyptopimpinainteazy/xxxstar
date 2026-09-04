# Workflow: Fix Bug

## When To Use
When fixing a reported or discovered bug.

## Steps
1. Reproduce the bug with a test that fails before the fix.
2. Inspect the source, trace execution path.
3. Fix the source code — do not modify the reproduction test to pass.
4. Verify the reproduction test now passes.
5. Run the full test suite for the area to check for regressions.
6. Add failure-path tests if the bug exposed an untested path.
7. Run `scripts/x3-detect-stubs.sh`.
8. Run `scripts/x3-detect-test-cheats.sh`.
9. Update docs.
10. Run `scripts/x3-post-task.sh`.

## Required Checks
- Reproduction test exists and now passes.
- No regression in existing tests.
- Root cause fixed, not symptom masked.

## Proof Commands
- Reproduction test output (before and after).
- Full area test suite.
- `scripts/x3-proof-check.sh`.

## Exit Criteria
- Bug reproduction test passes.
- Full test suite passes.
- Proof report filed.