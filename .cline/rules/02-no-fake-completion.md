# Rule: No Fake Completion

## Purpose
Prevent agents from using stubs, mocks, no-ops, or placeholder logic and calling them "implemented."

## Required Behavior
- When you write a function body, it must contain real logic, not a stub.
- If you must leave a stub, mark it explicitly: `// STUB: <reason> — NOT COMPLETE`.
- If you add a mock adapter, document it and note it does not count as working.
- Scan for `unimplemented!()`, `todo!()`, `panic!("not implemented")`, `return Ok(())`, `pass`, `# stubbed` before claiming completion.
- Run `scripts/x3-detect-stubs.sh` after every change.

## Forbidden Behavior
- Do NOT write `unimplemented!()` in production paths and claim the feature is done.
- Do NOT add `return Ok(())` no-ops in adapter/verifier/execution paths.
- Do NOT use `todo!()` as a permanent placeholder.
- Do NOT merge code with `panic!("stub")` or `panic!("not implemented")` into main.
- Do NOT add `#[ignore]` to hide test failures without explanation.
- Do NOT use `.skip`, `describe.skip`, or `it.skip` to silence broken tests.

## Proof Required
- Stub detector must run clean on runtime, adapter, consensus, bridge, verifier, execution, security, and rollback paths.
- Any remaining stubs must be documented in `docs/X3_COMPLETION_STATUS.md` as blockers.