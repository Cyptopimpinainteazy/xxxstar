# X3 Mainnet Proof Execution Master Guide

**Purpose**: Execute all proofs in correct order to verify mainnet readiness  
**Last Updated**: 2026-04-26  
**Owner**: X3 Chain Launch Committee  

---

## Quick Start (5 Minute Overview)

You have **6 proof scripts**. Run them in this order:

```bash
# 1. Fresh machine build (single-node baseline)
chmod +x launch-gates/fresh-machine-proof.sh
./launch-gates/fresh-machine-proof.sh

# 2. Multi-node consensus (CRITICAL for mainnet)
chmod +x launch-gates/multi-node-testnet-proof.sh
./launch-gates/multi-node-testnet-proof.sh

# 3. P0 blocker verification (are the 5 critical issues fixed?)
chmod +x launch-gates/verify-p0-blockers.sh
./launch-gates/verify-p0-blockers.sh

# 4. Code embarrassment scan (panic, unwrap, TODO in mainnet code?)
chmod +x launch-gates/embarrassment-scan.sh
./launch-gates/embarrassment-scan.sh

# Check results
ls -lah launch-gates/evidence/proof-*.log
```

All results go to: `launch-gates/evidence/proof-*.log`

---

## Proof 1: Fresh Machine Proof

**What it proves**: A clean machine can clone, build, test, and launch X3  
**Why it matters**: Proves reproducibility (most common mainnet blocker)  
**Time required**: 10-15 minutes  
**Scoring**: Max 95% (single-node proof only)  

### Run the proof:

```bash
chmod +x launch-gates/fresh-machine-proof.sh
./launch-gates/fresh-machine-proof.sh launch-gates/evidence/proof-fresh-machine.log
```

### What it checks (11 steps):

1. ✅ Clone repo to fresh `/tmp/x3-fresh-machine-$$/` directory
2. ✅ Verify Rust toolchain available
3. ✅ `cargo check --workspace --release` (600s timeout)
4. ✅ `cargo test` for 4 critical modules (300s each):
   - pallet-x3-settlement-engine
   - pallet-x3-bridge
   - pallet-cross-chain-validator
   - pallet-universal-asset-kernel
5. ✅ `cargo clippy --workspace` (code quality)
6. ✅ `cargo fmt --check` (formatting)
7. ✅ `cargo build -p x3-chain-node --release` (build node)
8. ✅ `x3-chain-node build-spec --chain dev` (chain spec)
9. ✅ Start node, verify RPC responds on localhost:9945 (30s)
10. ✅ Hazard scan: panic!, unwrap(), TODO, FIXME in critical paths
11. ✅ Verify git history available

### Expected output:

```
=== X3 Fresh Machine Proof ===
...
PASS: 11
FAIL: 0
RESULT: ✅ PASS
Score: 95% (single-node proof only)
```

### If it FAILS:

The error log tells you exactly what broke. Common failures:

- **Rust toolchain missing**: Install rustup
- **cargo check fails**: Missing dependencies or compilation errors
- **Tests fail**: Unit test bug in critical module
- **Node won't start**: Runtime initialization issue
- **RPC not responding**: Port already in use or RPC feature disabled

---

## Proof 2: Multi-Node Testnet Proof

**What it proves**: 4+ validators reach consensus and produce blocks  
**Why it matters**: CRITICAL for mainnet. Without this, you don't know consensus works.  
**Time required**: 5-10 minutes  
**Scoring**: Max 95% (local testnet only)  
**Addresses P0 Blocker**: CRITICAL-002 (Multi-node consensus never tested)  

### Run the proof:

```bash
chmod +x launch-gates/multi-node-testnet-proof.sh
./launch-gates/multi-node-testnet-proof.sh launch-gates/evidence/proof-multi-node-testnet.log
```

### What it checks:

1. ✅ Build 4-validator chain spec
2. ✅ Start Alice, Bob, Charlie, Dave validators
3. ✅ Wait for network to establish (peer discovery)
4. ✅ Verify blocks are producing (≥3 consecutive blocks)
5. ✅ All 4 validators responding to RPC
6. ✅ Chain continues if 1 validator dies (BFT property)

### Expected output:

```
✅ PASS: Multi-node consensus working: produced 5 consecutive blocks
✅ PASS: 4/4 validators responding
RESULT: ✅ PASS
Score: 95%
```

### If it FAILS:

- **Blocks not producing**: Network partition or consensus bug
- **Validators not responding**: Node crash or startup issue
- **Chain stops after validator dies**: BFT consensus broken

### Debug tips:

