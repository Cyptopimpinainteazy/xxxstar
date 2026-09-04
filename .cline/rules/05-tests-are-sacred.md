# Rule: Tests Are Sacred

## Purpose
Tests are the proof mechanism. Modifying tests to hide code failures is sabotage. Failing tests signal real problems.

## Required Behavior
- Run the full test suite for the changed area, not just the file you touched.
- If a test fails, fix the SOURCE code, not the test — unless the test is provably wrong.
- If you must remove a test, document why in the proof report and replace it with a stronger test.
- New features require new tests covering the happy path AND at least one failure path.
- Run `scripts/x3-detect-test-cheats.sh` before claiming any test suite passes.

## Forbidden Behavior
- Do NOT modify test assertions just to make CI green.
- Do NOT remove test cases that expose real bugs.
- Do NOT add `#[ignore]`, `.skip`, `describe.skip`, or `it.skip` to silence failures without explanation.
- Do NOT weaken assertions (e.g., `expect(value)` to `expect(value >= 1)` just to pass).
- Do NOT replace unit tests with snapshot tests to dodge assertion failures.
- Do NOT delete integration tests because they're "too slow" or "flaky" — fix the flakiness.

## Proof Required
- Test-cheat detector output before claiming test suite passes.
- Git diff of test changes reviewed for weakening/skipping/removing patterns.
- New test count listed in proof report.