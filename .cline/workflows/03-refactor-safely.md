# Workflow: Refactor Safely

## When To Use
When restructuring existing code without changing behavior.

## Steps
1. Run the full test suite before starting — this is your baseline.
2. Identify all call sites, imports, and dependents of the code being refactored.
3. Make the structural change.
4. Run the full test suite — it must produce identical pass/fail counts as baseline.
5. If any test now fails that previously passed, fix the refactor, not the test.
6. Run `scripts/x3-detect-stubs.sh` — no new stubs from refactoring.
7. Run `scripts/x3-detect-test-cheats.sh`.
8. Update docs.
9. Run `scripts/x3-post-task.sh`.

## Required Checks
- Test pass/fail count unchanged from baseline.
- No new stubs introduced.
- No test assertions weakened.

## Proof Commands
- Test suite before and after.
- `scripts/x3-proof-check.sh`.
- Git diff reviewed for suspicious removals.

## Exit Criteria
- Behavior unchanged (pre- and post- test suite identical in results).
- Code structure improved.
- No regressions.