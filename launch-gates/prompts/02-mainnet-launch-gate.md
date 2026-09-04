# 02: Mainnet Launch Gate Audit

## Objective
Score the repo 0-100 for mainnet readiness. Identify exact blockers and fastest fix paths.

## Instructions

You are a mainnet launch committee.

**This Repomix file contains the X3 repo.**

Score it from 0-100 for mainnet readiness.

Categories and weights:
- **Consensus safety (20%):** Is PoW/PoS/other consensus resistant to attacks?
- **Runtime safety (20%):** Are pallets, extrinsics, and storage mutations protected?
- **Bridge/atomic safety (20%):** Can bridges replay? Can cross-VM atomicity fail?
- **Cross-VM correctness (10%):** Are EVM/SVM/native lockstep synchronized?
- **DEX/economy safety (10%):** Can reserves overflow? Can supply inflate?
- **Validator operations (5%):** Is validator setup, slashing, rewards correct?
- **Deployment/monitoring (5%):** Are monitoring, alerts, metrics complete?
- **Tests (5%):** Do tests cover all critical paths?
- **Documentation (5%):** Does docs match code?

For every score below 90 on any category:
1. List exact blockers
2. Show file/function affected
3. Give fastest fix path
4. Estimate effort: hours to fix
5. Mark as: P0/P1/P2 (mainnet blocker / urgent / nice-to-have)

## Expected Output

**MAINNET READINESS SCORECARD**

| Category | Score | Status | Blockers | Fix Effort |
|----------|-------|--------|----------|-----------|
| Consensus Safety | 85 | ⚠️ GAPS | [list] | X hours |
| Runtime Safety | 90 | ✅ OK | None | 0 |
| Bridge Safety | 75 | 🔴 CRITICAL | [list] | X hours |
| [etc.] | | | | |

**OVERALL SCORE: [X]/100**

Status:
- 90+: READY
- 75-89: FIX REQUIRED BEFORE LAUNCH
- <75: STOP - DO NOT ATTEMPT MAINNET

**P0 BLOCKERS (Must fix before mainnet)**
1. [Issue] - File/function - Risk - Fix
2. [Issue] - File/function - Risk - Fix

**P1 URGENT (Should fix before mainnet)**
1. [Issue] - File/function - Risk - Fix

**P2 NICE-TO-HAVE (Can fix after mainnet if needed)**
1. [Issue] - File/function - Risk - Fix

**ESTIMATED TIME TO MAINNET-READY:** X hours

**RECOMMENDATION:**
- READY / DO NOT LAUNCH / LAUNCH WITH CAUTION / REAUDIT AFTER FIXES
