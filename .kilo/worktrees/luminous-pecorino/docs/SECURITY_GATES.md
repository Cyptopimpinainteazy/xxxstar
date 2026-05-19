# Security Gates Documentation

## Overview

Security gates are automated verification checkpoints that prevent unproven code from reaching production. The X3 Atomic Star blockchain implements three primary gates (S0, S1, Mainnet) that verify proof integrity at different stages of the development pipeline.

## Gate Architecture

### Gate Hierarchy

```
Development
    ↓
[S0 Gate: Pre-Commit]           Local Developer Machine
    - Scans claims structure    (Bash script - .git/hooks/pre-commit)
    - Verifies claim syntax
    - Blocks invalid commits
    ↓ (if passes)
GitHub Push
    ↓
[CI/CD: GitHub Actions]         Remote CI/CD Pipeline
    - Builds binary             (YAML workflow)
    - Runs tests
    ↓ (if passes)
[S1 Gate: Merge Gate]
    - Security verification     (Requires: ALL above pass)
    - Blocks unsafe merges
    - Required: Cannot override
    ↓ (if passes)
Main Branch
    ↓
[Testnet Gate: ≥ 0.85]          Scheduled (3 AM UTC)
    - Testnet readiness check   (./target/release/x3-proof testnet-gate)
    - Score threshold verified
    ↓ (if passes)
[Mainnet Gate: ≥ 0.95]          Scheduled (4 AM UTC)
    - Mainnet readiness check   (./target/release/x3-proof mainnet-gate)
    - Final production approval
```

## Gate Types

### S0: Pre-Commit Gate (Local)

**Purpose:** Catch invalid changes before committing

**Location:** `.git/hooks/pre-commit` (Developer machine)

**Trigger:** `git commit` command

**Scope:** Scans staged files for proof validity

**Commands Executed:**
```bash
./target/release/x3-proof scan-claims       # List all claims
./target/release/x3-proof verify [claim-id] # Verify each claim
```

**Time:** ~2-3 seconds

**Blocking:** YES (prevents commit if invalid)

**Override:** `git commit --no-verify` (not recommended)

**Example Output:**

```
🔍 Running S0 Pre-Commit Gate: Scanning claims...

Scanning proof-forge modules...
✓ Consensus module: 20 claims verified
✓ Custody module: 15 claims verified
✓ Asset Kernel module: 18 claims verified
...

✅ Pre-commit gate passed
23 claims verified
0 warnings
0 errors
```

**Failure Example:**

```
🔍 Running S0 Pre-Commit Gate: Scanning claims...

❌ ERROR: Claim syntax invalid
Module: X3Language
Claim #7: "Compile safety" - missing verification proof
Path: src/x3vm/compiler.rs:234

❌ S0 Pre-Commit Gate FAILED

To fix: Add missing proof to claim or revert changes
To skip: git commit --no-verify (not recommended)
```

---

### S1: Merge Gate (CI/CD - Required)

**Purpose:** Final verification before merging to main branch

**Location:** GitHub Actions (`.github/workflows/proof-gates.yml`)

**Trigger:** After S0 and all tests pass

**Scope:** Full security verification suite

**Commands Executed:**
```bash
./target/release/x3-proof security-gate --fail-hard
```

**Time:** ~30-60 seconds

**Blocking:** YES (prevents PR merge if fails)

**Override:** NO (cannot be overridden)

**Example Output:**

```
🛡️  Running S1 Merge Gate...

Checking security constraints...
✓ No unsafe code blocks detected
✓ All dependencies verified
✓ No unreviewed external calls
✓ Memory safety verified
✓ Consensus invariants checked
✓ Replay protection verified

All 15 security checks passed
✅ S1 Merge Gate PASSED
```

**Failure Example:**

```
🛡️  Running S1 Merge Gate...

❌ SECURITY VIOLATION: Unsafe consensus modification detected

File: src/consensus/validator.rs:156
Issue: Direct state mutation without proof
Severity: CRITICAL

❌ S1 Merge Gate FAILED

Action required:
1. Review changes for consensus invariant violations
2. Add proof documentation
3. Re-commit and push
```

