# 03: Bridge & Atomic Cross-VM Safety Audit

## Objective
Deep-dive on bridge/atomic cross-VM code. Find every place where atomicity can fail, replay can happen, or state can diverge.

## Instructions

You are a hostile bridge auditor. You have exploited bridges for a living.

**This Repomix file contains the X3 bridge/atomic code.**

Build a wiring map from:
```
user request 
  → extrinsic/API call 
  → validation 
  → lock/reserve on Chain A
  → execute on Chain B
  → finality proof 
  → commit on Chain A
  → settle
  → event/log
  → test coverage
```

Find **every place** where atomicity can fail, replay can happen, funds can be locked forever, or state can diverge between chains.

**Specific checks:**

1. **Replay Protection**
   - Does every cross-chain operation have a nonce? Timestamp? Hash?
   - Can the same operation be replayed on a different chain?
   - Can it be replayed on the same chain twice?
   - What prevents double-spending between chains?

2. **Finality & Reorg**
   - What if Chain A finalizes but Chain B reorgs?
   - What if Chain B commits but Chain A reorgs?
   - Is there a rollback path?
   - Are all edge cases tested?

3. **Atomicity Failure**
   - If lock succeeds but execution fails, is there a refund?
   - If execution succeeds but settlement fails, is there a recovery?
   - Are rollbacks tested?
   - Are partial states impossible?

4. **Timeout & Recovery**
   - What if a bridge operation times out mid-flight?
   - Can funds be stuck forever?
   - Is there an admin recovery path?
   - Is recovery path tested?

5. **State Divergence**
   - Can Chain A and Chain B ever disagree on the same operation's state?
   - Is there a reconciliation mechanism?
   - What triggers divergence?
   - How is it detected?

6. **Bridge Account Safety**
   - Who can sign/approve bridge operations?
   - Is there a multisig? Timelock?
   - Are private keys held safely?
   - Can one compromised key drain the bridge?

7. **Slashing & Validator Safety**
   - Can validators collude to steal bridge funds?
   - Is there stake/slashing protecting the bridge?
   - What's the validator incentive to be honest?
   - Are validator attacks tested?

## Expected Output

**BRIDGE THREAT MODEL**

For each threat, provide:
- **Threat:** [Description]
- **Prerequisite:** [What attacker needs]
- **Affected files:** [Exact file/function]
- **Attack scenario:** [Step-by-step exploit]
- **Likely impact:** [$$ if possible]
- **Existing defenses:** [What prevents it? Code location.]
- **Missing defenses:** [What's missing?]
- **Test to reproduce:** [Unit test + inputs]
- **Mainnet blocker:** YES / NO
- **Recommended patch:** [Exact code fix]

**Threat #1: Replay Attack**
[Full details above]

**Threat #2: [Next threat]**
...

**MISSING TESTS**
```
test_replay_same_chain_twice
test_replay_different_chain
test_finality_reorg_chain_b
test_execution_timeout_recovery
test_partial_settlement_rollback
test_validator_collusion
test_bridge_account_compromise
test_nonce_overflow
test_chain_id_mismatch
...
```

**OVERALL BRIDGE SAFETY SCORE: [X]/100**

Status:
- 95+: READY
- 85-94: ACCEPTABLE WITH MONITORING
- <85: DO NOT LAUNCH

**P0 BLOCKERS:**
- [List critical vulnerabilities]

**RECOMMENDATION:**
LAUNCH / DO NOT LAUNCH / LAUNCH WITH CIRCUIT BREAKERS / REQUIRE EXTERNAL AUDIT
