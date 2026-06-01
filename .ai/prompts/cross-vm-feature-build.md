# Cross-VM Feature Build Prompt

Use this prompt when building Cross-VM features.

Cross-VM is where the whole system lives or dies. Every step must be perfect.

## Feature Definition

```
Feature:
<DESCRIBE WHAT YOU ARE BUILDING>

Supported source domains:
- Native runtime / EVM / SVM / BTC / External

Supported destination domains:
- Native runtime / EVM / SVM / BTC / External
```

## The Cross-VM Law

> **Cross-VM behavior must be atomic, auditable, deterministic, replay-safe, and invariant-preserving.**

Every single one of those words matters.

## Step 1: Define the Canonical Execution Path

Document the exact sequence from user intent to final state:

```txt
user intent / API call / extrinsic
    ↓
X3IR representation
    ↓
dispatcher decision (which domains?)
    ↓
source domain lock/burn
    ↓
proof generation
    ↓
relay to destination
    ↓
destination domain verification
    ↓
destination settlement (mint/release)
    ↓
finality recorded
    ↓
both domains consistent
```

Every step must be:
1. **Named** (what is it called?)
2. **Testable** (how do we test it?)
3. **Recoverable** (what if it fails?)

## Step 2: Choose Your Atomicity Model

Which ONE model applies?

- **Single synchronous execution:** Source and destination in one atomic transaction. All-or-nothing.
- **Two-phase commit:** Lock on source, then settle on destination. Rollback if destination fails.
- **Lock/mint:** Source locks, destination mints immediately when proof arrives.
- **Burn/release:** Source burns, destination releases. No going back.
- **HTLC (Hash Time Locked Contract):** Locked with a secret. Refund if timeout.
- **Rollback/compensation:** If destination fails, compensate on source (restore funds).
- **Async settlement:** Optimistic: assume success until proven otherwise. Compensation if wrong.

Document why this model is correct for your feature:
```
Atomicity model: <chosen>

Reasoning:
- Why this model: <explain>
- What guarantees no half-state: <explain>
- What happens if step X fails: <explain>
- How is refund/rollback triggered: <explain>
```

## Step 3: Document State Transitions

For each critical step, document ALL states:

```md
## State Transition: Source Lock

**Before state:**
- Source domain: user has 100 USDC, nonce 42
- Destination domain: user has 0 ATOM, no lock
- Both: no ongoing settlement
- Invariants: canonical_supply == all balances

**Action:**
- User submits swap intent: "swap 100 USDC for ATOM"
- System: lock 100 USDC on source domain
- Nonce incremented to 43
- Lock recorded with intent_id

**After state (success):**
- Source domain: user has 0 USDC (locked), nonce 43
- Destination domain: settlement pending, no action yet
- Records: lock event emitted, proof generation started
- Invariants: canonical_supply == locked + balances (still preserved)

**Failure state:**
- Lock fails: user still has 100 USDC, nonce not incremented
- Proof fails: lock recorded but never consumed (timeout → refund)
- Destination settlement fails: source lock released, user gets USDC back
- Recovery: nonce unchanged, operation can be retried

**Timeout state:**
- Timeout expires: lock becomes refundable
- Refund initiated: user gets 100 USDC back, nonce still incremented (proof consumed)
- Destination: knows settlement failed, no mint happens
- Invariants: still preserved

**Replay attempt:**
- Same proof submitted again: intent_id already consumed
- System rejects: "Intent already executed"
- No state change: lock not re-applied, user not refunded twice
- Invariants: preserved
```

Do this for EVERY step in your canonical path.

## Step 4: Identify and Test Invariants

Core invariants you MUST test:

```txt
canonical_supply == native + evm + svm + external_locked + pending
  ↓ Test before/after every operation

intent_id executes at most once
  ↓ Test: replay the same proof, must be rejected

nonce is consumed exactly once
  ↓ Test: duplicate nonce rejected

failed settlement cannot mint/release
  ↓ Test: if destination fails, no asset created

timeout cannot steal funds from rightful owner
  ↓ Test: only original sender can refund

replay cannot execute twice
  ↓ Test: same proof submitted twice, second rejected

wrong chain_id cannot execute
  ↓ Test: proof for chain A submitted to chain B, rejected
```

Add custom invariants for your feature:
```
<your domain specific invariants>
```