---

### Testnet Gate (Automated)

**Purpose:** Verify testnet readiness (score ≥ 0.85)

**Location:** Scheduled GitHub Actions workflow

**Trigger:** Daily at 3 AM UTC (cron: `0 3 * * *`)

**Scope:** Proof score calculation across all 20 modules

**Commands Executed:**
```bash
./target/release/x3-proof testnet-gate -v
```

**Time:** ~2-3 minutes (full proof suite)

**Blocking:** NO (reference only; S1 blocks merge)

**Score Components:**

```
Proof Score = (
  Component 1 (Consensus): 0.99 +
  Component 2 (Custody): 0.99 +
  Component 3 (Formal Proofs): 1.00 +
  Component 4 (Security): 0.96 +
  Component 5 (Economic): 0.92 +
  Component 6 (Governance): 0.94 +
  Component 7 (Performance): 0.95 +
  Component 8 (Incident Response): 0.88
) / 8 = 0.95

Testnet Threshold: 0.85
Status: ✅ READY
```

**Example Output:**

```
🔗 Checking Testnet Readiness...

Running full proof suite on 20 modules...
  ✓ Consensus Module: 0.99
  ✓ Custody Module: 0.99
  ✓ Asset Kernel Module: 0.98
  ...
  
Overall Score: 0.94
Grade: A-

✅ Testnet Ready (0.94 ≥ 0.85 threshold)

Module Details:
  P7 Critical: 0.98 (5/5)
  P6 Advanced: 0.94 (5/5)
  P5 Economic: 0.92 (5/5)
  P4 Foundation: 0.88 (5/5)
```

---

### Mainnet Gate (Automated)

**Purpose:** Verify mainnet readiness

**Status:** ✅ **GO FOR MAINNET RC-1** — 100% score achieved

**Location:** Scheduled GitHub Actions workflow

**Trigger:** Scheduled verification

**Scope:** Full production readiness verification

**Results:**
- Overall Score: **100%**
- S0 Verified: **16/16**
- Blockers: **0**
- Decision: **GO**

**Machine Report:** [launch-gates/reports/X3-MAINNET-GO-NO-GO-20260501-203300.md](../launch-gates/reports/X3-MAINNET-GO-NO-GO-20260501-203300.md)

> **Historical Note:** This gate previously had a 0.95 threshold. The ProofForge system now uses a 100% / 16/16 S0 verification model with all gates passing on commit `2e0c3bdac9de8b60`.

---

## Gate Thresholds

### Score Interpretation

| Score | Grade | Status | Action |
|-------|-------|--------|--------|
| 100% | A+ | ✅ GO | Mainnet RC-1 authorized |
| 90-99% | A | ✅ GO | Ready for deployment |
| 85-89% | B+ | ⚠️ Review | Needs verification |
| < 85% | B/C | ❌ NO-GO | Not approved |

### Current Status

**Status:** ✅ GO FOR MAINNET RC-1  
**Overall Score:** 100% (16/16 S0 verified)  
**Commit:** `2e0c3bdac9de8b60`  
**Date:** 2026-05-02

```
All ProofForge Security Gates: ✅ PASSED
├─ S0 Blockers: 6/6 RESOLVED (S0-1 through S0-6)
├─ S1 Blockers: 3/3 RESOLVED (S1-1, S1-2, S1-3)
└─ RC-1 Scope: LOCKED

Launch Decision: GO
Overall Score: 100%
Blockers: 0
```

---

## Running Gates Locally

### Pre-Commit Gate (Developer)

```bash
# Manual execution
.git/hooks/pre-commit

# Or via script
./scripts/run-security-gates.sh s0

# Expected output
🔍 Running S0 Pre-Commit Gate...
✅ Pre-commit gate passed
```

### Merge Gate (Developer Testing)

```bash
# Test before creating PR
./scripts/run-security-gates.sh s1

# Expected output
🛡️  Running S1 Merge Gate...
✅ S1 Merge Gate PASSED
```

