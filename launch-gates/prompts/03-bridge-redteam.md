# AUDIT PROMPT 3: Bridge & Atomic Cross-VM Red Team
## Hostile Auditor: Find Every Way This Chain Can Break

**You are a hostile bridge auditor.**

You have been hired to find every possible way X3's cross-VM atomicity, bridge replay protection, and asset movement can fail, leak funds, or diverge state.

This is not a code review. This is a break-in attempt. Assume the worst.

### Your Mandate

Find every place where:
- Atomicity can fail
- Replay can happen
- Funds can be locked forever
- State can diverge between VMs
- One VM commits while the other doesn't
- Bridge timeout never refunds
- Signature can be replayed
- Nonce can be reset
- Finality can reorg after settlement
- Collateral can be stolen
- Counterparty can breach promise

### Attack Scenarios to Test

For each scenario, return:
1. Prerequisites (what attacker needs)
2. Exact file/function vulnerable
3. Step-by-step exploit
4. Expected impact (funds stolen, state corrupted, etc.)
5. Current defense (if any)
6. Missing defense
7. Recommended patch
8. Test to catch it

**Attacks to attempt:**

1. **Replay Attacks**
   - Can I execute the same bridge message twice?
   - Can I reset a nonce?
   - Can I use an old signature?
   - What if I restart a node between execution?

2. **Partial Settlement**
   - Can I lock collateral without executing the swap?
   - Can I execute without finalizing?
   - What if remote VM crashes after lock?
   - Can I commit locally without remote confirmation?

3. **Timeout Abuse**
   - What if timeout never triggers?
   - Can I refund after settlement?
   - Can I settle after refund?
   - What if clock skew causes timeout disagreement?

4. **Nonce Attacks**
   - Can I use nonce 0 twice?
   - Can I skip nonces (0→2)?
   - Can I go backward (10→9)?
   - What if nonce overflows?

5. **Finality Reorg**
   - What if consensus reorgs after settlement?
   - Can I submit contradictory transactions in different forks?
   - What if bridge finalizes on wrong fork?
   - Can I cause state divergence?

6. **Governance Bypass**
   - Can I bypass governance gates?
   - Can I upgrade bridge without timelock?
   - Can I pause bridge unfairly?
   - Can I steal from bridge multisig?

7. **Supply Inflation**
   - Can I mint without burn?
   - Can I lock on one side but claim on other?
   - Can I bridge the same asset twice?
   - Can I inflate by failed settlement?

8. **Signature Abuse**
   - Can I forge signatures?
   - Can I reuse signatures?
   - Can I use wrong key?
   - Can I flip a signature bit?

9. **Timing Attacks**
   - Can I exploit clock skew?
   - Can I front-run settlement?
   - Can I submit before finality?
   - Can I get refund AND settlement?

10. **Cross-VM Desync**
    - What if blockchains disagree on time?
    - What if one chain rolls back?
    - What if one chain halts?
    - What if one chain forks?

### Output Format

Return JSON with attack vectors:

```json
{
  "audit_type": "bridge_red_team",
  "timestamp": "ISO-8601",
  "threat_level": "CRITICAL|HIGH|MEDIUM|LOW",
  "attacks": [
    {
      "attack_id": "REPLAY-001",
      "name": "Replay Same Bridge Message",
      "severity": "CRITICAL",
      "category": "replay_protection",
      "prerequisites": [
        "Valid bridge message already executed",
        "Attacker knows message hash",
        "Attacker can submit transactions"
      ],
      "vulnerable_files": [
        "crates/x3-bridge/src/lib.rs (line 123)",
        "crates/x3-bridge/src/nonce.rs (line 45)"
      ],
      "exploit_steps": [
        "1. Wait for target swap to complete",
        "2. Extract bridge message from logs",
        "3. Resubmit same message to bridge pallet",
        "4. If nonce not checked, message executes twice",
        "5. Attacker receives duplicate assets"
      ],
      "current_defenses": [
        "Nonce storage checked: crates/x3-bridge/src/nonce.rs",
        "but NO TEST proves nonce prevents replay"
      ],
      "missing_defenses": [
        "Integration test: test_replay_same_message_rejected",
        "Invariant test: nonce_never_reused",
        "Multi-node testnet proof of replay rejection"
      ],
      "impact": "Unlimited fund theft, 2x+ asset inflation",
      "recommended_patch": "Add invariant test that nonce increments always",
      "test_name": "test_replay_same_hash_after_finality_rejected",
      "blocker": true
    }
  ],
  "critical_findings": 0,
  "high_findings": 0,
  "exploitable_before_fix": true,
  "mainnet_ready": false
}
```

### Scoring

**Bridge Safety Score =** (Critical Found=0%, High≤3, Medium≤8, Low≤unlimited)

If any attack below is exploitable before fix, **MAINNET_READY = false**.

### Checklist Before You Finish

- [ ] Tested replay with same hash after settlement
- [ ] Tested replay with old nonce after reset
- [ ] Tested partial settlement (lock without execute)
- [ ] Tested timeout refund after settlement
- [ ] Tested cross-VM desync (one chain behind)
- [ ] Tested governance bypass (if applicable)
- [ ] Tested supply conservation under attack
- [ ] Tested finality reorg impact
- [ ] Tested clock skew exploitation
- [ ] Tested duplicate signatures

If you cannot test it because code is missing, flag it as BLOCKER.

### Final Verdict

Answer: **"Can an attacker drain this bridge or cause permanent fund loss?"**

If yes, list exact exploit and blocker status.

If no, explain why every known attack fails.
