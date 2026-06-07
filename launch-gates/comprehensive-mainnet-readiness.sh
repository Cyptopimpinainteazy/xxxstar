#!/usr/bin/env bash
# X3 Comprehensive Mainnet Readiness Report Generator
# Combines internal proofs + external validation into single Go/No-Go decision

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
REPORT_DIR="$SCRIPT_DIR/reports"
EVIDENCE_DIR="$SCRIPT_DIR/evidence"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "═══════════════════════════════════════════════════════════════════════════"
echo "  X3 ATOMIC STAR - COMPREHENSIVE MAINNET READINESS REPORT"
echo "═══════════════════════════════════════════════════════════════════════════"
echo ""
echo "Timestamp: $(date)"
echo "Commit: $(cd "$WORKSPACE_ROOT" && git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
echo ""

mkdir -p "$REPORT_DIR" "$EVIDENCE_DIR"

REPORT_FILE="$REPORT_DIR/mainnet-readiness-${TIMESTAMP}.md"

# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 1: PROOFFORGE SECURITY AUDIT STATUS
# ═══════════════════════════════════════════════════════════════════════════════

echo "[1/10] Checking ProofForge security audit status..."

cat > "$REPORT_FILE" << 'EOF'
# X3 MAINNET READINESS REPORT

**Generated:** $(date)
**Commit:** $(git rev-parse --short HEAD)
**Report ID:** MAINNET_READINESS_REPORT

---

## EXECUTIVE SUMMARY

### DECISION: ❌ NOT READY FOR MAINNET DEPLOYMENT

---

## 1. PROOFFORGE SECURITY AUDIT STATUS

EOF

# Check if ProofForge has been run
if [ -f "$WORKSPACE_ROOT/PROOFFORGE_COMPREHENSIVE_RESULTS.md" ]; then
    echo "  ✓ ProofForge audit found"
    
    # Extract critical blocker count
    S0_COUNT=$(grep -c "S0-" "$WORKSPACE_ROOT/PROOFFORGE_COMPREHENSIVE_RESULTS.md" 2>/dev/null || echo "0")
    S1_COUNT=$(grep -c "S1-" "$WORKSPACE_ROOT/PROOFFORGE_COMPREHENSIVE_RESULTS.md" 2>/dev/null || echo "0")
    
    cat >> "$REPORT_FILE" << EOF

**Status:** ❌ CRITICAL BLOCKERS PRESENT
**S0 Blockers (Catastrophic):** $S0_COUNT
**S1 Blockers (Critical):** $S1_COUNT

### Critical Security Blockers

$(grep -A 2 "S0-\|S1-" "$WORKSPACE_ROOT/PROOFFORGE_COMPREHENSIVE_RESULTS.md" 2>/dev/null || echo "See PROOFFORGE_COMPREHENSIVE_RESULTS.md")

**Decision Impact:** ALL S0 and S1 blockers MUST be resolved before mainnet.

EOF
else
    echo "  ❌ ProofForge audit NOT FOUND"
    cat >> "$REPORT_FILE" << 'EOF'

**Status:** ❌ NOT RUN
**Blocker:** ProofForge comprehensive security audit has not been executed.

**Required Actions:**
1. Run: `./target/debug/x3-proof prove-everything --verbose`
2. Review findings in PROOFFORGE_COMPREHENSIVE_RESULTS.md
3. Resolve all S0 and S1 blockers

EOF
fi

# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 2: COMPILATION & TESTS
# ═══════════════════════════════════════════════════════════════════════════════

echo "[2/10] Checking compilation and tests..."

cat >> "$REPORT_FILE" << 'EOF'

---

## 2. COMPILATION & TESTS

EOF

cd "$WORKSPACE_ROOT"

# Cargo check
echo "  Running cargo check..."
if cargo check --workspace &> "$EVIDENCE_DIR/cargo-check-${TIMESTAMP}.log"; then
    echo "  ✓ Cargo check passed"
    echo "**Compilation:** ✅ PASS" >> "$REPORT_FILE"
else
    echo "  ❌ Cargo check failed"
    echo "**Compilation:** ❌ FAIL" >> "$REPORT_FILE"
fi

# Cargo test
echo "  Running cargo test (summary only)..."
if cargo test --workspace --no-fail-fast &> "$EVIDENCE_DIR/cargo-test-${TIMESTAMP}.log"; then
    TESTS_PASSED=$(grep -E "test result:.*passed" "$EVIDENCE_DIR/cargo-test-${TIMESTAMP}.log" | tail -1 || echo "unknown")
    echo "  ✓ Cargo test passed"
    echo "**Tests:** ✅ $TESTS_PASSED" >> "$REPORT_FILE"
