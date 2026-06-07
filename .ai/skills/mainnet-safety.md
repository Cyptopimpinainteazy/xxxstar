# Skill: Mainnet Safety Gate

Use this skill for any work that could affect mainnet readiness.

This prevents premature mainnet claims.

## Classification Framework

```txt
LOCAL ONLY
- Code incomplete or stub/demo only
- Not recommended for any production use
- Example: Parser partially written, no AST

DEVNET READY
- Happy path works locally
- No tests beyond basic compilation
- Suitable for internal testing only
- Example: Transfer feature, manual testing done

TESTNET READY
- All tests pass (unit, integration, e2e)
- Migration safe (if applicable)
- Monitoring configured for testnet
- Known issues documented
- NOT suitable for real assets
- Example: Feature fully coded, tested, but not audited

AUDIT READY
- Testnet-ready + ready for external security review
- Code clean (no stubs, no mocks in core)
- All tests pass
- Invariant tests pass (if state-changing)
- Documentation complete
- Example: Security audit scheduled

MAINNET CANDIDATE
- Audit completed (no critical findings)
- Stress tests passed
- Invariant tests passed
- Migration tested on production-like data
- Rollback plan documented
- Monitoring and alerts configured
- Awaiting governance approval or final sign-off
- Example: Code passed audit, awaiting mainnet deployment

MAINNET READY
- All above + governance approval obtained
- No known vulnerabilities
- Release notes complete
- Deployment plan finalized
- Ready to deploy to production
- Example: Code deployed to mainnet

(There is no "almost mainnet ready".)
```

## Required Output

```md
## Mainnet Safety Gate

### Task summary
- <what is being released>

### Mainnet-sensitive?
- YES / NO

### If YES, what domain?
- consensus
- bridge
- validator / rewards
- wallet / signing
- DEX / swaps
- treasury / governance
- runtime / storage
- RPC / API
- configuration / deployment
- other: <specify>

### Mainnet risk assessment

Risk Level: LOW / MEDIUM / HIGH / CRITICAL

| Risk Level | Criteria |
|-----------|----------|
| LOW | Local-only impact, or docs/comments only |
| MEDIUM | Feature isolated, testnet-tested, no financial impact |
| HIGH | Affects assets, supply, or validator participation |
| CRITICAL | Affects consensus, bridge settlement, or funds safety |

Reasoning:
- <why this risk level>

### Release readiness matrix

| Requirement | Status | Notes |
|------------|--------|-------|
| Code compiles | YES / NO / N/A | |
| Unit tests pass | YES / NO / N/A | |
| Integration tests pass | YES / NO / N/A | |
| E2E tests pass | YES / NO / N/A | |
| Invariant tests pass | YES / NO / N/A | only if state-changing |
| Security tests pass | YES / NO / N/A | only if security-critical |
| Audit completed | YES / NO / N/A | only if CRITICAL |
| Stress test passed | YES / NO / N/A | only if HIGH/CRITICAL |
| Migration tested | YES / NO / N/A | only if storage changes |
| Rollback plan documented | YES / NO / N/A | only if operational |
| Monitoring configured | YES / NO / N/A | only if mainnet |
| Governance approval | YES / NO / N/A | only if CRITICAL |

### Mainnet allowed now?

Classification:
- LOCAL ONLY
- DEVNET READY
- TESTNET READY
- AUDIT READY
- MAINNET CANDIDATE
- MAINNET READY

### Blockers preventing higher classification

- <blocker 1>
- <blocker 2>

### Path to mainnet

```txt
Current: TESTNET READY
Blocked by: Audit not completed
Next steps:
1. Complete security audit (4 weeks)
2. Address audit findings (2 weeks)
3. Retest after fixes (1 week)
4. Obtain governance approval (1 week)
Expected: MAINNET READY in 8 weeks
```
```

## Hard Rules

1. **Testnet-ready does NOT mean mainnet-ready.** Period.

2. **No mainnet claim without all of:**
   - All tests passing
   - Invariant tests passing (if state-changing)
   - Security audit completed (if critical)
   - Stress test passed (if high-load)
   - Migration tested (if storage changes)
   - Rollback plan documented
   - Monitoring configured
   - Governance approval (if required)

3. **No exceptions.**

4. **If any requirement is missing, you must be honest about the gap.**

## Mainnet Claim Checklist

To claim MAINNET READY, ALL must be true:

- [ ] Code compiles without warnings
- [ ] All test suites pass (unit, integration, e2e)
- [ ] Invariant tests pass (for state-changing code)
- [ ] Security red-team tests pass (all attacks have defenses)
- [ ] External audit completed (for critical domains)
- [ ] Stress test passed (for high-load domains)
- [ ] Migration tested on production-like data
- [ ] Rollback plan is documented and tested
- [ ] Monitoring and alerts are configured
- [ ] Governance approval is obtained (if required)
- [ ] Release notes are complete
- [ ] No hardcoded demo values in production code
- [ ] No fake proofs or stubs in core paths
- [ ] No secrets in logs or config

If ANY box is unchecked, your classification must be downgraded.

## Common Mistakes

### Mistake 1: Calling testnet-ready "mainnet-ready"

```txt
❌ WRONG:
- All tests pass? YES
- Audited? NO
- Mainnet ready? YES (WRONG!)

✅ RIGHT:
- All tests pass? YES
- Audited? NO
- Mainnet ready? NO (must wait for audit)
- Classification: TESTNET READY
```

### Mistake 2: Skipping stress tests

```txt
❌ WRONG:
- High-load feature written? YES
- Stress tested? NO
- Mainnet ready? YES (WRONG!)

✅ RIGHT:
- High-load feature written? YES
- Stress tested? NO (must run before mainnet)
- Mainnet ready? NO
- Classification: AUDIT READY (waiting for stress test)
```

### Mistake 3: Forgetting migration testing

```txt
❌ WRONG:
- Storage format changed? YES
- Migration tested on prod data? NO
- Mainnet ready? YES (WRONG!)

✅ RIGHT:
- Storage format changed? YES
- Migration tested on prod data? NO (must test before mainnet)
- Mainnet ready? NO
- Classification: AUDIT READY (waiting for migration test)
```

## Approval Checklist

Before signing off on mainnet safety:

- [ ] Risk level is accurately classified
- [ ] All applicable requirements are checked
- [ ] Missing requirements are documented as blockers
- [ ] Path to mainnet is clear
- [ ] Classification is honest
- [ ] No exceptions or special cases claimed

If any box is unchecked, classification needs review.

---

**Rule:** If you are unsure, classify lower. Better to be conservative than to push broken code to mainnet.
