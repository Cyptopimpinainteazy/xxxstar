# 🎯 Execution Guide: How to Use the Audit System

**Start:** April 24, 2026  
**Goal:** Get X3 to mainnet-ready status  
**Tool:** Repomix audit packs + AI-driven auditing  
**Timeline:** 1-2 weeks (if <10 major blockers)

---

## 🚀 Phase 1: Generate Your First Audit Packs (5 minutes)

```bash
# Navigate to audit directory
cd /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates

# Generate 5 packs
./repomix-mainnet-pack.sh

# Wait for completion (should show ✅ after each pack)
```

**What happens:** Script generates 5 markdown files in `packs/` directory
- `full-repo-*.md` - Complete codebase
- `runtime-consensus-*.md` - Runtime/consensus code only
- `bridge-atomic-*.md` - Cross-VM bridge code
- `tests-*.md` - All test files
- `git-drift-*.md` - Recent changes

**Verify success:**
```bash
ls -lh packs/ | grep ".md"
# Should show 5 files with sizes like 15M, 3M, 2M, etc.
```

---

## 📋 Phase 2: Run Your First Audit (15-20 minutes)

### Step 1: Copy the first pack

```bash
# Show where the pack is
echo "Full repo pack:"
ls -1 packs/full-repo-*.md
```

**Copy the entire contents of that file** (all thousands of lines)

### Step 2: Paste to Claude

1. Open a new Claude conversation
2. Paste the full repo pack content
3. Then paste this prompt from `prompts/01-wiring-audit.md`

**Prompt:** Paste the entire contents of `launch-gates/prompts/01-wiring-audit.md`

**Claude will analyze** and produce output like:

```
WIRING AUDIT RESULTS
════════════════════════════════════

Modules Found: 87
Connected modules: 83 ✅
Unwired modules: 3 ❌
Dead code candidates: 2

UNWIRED MODULES:
- x3-legacy-bridge (abandoned)
- x3-test-harness (dev only)

BLOCKERS FOUND: 1
- Missing integration with x3-settlement-engine

SCORE: 87/100 [GAPS IDENTIFIED]
```

### Step 3: Save the results

```bash
# Create file in reports/
cat > reports/01-wiring-audit-report.md << 'EOF'
# Wiring Audit Results - Round 1
[Paste Claude's output here]
EOF
```

### Step 4: Extract the score

```bash
# Create tracking file
cat > reports/MAINNET_READINESS.json << 'EOF'
{
  "round": 1,
  "date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "audits": {
    "wiring": 87
  }
}
EOF
```

---

## 🔄 Phase 3: Repeat for Other 4 Audits (15-20 minutes each)

| Audit | Pack to use | Prompt file | Save to |
|-------|------------|-------------|---------|
| 1 | full-repo-*.md | 01-wiring-audit.md | 01-wiring-audit-report.md |
| 2 | full-repo-*.md | 02-mainnet-launch-gate.md | 02-mainnet-gate-report.md |
| 3 | bridge-atomic-*.md | 03-bridge-safety-audit.md | 03-bridge-safety-report.md |
| 4 | full-repo-*.md | 04-invariant-hunter.md | 04-invariant-hunter-report.md |
| 5 | tests-*.md | 05-test-gap-audit.md | 05-test-gap-report.md |

**Can run in parallel** - don't need to wait for #1 to finish #2

After each audit:
1. Save report to `reports/XX-*.md`
2. Extract score to MAINNET_READINESS.json

---

## 📊 Phase 4: Analyze Results (10 minutes)

```bash
# View your scores
cat reports/MAINNET_READINESS.json
```

Expected format:

```json
{
  "round": 1,
  "overall_score": 82,
  "audits": {
    "wiring": 87,
    "mainnet_gate": 82,
    "bridge_safety": 72,
    "invariants": 71,
    "tests": 76
  },
  "p0_blockers": 5,
  "p1_blockers": 12
}
```

### Read the reports to understand blockers

```bash
# What needs to be fixed?
grep -i "blocker\|critical\|p0\|p1" reports/*.md
```

### Prioritize by severity

- **P0 (Mainnet Blocker)**: Fix immediately (1-8 hours each)
- **P1 (Urgent)**: Fix today (2-4 hours each)
- **P2 (Nice to have)**: Fix later (tomorrow or next week)

---

## 🔧 Phase 5: Fix the Top 3 Blockers (3-8 hours)

For each P0 blocker:

### Step 1: Create a branch
```bash
cd /home/lojak/Desktop/X3_ATOMIC_STAR
git checkout -b fix/descriptive-name
```

### Step 2: Read the audit report for exact location

Example from bridge safety audit:
```
Blocker: Replay attack via nonce reuse
File: crates/x3-bridge/src/core.rs:342
Fix: Add chain_id to nonce hash
Time: 2 hours
```

### Step 3: Make the fix
```bash
# Edit the file
vim crates/x3-bridge/src/core.rs
# Change line 342 to include chain_id
```

### Step 4: Add a test
```bash
# Create test file if doesn't exist
touch crates/x3-bridge/tests/test_replay_protection.rs

# Add test that exercises the fix
cargo test -p x3-bridge --test test_replay_protection
```

### Step 5: Commit atomically
```bash
git add -A
git commit -m "Fix: Add chain_id to nonce hash for replay protection (blocker #1 of 5)"
```

---

## 🔄 Phase 6: Re-Generate & Re-Audit (20 minutes)

After fixing 3-5 blockers:

