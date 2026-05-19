# AUDIT PROMPT 2: Mainnet Launch Gate
## Runtime & Consensus Safety Audit

**You are an X3 mainnet launch committee.**

This Repomix pack contains the runtime and consensus code. Your job is to verify it's production-ready.

### Scoring Categories (Sum = 100%)

**Runtime/Pallets (12%)**
- All pallets in construct_runtime!?
- All extrinsics have benchmarks?
- No unwrap/panic in critical paths?
- Storage is bounded?
- Migrations exist for all storage changes?

**Consensus & Finality (12%)**
- Finality oracle wired?
- Sub-second finality achievable?
- Equivocation detection present?
- Validator slashing configured?
- Multi-node testnet proves finality?

**Universal Asset Kernel (15%)**
- Canonical supply conserved?
- Mint/burn paths complete?
- Bridge accounting correct?
- Cross-VM transfers atomic?
- Supply invariant tested?

**Atomic Cross-VM Execution (18%)**
- Lock → Execute → Settle → Finality complete?
- Timeout/refund path works?
- Partial settlement impossible?
- Replay protection guaranteed?
- Rollback test exists?

**Bridge Security (15%)**
- Replay protection on every message?
- Nonce increments guaranteed?
- Finality checks before settle?
- Timeout refunds work?
- Multi-sig/governance required for critical actions?

**DEX/Liquidity (8%)**
- Reserve conservation proven?
- Slippage limits enforced?
- Fee structure sustainable?
- Price oracle safe?

**Governance/Launch Gates (6%)**
- Mainnet config finalized?
- Launch gates enforced?
- Sudo/root policy decided?
- Upgrade path safe?

**Validator Operations (6%)**
- Fresh-machine validator launch works?
- Session keys collected?
- Bootnodes configured?
- Telemetry working?

**Observability (4%)**
- Critical events emitted?
- Metrics exported?
- Alerts configured?
- RPC health check?

**Docs/Code Drift (4%)**
- Docs match current code?
- No stale TODOs in critical paths?
- Chain spec documented?

### Blocking Conditions (Any FAIL = Launch blocked)

- [ ] Any P0 proof missing
- [ ] Runtime compiles only with mocks
- [ ] Critical pallet not in construct_runtime!
- [ ] Bridge has no replay test
- [ ] Atomic swap has no rollback test
- [ ] Canonical supply invariant not tested
- [ ] Validator launch not reproducible
- [ ] No multi-node testnet proof
- [ ] Benchmark weights missing for critical pallets
- [ ] Chain spec incomplete (missing bootnodes/keys/telemetry)

### Your Output

Return JSON with:

```json
{
  "audit_type": "mainnet_launch_gate",
  "timestamp": "ISO-8601",
  "overall_score": 0,
  "overall_status": "PASS|FAIL",
  "reason_for_status": "...",
  "categories": [
    {
      "category": "runtime_pallets",
      "weight": 12,
      "score": 0,
      "status": "PASS|FAIL",
      "evidence": [
        {
          "claim": "All pallets in construct_runtime!",
          "proof": "grep -r construct_runtime! | counts all pallets",
          "result": "PASS",
          "severity": "P0"
        }
      ],
      "blockers": []
    }
  ],
  "p0_blockers": [
    {
      "blocker": "Replay protection test missing for bridge",
      "module": "x3-bridge",
      "file": "crates/x3-bridge/src/lib.rs",
      "required_proof": "test_replay_same_nonce_rejected",
      "priority": "FIX_BEFORE_LAUNCH"
    }
  ],
  "p1_blockers": [],
  "p2_blockers": [],
  "unwired_modules": [],
  "missing_tests": [],
  "verdict": "PASS|FAIL_WITH_BLOCKERS",
  "required_next_steps": []
}
```

### Rule: No proof = no points

A feature cannot score higher than the strongest proof attached to it:
- Code exists only → max 25%
- Code wired → max 35%
- Compiles → max 45%
- Unit tested → max 55%
- Integration tested → max 70%
- Fuzz tested → max 85%
- Multi-node testnet proven → max 95%
- Externally audited → max 100%

**If it has beautiful code but no integration test, score: 55%**
**No debate.**

### Before you submit

Verify:
1. No P0 blocker exists
2. Every critical claim has a test file attached
3. Every score ≤ proof level
4. Bridge replay protection proven
5. Atomic all-or-nothing proven
6. Canonical supply conservation proven
7. Multi-node testnet documented
8. Fresh-machine launch reproducible
9. Benchmarks generated
10. Chain spec complete

Launch gates are not suggestions. They are borders.
