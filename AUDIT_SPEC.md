# Audit Specification

## Scope
- Rust workspace crates and runtime-critical pallets/modules
- `x3-lang` parser/typechecker/compiler/emitter/runner flows
- Bridge/consensus/execution paths used by production releases
- CI/CD gate definitions and repository guard scripts

## Required Gates
The release candidate must pass all of the following:

1. `make guard`
   - `scripts/agent_guard.py`
   - `scripts/no_stub_guard.py`
   - `scripts/test_cheat_guard.py`
2. `make test`
   - Python `x3-lang` parser/typechecker/e2e mocked suite
   - Rust compiler tests in `x3-lang/compiler`
3. `make audit`
   - `scripts/invariant_guard.py`
   - `scripts/mainnet_release_gate.py`
4. `make mainnet-check`
5. `make fresh-machine-check`

## Security Baseline
- No embedded private keys, mnemonics, or API key materials.
- No production stubs in critical paths (`TODO`, `FIXME`, `unimplemented!`, `todo!`, `panic!("stub")`, placeholders).
- No skipped/ignored tests in committed test files without explicit security review approval.

## Evidence Requirements
For each release candidate, archive:

- Command outputs for all required gates
- Commit SHA and branch name
- Date/time (UTC) and runner environment details
- Any exceptions with owner, risk, and expiration date

## Failure Policy
- Any failed gate blocks release.
- Any unresolved critical finding blocks release.
- Waivers require written approval by maintainers and must include expiry.

## Reporting
- Publish a short release audit summary in `RELEASE_GATES.md`.
- Track unresolved findings in `SECURITY.md` until closed.