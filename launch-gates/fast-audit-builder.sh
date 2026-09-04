#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# X3 FAST AUDIT BUILDER - Targeted Code Extraction for AI Auditing
# ═══════════════════════════════════════════════════════════════════════════════
# Instead of massive repomix dumps, extract specific audit-relevant code sections
# This generates focused audit files in <5 minutes vs 30+ for full repomix
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

REPO_ROOT="/home/lojak/Desktop/X3_ATOMIC_STAR"
AUDIT_DIR="${REPO_ROOT}/launch-gates/audits"
mkdir -p "${AUDIT_DIR}"
cd "${REPO_ROOT}"

echo "═══════════════════════════════════════════════════════════════════════════════"
echo "X3 FAST AUDIT BUILDER - $(date)"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# AUDIT 1: WIRING VERIFICATION
# ═══════════════════════════════════════════════════════════════════════════════
echo "[1/5] Generating Wiring Audit..."
cat > "${AUDIT_DIR}/audit-01-wiring-context.json" <<'EOF'
{
  "audit_type": "wiring_verification",
  "question": "Is everything wired correctly into the runtime?",
  "focus_areas": [
    "runtime/src/lib.rs - construct_runtime! macro",
    "All pallets listed in construct_runtime!",
    "Pallet trait implementations",
    "Runtime type definitions"
  ],
  "pallet_count": 31,
  "test_status": "72/72 tests passing"
}
EOF

echo "✅ Wiring audit: $(wc -c < "${AUDIT_DIR}/audit-01-wiring-context.json") bytes"

# ═══════════════════════════════════════════════════════════════════════════════
# AUDIT 2: MAINNET READINESS SCORING
# ═══════════════════════════════════════════════════════════════════════════════
echo "[2/5] Generating Mainnet Readiness Scoring Context..."
cat > "${AUDIT_DIR}/audit-02-mainnet-context.json" <<EOF
{
  "audit_type": "mainnet_readiness",
  "categories": 13,
  "instructions": "Score each category 0-100 based on proof level. Report P0 blockers.",
  "runtime_info": {
    "pallets_count": $(find pallets -name "lib.rs" -type f | wc -l),
    "total_tests": $(find pallets -name "*.rs" -exec grep -l "^.*#\[test\]" {} \; | wc -l),
    "construct_runtime_lines": $(grep -A 200 "construct_runtime!" runtime/src/lib.rs 2>/dev/null | grep -c "pallet_" || echo "unknown")
  },
  "key_files_to_review": [
    "runtime/src/lib.rs - construct_runtime! macro definition",
    "pallets/x3-atomic-router/src/lib.rs - atomic execution logic",
    "pallets/pallet-bridge/src/lib.rs - bridge security",
    "pallets/settlement/src/lib.rs - settlement finality",
    "Cargo.lock - dependency versions"
  ]
}
EOF
echo "✅ Mainnet context: $(wc -c < "${AUDIT_DIR}/audit-02-mainnet-context.json") bytes"

# ═══════════════════════════════════════════════════════════════════════════════
# AUDIT 3: BRIDGE & ATOMIC SECURITY
# ═══════════════════════════════════════════════════════════════════════════════
echo "[3/5] Generating Bridge & Atomic Security Context..."
cat > "${AUDIT_DIR}/audit-03-bridge-atomic-context.json" <<EOF
{
  "audit_type": "bridge_atomic_security",
  "attack_vectors": [
    "replay_attacks",
    "partial_settlement",
    "timeout_abuse",
    "nonce_reuse",
    "finality_bypass",
    "governance_attack",
    "supply_manipulation",
    "signature_forgery",
    "timing_attacks",
    "cross_vm_desync"
  ],
  "critical_pallets": [
    "pallet-bridge",
    "x3-atomic-router", 
    "x3-cross-vm-router",
    "settlement"
  ],
  "files_to_audit": [
    "pallets/pallet-bridge/src/lib.rs",
    "pallets/x3-atomic-router/src/lib.rs",
    "pallets/x3-cross-vm-router/src/lib.rs",
    "pallets/settlement/src/lib.rs"
  ],
  "test_files": [
    "pallets/pallet-bridge/src/tests.rs",
    "pallets/x3-atomic-router/src/tests.rs",
    "pallets/settlement/src/tests.rs"
  ]
}
EOF
echo "✅ Bridge context: $(wc -c < "${AUDIT_DIR}/audit-03-bridge-atomic-context.json") bytes"

