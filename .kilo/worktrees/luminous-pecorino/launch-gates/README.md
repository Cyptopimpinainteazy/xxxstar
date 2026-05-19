# 🚀 Launch Gates: X3 Mainnet Readiness Audit System

**Version:** 1.0  
**Date:** April 24, 2026  
**Status:** ✅ Ready to execute  
**Framework:** Repomix + AI-driven evidence-based auditing  

---

## 📌 What This Is

A **systematic, evidence-based audit system** to verify X3 is mainnet-ready. No guessing. No surprises. 100% coverage on:

- **Wiring:** Every module connected, nothing orphaned
- **Bridge safety:** All cross-VM attack vectors covered
- **Invariants:** All critical properties enforced + tested
- **Tests:** All edge cases covered
- **Governance:** Upgrades are atomic + safe

---

## 🎯 Quick Start (2 minutes)

```bash
cd /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates

# 1. Generate audit packs
./repomix-mainnet-pack.sh

# 2. View what was generated
ls -lh packs/

# 3. Read quick reference
cat QUICK_REFERENCE.md

# 4. For each audit, follow the 5-step workflow in MAINNET_AUDIT_WORKFLOW.md
```

---

## 📚 Documentation

### 🔍 QUICK_REFERENCE.md
**Read this first.** 2-page overview of:
- What each audit does
- Expected output format
- Progress tracking
- Mainnet ready checklist

### 📖 MAINNET_AUDIT_WORKFLOW.md
**Read this for details.** Complete step-by-step guide including:
- How to run each audit
- What each audit finds
- How to fix blockers
- How to track progress
- Timeline estimates

---

## 🔨 How It Works

### Step 1: Generate Packs
```bash
./repomix-mainnet-pack.sh
```

Creates 5 targeted markdown files in `packs/`:
- `full-repo-*.md` - complete codebase
- `runtime-consensus-*.md` - runtime + consensus only
- `bridge-atomic-*.md` - cross-VM code
- `tests-*.md` - all test suites
- `git-drift-*.md` - recent changes + logs

### Step 2: Run Audits
For each pack, use corresponding audit prompt:

```
full-repo-*.md    → prompts/01-wiring-audit.md
full-repo-*.md    → prompts/02-mainnet-launch-gate.md
bridge-atomic-*.md → prompts/03-bridge-safety-audit.md
full-repo-*.md    → prompts/04-invariant-hunter.md
tests-*.md        → prompts/05-test-gap-audit.md
```

### Step 3: Track Results
Save each audit result to `reports/`. Update `MAINNET_READINESS.json`.

### Step 4: Fix Blockers
Implement fixes in code, add tests, commit atomically.

### Step 5: Re-Audit
Regenerate packs, re-run audits, verify improvements.

---

## 📋 The 5 Audits

| # | Audit | Purpose | Pack | Time |
|---|-------|---------|------|------|
| 1 | Wiring | Prove all modules connected | full-repo | 5-15min |
| 2 | Launch Gate | Get 0-100 readiness score | full-repo | 10-20min |
| 3 | Bridge Safety | Find all cross-VM attacks | bridge-atomic | 20-30min |
| 4 | Invariant Hunter | Verify all properties tested | full-repo | 15-25min |
| 5 | Test Gap | Identify untested scenarios | tests | 10-20min |

---

## 📊 Progress Tracking

After each audit round, update `MAINNET_READINESS.json`:

```json
{
  "round": 1,
  "date": "2026-04-24T00:00:00Z",
  "status": "GAPS IDENTIFIED",
  "overall_score": 82,
  "scores_by_category": {
    "wiring": 87,
    "consensus": 85,
    "bridge": 72,
    "invariants": 71,
    "tests": 76
  },
  "p0_blockers": 5,
  "p1_blockers": 12,
  "estimated_hours_to_fix": 24,
  "next_action": "Fix bridge replay nonce vulnerability"
}
```

---

## ✅ Mainnet Ready Criteria

All of these must be true:

