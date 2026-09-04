# Skill: Canonical Path + Reachability

Use this skill before claiming any feature is implemented.

This prevents you from building code that nothing calls.

## Required Check

```md
## Canonical Path + Reachability

### Canonical path

Document the one true execution path from user input to final state:

```txt
entrypoint (API/CLI/extrinsic/RPC)
    ↓
dispatcher / router
    ↓
core logic / algorithm
    ↓
state change / storage update
    ↓
event / result / response
```

Every step must be:
1. **Real** (not stubbed)
2. **Callable** (from a user entrypoint)
3. **Tested** (with passing test)

### Entrypoint

List every way the feature can be triggered:

| Entrypoint | Caller | Method | Public? |
|-----------|--------|--------|---------|
| POST /api/swap | user | HTTP | YES |
| extrinsic swap::execute | user | signed extrinsic | YES |
| CLI swap --from USDC --to ATOM | user | command line | YES |
| X3IR op: swap() | system | language | YES |

At least one entrypoint must be public and accessible.

### Changed code is called by

For each changed file/function:

| File | Function | Called by | Called where | Proof |
|------|----------|-----------|-------------|-------|
| swap.rs | execute() | dispatch | api::swap() | test_swap_api_calls_execute |
| settlement.rs | settle() | execute() | after lock | test_execute_calls_settle |

Prove with: test name, call trace, or command.

### Runtime reachable?

Prove reachability with:

- [ ] Unit test calling the function: `test_<function>_called_directly()`
- [ ] Integration test calling via API/extrinsic: `test_<feature>_via_api()`
- [ ] E2E test calling via public entrypoint: `test_<feature>_end_to_end()`

All three layers: function → API → user.

### Dead or duplicate paths found

If code can be called via multiple paths:

| Path | Used? | Canonical? | Action |
|------|-------|-----------|--------|
| api::swap() | YES | YES (public) | Keep |
| internal::swap() | NO | NO | Remove or clarify |
| legacy::swap_v1() | NO | NO | Deprecate |

Do not leave competing paths undefined.

### Proof

For each claim of reachability, provide proof:

```txt
Proof that execute() is called by dispatch():

1. Test: test_dispatch_calls_execute()
   cargo test --test integration -- test_dispatch_calls_execute

2. Code trace: runtime/dispatch.rs:42 calls swap::execute()

3. Command: cargo tree --edges normal | grep "dispatch.*execute"
```

If you cannot provide proof, the code is dead.
```

## Score Caps for Reachability

| Condition | Max Score |
|-----------|-----------|
| Unreachable code | 55% |
| Helper only, no entrypoint | 60% |
| Competing paths unresolved | 65% |
| Canonical path unclear | 70% |
| Function written, not called | 50% |

## How to Avoid Dead Code

### Before Writing Code

1. **Identify the entrypoint.** How will users trigger this?
   - HTTP endpoint? Command line? Extrinsic? RPC?
   - Write down the path from user action to your code.

2. **Identify the dispatcher.** What calls your code?
   - API router? Runtime dispatcher? Match handler?
   - Verify it actually makes the call (grep/callgraph).

3. **Write one integration test.** Prove the path works end-to-end.
   ```rust
   #[test]
   fn test_swap_entrypoint_to_result() {
       // User submits swap via HTTP API
       let response = client.post("/api/swap")
           .body(json!({"from": "USDC", "to": "ATOM", "amount": 100}))
           .send();
       
       // Verify response contains result
       assert!(response.status().is_success());
       assert_eq!(response.json()["result"], "completed");
   }
   ```

4. **Run the test.** If it fails, your entrypoint is broken.

### After Writing Code

1. **Search for callers.** Use grep or call hierarchy tool.
   ```bash
   grep -r "execute(" --include="*.rs" | grep -v "test" | grep -v "comment"
   ```

2. **Verify the caller is reachable.** Work backwards to public entrypoint.
   ```
   public_api() → dispatcher() → execute() ✓ (reachable)
   internal_helper() → (no public caller?) ✗ (dead code)
   ```

3. **Write a test from entrypoint to your code.**
   ```rust
   #[test]
   fn test_execute_reachable_from_api() {
       // This test should not exist for dead code
   }
   ```

## Approval Checklist

Before signing off on reachability:

- [ ] Canonical path is documented
- [ ] At least one public entrypoint exists
- [ ] Entrypoint is callable by users
- [ ] Changed code is called from the entrypoint
- [ ] Call chain is proven (test or trace)
- [ ] No competing paths without clarification
- [ ] Integration test proves end-to-end path works
- [ ] No dead helper functions without clear purpose

If any box is unchecked, reachability is incomplete.

---

**When this skill is complete:** Someone outside your team can run a command and trigger your feature from end-to-end. If they can't, the feature is not implemented.
