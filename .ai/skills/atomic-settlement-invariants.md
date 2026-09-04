# Skill: Atomic Settlement Invariants

Use this skill when touching bridge, Cross-VM settlement, asset movement, mint/burn, lock/unlock, DEX, staking, or treasury logic.

This prevents half-complete state and supply breakage.

## Required Invariants

```txt
CRITICAL INVARIANT (supply):

canonical_supply == native + evm + svm + external_locked + pending

This must hold before, during, and after every operation.

ATOMIC EXECUTION INVARIANTS:

intent_id executes at most once.
  If replayed: second execution rejected, no state change.

nonce is consumed exactly once.
  If duplicated: second attempt rejected, no state change.

failed settlement cannot mint/release.
  If dest settlement fails: no asset created, no state change.

timeout refund cannot double-spend.
  Refund and settlement are mutually exclusive (one succeeds, other fails).

rollback cannot create supply.
  If operation rolls back: all changes reversed, balances intact.

compensation cannot exceed locked value.
  Refund amount must equal original lock amount (no creation/loss).

destination release requires valid source proof.
  If source didn't lock: dest cannot release (no ghost withdrawals).

chain_id/domain separator prevents cross-domain replay.
  Proof meant for chain A cannot execute on chain B.

SAFETY INVARIANTS:

No authorization check bypass.
  Only authorized callers can move funds.

No state inconsistency.
  Before operation: balances OK
  After operation: balances OK
  Never: half-locked, half-minted state

All events match state changes.
  Event: "Transfer 100 ATOM"
  State: balance decreased by exactly 100
  Never: event without state change or vice versa
```

## Required Output

```md
## Atomic Settlement Invariant Check

### Invariants relevant to this change
- <invariant 1>
- <invariant 2>

### What could break each invariant

For each invariant:

| Invariant | Failure Mode | Prevention |
|-----------|-------------|-----------|
| canonical_supply preserved | mint without lock | source lock verified before dest mint |
| intent_id executes once | replay | nonce/proof-hash checked |
| nonce consumed once | duplicate nonce | nonce storage checked |
| ... | | |

### Tests proving safety

For each invariant:

- [ ] test_<invariant>_preserved_after_operation()
  - Proof: before state + operation + after state all satisfy invariant

### Failure cases tested

- [ ] test_replay_rejected()
- [ ] test_failed_settlement_no_mint()
- [ ] test_timeout_refund_no_double_spend()
- [ ] test_partial_failure_rolled_back()

### Supply breakdown

```txt
Before operation:
  native:        1000 ATOM
  evm:           2000 USDC
  locked:          100 ATOM
  pending:         0
  total:         1100 ATOM value

Operation: EVM → Native swap (exchange rate 1:1)

After operation:
  native:        1100 ATOM (gained 100 from swap)
  evm:           1900 USDC (lost 100)
  locked:          0 ATOM (released)
  pending:         0
  total:         1100 ATOM value (preserved!)
```

### Result
- PASS / FAIL / NOT RUN
```

## How to Write Supply Invariant Tests

### Test the supply is preserved

```rust
#[test]
fn test_canonical_supply_preserved_after_settlement() {
    // Get starting state
    let before = SupplySnapshot {
        native_balance: get_native_balance(),
        evm_balance: get_evm_balance(),
        locked: get_locked_balance(),
        pending: get_pending_balance(),
    };
    let before_total = before.sum();
    
    // Execute settlement
    let proof = generate_proof();
    let result = settle_on_destination(proof);
    assert!(result.is_ok(), "Settlement should succeed");
    
    // Get ending state
    let after = SupplySnapshot {
        native_balance: get_native_balance(),
        evm_balance: get_evm_balance(),
        locked: get_locked_balance(),
        pending: get_pending_balance(),
    };
    let after_total = after.sum();
    
    // Verify supply is preserved
    assert_eq!(before_total, after_total, 
        "Supply broken! Before: {}, After: {}", before_total, after_total);
}
```

### Test atomicity (intent executes at most once)

```rust
#[test]
fn test_intent_id_executes_once() {
    let intent_id = 123;
    let proof = generate_proof_for_intent(intent_id);
    
    // First execution succeeds
    let result1 = execute_intent(intent_id, proof.clone());
    assert!(result1.is_ok(), "First execution should succeed");
    let state_after_first = get_state();
    
    // Second execution with same intent_id must fail
    let result2 = execute_intent(intent_id, proof);
    assert!(result2.is_err(), "Duplicate execution must be rejected");
    assert_eq!(result2.err(), Some(Error::DuplicateIntentId));
    
    // Verify state did not change after second attempt
    let state_after_second = get_state();
    assert_eq!(state_after_first, state_after_second, "State changed on duplicate!");
}
```

### Test rollback preserves state

```rust
#[test]
fn test_failed_settlement_rolls_back() {
    let before_balance = get_balance(user);
    let before_locked = get_locked(user);
    
    // Start settlement
    lock_on_source(user, 100);
    assert_eq!(get_balance(user), before_balance - 100, "Lock should reduce balance");
    assert_eq!(get_locked(user), before_locked + 100, "Locked should increase");
    
    // Destination settlement fails (bad proof, timeout, etc.)
    let bad_proof = generate_bad_proof();
    let result = settle_on_destination(bad_proof);
    assert!(result.is_err(), "Bad proof should fail");
    
    // Verify rollback: state restored
    assert_eq!(get_balance(user), before_balance, "Balance should be restored");
    assert_eq!(get_locked(user), before_locked, "Locked should be released");
}
```

## Hard Rules

1. **Every operation that touches balances must test supply invariant.**
   - mint, burn, transfer, lock, unlock, deposit, withdraw, refund, swap

2. **Every operation that can replay must test replay invariant.**
   - bridge settlement, cross-chain dispatch, proof-based operations

3. **Every operation with timeout must test timeout refund invariant.**
   - HTLC, escrow, time-locked operations

4. **Every operation that can fail must test rollback invariant.**
   - Two-phase operations, external calls, settlement

5. **Missing a test = caps your score hard.**
   - No supply test → max 45%
   - No replay test → max 55%
   - No timeout test → max 60%
   - No rollback test → max 60%

## Approval Checklist

Before signing off on atomic settlement invariants:

- [ ] All relevant invariants are identified
- [ ] Supply invariant is tested
- [ ] Replay invariant is tested (if applicable)
- [ ] Timeout refund invariant is tested (if applicable)
- [ ] Rollback invariant is tested (if applicable)
- [ ] All tests pass
- [ ] Supply breakdown shows balance after each step

If any box is unchecked, invariant testing is incomplete.

---

**Next:** Coordinate with domain agent to ensure invariant tests are part of final validation.
