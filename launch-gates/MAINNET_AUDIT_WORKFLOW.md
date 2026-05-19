# 🚀 X3 Mainnet Readiness Audit Workflow

**Date:** April 24, 2026  
**Status:** Initial setup  
**Target:** 100% mainnet-ready with zero guess-work

---

## 🎯 Overview

This workflow uses **Repomix** to generate evidence packs of the X3 codebase, then forces AI to **prove** integration, not vibe-check.

Every audit produces:
1. **Wiring map** - what's connected, what's orphaned
2. **Blocker list** - ranked by severity
3. **Missing tests** - specific test names needed
4. **Invariant safety** - all critical properties verified
5. **Bridge threat model** - all attack paths covered

---

## 📋 Step 1: Generate Audit Packs

Run the script to create 5 targeted Repomix packs:

```bash
cd /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates
./repomix-mainnet-pack.sh
```

This generates:
- `full-repo-*.md` → Complete repo snapshot
- `runtime-consensus-*.md` → Runtime/consensus only
- `bridge-atomic-*.md` → Bridge/atomic cross-VM code
- `tests-*.md` → All test suites
- `git-drift-*.md` → Recent changes + logs

---

## 🔍 Step 2: Run Audits

For each pack, paste it to Claude + prompt from `prompts/` folder.

### **Audit 2.1: Wiring Audit** (Full repo pack)

**Prompt:** `launch-gates/prompts/01-wiring-audit.md`

**What it checks:**
- Is every module actually reachable?
- What's connected to runtime/CLI/RPC/frontend?
- What's orphaned or stubbed?

**Expected output format:**
```
WIRING MAP
✅ x3-bridge: Extrinsic A → Runtime → Tests
❌ x3-unused-module: NO WIRING - recommend removal

UNWIRED MODULES
- x3-leftover-code (dead code candidate)

SCORE: 87/100 [GAPS IDENTIFIED]
```

**Time:** 5-15 min per run

---

### **Audit 2.2: Mainnet Launch Gate** (Full repo pack)

**Prompt:** `launch-gates/prompts/02-mainnet-launch-gate.md`

**What it checks:**
- 0-100 readiness score by category
- P0/P1/P2 blockers
- Fastest fix paths

**Expected output:**
```
MAINNET READINESS SCORECARD

Consensus Safety:      85/100 ⚠️
  Blocker: No equivocation detection
  Fix: 4 hours
  File: crates/x3-consensus/src/lib.rs

Runtime Safety:        92/100 ✅
Storage Safety:        88/100 ⚠️
Bridge Safety:         72/100 🔴 CRITICAL
  Blocker: No replay protection for cross-chain nonces
  Fix: 8 hours
  File: crates/x3-bridge/src/core.rs

OVERALL: 82/100 [FIX REQUIRED]
TIME TO MAINNET: 12 hours
```

**Time:** 10-20 min per run

---

### **Audit 2.3: Bridge Safety** (Bridge pack)

**Prompt:** `launch-gates/prompts/03-bridge-safety-audit.md`

**What it checks:**
- Replay attack vectors
- Atomicity failure modes
- Finality reorg handling
- Validator collusion scenarios

**Expected output:**
```
BRIDGE THREAT MODEL

Threat #1: Replay Attack (Same Nonce, Different Chain)
  Prerequisite: Attacker intercepts transaction
  Affected file: crates/x3-bridge/src/validation.rs:42
  Attack: Send nonce=1 to chain A, intercept+resend to chain B
  Impact: Potential double-swap ($XX at risk)
  Existing defense: Nonce checked against chain A state
  MISSING DEFENSE: No chain ID inclusion in nonce hash
  Test: test_replay_cross_chain_nonce
  Blocker: YES - P0
  Fix: Include chain_id in nonce hash computation

Threat #2: ...

MISSING TESTS
- test_replay_same_chain_twice ❌
- test_finality_reorg_rollback ❌
- test_validator_collusion_impact ❌

BRIDGE SAFETY SCORE: 68/100 [DO NOT LAUNCH]
```

**Time:** 20-30 min per run

