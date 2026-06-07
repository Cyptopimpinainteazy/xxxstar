# Workflow Hook: Release Gate

Run before any release candidate, testnet deployment, or mainnet claim.

This is the final gate. No work proceeds to production without passing this.

## Required Output

```md
## Release Gate

### Task summary
- <what is being released>

### Release Classification

Choose ONE:

- **LOCAL ONLY:** Code incomplete or stub only. Not for any production use.
- **DEVNET READY:** Happy path works locally. No guardrails. For internal testing only.
- **TESTNET READY:** All tests pass, migration safe, monitoring configured. NOT for real assets.
- **AUDIT READY:** Testnet-ready + ready for external security review.
- **MAINNET CANDIDATE:** Audited, stress tested, awaiting final approval.
- **MAINNET READY:** All checks pass, approved, ready to deploy to production.

Selected: <LOCAL ONLY / DEVNET / TESTNET / AUDIT / MAINNET CANDIDATE / MAINNET READY>

### Evidence Summary

**CI Status:**
```txt
All checks: PASS / FAIL / PARTIAL

If FAIL or PARTIAL, do not proceed.
```

**Test Coverage:**

| Category | Count | Pass | Required |
|----------|-------|------|----------|
| Unit | <n> | YES/NO | YES |
| Integration | <n> | YES/NO | YES |
| E2E | <n> | YES/NO | For TESTNET+ |
| Invariant | <n> | YES/NO | For state-changing |
| Security | <n> | YES/NO | For security-critical |
| Fuzz/property | <n> | YES/NO | For security-critical |
| Stress | <n> | YES/NO | For HIGH/CRITICAL risk |

All required tests passing? YES / NO

If NO, do not proceed.

**Cross-VM Safety (if applicable):**

- Atomicity verified: YES / NO
- Replay tested: YES / NO
- Timeout tested: YES / NO
- Rollback tested: YES / NO

All required tests passing? YES / NO

**Mainnet Safety (if claiming mainnet-ready or candidate):**

- Audit completed: YES / NO
- Audit findings resolved: YES / NO / N/A
- Stress test passed: YES / NO / N/A
- Monitoring configured: YES / NO
- Rollback plan: YES / NO
- Governance approval: YES / NO / N/A
- Migration tested: YES / NO / N/A
- Release notes complete: YES / NO

All required items done? YES / NO

### Mainnet Readiness

If claiming MAINNET READY or MAINNET CANDIDATE:

**Blockers:**

P0 (blocking release):
- [ ] <blocker 1>
- [ ] <blocker 2>

P1 (should fix before release):
- [ ] <blocker 1>

**Risks:**

| Risk | Mitigation | Status |
|------|-----------|--------|
| <risk 1> | <how mitigated> | PASS / FAIL |
| <risk 2> | <how mitigated> | PASS / FAIL |

All risks mitigated? YES / NO

### Release Status

**FINAL DECISION:**

```txt
Status: <LOCAL ONLY / DEVNET READY / TESTNET READY / AUDIT READY / MAINNET CANDIDATE / MAINNET READY>

Reasoning:
- <reason 1>
- <reason 2>

If not MAINNET READY:
Blockers preventing higher status:
- <blocker 1>
- <blocker 2>

Path forward:
- <next step 1>
- <next step 2>
```

**Go/NoGo:**

```txt
GO for release? YES / NO

If NO:
- What must be done? <list>
- Who should do it? <owner>
- By when? <date>

Reschedule release to: <date>
```
```

## Hard Rules for Mainnet Claims

To claim MAINNET READY, ALL must be true:

- [ ] Code compiles without warnings
- [ ] All test suites pass (unit, integration, e2e, invariant, security)
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
- [ ] Performance is acceptable
- [ ] No known critical vulnerabilities

If ANY item is missing or fails, you must classify lower.

## Release Sign-Off

```md
## Agent Sign-Offs

### Supervisor
- Route to agents? ✓
- Approval obtained? ✓

### Primary Agent
- Domain work complete? ✓
- Quality gate passed? ✓

### Supporting Agents (if CRITICAL risk)
- Invariant Test Engineer? ✓
- Security Red-Team? ✓
- Release Closer? ✓

### Release Closer
- All gates passed? ✓
- Release status classified? ✓
- Go/NoGo decision? GO

All agents agree? YES / NO

If NO, resolve disagreements before release.
```

## Approval Checklist

Before release is authorized:

- [ ] All CI passes
- [ ] All required tests pass
- [ ] Mainnet safety gate completed (if applicable)
- [ ] Release status is classified
- [ ] No P0 blockers remain
- [ ] Go/NoGo decision is made
- [ ] All agents have signed off

If any box is unchecked, release is not authorized.

---

**Final Rule:** When in doubt, classify lower and wait. Better to be conservative than to push broken code to production.
