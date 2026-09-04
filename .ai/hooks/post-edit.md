# Workflow Hook: Post-Edit

Run after code changes are made.

This hook validates the work and prepares for final output.

## Required Output

```md
## Post-Edit Validation

### Changed Files

List all files modified:
- `path/to/file1.rs`
- `path/to/file2.rs`
- <etc>

### Evidence Gate

For each change, provide proof it works:

**Proof:** <test name or command>
- Files changed: <list>
- Tests added: <list>
- Commands run: <list>
- Results: <output excerpt>

### Stub / Mock / Fake Scan

Search changed code for:

```txt
grep -r "TODO\|FIXME\|unimplemented\|todo!\|panic!\|stub\|mock\|fake\|hardcoded\|demo" <changed_files>
```

Found issues:
- [ ] <issue>: <location> — FIX IMMEDIATELY or flag as BLOCKED

### Reachability Check

For each changed function:

| Function | Called by | Entrypoint | Reachable? | Test |
|----------|-----------|-----------|----------|------|
| execute() | dispatch() | /api/swap | YES | test_swap_api |

### Regression Check

**Upstream dependencies checked?**
- [ ] YES / NO

Which subsystems call changed code?
- <subsystem 1>
- <subsystem 2>

Tests run to verify no regression:
- cargo test --package <upstream> -- --nocapture

Result: PASS / FAIL

**Downstream dependencies checked?**
- [ ] YES / NO

Which subsystems does changed code call?
- <subsystem 1>
- <subsystem 2>

Tests run to verify those calls still work:
- cargo test --package <downstream> -- --nocapture

Result: PASS / FAIL

### Test Integrity Gate

Have any tests been weakened?
- [ ] YES / NO

If YES, revert. If NO, continue.

### Missing Work Classification

For each gap found:

| Item | Type | Status | Reason |
|------|------|--------|--------|
| X3IR emitter incomplete | OUT_OF_SCOPE | DEFERRED | Scheduled for phase 2 |
| Replay test missing | FIXABLE_NOW | OPEN | Add before final output |
| Audit required | BLOCKED | OPEN | Blocked on security team schedule |

Types:
- `FIXABLE_NOW` = Can fix in this session
- `BLOCKED` = External dependency
- `OUT_OF_SCOPE` = Not in task definition
- `DEFERRED` = Deliberately postponed
- `SECURITY_RISK` = Security gap
- `INVARIANT_RISK` = Invariant violation
- `REGRESSION_RISK` = May break existing code

### If FIXABLE_NOW items remain:

Return to build mode. Do not proceed to final output.

### If only BLOCKED/DEFERRED/OUT_OF_SCOPE remain:

Proceed to final output with honest classification.
```

## Validation Commands

Before post-edit is complete, run:

```bash
# Format
cargo fmt --all --check

# Type check
cargo check --workspace

# Lints
cargo clippy --workspace --all-targets -- -D warnings

# Tests
cargo test --workspace

# All features
cargo test --all-features

# Integration tests (if applicable)
cargo test --test integration

# Invariant tests (if applicable)
cargo test --package invariants
```

All must pass.

## Approval Checklist

Before proceeding to pre-commit:

- [ ] All changed files are listed
- [ ] Evidence gate shows proof of work
- [ ] Stub/mock/fake scan completed
- [ ] Reachability proven for all new code
- [ ] Regression checks passed (upstream and downstream)
- [ ] Test integrity verified (no weakened tests)
- [ ] All FIXABLE_NOW items resolved
- [ ] Missing work is classified honestly
- [ ] Validation commands pass

If any box is unchecked, post-edit is incomplete. Do not commit yet.

---

**Next:** If FIXABLE_NOW items remain, return to coding. Otherwise, proceed to pre-commit.