---

### **Audit 2.4: Invariant Hunter** (Full repo pack)

**Prompt:** `launch-gates/prompts/04-invariant-hunter.md`

**What it checks:**
- Is every critical property enforced?
- Are they tested?
- Are they monitored?
- Are they migration-safe?

**Expected output:**
```
CRITICAL INVARIANTS

Invariant: Total supply = sum of balances
  Enforcement: ✅ pallets/x3-coin/src/lib.rs:120
  Tested: ✅ tests/coin_supply_test.rs
  Monitored: ❌ NO METRICS
  Migration safe: ✅
  Score: 85/100

Invariant: Atomic swap = all-or-nothing
  Enforcement: ⚠️ Partial (some paths missing)
  Tested: ⚠️ Basic test, no timeout scenario
  Monitored: ❌
  Migration safe: ⚠️ Unclear
  Score: 60/100 [BLOCKER]

INVARIANT SAFETY: 71/100 [GAPS]
```

**Time:** 15-25 min per run

---

### **Audit 2.5: Test Gap Audit** (Tests pack)

**Prompt:** `launch-gates/prompts/05-test-gap-audit.md`

**What it checks:**
- What critical scenarios are NOT tested?
- What edge cases are missing?
- What attack paths have no test?

**Expected output:**
```
TEST COVERAGE

Atomicity:       ✅ Full
Replay/Nonce:    ⚠️ 2 gaps
Finality:        ❌ 3 critical gaps
Bridge Timeout:  ⚠️ Missing recovery test
Amount edge:     ⚠️ Dust amount untested
Chain ID:        ✅ Full
Governance:      ✅ Full
Validator:       ❌ Equivocation test missing

MISSING TEST BACKLOG

CRITICAL:
- test_finality_reorg_after_cross_chain_settlement()
- test_validator_equivocation_detection()
- test_bridge_timeout_automatic_recovery()
- test_atomic_swap_partial_execution_prevention()

HIGH:
- test_nonce_replay_across_chains()
- test_amount_zero_rejection()
- ...

TOTAL: 24 tests missing
EFFORT: 20 hours
```

**Time:** 10-20 min per run

---

## 📊 Step 3: Aggregate Results

Create a **MAINNET_READINESS.json** tracking file:

```json
{
  "audit_date": "2026-04-24",
  "overall_score": 82,
  "status": "GAPS IDENTIFIED",
  "categories": {
    "wiring": { "score": 87, "status": "gaps", "blockers": 2 },
    "consensus": { "score": 85, "status": "gaps", "blockers": 1 },
    "bridge": { "score": 72, "status": "critical", "blockers": 5 },
    "invariants": { "score": 71, "status": "gaps", "blockers": 3 },
    "tests": { "score": 76, "status": "gaps", "missing": 24 }
  },
  "p0_blockers": [
    { "id": "bridge_replay_nonce", "file": "crates/x3-bridge/src/core.rs", "fix_hours": 8 },
    { "id": "atomic_partial_execution", "file": "crates/x3-atomic-trade/src/settlement.rs", "fix_hours": 6 },
    { "id": "finality_reorg_handling", "file": "crates/x3-consensus/src/finality.rs", "fix_hours": 10 }
  ],
  "next_step": "Fix P0 blockers, re-pack, re-audit"
}
```

---

## 🔧 Step 4: Fix Blockers

For each P0 blocker from audits:

1. **File the blocker** - create GitHub issue with exact location
2. **Implement fix** - use suggested patch from audit
3. **Add test** - use specific test name from audit
4. **Commit** - atomic commit per blocker
5. **Regenerate pack** - only affected subsystem

```bash
# Example fix flow
git checkout -b fix/bridge-replay-nonce
# ... make changes ...
cargo test -p x3-bridge
git commit -m "Fix: Add chain_id to nonce hash (bridge replay #123)"

# Re-run only bridge pack
repomix \
  --output launch-gates/packs/bridge-atomic-$TIMESTAMP.md \
  --include "crates/x3-bridge*/**/*.rs" \
  X3_ATOMIC_STAR/
```