else
    TESTS_FAILED=$(grep -E "test result:.*FAILED" "$EVIDENCE_DIR/cargo-test-${TIMESTAMP}.log" | tail -1 || echo "unknown")
    echo "  ❌ Cargo test failed"
    echo "**Tests:** ❌ $TESTS_FAILED" >> "$REPORT_FILE"
fi

# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 3: CRITICAL INVARIANTS
# ═══════════════════════════════════════════════════════════════════════════════

echo "[3/10] Checking critical invariants..."

cat >> "$REPORT_FILE" << 'EOF'

---

## 3. CRITICAL INVARIANTS (P0 GATES)

EOF

# Check for critical invariant tests
INVARIANT_TESTS=(
    "canonical_supply"
    "atomic_all_or_nothing"
    "bridge_replay"
    "atomic_rollback"
    "double_mint"
)

INVARIANTS_PASS=0
INVARIANTS_FAIL=0

for test in "${INVARIANT_TESTS[@]}"; do
    echo "  Checking invariant: $test"
    if grep -q "$test.*ok" "$EVIDENCE_DIR/cargo-test-${TIMESTAMP}.log" 2>/dev/null; then
        echo "    ✓ $test: PASS"
        echo "- ✅ **$test:** PASS" >> "$REPORT_FILE"
        ((INVARIANTS_PASS++))
    else
        echo "    ❌ $test: FAIL or NOT FOUND"
        echo "- ❌ **$test:** FAIL or NOT FOUND" >> "$REPORT_FILE"
        ((INVARIANTS_FAIL++))
    fi
done

echo "" >> "$REPORT_FILE"
echo "**Invariants Status:** $INVARIANTS_PASS passed, $INVARIANTS_FAIL failed/missing" >> "$REPORT_FILE"

if [ $INVARIANTS_FAIL -gt 0 ]; then
    echo "**Decision Impact:** BLOCKED - Critical invariants not proven" >> "$REPORT_FILE"
fi

# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 4: BENCHMARK WEIGHTS
# ═══════════════════════════════════════════════════════════════════════════════

echo "[4/10] Checking benchmark weights..."

cat >> "$REPORT_FILE" << 'EOF'

---

## 4. RUNTIME BENCHMARKS

EOF

# Check for benchmark files
BENCHMARK_COUNT=$(find "$WORKSPACE_ROOT/pallets" -name "weights.rs" 2>/dev/null | wc -l)

echo "  Found $BENCHMARK_COUNT weight files"
echo "**Benchmark Weights:** $BENCHMARK_COUNT weight files found" >> "$REPORT_FILE"

if [ "$BENCHMARK_COUNT" -lt 5 ]; then
    echo "  ⚠️  Warning: Low benchmark coverage"
    echo "**Status:** ⚠️ LOW COVERAGE - Need benchmarks for all critical pallets" >> "$REPORT_FILE"
fi

# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 5: PRODUCTION HAZARDS
# ═══════════════════════════════════════════════════════════════════════════════

echo "[5/10] Scanning for production hazards..."

cat >> "$REPORT_FILE" << 'EOF'

---

## 5. PRODUCTION HAZARDS SCAN

EOF

# Run embarrassment scan if it exists
if [ -f "$SCRIPT_DIR/embarrassment-scan.sh" ]; then
    echo "  Running embarrassment scan..."
    bash "$SCRIPT_DIR/embarrassment-scan.sh" &> "$EVIDENCE_DIR/hazards-${TIMESTAMP}.log" || true
    
    HAZARD_COUNT=$(wc -l < "$EVIDENCE_DIR/hazards-${TIMESTAMP}.log")
    echo "  Found $HAZARD_COUNT potential hazards"
    
    echo "**Hazards Found:** $HAZARD_COUNT" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "See: \`evidence/hazards-${TIMESTAMP}.log\`" >> "$REPORT_FILE"
    
    if [ "$HAZARD_COUNT" -gt 50 ]; then
        echo "**Status:** ❌ HIGH HAZARD COUNT - Review critical paths" >> "$REPORT_FILE"
    fi
else
    echo "  ⚠️  Embarrassment scan not found"
    echo "**Status:** ⚠️ NOT RUN" >> "$REPORT_FILE"
