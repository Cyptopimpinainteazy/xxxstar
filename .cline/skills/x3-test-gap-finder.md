# Skill: X3 Test-Gap Finder

## Purpose
Identify missing unit, integration, property, fuzz, and failure-path tests. Every execution path needs coverage.

## Use When
- After implementation, before claiming complete.
- During test-repair workflow.
- When coverage is unknown.

## Inputs To Inspect
- `tests/` — integration tests.
- `*tests.rs` — inline unit tests.
- `*test*.rs`, `*.spec.ts`, `*.test.ts` — test files.
- `integration-tests/` — integration test suites.
- Cargo.toml `[dev-dependencies]` — test tooling.
- `#[ignore]`, `.skip`, `describe.skip` — suppressed tests.

## Checks To Perform
- Happy path: at least one test exists.
- Failure path: at least one error/failure test per module.
- Edge cases: boundary values, empty inputs, max values.
- Cross-VM paths: integration tests for each VM pair.
- Rollback paths: refund/timeout tests.
- Property invariants: fuzz or property tests for critical math.

## Proof To Require
- Test file inventory mapped to source modules.
- Gap report: which paths lack tests.
- Suppressed tests list with reasons.

## Output Format
- Module: <name>
- Happy path tests: <count>
- Failure path tests: <count>
- Edge case tests: <count>
- Gaps: [list of missing test scenarios]
- Suppressed tests: [list with reasons]