```bash
# Check individual validator logs
tail -50 /tmp/x3-multinode-testnet-$PID/validator-0.log

# Manually check if validators are networked
curl localhost:9944 -d '{"jsonrpc":"2.0","method":"system_health","id":1}' | jq .

# Watch block production in real-time
watch -n 1 'curl -s localhost:9944 -d "{\"jsonrpc\":\"2.0\",\"method\":\"chain_getHeader\",\"id\":1}" | jq .result.number'
```

---

## Proof 3: P0 Blocker Verification

**What it proves**: The 5 critical P0 blockers are actually fixed  
**Why it matters**: NO-GO decision is based on these. All 5 must be fixed.  
**Time required**: 5 minutes  
**Scoring**: Max 98% if all pass, or proportional if partial  

### The 5 P0 Blockers:

| ID | Issue | Component | Test |
|----|-------|-----------|------|
| CRITICAL-001 | Validator equivocation NOT detected | Consensus | Equivocation pallet exists + tests pass |
| CRITICAL-002 | Multi-node consensus NEVER tested | Network | Multi-node testnet proof must pass |
| CRITICAL-003 | Sender address forgery (unvalidated) | Authorization | xvm_transfer validates sender |
| CRITICAL-004 | Storage unbounded growth | Storage | Transfers get pruned |
| CRITICAL-005 | Vault solvency NOT tested | Economics | Vault solvency invariant test passes |

### Run the verification:

```bash
chmod +x launch-gates/verify-p0-blockers.sh
./launch-gates/verify-p0-blockers.sh launch-gates/evidence/proof-p0-blockers.log
```

### What it checks:

1. **CRITICAL-001**: Equivocation pallet found + tests passing
2. **CRITICAL-002**: Delegates to multi-node-testnet-proof.sh
3. **CRITICAL-003**: xvm_transfer validates sender parameter
4. **CRITICAL-004**: Storage pruning found in code + tests exist
5. **CRITICAL-005**: Vault solvency tests found + passing

### Expected output:

```
PASS: 5/5
FAIL: 0/5
RESULT: ✅ ALL P0 BLOCKERS ADDRESSED
Score: 98%
Status: Ready for mainnet deployment
```

### If ANY blocker FAILS:

**DO NOT PROCEED WITH MAINNET**

Fix the blocker:
1. Identify what's missing (equivocation detection pallet? solvency test?)
2. Implement the fix
3. Add tests
4. Run this proof again
5. Only proceed when all 5 pass

---

## Proof 4: Embarrassment Scan

**What it proves**: No panic!, unwrap(), TODO, or other hazards in mainnet code  
**Why it matters**: Mainnet will find every crash in production. Better to find them first.  
**Time required**: 2-5 minutes  
**Scoring**: 95% if clean, 0% if critical hazards found  

### Run the scan:

```bash
chmod +x launch-gates/embarrassment-scan.sh
./launch-gates/embarrassment-scan.sh launch-gates/evidence/proof-embarrassment-scan.log
```

### What it searches for:

- **P0 (CRITICAL)**: `panic!()`, `.unwrap()`, `.expect()`, private keys, Alice/Bob test keys
- **P1 (HIGH)**: `TODO`, `FIXME`, stub implementations
- **P2 (MEDIUM)**: Hardcoded values, dev-only code, unbounded loops

### Expected output:

```
P0 (CRITICAL): 0
P1 (HIGH): 0
P2 (MEDIUM): 2
RESULT: ✅ PASS
Score: 95%
```

### If it finds P0 hazards:

- Remove all `panic!()` from consensus code
- Remove all `.unwrap()` from validator code
- Remove all test keys/hardcoded values
- Re-run scan until P0 = 0

### Common false positives:

```rust
// OK - this is fine (error handling)
match something {
    Ok(x) => x,
    Err(_) => return,
}

// BAD - this will crash
let x = something.unwrap();

// OK - this is fine (fallback in test)
const TEST_ALICE: &str = "Alice"; // Only in tests/

// BAD - this is in production code
const MAGIC_NUMBER: u64 = 12345; // Used as hardcoded value
```

---

## Proof 5: Genesis Ceremony Checklist

**What it verifies**: All pre-genesis configuration is correct  
**When to use**: 24 hours before genesis block time  
**Scoring**: Pass/Fail (no partial credit)  

### Check off all items:

```bash
cat launch-gates/GENESIS_CEREMONY_CHECKLIST.md
```

**Critical items (non-negotiable):**

- [ ] Chain ID finalized and locked
- [ ] Total supply finalized and cannot change
- [ ] No test accounts in genesis (Alice/Bob removed)
- [ ] All validator keys collected and unique
- [ ] Chain spec hash announced publicly
- [ ] Runtime WASM hash in genesis
- [ ] Multi-validator dry run passed
- [ ] RPC endpoints working
- [ ] All governance members confirmed
- [ ] Sudo key expiry set (or sudo disabled)
- [ ] Emergency halt procedure tested
- [ ] Disaster recovery runbooks available