### Testnet Readiness

```bash
# Check testnet qualification
./scripts/run-security-gates.sh testnet

# Output includes score and breakdown
✅ Testnet Ready (0.94 ≥ 0.85 threshold)
```

### Mainnet Readiness

```bash
# Check mainnet qualification
./scripts/run-security-gates.sh mainnet

# Output shows gap if not ready
⚠️  Mainnet Candidate (0.94 at 0.95 threshold, gap: 0.01)
```

### Run All Gates

```bash
# Test complete sequence
./scripts/run-security-gates.sh all

# Output
✓ S0 gate: PASSED
✓ S1 gate: PASSED
✓ Testnet gate: PASSED (0.94 ≥ 0.85)
⚠️  Mainnet gate: CANDIDATE (0.94 at 0.95)
```

---

## GitHub Actions Integration

### Workflow Execution

View gate execution in GitHub:

1. Go to **Actions** tab
2. Select **"ProofForge Gates - Automated Proof Verification"**
3. Click recent workflow run
4. Expand jobs:
   - ✅ **build** - Compiles binary
   - ✅ **test** - Runs test suite
   - ✅ **s0-gate** - Pre-commit scanning
   - ✅ **s1-merge-gate** - Merge verification
   - ✅ **dashboard** - Metrics export

### Interpreting Results

**Green ✅ - Gate Passed**
```
✓ Step: Run S0 Pre-Commit Gate
  Completed successfully in 2.3s
```

**Red ❌ - Gate Failed**
```
✗ Step: Run S1 Merge Gate  
  Error: Security violation detected
  See logs for details
```

**Skipped ⊘ - Conditional Skip**
```
⊘ Step: Deploy Dashboard
  Skipped (only runs on success)
```

### Pull Request Status Checks

When you create a pull request:

```
All checks must pass before merge:

✅ build — ProofForge binary compiled
   (Required: click Details to view)

✅ test — 2700+ integration tests passed
   (Required: click Details to view)

✅ s0-gate — Code structure verified
   (Required: click Details to view)

✅ s1-merge-gate — Security verified
   ⭐ REQUIRED (cannot merge if fails)
   (Required: click Details to view)

✅ dashboard — Proof metrics exported
   (Information: optional)
```

If any check fails ❌:
1. Click **Details** link
2. View error message and logs
3. Fix issue locally
4. Push updated code
5. Checks automatically re-run

---

## Modifying Gate Thresholds

⚠️ **WARNING:** Changing thresholds affects blockchain security guarantees

### Current Configuration

> **Note:** The threshold-based model (0.85/0.95) has been replaced by the ProofForge 100%/16/16 S0 verification model as of 2026-05-02.

**Historical Configuration (pre-RC-1):**
```yaml
# .github/workflows/proof-gates.yml

gates:
  testnet_threshold: 0.85  # Testnet minimum
  mainnet_threshold: 0.95  # Mainnet minimum
  s0_strict_mode: true     # Pre-commit enforcement
  s1_allow_override: false  # Cannot bypass merge gate
```

**Current Configuration (RC-1):**
```yaml
# ProofForge gates now use 100% / 16/16 S0 model
gates:
  s0_verified: 16/16       # All S0 claims verified
  s1_verified: 9/9         # All S1 claims verified  
  score: 100%              # GO for RC-1
  blockers: 0              # No launch blockers
```

### Modifying Thresholds

**To increase mainnet threshold:**

```yaml
# In .github/workflows/proof-gates.yml
mainnet_threshold: 0.96  # Increase from 0.95
```

**To decrease testnet threshold (not recommended):**

```yaml
# In .github/workflows/proof-gates.yml
testnet_threshold: 0.80  # Decrease from 0.85 (⚠️ reduces rigor)
```

### Impact Analysis

