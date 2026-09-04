# Workflow: Add Tests

## When To Use
When adding test coverage to existing code — unit, integration, fuzz, property, or failure-path tests.

## Steps
1. Identify the target code and its current test coverage.
2. Map untested paths: happy path, failure paths, edge cases, property invariants.
3. Write tests. Each test must have a clear assertion, not just "run without panic".
4. Run the new tests — they may pass or expose real bugs.
5. If a new test exposes a bug, do NOT disable the test. Fix the source or document the gap.
6. Run the full test suite to check for regressions.
7. Run `scripts/x3-detect-test-cheats.sh`.
8. Update docs.
9. Run `scripts/x3-post-task.sh`.

## Required Checks
- New tests have real assertions.
- No `#[ignore]` or `.skip` without documented reason.
- No weakening of existing assertions.
- Gap between old and new coverage is documented.

## Proof Commands
- Test suite with new tests passing.
- `scripts/x3-proof-check.sh`.
- Test-cheat detector.

## Exit Criteria
- New tests added with real assertions.
- Full test suite passes.
- Coverage gap reduced.