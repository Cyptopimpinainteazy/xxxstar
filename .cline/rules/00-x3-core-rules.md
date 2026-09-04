# Rule: X3 Core Rules

## Purpose
Establishes the foundational operating law for all coding sessions. Every agent must follow these rules before, during, and after producing code.

## Required Behavior
- Inspect actual source files, not just markdown docs, before making claims.
- Identify the language, framework, test runner, and build system in use.
- Locate all relevant source directories, test directories, and config files.
- Use `scripts/x3-pre-task.sh` before starting work.
- Use `scripts/x3-post-task.sh` after finishing work.
- Run `scripts/x3-proof-check.sh` and never suppress its failures.
- Run `scripts/x3-detect-stubs.sh` and investigate every finding in runtime/security paths.
- Run `scripts/x3-detect-test-cheats.sh` before claiming tests pass.
- Update `docs/X3_PROOF_LEDGER.md`, `docs/X3_COMPLETION_STATUS.md`, and `docs/X3_NEXT_TASKS.md`.
- End every meaningful response with the X3 Proof Report format.

## Forbidden Behavior
- Do NOT claim "done", "complete", "wired", "working", "implemented", or "production-ready" without verified proof.
- Do NOT skip the stub detector or test-cheat detector.
- Do NOT fabricate proof output.
- Do NOT hide or suppress error output from proof commands.
- Do NOT mark a task complete because a file was created — wiring is required.

## Proof Required
- Proof commands must be run and their raw output referenced.
- Stub detector output must be clean for runtime/security paths.
- Test-cheat detector must show no unexplained removals.
- Status bar must reflect actual proof results, not optimistic estimates.