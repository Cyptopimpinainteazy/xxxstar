# X3 ProofGate - Execution Manual
## Proof-Based Mainnet Readiness System

**Philosophy:** A feature is not real until it is: **wired, tested, observable, recoverable, and reproducible from a clean machine.**

No percentage score without attached evidence. No "95% ready" lies. Every claim gets a proof tier attached.

---

## Quick Start (5 minutes)

```bash
cd /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates

# Step 1: Generate all proofs
./run-proof-commands.sh

# Step 2: Check evidence
ls -lh evidence/proof-*.log

# Step 3: Build repomix packs for AI audit
./build-repomix-pack.sh

# Step 4: Copy full pack to Claude, paste audit prompt
# (See "Audit Workflow" section below)
```

---

## Files Structure

```
launch-gates/
├── proofs.yaml                    # Feature proof requirements (THE LAW)
├── invariants.yaml                # Critical invariants that must be tested
├── run-proof-commands.sh           # Execute all 12 proof commands
├── build-repomix-pack.sh           # Generate Repomix packs for AI audit
├── repomix-mainnet-pack.sh         # Legacy pack builder (alternative)
├── evidence/                       # Output: All proof logs & hashes
│   ├── proof-01-cargo-check.log
│   ├── proof-02-cargo-test.log
│   ├── proof-03-clippy.log
│   ├── proof-04-fmt-check.log
│   ├── proof-05-hazard-scan.log
│   ├── proof-06-runtime-check.log
│   ├── proof-07-bridge-tests.log
│   ├── proof-08-atomic-tests.log
│   ├── proof-09-atlas-tests.log
│   ├── proof-10-finality-tests.log
│   ├── proof-11-chain-spec.log
│   ├── proof-12-hash.log
│   ├── evidence.sha256             # Reproducible hash of all proofs
│   └── proof-status.txt            # Any blockers found
├── repomix/                        # Output: AI-readable code packs
│   ├── x3-full-repo-TIMESTAMP.md
│   ├── x3-bridge-atomic-TIMESTAMP.md
│   ├── x3-runtime-consensus-TIMESTAMP.md
│   ├── x3-tests-TIMESTAMP.md
│   ├── x3-git-drift-TIMESTAMP.md
│   └── pack-manifest-TIMESTAMP.txt
├── reports/                        # Output: AI audit reports
│   ├── 01-wiring-audit.md
│   ├── 02-runtime-audit.md
│   ├── 03-bridge-safety.md
│   ├── 04-invariant-gaps.md
│   ├── 05-test-coverage.md
│   └── FINAL_GO_NO_GO.md
├── prompts/                        # Audit prompts for Claude
│   ├── 01-wiring-audit.md
│   ├── 02-mainnet-launch-gate.md
│   ├── 03-bridge-safety-audit.md
│   ├── 04-invariant-hunter.md
│   └── 05-test-gap-audit.md
└── genesis-checklist.md            # (pending) Genesis ceremony checklist
```

---

## Proof Tiers

Every feature scores based on strongest proof attached:

```
Level 10: code_exists           - Exists only in docs/comments
Level 25: code_created          - Code files present
Level 35: wired                 - Integrated into runtime/API
Level 45: compiles              - cargo check passes
Level 55: unit_tested           - cargo test passes
Level 70: integration_tested    - Integration tests pass
Level 85: fuzz_invariant        - Fuzz/chaos tests pass
Level 95: testnet_proven        - Multi-node testnet proves it
Level 100: audit_approved       - External audit or formal proof
```

**Rule:** A feature is not real until it is: wired + tested + observable + recoverable + reproducible.

---

## 12-Step Proof Execution

### Step 1: Workspace Compiles

```bash
cargo check --workspace
```

**Evidence file:** `evidence/proof-01-cargo-check.log`

**What it proves:** All code is syntactically valid and type-correct (without requiring full link)

**Blocker if fails:** YES - If it doesn't compile, nothing else matters

---

### Step 2: All Tests Pass

```bash
cargo test --workspace --lib
```

**Evidence file:** `evidence/proof-02-cargo-test.log`

**What it proves:** Unit tests pass; code behavior matches expected outcomes

**Blocker if fails:** YES - If tests fail, feature is broken

---

