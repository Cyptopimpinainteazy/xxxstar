# 04: Invariant Hunter Audit

## Objective
Extract all critical business/security invariants. Verify each is enforced, tested, monitored, documented, and protected.

## Instructions

You are an invariant hunter.

**This Repomix file contains the X3 repo.**

Extract **all critical invariants** from code and docs. An invariant is a property that must ALWAYS be true.

Examples:
- Total supply of X3 token = sum of all balances
- Every atomic transaction is either fully executed or fully rolled back
- Nonces never repeat
- Bridge reserves never go negative
- Staking total never exceeds cap

For every invariant, check:

1. **Is it enforced in runtime?**
   - Where is it checked? File/function?
   - What happens if it's violated?
   - Can it ever be violated?

2. **Is it tested?**
   - Unit test? Integration test? Property test?
   - Does test cover violation scenario?
   - Does test use worst-case inputs?

3. **Is it monitored?**
   - Are metrics/alerts in place?
   - What alerts if invariant is about to break?
   - Is there a dashboard?

4. **Is it documented?**
   - Is invariant clearly stated in docs?
   - Does code comment match docs?

5. **Is it protected during migration?**
   - If you upgrade the runtime, does invariant hold?
   - Are there migration tests?
   - Can a migration break it?

**Focus intensely on:**
- Canonical supply (can it ever exceed or go negative?)
- Atomic execution (can partial settlement happen?)
- Bridge accounting (can reserves diverge?)
- Nonce uniqueness (can duplicates occur?)
- Asset registry correctness (can assets be minted without permission?)
- Staking/slashing correctness (can slashing steal funds or fail silently?)
- DEX reserve conservation (can reserves ever go negative?)
- Governance permissions (can unprivileged users trigger governance?)

## Expected Output

**CRITICAL INVARIANTS**

| Invariant | Enforcement | Tested | Monitored | Documented | Migration Safe | Status |
|-----------|------------|--------|-----------|-----------|-----------------|--------|
| Total supply = sum of balances | ✅ [file] | ✅ [test] | ✅ | ✅ | ✅ | ✅ OK |
| Atomic = all-or-nothing | ❌ MISSING | ❌ | ❌ | ✅ | ❌ | 🔴 BLOCKER |

**INVARIANT #1: [Name]**
- **Definition:** [Formal statement]
- **Business impact:** [Why it matters]
- **Enforcement location:** [File/function]
- **Enforcement mechanism:** [How is it checked?]
- **Tests:**
  - [Test name] ✅
  - [Missing test] ❌
  - [Weakness] ⚠️
- **Monitoring:**
  - Alerts: [Yes/No]
  - Dashboard: [Yes/No]
  - Metrics: [List]
- **Migration risk:** [None/Low/High/CRITICAL]
- **Score:** [0-100]

[Repeat for each invariant]

**MISSING INVARIANTS**
- [List invariants that should exist but don't]

**OVERALL INVARIANT SAFETY: [X]/100**

P0 Gaps:
- [List invariants with missing enforcement/tests]

Recommendation:
READY / FIX REQUIRED / STOP - CRITICAL GAPS
