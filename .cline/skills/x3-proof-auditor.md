# Skill: X3 Proof Auditor

## Purpose
Verify every completion claim against actual command output. The auditor is the final arbiter of whether something is actually done.

## Use When
- After any implementation, bugfix, or refactor.
- Before filing a proof report.
- When another agent claims something is "done."

## Inputs To Inspect
- Changed files (via git diff).
- `scripts/x3-proof-check.sh` output.
- `scripts/x3-detect-stubs.sh` output.
- `scripts/x3-detect-test-cheats.sh` output.
- Test suite output.
- Runtime wiring (is code reachable from execution path).

## Checks To Perform
- Did proof commands actually run?
- Did they pass or fail?
- Are stubs present in critical paths?
- Were tests weakened or removed?
- Is the code wired into runtime/build/CLI?
- Is the completion claim supported by evidence?

## Proof To Require
- Proof command output with exit codes.
- Clean stub detector on critical paths.
- Clean test-cheat detector.
- Wiring verification.

## Output Format
- Verdict: PASS / PARTIAL / FAIL
- Evidence summary
- Gaps identified