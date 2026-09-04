# Skill: Replay + Timeout Safety

Use this skill for bridge, signing, proof, HTLC, relayer, settlement, wallet, and Cross-VM dispatch tasks.

This ensures operations cannot be executed twice accidentally.

## Required Checks

```md
## Replay + Timeout Safety

### Replay keys

For every operation that moves funds or changes state, document:

| Key | Value | Storage | Verified | Test |
|-----|-------|---------|----------|------|
| nonce | `sender_nonce` | on-chain | ✓ check before exec | test_duplicate_nonce_rejected |
| intent_id | `hash(op)` | on-chain | ✓ check before exec | test_duplicate_intent_rejected |
| chain_id | `1` / `2` / `137` | in op | ✓ check in verification | test_wrong_chain_id_rejected |
| domain_sep | `domain(chain, op_type)` | in proof | ✓ check in verification | test_domain_sep_verified |
| proof_hash | `hash(proof)` | on-chain | ✓ stored after exec | test_proof_hash_tracked |
| signer | `recovered from sig` | in op | ✓ recovered and checked | test_invalid_signature_rejected |

Rules:
- Nonce must be unique per sender
- Intent ID must be unique globally
- Chain ID must be in the replay key (prevents cross-chain replay)
- Proof hash must be tracked (prevents proof reuse)
- Signer must be verified (prevents unauthorized execution)
- All checks must happen BEFORE state change

### Timeout model

Document the timeout behavior:

**When timeout starts:**
- On-chain block height? (block X)
- Wall-clock time? (avoid if possible)
- Settlement initiated time? (block Y)

**When timeout expires:**
- At block X + N?
- At time T + D?
- Is there a grace period?

**Source of truth:**
- On-chain block height? (deterministic, good)
- Wall-clock? (nondeterministic, bad)
- External service? (centralized, bad)

**Refund authority:**
- Only original sender? (good)
- Relayer? (bad, prevents censorship resistance)
- Anyone? (bad, allows theft)

**Settlement after timeout:**
- Allowed? (bad, can lead to double-spend)
- NOT allowed? (good, prevents timeout abuse)

### Required negative tests

All of these must have passing tests:

- [ ] test_duplicate_nonce_rejected()
  - Submit same nonce twice → second rejected

- [ ] test_duplicate_proof_rejected()
  - Submit same proof twice → second rejected

- [ ] test_wrong_chain_id_rejected()
  - Change chain ID in proof → rejected

- [ ] test_expired_proof_rejected()
  - Wait until timeout → refund allowed, settlement not allowed

- [ ] test_refund_by_wrong_caller_rejected()
  - Non-sender attempts refund → rejected

- [ ] test_settlement_after_refund_rejected()
  - Settlement arrives after refund → rejected

- [ ] test_invalid_signature_rejected()
  - Modify signature in proof → rejected

### Hard Rules

1. **No replay tracking in memory only.** Must be on-chain state.

2. **No wall-clock time in deterministic runtime logic.** Use block height.

3. **No settlement after consumed nonce.** Once nonce is consumed, no replay possible.

4. **Nonce must be checked BEFORE any state change.** Not after.

5. **Timeout must be deterministic.** Block height, not sleep/delay.

## Test Templates

### Test: Duplicate Nonce Rejected

```rust
#[test]
fn test_duplicate_nonce_rejected() {
    let sender = setup_account();
    let nonce = 1;
    
    // First execution succeeds
    let result1 = execute_with_nonce(sender, nonce, operation);
    assert!(result1.is_ok(), "First execution should succeed");
    
    // Verify nonce is consumed
    assert!(is_nonce_consumed(sender, nonce), "Nonce should be consumed");
    
    // Second execution with same nonce fails
    let result2 = execute_with_nonce(sender, nonce, operation);
    assert!(result2.is_err(), "Duplicate nonce should fail");
    assert_eq!(result2.err(), Some(Error::ReplayDetected));
}
```

### Test: Timeout Refund Only by Owner

```rust
#[test]
fn test_timeout_refund_only_owner() {
    let (sender, recipient) = setup_accounts();
    let lock_amount = 100;
    
    // Lock on source
    lock_on_source(sender, lock_amount);
    assert_eq!(get_balance(sender), initial - lock_amount);
    
    // Advance time past timeout
    advance_blocks(timeout_blocks + 1);
    
    // Attacker cannot refund
    let attacker = create_account();
    let result = refund_on_source(attacker, sender, lock_amount);
    assert!(result.is_err(), "Attacker cannot refund");
    
    // Only sender can refund
    let result = refund_on_source(sender, sender, lock_amount);
    assert!(result.is_ok(), "Sender can refund");
    assert_eq!(get_balance(sender), initial, "Balance fully restored");
}
```

### Test: No Settlement After Timeout

```rust
#[test]
fn test_no_settlement_after_timeout() {
    let (sender, recipient) = setup_accounts();
    let amount = 100;
    
    // Lock on source
    lock_on_source(sender, amount);
    
    // Timeout expires
    advance_blocks(timeout_blocks + 1);
    
    // Refund initiated
    let refund_result = refund_on_source(sender, sender, amount);
    assert!(refund_result.is_ok(), "Refund should succeed");
    
    // Settlement now arrives (too late)
    let proof = generate_proof();
    let settlement_result = settle_on_dest(proof);
    assert!(settlement_result.is_err(), "Settlement after timeout should fail");
    assert_eq!(settlement_result.err(), Some(Error::ExpiredProof));
}
```

## Approval Checklist

Before signing off on replay/timeout safety:

- [ ] Nonce/intent_id/proof_hash are documented
- [ ] Chain ID is part of replay key
- [ ] Timeout model is documented
- [ ] Refund is only available to original sender
- [ ] Settlement after timeout is blocked
- [ ] All negative tests pass
- [ ] No memory-only tracking (all on-chain)
- [ ] No wall-clock time in deterministic paths

If any box is unchecked, safety testing is incomplete.

---

**Next:** Verify Security Red-Team has tested all attack scenarios.