## Step 5: Plan All Test Cases

Required tests (ALL must pass):

- [ ] **Golden path:** lock → proof → settle → success
- [ ] **Timeout path:** lock → no proof → timeout → refund
- [ ] **Replay path:** proof submitted twice → second rejected
- [ ] **Invalid proof path:** bad signature → rejected
- [ ] **Partial failure path:** source succeeds, destination fails → refund
- [ ] **Invariant preservation path:** before/during/after all invariants hold
- [ ] **Cross-domain path:** verify both domains agree on final state

```rust
#[test]
fn test_cross_vm_golden_path() {
    // Setup
    let (source_user, dest_user) = setup_accounts();
    let amount = 100;
    
    // Source lock
    lock_on_source(source_user, amount);
    assert_eq!(get_balance(source_user), initial - amount);
    
    // Proof generation
    let proof = generate_proof();
    
    // Destination settlement
    settle_on_dest(proof);
    assert_eq!(get_balance(dest_user), initial + amount);
    
    // Verify invariants
    assert_eq!(
        canonical_supply_before,
        canonical_supply_after,
        "Supply should be preserved"
    );
}

#[test]
fn test_cross_vm_replay_rejected() {
    // Execute once
    let proof = generate_proof();
    settle_on_dest(proof.clone());
    
    // Attempt replay
    let result = settle_on_dest(proof);
    assert!(result.is_err(), "Replay must be rejected");
    assert_eq!(result.err(), Some(Error::DuplicateIntent));
}

#[test]
fn test_cross_vm_timeout_refund() {
    // Lock on source
    lock_on_source(user, 100);
    
    // Wait for timeout
    advance_blocks(timeout_blocks);
    
    // Refund by sender
    refund_on_source(user, 100);
    assert_eq!(get_balance(user), initial);
    
    // Proof no longer valid
    let proof = generate_proof();
    let result = settle_on_dest(proof);
    assert!(result.is_err(), "Settlement after refund blocked");
}
```

## Step 6: Run All Validation

Before claiming feature is complete:

```bash
# Compilation
cargo check --workspace

# All tests
cargo test --test cross_vm_<feature> -- --nocapture

# Invariant tests
cargo test --package invariants -- --nocapture

# Integration
cargo test --test integration -- cross_vm

# No stubs
grep -r "unimplemented\|todo!\|panic!\|fake_proof" \
  src/<feature> --include="*.rs"
```

All must pass. Zero tolerance.

## Step 7: Produce Final Output

Include these sections:

```md
## Cross-VM Feature: <name>

### Canonical Path
<diagram or text>

### Atomicity Model
<chosen model and justification>

### State Transitions
<before/action/after/failure for each step>

### Invariants
<list and tests proving each>

### Tests
<all tests documented and passing>

### Validation Results
<CI output showing all pass>

### Risk Assessment
<risk level and blockers>

### Scoreboard
<module>  █████░░░░░  50%  Status: <honest status>

### Still Missing
<only BLOCKED/DEFERRED items, no FIXABLE_NOW>

### Next Action
<what should happen next>
```

## The Money Rules

1. **Atomicity is not negotiable.** If you cannot prove it, do not claim it.

2. **Replay protection is not optional.** Every operation must have unique replay key.

3. **Timeout handling must exist.** Undefined timeout behavior = DoS vector.

4. **All domains must agree on final state.** If they disagree, funds are lost.

5. **Invariants are the law.** If an invariant can break, operation is not complete.

6. **Tests are proof.** If you cannot write a test, you do not understand the feature.

7. **No fake proofs in core path.** Demo proofs must be feature-gated OFF.

8. **No hardcoded demo values.** Chain IDs, addresses, keys, proofs must be real.

9. **Documentation is required.** If you cannot explain the state machine, you are not ready.

10. **Cross-VM is not a feature.** It is a failure machine until every path is proven.

---

## Start Checklist

Before writing code:

- [ ] Canonical path is documented
- [ ] Atomicity model is chosen and justified
- [ ] State transitions are documented (before/action/after/failure/timeout/replay)
- [ ] Invariants are identified
- [ ] Test cases are planned
- [ ] Team agrees on all above

If any box is unchecked, do not start coding yet. Design first.

---

**Final instruction:** When you claim this feature is done, you should be able to trace it from user input to final state, and every step should be tested and proven. If you cannot, keep working.