**Before signing off on genesis:**

1. Print the checklist
2. Have each committee member sign off
3. Store signed copy in vault
4. Post checklist hash to governance

---

## Proof 6: Disaster Recovery Runbooks

**What they verify**: Team knows what to do if things go wrong  
**When to test**: Before genesis (simulate incidents)  
**Scoring**: Pass if team successfully completes all scenarios  

### Available runbooks:

1. **Chain Stall** — blocks stop producing
2. **Finality Stall** — blocks produce but don't finalize
3. **Validator Equivocation** — validator misbehaves
4. **Bad Runtime Upgrade** — invalid code deployed
5. **Bridge Exploit** — cross-chain attack
6. **RPC Outage** — node API down
7. **Database Corruption** — storage integrity lost
8. **Validator Key Compromise** — secret key stolen
9. **Governance Attack** — malicious vote passes
10. **DEX Reserve Anomaly** — liquidity pool broken

### Test the runbooks:

```bash
# 1. Read the runbooks
cat launch-gates/DISASTER_RECOVERY_RUNBOOKS.md

# 2. Simulate each scenario
# Pick incident #1 (Chain Stall)
# Have your team follow the "First 5 Minutes" section
# Time how long until they would recover

# 3. Log any gaps
# - Missing commands?
# - Unclear procedures?
# - Wrong contact info?

# 4. Update runbooks
# - Fix gaps
# - Re-test
```

### Success criteria:

- ✅ Team can execute any runbook in < 5 minutes
- ✅ No missing contact information
- ✅ All commands tested and working
- ✅ Escalation paths clear

---

## Mainnet Readiness Score Card

Once all proofs complete, calculate your score:

| Proof | Max Score | Your Score | Pass? |
|-------|-----------|-----------|-------|
| Fresh Machine | 95% | _____ | ✅ / ❌ |
| Multi-Node | 95% | _____ | ✅ / ❌ |
| P0 Blockers | 98% | _____ | ✅ / ❌ |
| Embarrassment | 95% | _____ | ✅ / ❌ |
| Genesis Checklist | 100% | _____ | ✅ / ❌ |
| Disaster Recovery | 100% | _____ | ✅ / ❌ |
| **TOTAL** | **577%** | **_____** | **✅ / ❌** |

### Final Decision:

```
MAINNET READINESS: 
  ✅ GO (all proofs passing, no P0 blockers)
  ⚠️  CONDITIONAL GO (minor issues, low risk)
  ❌ NO-GO (critical issues unresolved)
```

---

## Proof Maintenance

### Re-run proofs if:

- Code changes in critical paths
- Dependency updates
- Monthly regression testing
- Before each network upgrade
- After any incident

### Archive proofs:

```bash
mkdir -p launch-gates/evidence/archive-$(date +%Y-%m-%d)
cp launch-gates/evidence/proof-*.log launch-gates/evidence/archive-$(date +%Y-%m-%d)/
```

### Audit proof infrastructure:

```bash
# Verify all proof scripts exist and are executable
find launch-gates -name "*.sh" -type f | xargs ls -l

# Check proof logs for PASS/FAIL
grep "RESULT:" launch-gates/evidence/proof-*.log
```

---

## FAQ

**Q: Can I skip the multi-node testnet proof?**  
A: No. This is CRITICAL-002 verification. Without it you don't know if consensus works. Non-negotiable.

**Q: What if fresh-machine proof fails?**  
A: Fix the build issue. The proof must pass on any clean machine, not just yours.

**Q: How often should I run these proofs?**  
A: At minimum: once before genesis. Recommended: monthly regression testing.

**Q: What if P0 blockers are only 90% fixed?**  
A: 90% doesn't matter. Either it's fixed (100%) or it's not (0%). Mainnet doesn't accept "mostly fixed".

**Q: Can I combine the proofs into one script?**  
A: No. Each proof is independent. Running them separately helps isolate failures.

---

## Support & Escalation

**Problem**: Proof script fails  
**First**: Read the proof log carefully - it tells you what failed  
**Next**: Check if it's a known issue (see FAQ)  
**Escalate to**: Chain architect  

**Problem**: Don't understand a P0 blocker  
**First**: Read the blocker description in proofs.yaml  
**Next**: Check the mitigation code in crates/  
**Escalate to**: Security team  

**Problem**: Disaster recovery runbook is outdated  
**First**: Update the runbook with current procedures  
**Next**: Have team practice it  
**Escalate to**: Operations lead  

---

**Document Version**: 1.0  
**Last Updated**: 2026-04-26  
**Next Review**: Before genesis ceremony