```
✅ Overall audit score: ≥95/100
✅ All category scores: ≥90/100
✅ P0 blockers: 0 remaining
✅ All critical tests: ✅ passing
✅ Wiring: 100/100 (no dead code)
✅ Bridge safety: 95+/100 (all threats mitigated)
✅ Invariants: All enforced + tested + monitored
✅ Testnet: 72+ hours stable
✅ Validators: Multi-node setup verified
✅ Genesis: Config validated
✅ Deployment: Scripts tested
✅ Monitoring: Alerts configured
```

---

## 🗂️ Directory Structure

```
launch-gates/
├── README.md                          ← You are here
├── QUICK_REFERENCE.md                 ← Start here (2 pages)
├── MAINNET_AUDIT_WORKFLOW.md          ← Full guide (detailed)
├── repomix-mainnet-pack.sh            ← Executable script
│
├── prompts/                           ← Audit instructions
│   ├── 01-wiring-audit.md
│   ├── 02-mainnet-launch-gate.md
│   ├── 03-bridge-safety-audit.md
│   ├── 04-invariant-hunter.md
│   └── 05-test-gap-audit.md
│
├── packs/                             ← Generated audit packs
│   ├── full-repo-TIMESTAMP.md
│   ├── runtime-consensus-TIMESTAMP.md
│   ├── bridge-atomic-TIMESTAMP.md
│   ├── tests-TIMESTAMP.md
│   └── git-drift-TIMESTAMP.md
│
└── reports/                           ← Audit results (create as you go)
    ├── 01-wiring-audit-report.md
    ├── 02-mainnet-gate-report.md
    ├── 03-bridge-safety-report.md
    ├── 04-invariant-hunter-report.md
    ├── 05-test-gap-report.md
    └── MAINNET_READINESS.json
```

---

## 🚀 Workflow

```
┌─────────────────────────────────────────────────────────────┐
│ Generate Packs: ./repomix-mainnet-pack.sh                  │
│ Output: 5 markdown files in packs/                          │
└────────────────────┬────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ Run Audits (can parallel, ~50 min total)                    │
│ 1. Copy pack → Paste to Claude + prompt → Save report      │
│ 2. Repeat for each of 5 packs                              │
└────────────────────┬────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ Track Scores: Update MAINNET_READINESS.json                 │
│ Identify P0/P1/P2 blockers                                  │
└────────────────────┬────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ Fix Blockers: 1-8 hours per blocker                         │
│ 1. git checkout -b fix/description                          │
│ 2. Implement fix + add test                                │
│ 3. cargo test                                              │
│ 4. git commit                                              │
└────────────────────┬────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ Re-Generate Packs: ./repomix-mainnet-pack.sh (new timestamp)│
│ Re-Run Audits: With new packs                               │
│ Loop until score ≥ 95/100                                   │
└─────────────────────────────────────────────────────────────┘
                     ↓
                  MAINNET READY ✅
```

---

## 📈 Timeline Estimate

| Phase | Task | Duration | Status |
|-------|------|----------|--------|
| Setup | ✅ Complete | - | ✅ DONE |
| Audit 1 | Generate + run 5 audits | 2-3 hrs | Ready |
| Fixes | Implement blockers | 12-24 hrs | Pending |
| Audit 2 | Re-run to verify fixes | 2-3 hrs | Pending |
| Cleanup | Edge cases + polish | 4-8 hrs | Pending |
| Audit 3 | Final verification | 1-2 hrs | Pending |
| **Total** | **Setup → Mainnet Ready** | **~30-40 hrs** | |

---

## 🎯 What Success Looks Like

```
MAINNET READINESS (Round 3)
════════════════════════════════════════════════════════════

Wiring Audit:           ✅ 100/100 (No dead code found)
Mainnet Launch Gate:    ✅ 97/100  (All critical systems ready)
Bridge Safety:          ✅ 98/100  (All attack paths mitigated)
Invariant Hunter:       ✅ 100/100 (All properties enforced)
Test Gap Audit:         ✅ 100/100 (All scenarios covered)

────────────────────────────────────────────────────────────
OVERALL:                ✅ 99/100  [MAINNET READY]
P0 Blockers:            0
P1 Blockers:            0
Risk Level:             MINIMAL
Recommendation:         🚀 LAUNCH
════════════════════════════════════════════════════════════
```

