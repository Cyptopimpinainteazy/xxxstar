# Workflow: Security Review

## When To Use
Before claiming any feature touching auth, keys, bridges, signatures, or asset transfers is complete.

## Steps
1. Identify all security-critical paths: auth, signing, key management, bridge message validation, rollback, replay protection.
2. For each path, verify real logic exists — no `return Ok(())` pass-throughs.
3. Search for hardcoded keys, seeds, or tokens.
4. Verify signature verification is not bypassable.
5. Verify bridge messages are validated (merkle proofs, signatures, finality).
6. Verify rollback and refund paths exist and are tested.
7. Verify replay protection on cross-chain operations.
8. Review unsafe code blocks.
9. Run `scripts/x3-detect-stubs.sh` with critical-path scanning.
10. File findings in proof report.

## Required Checks
- Auth checks are real.
- No hardcoded secrets.
- Bridge validation present.
- Rollback paths tested.
- Replay protection present.
- Unsafe blocks documented.

## Proof Commands
- `scripts/x3-detect-stubs.sh` on security paths.
- Manual grep for hardcoded keys/secrets.
- Rollback/replay test suite.

## Exit Criteria
- Critical paths reviewed.
- Findings documented.
- No HIGH-severity unresolved stubs in security paths.