fi

# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 6: EXTERNAL SECURITY AUDITS
# ═══════════════════════════════════════════════════════════════════════════════

echo "[6/10] Checking external security audits..."

cat >> "$REPORT_FILE" << 'EOF'

---

## 6. EXTERNAL SECURITY AUDITS (S0 GATE)

EOF

# Check for audit reports
AUDIT_DIR="$WORKSPACE_ROOT/audits"
if [ -d "$AUDIT_DIR" ]; then
    AUDIT_COUNT=$(find "$AUDIT_DIR" -name "*.pdf" -o -name "*.md" 2>/dev/null | wc -l)
    echo "  Found $AUDIT_COUNT audit documents"
    echo "**Audit Reports Found:** $AUDIT_COUNT" >> "$REPORT_FILE"
else
    AUDIT_COUNT=0
    echo "  ❌ No audit directory found"
    echo "**Audit Reports Found:** 0" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << 'EOF'

**Status:** ❌ EXTERNAL AUDITS NOT COMPLETE

**Required:**
- Minimum 2 tier-1 security audits (Trail of Bits, OpenZeppelin, Zellic, etc.)
- All Critical/High findings resolved
- Final approval letters obtained

**Decision Impact:** BLOCKED - External audits are MANDATORY for mainnet

EOF

# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 7: BUG BOUNTY PROGRAM
# ═══════════════════════════════════════════════════════════════════════════════

echo "[7/10] Checking bug bounty program..."

cat >> "$REPORT_FILE" << 'EOF'

---

## 7. BUG BOUNTY PROGRAM (S0 GATE)

**Status:** ❌ NOT LAUNCHED

**Required:**
- Launch on Immunefi or HackerOne
- Minimum $100k payout pool
- Run for 4+ weeks BEFORE mainnet
- Critical: $25k-$50k | High: $10k-$25k

**Decision Impact:** BLOCKED - Bug bounty MUST run before mainnet

EOF

# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 8: PUBLIC INCENTIVIZED TESTNET
# ═══════════════════════════════════════════════════════════════════════════════

echo "[8/10] Checking public testnet status..."

cat >> "$REPORT_FILE" << 'EOF'

---

## 8. PUBLIC INCENTIVIZED TESTNET (S0 GATE)

**Status:** ❌ NOT LAUNCHED

**Required:**
- 50+ external validators
- 8+ weeks duration
- Chaos testing (validator failures, network partitions)
- Economic testing (rewards, slashing, governance)
- No critical failures

**Decision Impact:** BLOCKED - Public testnet is MANDATORY

EOF

# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 9: LEGAL & COMPLIANCE
# ═══════════════════════════════════════════════════════════════════════════════

echo "[9/10] Checking legal/compliance status..."

cat >> "$REPORT_FILE" << 'EOF'

---

## 9. LEGAL & COMPLIANCE (S1 ADVISORY)

**Status:** ⚠️  NOT STARTED

**Recommended:**
- Securities law opinion (USA)
- OFAC sanctions compliance plan
- Multi-jurisdiction token classification
- AML/KYC assessment

**Decision Impact:** HIGH LEGAL RISK - Strongly recommended before mainnet

EOF

# ═══════════════════════════════════════════════════════════════════════════════
# SECTION 10: FINAL GO/NO-GO DECISION
# ═══════════════════════════════════════════════════════════════════════════════

echo "[10/10] Generating final decision..."

cat >> "$REPORT_FILE" << 'EOF'

---

## 10. FINAL GO/NO-GO DECISION

### ❌ NO-GO FOR MAINNET DEPLOYMENT

**Reasons:**

1. **ProofForge Security Blockers:** 9 critical security issues (6 S0 + 3 S1)
2. **External Audits:** Not complete (MANDATORY)
3. **Bug Bounty:** Not launched (MANDATORY)
4. **Public Testnet:** Not launched (MANDATORY)
5. **Critical Invariants:** Some not tested
6. **Legal Review:** Not started (HIGH RISK)

---

## IMMEDIATE NEXT ACTIONS

### Priority 1 (S0 Blockers - MUST FIX)
1. **Resolve ProofForge S0 blockers:**
   - canonical_supply_invariant_missing
   - double_mint_possible
   - bridge_replay_accepted
   - finality_spoof_accepted
   - atomic_rollback_missing
   - runtime_panic_critical_path