---

## 💾 Saved Examples

**After first audit run**, this directory will contain real results like:

```
reports/
├── 01-wiring-audit-report.md
│   Contains: 87/100 score, list of 3 unwired modules
│
├── 02-mainnet-gate-report.md
│   Contains: Category scores, 5 P0 blockers with fix times
│
├── 03-bridge-safety-report.md
│   Contains: 7 threat scenarios, 12 missing tests
│
├── 04-invariant-hunter-report.md
│   Contains: 15 critical invariants, 3 missing enforcement
│
├── 05-test-gap-report.md
│   Contains: 24 missing tests, estimated 20 hours to add
│
└── MAINNET_READINESS.json
    Contains: Scores, blockers, timeline estimate
```

---

## 🔧 Commands Reference

```bash
# Start here
cd /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates

# Generate audit packs
./repomix-mainnet-pack.sh

# Check what was generated
ls -lh packs/

# View a pack (shows structure, not full content)
head -100 packs/full-repo-*.md

# After making fixes, regenerate just bridge pack
repomix \
  --output packs/bridge-atomic-FIX.md \
  --include "crates/x3-bridge*/**/*.rs" \
  /home/lojak/Desktop/X3_ATOMIC_STAR

# Run full test suite
cd /home/lojak/Desktop/X3_ATOMIC_STAR && cargo test -p x3-chain-node --lib

# Check status
ls -1 reports/ | wc -l
echo "Reports created so far"
```

---

## 🎓 Learning Path

1. **Read QUICK_REFERENCE.md** (5 min) - Overview of 5 audits
2. **Read MAINNET_AUDIT_WORKFLOW.md** (15 min) - Complete process
3. **Run `./repomix-mainnet-pack.sh`** (5 min) - Generate packs
4. **Run first audit** (20 min) - Copy pack, paste prompt, save report
5. **Track score** (5 min) - Update MAINNET_READINESS.json
6. **Fix first blocker** (2-8 hours) - Implement fix + test
7. **Re-pack and re-audit** (30 min) - Verify improvement
8. **Repeat** - Until score ≥95/100

---

## 🚨 Troubleshooting

**"repomix command not found"**
```bash
# Check installation
which repomix
# Should show: /home/lojak/.local/bin/repomix

# If not installed, install via npm
npm install -g repomix
```

**"Pack file is too large"**
- That's fine! Repomix will chunk it appropriately
- Token count shown in script output
- Claude can handle 100K+ token files

**"Audit output doesn't match expected format"**
- Check prompt was pasted completely
- Check pack was included (not truncated)
- Try a simpler pack first (e.g., tests-only)

**"Tests are slow after fixes"**
```bash
# Run in background
cargo test -p x3-chain-node --lib &
disown

# Check after lunch
```

---

## 📞 Support

If something isn't working:

1. **Check all files exist**
   ```bash
   ls -1 prompts/ | wc -l  # Should be 5
   ```

2. **Verify repomix**
   ```bash
   repomix --version
   ```

3. **Check repo path**
   ```bash
   ls -la /home/lojak/Desktop/X3_ATOMIC_STAR | head
   ```

4. **Run script with debug**
   ```bash
   bash -x repomix-mainnet-pack.sh 2>&1 | tail -50
   ```

---

## ✨ Next Steps

1. ✅ **Setup complete** - Infrastructure ready
2. ⏳ **Generate packs** - Run `./repomix-mainnet-pack.sh`
3. ⏳ **Run first audit** - 50 min to complete all 5
4. ⏳ **Identify blockers** - P0/P1/P2 classification
5. ⏳ **Fix blockers** - 12-24 hours
6. ⏳ **Re-audit** - Verify improvements
7. ⏳ **Launch** - When score ≥95/100

---

**Created:** April 24, 2026  
**Status:** ✅ Ready to execute  
**Framework:** Repomix + AI auditing  
**Goal:** ZERO guessing. 100% mainnet-ready verification.

**Start:** `cd launch-gates && ./repomix-mainnet-pack.sh`