| Change | Impact | Recommendation |
|--------|--------|-----------------|
| Increase mainnet threshold 0.95→0.96 | More rigorous, delays launch | Acceptable for security |
| Decrease testnet threshold 0.85→0.80 | Less rigorous, faster testnet | Not recommended |
| Disable S0 gate | No pre-commit verification | ⚠️ Not recommended |
| Allow S1 override | Manual merge bypass | ⚠️ CRITICAL: Do not do this |

---

## Troubleshooting

### Gate Failures

#### Problem: "Binary not found"
```
❌ ERROR: ./target/release/x3-proof: No such file or directory
```

**Solution:**
```bash
# Build binary
cargo build -p proof-forge --release

# Verify
ls -lh target/release/x3-proof
```

#### Problem: "Permission denied"
```
❌ ERROR: ./x3-proof: Permission denied
```

**Solution:**
```bash
# Fix permissions
chmod +x target/release/x3-proof
chmod +x .git/hooks/pre-commit
chmod +x scripts/run-security-gates.sh
```

#### Problem: "Claim verification failed"
```
❌ ERROR: Module X3VM claim #5 failed verification
```

**Solution:**
1. Review claim file: `proof-forge/claims/x3vm.toml`
2. Run specific module: `./target/release/x3-proof verify x3vm`
3. Check for breaking changes
4. Update proof if intentional
5. Re-commit

#### Problem: "Timeout"
```
❌ ERROR: Timeout after 60 seconds
```

**Solution:**
```bash
# Run in parallel mode
./target/release/x3-proof prove-all --parallel

# Or test individual modules
./target/release/x3-proof prove consensus
./target/release/x3-proof prove custody
```

#### Problem: "GitHub Actions workflow fails"
```
✗ s1-merge-gate FAILED
```

**Solution:**
1. Click workflow run in GitHub Actions
2. View error in logs
3. Reproduce locally: `./scripts/run-security-gates.sh s1`
4. Fix issue
5. Push updated code
6. Workflow automatically re-runs

### Score Anomalies

#### Problem: Score dropped unexpectedly
```
Overall Score: 0.92 (was 0.94)
```

**Investigation:**
```bash
# Check component breakdown
./target/release/x3-proof dashboard -v | grep -E "Component|Score"

# Identify which component degraded
./target/release/x3-proof prove consensus    # Test individual modules
./target/release/x3-proof prove custody

# Compare to baseline
./target/release/x3-proof dashboard --compare baseline.json
```

**Resolution:**
1. Find degraded component
2. Run full test suite: `cargo test --all`
3. Address test failures
4. Re-run gates: `./scripts/run-security-gates.sh all`

---

## Best Practices

### For Developers

✅ **DO:**
- Run `./scripts/run-security-gates.sh s0` before committing
- Read gate error messages carefully
- Fix issues instead of bypassing gates
- Test locally before pushing to GitHub
- Review proof scores regularly

❌ **DON'T:**
- Use `git commit --no-verify` routinely
- Ignore pre-commit hook failures
- Push code that fails S0 gate
- Try to bypass S1 gate (you can't)
- Change thresholds without review

### For Maintainers

✅ **DO:**
- Monitor daily mainnet/testnet gates
- Review threshold modifications carefully
- Archive gate logs for compliance
- Update documentation when changing gates
- Run annual security audit of gates

❌ **DON'T:**
- Allow S1 override capability
- Decrease thresholds to hide issues
- Skip gate verification
- Modify gate logic without tests
- Ignore repeated gate failures

---

## References

- [Development Setup Guide](./DEVELOPMENT_SETUP.md)
- [GitHub Pages Dashboard](./GITHUB_PAGES_SETUP.md)
- [CI/CD Integration](./CI_CD_INTEGRATION.md)
- [ProofForge CLI](./PROOFFORGE_CLI.md)
- [GitHub Actions Docs](https://docs.github.com/en/actions)

---

**Last Updated:** 2026-05-02  
**Status:** ✅ GO FOR MAINNET RC-1  
**Report:** [launch-gates/reports/X3-MAINNET-GO-NO-GO-20260501-203300.md](../launch-gates/reports/X3-MAINNET-GO-NO-GO-20260501-203300.md)
