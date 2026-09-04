# Workflow: Mainnet Readiness Review

## When To Use
Before claiming any feature, RC, or release is mainnet-ready.

## Steps
1. Verify all production gate requirements from `.clinerules/production-gates.md` are met.
2. Run `make guard` if available, or equivalent invariant checks.
3. Run full test suite — all tests must pass, no skipped/ignored tests without documentation.
4. Run security review workflow.
5. Run cross-VM review workflow if applicable.
6. Verify all stubs in production paths are resolved.
7. Verify all rollback/replay/failure-path tests pass.
8. Run `scripts/x3-proof-check.sh` — must exit 0.
9. Run `scripts/x3-detect-stubs.sh` — must be clean on runtime/security/bridge paths.
10. Verify deployment scripts and configuration are production-grade.
11. File mainnet-readiness assessment in proof report.

## Required Checks
- All production gates pass.
- Full test suite green.
- Security review done.
- Cross-VM review done (if applicable).
- Stub detector clean on critical paths.
- Deployment config reviewed.

## Proof Commands
- `make guard` or equivalent.
- Full test suite.
- `scripts/x3-proof-check.sh`.
- `scripts/x3-detect-stubs.sh`.

## Exit Criteria
- All gates pass.
- Proof report filed with mainnet-readiness verdict.
- Remaining gaps documented as blockers.