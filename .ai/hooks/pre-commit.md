# Workflow Hook: Pre-Commit

Run before any commit.

This hook ensures the code is ready and the score is honest.

## Required Output

```md
## Pre-Commit Checklist

### Changed Files

```txt
git diff --name-only
```

List:
- <file 1>
- <file 2>

### Validation Commands

Must all pass:

```txt
cargo fmt --all --check                      ✓ PASS / ✗ FAIL
cargo check --workspace                      ✓ PASS / ✗ FAIL
cargo clippy --workspace --all-targets       ✓ PASS / ✗ FAIL
cargo test --workspace                       ✓ PASS / ✗ FAIL
cargo test --all-features                    ✓ PASS / ✗ FAIL
cargo test --test integration                ✓ PASS / ✗ FAIL
```

All tests passing? YES / NO

If NO, do not commit. Fix failing tests first.

### Regression Results

Upstream regression check:
- Command: `cargo test --package <upstream>`
- Result: PASS / FAIL / SKIPPED

Downstream regression check:
- Command: `cargo test --package <downstream>`
- Result: PASS / FAIL / SKIPPED

Both pass? YES / NO

If NO, fix regressions before commit.

### Score Cap Applied

Estimated score: <percent>
Applied caps:
- <cap 1>: <reason>
- <cap 2>: <reason>

Strictest cap: <percent>
Final score: min(estimated, cap) = <percent>

### Completion Scoreboard

```txt
<module>/<subsystem>  █████░░░░░  50%  Status: <honest status>
```

### Still Missing

What remains undone?

| Item | Type | Reason | Next Owner |
|------|------|--------|-----------|
| X3IR emitter | OUT_OF_SCOPE | Phase 2 | @alice |
| Bridge timeout test | BLOCKED | Waiting for proof service | @bob |
| Audit | DEFERRED | Scheduled month 2 | @security-team |

### FIXABLE_NOW Check

Remaining FIXABLE_NOW items?
- [ ] NO (safe to commit)
- [ ] YES (must fix before commit, do not proceed)

If YES, return to post-edit. Fix all FIXABLE_NOW items, then return here.

### Commit Safety

Safe to commit?
- [ ] YES
  - All tests pass
  - No regressions
  - Score is honest
  - No FIXABLE_NOW items
  - Handoff pack is ready

- [ ] NO
  - <reason>
  - <blocker 1>
  - <blocker 2>

Do not commit if NO.

### Commit Message

```txt
<module>: <what changed>

<optional longer description>

Test: <test name or command>
Score: <percent>% — <status>
Remaining: <BLOCKED / DEFERRED items only>

Signed-off-by: <agent>
```

Example:

```txt
parser: Add support for custom transfer syntax

Added new syntax: custom_transfer(to, amount, memo)
- Parser recognizes new syntax
- Lowers to X3IR CustomOp
- Emitter generates runtime dispatch call
- All tests pass

Test: cargo test --test parser_custom_transfer
Score: 65% — Parser and AST work; emitter needs hardening for edge cases
Remaining: DEFERRED emitter optimization, BLOCKED stress test (in q2)

Signed-off-by: x3ir-compiler-agent
```
```

## Approval Checklist

Before committing:

- [ ] Format check passes
- [ ] Compilation passes
- [ ] All tests pass
- [ ] No regressions
- [ ] Score cap applied
- [ ] Scoreboard updated
- [ ] Missing work classified
- [ ] No FIXABLE_NOW items
- [ ] Commit message is clear
- [ ] No secrets in code or logs

If any box is unchecked, do not commit. Fix the issue first.

---

**Rule:** Commit only when you would be comfortable if someone pulled this code and ran it immediately. If not, keep working.