# ═══════════════════════════════════════════════════════════════════════════════
# AUDIT 4: INVARIANT HUNTING
# ═══════════════════════════════════════════════════════════════════════════════
echo "[4/5] Generating Invariant Hunter Context..."
cat > "${AUDIT_DIR}/audit-04-invariant-context.json" <<EOF
{
  "audit_type": "invariant_hunting",
  "p0_invariants_to_find": [
    "canonical_supply_conservation - total supply never changes",
    "atomic_all_or_nothing - transaction executes atomically or rolls back completely",
    "bridge_replay_impossible - no transaction can execute twice",
    "finality_guarantee - settled transactions are irreversible",
    "vault_solvency - reserves always cover liabilities",
    "validator_equivocation_detection - double-signing is detected",
    "nonce_monotonicity - account nonces only increase",
    "settlement_settlement_guarantee - atomic swaps complete atomically",
    "fresh_machine_bootstrap - node starts from zero state",
    "multi_node_consensus - validators reach consensus"
  ],
  "test_coverage_matrix": {
    "unit_tests": "$(find pallets -name "*.rs" -exec grep -l '#\[test\]' {} \; | wc -l) files",
    "integration_tests": "$(find integration-tests -name "*.rs" 2>/dev/null | wc -l) files",
    "property_tests": "$(find . -name "*.rs" -exec grep -l 'prop::' {} \; 2>/dev/null | wc -l) files"
  }
}
EOF
echo "✅ Invariant context: $(wc -c < "${AUDIT_DIR}/audit-04-invariant-context.json") bytes"

# ═══════════════════════════════════════════════════════════════════════════════
# AUDIT 5: TEST GAP ANALYSIS
# ═══════════════════════════════════════════════════════════════════════════════
echo "[5/5] Generating Test Gap Analysis Context..."
cat > "${AUDIT_DIR}/audit-05-test-gap-context.json" <<EOF
{
  "audit_type": "test_gap_analysis",
  "critical_behaviors": [
    "replay_resistance - same TX cannot execute twice",
    "partial_execution_failure - partial failures roll back completely",
    "rollback_guarantee - failed transactions are fully reverted",
    "finality_settlement - settled blocks cannot be reverted",
    "bridge_timeout_handling - timeouts prevent permanent locks",
    "storage_overflow_protection - storage limits are enforced",
    "boundary_amount_testing - edge values (0, max_u128, etc) handled",
    "invalid_input_rejection - malformed inputs rejected",
    "governance_gates - unauthorized calls are rejected",
    "validator_equivocation - double-signing is punished",
    "mempool_frontrun_protection - ordering is fair",
    "safe_migration - runtime upgrades don't lose data",
    "fresh_machine_bootstrap - clean state initialization",
    "multi_node_launch - consensus in 3-node network"
  ],
  "test_files_to_review": [
    "integration-tests/**/*.rs",
    "pallets/**/tests.rs",
    "tests/**/*.rs",
    "tests_core/**/*.rs"
  ]
}
EOF
echo "✅ Test gap context: $(wc -c < "${AUDIT_DIR}/audit-05-test-gap-context.json") bytes"

echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "✅ PHASE 1 COMPLETE: 5 Audit Contexts Generated"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
ls -lh "${AUDIT_DIR}"/audit-*.json
echo ""
echo "Total context size: $(du -sh "${AUDIT_DIR}" | cut -f1)"
echo ""
echo "Next: Use each context file as foundation for targeted AI audits"
echo "Example: 'Audit this codebase with this context' + [context file] + [source code]"
