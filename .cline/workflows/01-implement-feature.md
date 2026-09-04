# Workflow: Implement Feature

## When To Use
When adding a new feature, module, pallet, contract, or endpoint.

## Steps
1. Follow Start Task workflow.
2. Inspect existing similar features for patterns.
3. Write source code with real logic, no stubs.
4. Write tests covering happy path + at least one failure path.
5. Wire the feature into the runtime/build/CLI/router.
6. Run the strongest proof command for the language.
7. Run `scripts/x3-detect-stubs.sh`.
8. Run `scripts/x3-detect-test-cheats.sh`.
9. Update `docs/X3_PROOF_LEDGER.md`.
10. Update `docs/X3_COMPLETION_STATUS.md`.
11. Update `docs/X3_NEXT_TASKS.md`.
12. Run `scripts/x3-post-task.sh`.

## Required Checks
- Source compiles cleanly.
- Tests pass.
- Feature is wired into runtime (verifiable via grep/runtime config).
- Stub detector clean on runtime/security paths.
- Test-cheat detector shows no suspicious changes.

## Proof Commands
- Language-appropriate build + test.
- `scripts/x3-proof-check.sh`.
- For Rust pallets: verify `construct_runtime!` entry.

## Exit Criteria
- Feature code exists and compiles.
- Tests pass (happy + failure path).
- Wiring confirmed.
- Proof report filed.