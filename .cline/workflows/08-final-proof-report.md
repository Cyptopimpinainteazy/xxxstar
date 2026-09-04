# Workflow: Final Proof Report

## When To Use
At the end of every coding session — this is the mandatory output format.

## Steps
1. List all files changed during the session.
2. Run `scripts/x3-proof-check.sh` and capture output.
3. Run `scripts/x3-detect-stubs.sh` and capture findings.
4. Run `scripts/x3-detect-test-cheats.sh` and capture findings.
5. Read `docs/X3_COMPLETION_STATUS.md` for current area statuses.
6. Compose the X3 Proof Report:
   - Claim
   - Status Bar
   - Files Changed
   - Proof Commands Run
   - Proof Result
   - Proven
   - Not Proven Yet
   - Blockers
   - Next Best Task
   - Next 10 Tasks
   - No-Bullshit Verdict
7. Append to `docs/X3_PROOF_LEDGER.md`.
8. Update `docs/X3_COMPLETION_STATUS.md` if completion percentages changed.
9. Update `docs/X3_NEXT_TASKS.md` with exactly 10 tasks.
10. Run `scripts/x3-post-task.sh`.

## Required Checks
- All proof commands run (not skipped).
- Status bar filled with honest percentages.
- 10 concrete next tasks listed.
- No fake completion language.

## Proof Commands
- `scripts/x3-proof-check.sh`
- `scripts/x3-detect-stubs.sh`
- `scripts/x3-detect-test-cheats.sh`

## Exit Criteria
- X3 Proof Report posted.
- Proof ledger updated.
- Completion status updated.
- Next 10 tasks updated.