### Step 3: No Clippy Warnings

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

**Evidence file:** `evidence/proof-03-clippy.log`

**What it proves:** Code follows Rust idioms and best practices

**Blocker if fails:** NO - Can fix warnings after launch, but not preferred

---

### Step 4: Code is Formatted

```bash
cargo fmt --all -- --check
```

**Evidence file:** `evidence/proof-04-fmt-check.log`

**What it proves:** Code follows consistent style (maintainability signal)

**Blocker if fails:** NO - Can format before commit

---

### Step 5: Production Hazard Scan

```bash
rg -l "panic!|unwrap\(|expect\(" crates/x3-* pallets/x3-* runtime/ | grep -v test
```

**Evidence file:** `evidence/proof-05-hazard-scan.log`

**What it proves:** No production hazards that could crash validators

**Blocker if fails:** PARTIAL - Hazards found = lower score until fixed

---

### Step 6: Runtime Compiles

```bash
cargo check -p x3-runtime
```

**Evidence file:** `evidence/proof-06-runtime-check.log`

**What it proves:** Runtime specifically (the most critical component) compiles

**Blocker if fails:** YES - This is hardcoded in proofs.yaml as hard fail

---

### Step 7: Bridge Tests Pass

```bash
cargo test -p x3-bridge --lib
```

**Evidence file:** `evidence/proof-07-bridge-tests.log`

**What it proves:** Cross-VM bridge operations work correctly

**Blocker if fails:** YES - If bridge tests fail, mainnet launch fails

---

### Step 8: Atomic Execution Tests Pass

```bash
cargo test -p x3-atomic-trade --lib
```

**Evidence file:** `evidence/proof-08-atomic-tests.log`

**What it proves:** Atomic swaps with rollback/replay protection work

**Blocker if fails:** YES - Atomic execution is P0 critical

---

### Step 9: Atlas Kernel (Asset) Tests Pass

```bash
cargo test -p x3-atlas-kernel --lib
```

**Evidence file:** `evidence/proof-09-atlas-tests.log`

**What it proves:** Universal asset kernel maintains canonical supply invariant

**Blocker if fails:** YES - Asset supply conservation is P0 critical

---

### Step 10: Finality Oracle Tests Pass

```bash
cargo test -p x3-finality-oracle --lib
```

**Evidence file:** `evidence/proof-10-finality-tests.log`

**What it proves:** Sub-second finality consensus works

**Blocker if fails:** YES - Finality is core feature

---

### Step 11: Chain Spec Builds

```bash
cargo run --release -- build-spec --chain mainnet
```

**Evidence file:** `evidence/proof-11-chain-spec.log`

**What it proves:** Genesis configuration is valid and generates successfully

**Blocker if fails:** NO - Chain spec can be fixed if structure is wrong

---

### Step 12: Hash All Evidence

```bash
sha256sum evidence/proof-*.log > evidence/evidence.sha256
```

**Evidence file:** `evidence/evidence.sha256`

**What it proves:** Evidence is immutable and reproducible

**Blocker if fails:** NO - Always succeeds (just stores hashes)

---

## Critical Invariants (From invariants.yaml)

### P0 Invariants (Must test before mainnet)

1. **Canonical Supply Conservation**
   - Supply in = Supply out always
   - No phantom minting or burning
   - Test: `cargo test -p x3-atlas-kernel canonical_supply`

2. **Atomic All-or-Nothing**
   - Swap settles completely or rolls back completely
   - No partial settlements
   - Test: `cargo test -p x3-atomic-trade atomic.*all_or_nothing`

3. **Bridge Replay Impossible**
   - Message cannot execute twice
   - Each message has unique (chain_id, nonce, hash)
   - Test: `cargo test -p x3-bridge replay`

4. **Atomic Timeout Recovery**
   - Failed swaps auto-rollback after 1 hour
   - Collateral returns automatically
   - Test: `cargo test -p x3-atomic-trade timeout`

5. **No Production Hazards**
   - No panic/unwrap/expect in critical paths
   - All errors return Result<T, E>
   - Test: `rg panic!|unwrap\(|expect\(`

6. **Bridge State Consistency**
   - Lock/unlock queues match across VMs
   - No orphaned messages
   - Test: `cargo test -p x3-bridge state_consistency`

