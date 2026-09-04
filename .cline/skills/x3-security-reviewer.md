# Skill: X3 Security Reviewer

## Purpose
Check auth, signing, replay protection, unsafe code, secret handling, bridge safety, and external calls.

## Use When
- Before any feature goes to testnet or mainnet.
- When touching auth, keys, signatures, bridges, or asset paths.
- During security review workflow.

## Inputs To Inspect
- Auth-related code in pallets, runtime, adapters.
- Key management and secret storage.
- Signature verification paths.
- Bridge message validation.
- Unsafe Rust blocks.
- External RPC/API calls.
- Hardcoded addresses, keys, endpoints.

## Checks To Perform
- Auth: real checks, not `return Ok(())`.
- Keys: no hardcoded private keys or seeds.
- Signatures: verification cannot be bypassed.
- Bridge: incoming messages are validated (proofs).
- Unsafe: documented, minimized, reviewed.
- Secrets: env vars or vault, not source code.
- Permissions: least privilege enforced.

## Proof To Require
- Grep for hardcoded keys/secrets — must be clean.
- Grep for `unsafe` — must be documented.
- Stub detector clean on security paths.
- Auth tests pass.

## Output Format
- Auth: PASS / FAIL (reason)
- Key Management: PASS / FAIL
- Signature Verification: PASS / FAIL
- Bridge Validation: PASS / FAIL
- Unsafe Blocks: <count>, all documented / undocumented
- Verdict: PASS / FAIL