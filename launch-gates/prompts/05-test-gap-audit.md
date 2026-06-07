# 05: Test Coverage & Gap Audit

## Objective
Identify critical mainnet behaviors that are NOT tested. Build a backlog of missing tests.

## Instructions

You are a QA engineer tasked with finding untested production-critical behaviors.

**This Repomix file contains the test suite.**

Analyze the test suite. Tell me what critical mainnet behaviors are NOT tested.

**Mandatory test coverage areas:**

1. **Atomicity & Settlement**
   - ✅/❌ Atomic swap succeeds end-to-end
   - ✅/❌ Atomic swap rolls back if Chain B fails
   - ✅/❌ Partial settlement is impossible
   - ✅/❌ Settlement can't be skipped/doubled

2. **Replay & Nonce**
   - ✅/❌ Same nonce cannot be replayed on same chain
   - ✅/❌ Same nonce cannot be replayed on different chain
   - ✅/❌ Nonce overflow handling
   - ✅/❌ Nonce gap handling (missing nonce N, then N arrives)

3. **Finality & Reorg**
   - ✅/❌ Reorg after finality is detected
   - ✅/❌ Finality rollback is handled safely
   - ✅/❌ Partial reorg of cross-chain operation
   - ✅/❌ Long reorg doesn't break state

4. **Bridge Timeout & Recovery**
   - ✅/❌ Operation times out mid-flight
   - ✅/❌ Timeout triggers automatic refund
   - ✅/❌ Admin can manually recover stuck funds
   - ✅/❌ Recovery doesn't double-spend

5. **Storage Overflow & Bounds**
   - ✅/❌ Unbounded storage growth is prevented
   - ✅/❌ Balance overflow is caught
   - ✅/❌ Supply overflow is caught
   - ✅/❌ Nonce overflow is handled

6. **Amount Edge Cases**
   - ✅/❌ Zero amount is rejected
   - ✅/❌ Max amount works
   - ✅/❌ Dust amounts (1 wei) handled
   - ✅/❌ Amounts exceed balance (rejected)

7. **Chain ID & VM Mismatches**
   - ✅/❌ Wrong chain ID is rejected
   - ✅/❌ Wrong VM target is rejected
   - ✅/❌ Spoofed chain ID is caught
   - ✅/❌ Cross-VM message tampering detected

8. **Governance & Permissions**
   - ✅/❌ Unprivileged user cannot trigger governance
   - ✅/❌ Governance call cannot bypass launch gates
   - ✅/❌ Timelock is enforced
   - ✅/❌ Multisig requirement is enforced

9. **Validator Equivocation & Collusion**
   - ✅/❌ Validator cannot equivocate (sign two conflicting blocks)
   - ✅/❌ Equivocation triggers slashing
   - ✅/❌ Validator collusion can be detected
   - ✅/❌ Malicious validators cannot force invalid state

10. **Mempool & Front-run**
    - ✅/❌ Front-running protection exists
    - ✅/❌ Sandwich attack is prevented
    - ✅/❌ MEV extraction is limited

11. **Migration Safety**
    - ✅/❌ Storage migration succeeds
    - ✅/❌ Pallet version upgrade works
    - ✅/❌ Invariants hold after migration
    - ✅/❌ Rollback from failed migration

12. **Launch Config Safety**
    - ✅/❌ Mainnet config is valid
    - ✅/❌ Testnet config doesn't work on mainnet
    - ✅/❌ Genesis spec is correct
    - ✅/❌ Bootnodes are reachable

## Expected Output

**TEST COVERAGE SCORECARD**

| Area | Tested | Missing | Priority |
|------|--------|---------|----------|
| Atomicity | ✅ | None | HIGH |
| Replay/Nonce | ⚠️ | 2 gaps | HIGH |
| Finality/Reorg | ❌ | 3 gaps | CRITICAL |
| Bridge Timeout | ⚠️ | Recovery path | HIGH |
| Storage Bounds | ✅ | None | HIGH |
| Amount Edge Cases | ⚠️ | 1 gap | MEDIUM |
| Chain ID Mismatch | ✅ | None | MEDIUM |
| Governance | ✅ | None | HIGH |
| Validator Attacks | ❌ | Equivocation test | CRITICAL |
| MEV/Front-run | ⚠️ | Test unclear | MEDIUM |
| Migration | ⚠️ | 1 gap | HIGH |
| Launch Config | ✅ | None | MEDIUM |

**MISSING TEST BACKLOG**

Priority: CRITICAL
```
test_finality_reorg_after_settlement()
test_validator_equivocation_slashing()
test_bridge_timeout_recovery_manual()
test_atomic_partial_settlement_prevention()
```

Priority: HIGH
```
test_nonce_replay_cross_chain()
test_finality_reorg_partial_cross_chain()
test_bridge_account_compromise_impact()
...
```

Priority: MEDIUM
```
test_dust_amount_handling()
...
```

**TOTAL TESTS:**
- Existing: X
- Missing: X
- Coverage: X%

**RECOMMENDATION:**
- Ready to ship: [YES/NO]
- Tests to add before mainnet: [N critical, N high, N medium]
- Estimated effort: X hours