7. **Runtime Weights Exist**
   - Every critical extrinsic benchmarked
   - Weights from actual benchmarks, not guesses
   - Test: `ls -la runtime/src/weights/`

### P1 Invariants (Should test)

8. **DEX Reserve Invariant** - `r_a * r_b = k`
9. **Slashing Consistency** - Slashing deterministic across nodes

### P2 Invariants (Nice to have)

10. **Governance Voting Fairness** - 1 token = 1 vote

---

## Hard Fail Gates (9 total)

If ANY of these fail, overall status = **FAIL**:

1. ❌ Bridge replay protection test missing
2. ❌ Atomic rollback test missing
3. ❌ Canonical supply test missing
4. ❌ Runtime compile fails
5. ❌ Fresh machine cannot build node
6. ❌ Multi-node testnet fails
7. ❌ Production hazards found (panic/unwrap/expect)
8. ❌ Chain spec incomplete
9. ❌ Validator launch not reproducible

---

## Audit Workflow (Using AI Packs)

### Workflow Step 1: Generate Evidence

**Time:** 10-30 minutes depending on test speed

```bash
cd /home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates
./run-proof-commands.sh
```

**Output:** 12 proof logs in `evidence/`

**Check results:**
```bash
ls -lh evidence/proof-*.log
cat evidence/proof-status.txt 2>/dev/null  # Any blockers?
cat evidence/evidence.sha256  # Reproducibility hash
```

---

### Workflow Step 2: Generate Repomix Packs

**Time:** 5-10 minutes

```bash
./build-repomix-pack.sh
```

**Output:** 5 markdown packs in `repomix/`

**Check generation:**
```bash
ls -lh repomix/x3-*.md
cat repomix/pack-manifest-*.txt
```

---

### Workflow Step 3: Run Wiring Audit

**Time:** AI-dependent (typically 10-20 min)

**Steps:**
1. Copy `repomix/x3-full-repo-TIMESTAMP.md` (full repository pack)
2. Paste this prompt: [See prompts/01-wiring-audit.md]
3. Wait for Claude analysis
4. Save report to: `reports/01-wiring-audit.md`

**Output:** Wiring report showing:
- All modules wired into runtime
- Unwired modules (if any)
- Missing extrinsics
- Dead code

---

### Workflow Step 4: Run Bridge Safety Audit

**Time:** AI-dependent (typically 10-20 min)

**Steps:**
1. Copy `repomix/x3-bridge-atomic-TIMESTAMP.md` (critical path pack)
2. Paste this prompt: [See prompts/03-bridge-safety-audit.md]
3. Wait for Claude analysis
4. Save report to: `reports/03-bridge-safety.md`

**Output:** Bridge safety report showing:
- Replay protection implementation
- Timeout handling
- State consistency
- Lock/unlock logic

---

### Workflow Step 5: Run Invariant Hunter

**Time:** AI-dependent (typically 15-25 min)

**Steps:**
1. Copy `repomix/x3-full-repo-TIMESTAMP.md` (full repository pack)
2. Paste this prompt: [See prompts/04-invariant-hunter.md]
3. Wait for Claude analysis
4. Save report to: `reports/04-invariant-gaps.md`

**Output:** Invariant report showing:
- Which P0 invariants have tests
- Which are missing tests (blockers)
- Recommendations for additions

---

### Workflow Step 6: Generate Final Report

**Time:** Manual, 30 minutes

**Script:** (manual for now, automated version coming)

```bash
cat > reports/FINAL_GO_NO_GO.md << 'EOF'
# X3 Mainnet Go/No-Go Decision
## [DATE]

### Overall Score
[Calculate from all audits]

### Hard Fail Gates Status
- [x] Bridge replay protection: PASS
- [x] Atomic all-or-nothing: PASS
- [x] Canonical supply: PASS
- [x] Runtime compiles: PASS
- [x] Fresh machine: PENDING
- [x] Multi-node testnet: PENDING
- [x] No production hazards: PASS
- [x] Chain spec complete: PASS
- [x] Validator launch proven: PENDING

### P0 Blockers
[List any findings]

### Recommendation
[GO / NO-GO]

EOF
cat reports/FINAL_GO_NO_GO.md
```