---

## 🔄 Step 5: Re-Audit

After fixes, re-run the same audit with new pack.

```bash
# Generate fresh pack
./repomix-mainnet-pack.sh

# Re-run wiring audit with full-repo-NEW.md
# Re-run bridge safety with bridge-atomic-NEW.md
# Re-run test gap with tests-NEW.md
```

Expected progression:

```
Audit #1:  [████░░░░░] 82/100 - GAPS
Audit #2:  [█████░░░░] 85/100 - IMPROVE
Audit #3:  [██████░░░] 88/100 - IMPROVE
Audit #4:  [███████░░] 92/100 - ACCEPTABLE
Audit #5:  [████████░] 96/100 - READY
```

---

## 📈 Mainnet Readiness Dashboard

Track progress in a simple table:

| Date | Wiring | Consensus | Runtime | Bridge | Invariants | Tests | Overall | Status |
|------|--------|-----------|---------|--------|-----------|-------|---------|--------|
| 2026-04-24 | 87 | 85 | 92 | 72 | 71 | 76 | 82 | GAPS |
| 2026-04-25 | 90 | 88 | 94 | 78 | 80 | 82 | 85 | IMPROVE |
| 2026-04-26 | 92 | 90 | 95 | 85 | 85 | 88 | 89 | ACCEPTABLE |
| 2026-04-27 | 95 | 93 | 97 | 90 | 92 | 94 | 93 | READY |

---

## ✅ Launch Criteria

**All of these must be true:**

- [ ] Overall score ≥ 95/100
- [ ] All category scores ≥ 90/100
- [ ] Zero P0 blockers
- [ ] All critical tests passing
- [ ] All invariants enforced + tested + monitored
- [ ] Bridge/atomic tests 100% passing
- [ ] Recent git diff shows no new TODOs in critical paths
- [ ] Testnet running 72 hours without issues
- [ ] Multi-node validator setup verified
- [ ] Genesis config validated
- [ ] Deployment scripts tested
- [ ] Monitoring + alerts ready

---

## 🚀 Quick Start

```bash
# 1. Generate packs
cd /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates
./repomix-mainnet-pack.sh

# 2. See what was generated
ls -lh packs/

# 3. For each pack + corresponding prompt:
#    a. Copy pack contents (full-repo-*.md)
#    b. Paste to Claude
#    c. Add prompt from prompts/ folder
#    d. Run audit
#    e. Save report to reports/

# 4. Create MAINNET_READINESS.json with scores

# 5. Fix blockers

# 6. Re-pack + re-audit

# Repeat until all scores ≥ 95/100
```

---

## 📝 Files Generated

```
launch-gates/
├── repomix-mainnet-pack.sh          ← Run to generate packs
├── prompts/
│   ├── 01-wiring-audit.md           ← Prove everything is wired
│   ├── 02-mainnet-launch-gate.md    ← Get 0-100 score
│   ├── 03-bridge-safety-audit.md    ← Find all bridge bugs
│   ├── 04-invariant-hunter.md       ← Verify all properties
│   └── 05-test-gap-audit.md         ← Find missing tests
├── packs/                            ← Generated Repomix files
│   ├── full-repo-*.md
│   ├── runtime-consensus-*.md
│   ├── bridge-atomic-*.md
│   ├── tests-*.md
│   └── git-drift-*.md
└── reports/                          ← Audit results
    ├── 01-wiring-audit-report.md
    ├── 02-mainnet-gate-report.md
    ├── 03-bridge-safety-report.md
    ├── 04-invariant-hunter-report.md
    ├── 05-test-gap-report.md
    └── MAINNET_READINESS.json
```

---

## 🎯 Current State

**Status:** ✅ Audit infrastructure ready  
**Next:** Run first full audit with Repomix packs  
**Timeline:** 1-2 weeks to mainnet-ready (assuming no major gaps)

---

*Setup Date: April 24, 2026*  
*Framework: Repomix + AI-driven evidence-based audit*  
*Goal: ZERO guessing. ZERO surprises. MAINNET READY.*