### Step 1: Generate new packs
```bash
cd /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates
./repomix-mainnet-pack.sh
```

This generates NEW files with `-NEW` in name (or newer timestamp)

### Step 2: Re-run affected audits

Only re-run the audits that cover what you fixed:
- Fixed bridge? → Re-run audit #3
- Fixed wiring? → Re-run audit #1
- Fixed tests? → Re-run audit #5

### Step 3: Compare scores
```bash
# Old round
cat reports/MAINNET_READINESS.json | grep bridge_safety

# New round - expect improvement
# 72 → 78 or higher
```

---

## 📈 Phase 7: Track Progress

After each audit round, append to progress log:

```bash
cat >> reports/PROGRESS.md << 'EOF'

## Round 2 - April 25, 2026

**Changes:** Fixed bridge replay, added wiring integration, 12 new tests

| Category | Round 1 | Round 2 | Trend |
|----------|---------|---------|-------|
| Wiring | 87 | 92 | ⬆️ +5 |
| Bridge | 72 | 81 | ⬆️ +9 |
| Tests | 76 | 84 | ⬆️ +8 |
| **Overall** | 82 | 86 | ⬆️ +4 |

**Blockers remaining:** 3 P0, 8 P1

**Next:** Fix atomic settlement timeout handling

EOF
```

---

## ✅ Phase 8: Repeat Until Ready

Keep cycling:
```
Fix blockers (3-8h) → Re-pack (5min) → Re-audit (50min) → Check score → Repeat
```

Continue until:
- ✅ Overall score ≥ 95/100
- ✅ All categories ≥ 90/100
- ✅ Zero P0 blockers
- ✅ Bridge safety ≥ 95/100

---

## 🎯 What "Ready" Looks Like

```
AUDIT ROUND 5 - READY FOR MAINNET
═════════════════════════════════════════

Wiring:          ✅ 100/100 (All modules connected)
Mainnet Gate:    ✅ 97/100  (All systems ready)
Bridge Safety:   ✅ 96/100  (All attacks mitigated)
Invariants:      ✅ 100/100 (All properties verified)
Test Gaps:       ✅ 100/100 (All scenarios tested)

OVERALL: ✅ 99/100

P0 Blockers: 0
P1 Blockers: 0
P2 Blockers: 2 (deferred to post-launch)

Recommendation: 🚀 LAUNCH
═════════════════════════════════════════
```

---

## 📅 Timeline Example

| Date | Audit | Fixes | Score | Status |
|------|-------|-------|-------|--------|
| Apr 24 | Round 1 | — | 82 | 🔴 Gaps found |
| Apr 25 | Round 2 | 5 fixes | 86 | 🟡 Improving |
| Apr 26 | Round 3 | 4 fixes | 90 | 🟡 Close |
| Apr 27 | Round 4 | 2 fixes | 94 | 🟡 Very close |
| Apr 28 | Round 5 | 1 fix | 97 | 🟢 Ready |

---

## 🚨 Common Pitfalls & Solutions

**❌ "I generated packs but got an error"**
```bash
# Check repomix is installed
which repomix

# If not, install
npm install -g repomix

# Try again
./repomix-mainnet-pack.sh
```

**❌ "Claude didn't output expected format"**
- Ensure FULL prompt was pasted (check it's in report)
- Ensure FULL pack was pasted (check size in reports)
- Try again with simpler prompt (e.g., just wiring first)

**❌ "Tests are taking too long"**
```bash
# Run in background
cd /home/lojak/Desktop/X3_ATOMIC_STAR
cargo test -p x3-chain-node --lib > test-output.log 2>&1 &
```

**❌ "I fixed a blocker but score didn't improve"**
- Did you regenerate packs? `./repomix-mainnet-pack.sh`
- Did you use the NEW pack in audit? (Check timestamp)
- Did fix actually deploy? (`cargo build` to verify)

---

## 💾 File Locations to Remember

```
Main repo:
  /home/lojak/Desktop/X3_ATOMIC_STAR

Audit infrastructure:
  /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates

Packs (generated):
  /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/packs/

Prompts (audit instructions):
  /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/prompts/

Reports (your results):
  /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/reports/

Score tracking:
  /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/reports/MAINNET_READINESS.json
```

---

## 🎓 Key Commands (Copy-Paste Ready)

```bash
# Start
cd /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates

# Generate packs
./repomix-mainnet-pack.sh

# Check packs created
ls -lh packs/ | head

# View documentation
cat QUICK_REFERENCE.md

# Check progress
cat reports/MAINNET_READINESS.json

# After fixes, regenerate
./repomix-mainnet-pack.sh

# Run tests
cd .. && cargo test -p x3-chain-node --lib
```

---

## ⏱️ Realistic Timeline

- **Day 1:** Generate packs + run all 5 audits (3 hours)
- **Days 2-3:** Fix 5-8 P0/P1 blockers (12-24 hours, can parallel)
- **Day 4:** Re-audit + fix remaining issues (4 hours)
- **Day 5:** Final verification + testnet (4 hours)
- **Days 6+:** Testnet stability (72+ hours, runs in background)

**Total:** 1-2 weeks to mainnet-ready (assuming <10 blockers)

---

## 🚀 Start Now

```bash
# Right now, run this
cd /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates && ./repomix-mainnet-pack.sh

# Then follow Phase 2 above with first pack
```

---

*Execution guide created April 24, 2026*  
*Framework: Repomix + AI auditing*  
*Goal: Mainnet-ready with 100% confidence*