---

## Score Calculation

### Feature Score
```
score = proof_tier_value
# Example: If atomic_rollback_proof has integration_tested proof:
# score = 70 (integration_tested tier value)
```

### Category Score
```
category_score = average(feature_scores_in_category) * category_weight
# Example: Atomic Execution (18% weight)
# atomic_execution_score = (75 + 85 + 75) / 3 * 0.18 = 26.5
```

### Overall Score
```
overall_score = sum(category_scores)
# Example:
# Runtime (12%): 85 * 0.12 = 10.2
# Consensus (12%): 80 * 0.12 = 9.6
# Asset Kernel (15%): 90 * 0.15 = 13.5
# Atomic (18%): 78 * 0.18 = 14.0
# Bridge (15%): 88 * 0.15 = 13.2
# DEX (8%): 75 * 0.08 = 6.0
# Governance (6%): 85 * 0.06 = 5.1
# Validator (6%): 70 * 0.06 = 4.2
# Observability (4%): 80 * 0.04 = 3.2
# Docs (4%): 70 * 0.04 = 2.8
# TOTAL: 81.8
```

### Penalty Rules
```
- P0 blocker older than 7 days: -5% global
- P0 blocker older than 14 days: RELEASE FREEZE (status = FAIL)
- Proof test failing: score = 0 for that feature
```

### Mainnet Ready Criteria (ALL must be true)
```
✅ overall_score >= 95
✅ all_categories >= 90
✅ p0_blockers == 0
✅ p1_blockers >= 3 (not too many)
✅ bridge_safety >= 95
✅ fresh_machine == PASS
✅ multi_node_testnet == PASS
✅ all_hard_gates == PASS
✅ testnet_stable >= 72 hours
✅ no_production_hazards == PASS
```

---

## Interpretation Examples

### Example 1: 95% Score is a LIE if...

```
Score: 95% ✅
BUT:
- Atomic rollback not tested ❌
- Bridge replay test missing ❌
- Supply invariant no test ❌

Verdict: FRAUD
Reality: 35% (code_exists tier only)
```

### Example 2: 78% Score is TRUSTWORTHY if...

```
Score: 78% ✅
Evidence:
- Runtime compiles ✅
- All tests pass ✅
- Bridge tests pass ✅
- Atomic tests pass (mostly) ⚠️
- Fresh machine works ✅
- Multi-node testnet works ✅
- ONE P1 blocker: DEX reserve test missing

Verdict: CREDIBLE
Reason: Every score point has evidence attached
```

---

## Token Budget Management

Each audit uses these tokens:

- **Wiring Audit:** ~8K tokens (full repo scan)
- **Bridge Safety:** ~6K tokens (critical path scan)
- **Invariant Hunter:** ~7K tokens (full repo scan)
- **Runtime Audit:** ~6K tokens (runtime + tests)
- **Test Gap Audit:** ~5K tokens (all tests scan)

**Total budget for full audit:** ~32K tokens (very reasonable)

---

## Next Steps

1. ✅ **Run proofs:** `./run-proof-commands.sh` (generates 12 proof logs)
2. ✅ **Build packs:** `./build-repomix-pack.sh` (generates 5 markdown files)
3. ⏳ **Run wiring audit:** Copy full pack → paste prompt 01 → save report
4. ⏳ **Run bridge audit:** Copy bridge pack → paste prompt 03 → save report
5. ⏳ **Run invariant audit:** Copy full pack → paste prompt 04 → save report
6. ⏳ **Run test gap audit:** Copy tests pack → paste prompt 05 → save report
7. ⏳ **Generate GO/NO-GO:** Consolidate all reports → final decision

---

## Philosophy

**"A feature is not real until it is: wired, tested, observable, recoverable, and reproducible from a clean machine."**

Every claim in this document must have evidence. Every evidence file is reproducible. Every result is verifiable.

This is how mainnet gets built: not with hope, but with proof.

---

## Questions?

See: `proofs.yaml` - The law of what scores what

See: `invariants.yaml` - The critical properties that must hold

See: `launch-gates/reports/` - Actual audit findings with evidence

---

**Last Updated:** 2026-04-24
**ProofGate Version:** 1.0
**Status:** Ready for execution