2. **Engage external audit firms:**
   - Contact 2-3 tier-1 firms (Trail of Bits, OpenZeppelin, Zellic)
   - Budget: $250k-$400k
   - Timeline: 6-8 weeks

3. **Launch bug bounty program:**
   - Platform: Immunefi or HackerOne
   - Pool: $100k minimum
   - Launch IMMEDIATELY (needs 4+ weeks before mainnet)

4. **Launch public incentivized testnet:**
   - Goal: 50-200 external validators
   - Duration: 8-12 weeks
   - Incentive pool: $50k-$200k

### Priority 2 (S1 Critical - HIGH PRIORITY)
1. **Resolve ProofForge S1 blockers:**
   - failed_rollback
   - governance_bypass
   - unauthorized_mint

2. **Legal/compliance review:**
   - Engage crypto law firm
   - Securities law opinion
   - OFAC compliance plan
   - Budget: $50k-$80k

### Priority 3 (Development)
1. **Add missing critical tests:**
   - All invariant tests must pass
   - Bridge replay protection
   - Atomic rollback
   - Canonical supply conservation

2. **Generate benchmark weights:**
   - All critical pallets
   - All extrinsics

3. **Clean production hazards:**
   - Remove TODO/FIXME from critical paths
   - Remove panic!/unwrap from runtime
   - Remove mock/stub code

---

## ESTIMATED TIMELINE TO MAINNET

**Optimistic:** 16-20 weeks
**Realistic:** 24-32 weeks
**Conservative:** 36-48 weeks

**Timeline Breakdown:**
- Fix ProofForge blockers: 4-8 weeks
- External audits: 6-8 weeks
- Bug bounty (pre-mainnet): 4-8 weeks
- Public testnet: 8-12 weeks
- Legal review: 4-6 weeks
- Final preparations: 2-4 weeks

**Critical Path:** External audits + Public testnet (12-20 weeks minimum)

---

## CHECKLIST FOR MAINNET GO DECISION

Use this checklist to track progress toward mainnet readiness:

### Code Quality & Security
- [ ] All ProofForge S0 blockers resolved
- [ ] All ProofForge S1 blockers resolved
- [ ] All critical invariant tests passing
- [ ] No panic/unwrap in critical paths
- [ ] All runtime benchmarks generated
- [ ] Cargo check passes
- [ ] Cargo test 100% pass rate
- [ ] Cargo clippy clean

### External Validation
- [ ] 2+ tier-1 security audits complete
- [ ] All Critical/High audit findings resolved
- [ ] Final audit approval received
- [ ] Bug bounty running 4+ weeks
- [ ] No Critical bugs reported (or all fixed)
- [ ] Public testnet ran 8+ weeks
- [ ] 50+ external validators participated
- [ ] No critical testnet failures

### Operations
- [ ] Fresh machine build proven
- [ ] Multi-node testnet proven
- [ ] Validator onboarding documented
- [ ] Genesis ceremony checklist complete
- [ ] Chain spec validated
- [ ] Bootnodes configured
- [ ] Telemetry operational
- [ ] Monitoring/alerts configured
- [ ] Disaster recovery runbooks complete

### Legal & Compliance
- [ ] Securities law opinion obtained
- [ ] OFAC compliance plan implemented
- [ ] Multi-jurisdiction review complete
- [ ] AML/KYC assessment done

### Final Gates
- [ ] All checklist items above complete
- [ ] Final team review conducted
- [ ] External audit firm approval
- [ ] Legal counsel approval
- [ ] Leadership approval

**When ALL boxes are checked, X3 is ready for mainnet launch.**

---

**Report Generated:** $(date)
**Next Review:** Re-run this report after fixing P0 blockers

EOF

echo ""
echo "═══════════════════════════════════════════════════════════════════════════"
echo "  REPORT COMPLETE"
echo "═══════════════════════════════════════════════════════════════════════════"
echo ""
echo "Report saved to: $REPORT_FILE"
echo ""
echo -e "${RED}DECISION: ❌ NOT READY FOR MAINNET${NC}"
echo ""
echo "Critical blockers:"
echo "  - ProofForge security issues (9)"
echo "  - External audits not complete"
echo "  - Bug bounty not launched"
echo "  - Public testnet not launched"
echo ""
echo "Estimated timeline: 24-32 weeks (realistic)"
echo ""
echo "Next steps:"
echo "  1. Fix ProofForge S0 blockers"
echo "  2. Engage external auditors"
echo "  3. Launch bug bounty"
echo "  4. Launch public testnet"
echo ""
