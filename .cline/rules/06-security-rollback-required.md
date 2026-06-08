# Rule: Security and Rollback Required

## Purpose
Every production feature must consider security, rollback, replay, and failure paths before being called complete.

## Required Behavior
- Auth checks must be real, not `return Ok(())` pass-throughs.
- Secret handling must use proper key management, not hardcoded strings.
- Bridge paths must validate signatures/merkle proofs, not trust incoming data.
- Rollback paths must exist and be tested — revert state, refund assets, cancel operations.
- Replay protection must be present on all cross-chain operations.
- Unsafe code must be documented, minimized, and reviewed.

## Forbidden Behavior
- Do NOT ship `// TODO: add auth` in production paths.
- Do NOT hardcode API keys, private keys, or seed phrases.
- Do NOT skip signature verification with `if true` or `// skip for now`.
- Do NOT ship bridge code that doesn't verify source chain proofs.
- Do NOT ship rollback logic that is never tested.
- Do NOT claim mainnet-readiness without security review.

## Proof Required
- Security review check must pass or be explicitly waived with reason.
- Rollback tests must exist and pass.
- Replay protection must be demonstrable.
- Unsafe code blocks must be audited.