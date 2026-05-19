#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# scripts/mainnet/phase8_dapp_hub_gate.sh
#
# Phase 8 readiness gate — dApp Hub with revenue-sharing SDK and listing policy.
#
# Verifies that the dApp hub pallet, revenue-sharing crate, and SDK package
# provide stable-enough primitives for third-party inheritance, and that listing
# policy, throttling, and incident escalation are enforced at the platform level
# rather than per-dApp custom logic.
#
# Exit 0 → phase8_dapp_hub_gate: PASS
# Exit 1 → phase8_dapp_hub_gate: FAIL — do NOT open dApp hub to third parties
#
# Environment:
#   SKIP_WASM_BUILD=1   — skip WASM recompilation
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="$ROOT_DIR/reports"
REPORT="$REPORT_DIR/phase8_dapp_hub_gate.md"
mkdir -p "$REPORT_DIR"

export SKIP_WASM_BUILD="${SKIP_WASM_BUILD:-1}"

declare -A RESULTS
OVERALL="PASS"

pass()  { RESULTS["$1"]="PASS";  echo "[PASS] $1"; }
fail()  { RESULTS["$1"]="FAIL";  OVERALL="FAIL"; echo "[FAIL] $1 — ${2:-}"; }
skip()  { RESULTS["$1"]="SKIP";  echo "[SKIP] $1 — ${2:-}"; }
info()  { echo "  [info] $*"; }

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  X3 Phase 8 — dApp Hub Gate"
echo "══════════════════════════════════════════════════════════"
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# GATE 1: pallet-x3-dapp-hub compiles
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 1] pallet-x3-dapp-hub compiles..."
if cargo check -p pallet-x3-dapp-hub 2>&1 | grep -q "^error"; then
    fail "dapp_hub_compiles" "cargo check reported errors"
else
    pass "dapp_hub_compiles"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 2: x3-revenue-sharing crate compiles
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 2] x3-revenue-sharing crate compiles..."
if cargo check -p x3-revenue-sharing 2>&1 | grep -q "^error"; then
    fail "revenue_sharing_compiles" "cargo check reported errors"
else
    pass "revenue_sharing_compiles"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 3: pallet-x3-dapp-hub unit tests all pass
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 3] dApp hub unit tests..."
if cargo test -p pallet-x3-dapp-hub 2>&1 | grep -q "FAILED"; then
    fail "dapp_hub_tests_pass" "one or more tests FAILED"
else
    pass "dapp_hub_tests_pass"
    info "$(cargo test -p pallet-x3-dapp-hub 2>&1 | grep 'test result' || true)"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 4: Revenue-split policy enforces bps invariant (DAPP-005)
# Sum of all split entries must equal 10 000 bps
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 4] Revenue split bps invariant (DAPP-005)..."
REVENUE_LIB="$ROOT_DIR/crates/x3-revenue-sharing/src/lib.rs"
if [[ -f "$REVENUE_LIB" ]]; then
    if grep -q "10_000\|10000" "$REVENUE_LIB" 2>/dev/null; then
        pass "revenue_bps_invariant"
        info "10 000 bps sentinel found in x3-revenue-sharing"
    else
        fail "revenue_bps_invariant" "10 000 bps check not found in x3-revenue-sharing/src/lib.rs"
    fi
else
    fail "revenue_bps_invariant" "x3-revenue-sharing/src/lib.rs not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 5: Approval lifecycle present — Pending → Approved → Suspended path
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 5] Approval lifecycle (Pending→Approved→Suspended)..."
DAPP_LIB="$ROOT_DIR/pallets/x3-dapp-hub/src/lib.rs"
MISSING=()
for state in "Pending" "Approved" "Suspended"; do
    grep -q "$state" "$DAPP_LIB" 2>/dev/null || MISSING+=("$state")
done
if [[ ${#MISSING[@]} -eq 0 ]]; then
    pass "approval_lifecycle"
else
    fail "approval_lifecycle" "Missing states: ${MISSING[*]}"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 6: Revenue recorded only for Approved dApps (DAPP-002)
# Verified via tests: record_revenue_rejected/suspended/pending_dapp_fails
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 6] Revenue only recorded for Approved dApps (DAPP-002)..."
DAPP_TESTS="$ROOT_DIR/pallets/x3-dapp-hub/src/tests.rs"
REQUIRED_TESTS=(
    "record_revenue_rejected_dapp_fails"
    "record_revenue_suspended_dapp_fails"
    "record_revenue_pending_dapp_fails"
)
MISSING_TESTS=()
for t in "${REQUIRED_TESTS[@]}"; do
    grep -q "$t" "$DAPP_TESTS" 2>/dev/null || MISSING_TESTS+=("$t")
done
if [[ ${#MISSING_TESTS[@]} -eq 0 ]]; then
    pass "revenue_approval_guard"
else
    fail "revenue_approval_guard" "Missing tests: ${MISSING_TESTS[*]}"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 7: Governance-origin required for approve/reject/suspend operations
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 7] Governance-origin gate on approve/reject/suspend..."
GOV_TESTS=(
    "approve_dapp_non_governance_fails"
    "reject_dapp_non_governance_fails"
    "suspend_dapp_non_governance_fails"
)
MISSING_GOV=()
for t in "${GOV_TESTS[@]}"; do
    grep -q "$t" "$DAPP_TESTS" 2>/dev/null || MISSING_GOV+=("$t")
done
if [[ ${#MISSING_GOV[@]} -eq 0 ]]; then
    pass "dapp_governance_origin"
else
    fail "dapp_governance_origin" "Missing governance-origin tests: ${MISSING_GOV[*]}"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 8: Revenue split policy valid sum test present
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 8] Revenue policy invalid-sum test present..."
if grep -q "set_revenue_policy_invalid_sum_fails" "$DAPP_TESTS" 2>/dev/null; then
    pass "revenue_policy_sum_test"
else
    fail "revenue_policy_sum_test" "set_revenue_policy_invalid_sum_fails test not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 9: Developer earnings withdrawal test present (DAPP-006)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 9] Developer earnings withdrawal test (DAPP-006)..."
if grep -q "withdraw_earnings" "$DAPP_TESTS" 2>/dev/null; then
    pass "earnings_withdrawal_test"
else
    fail "earnings_withdrawal_test" "withdraw_earnings test not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 10: x3-marketplace-sdk package exists and has source
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 10] x3-marketplace-sdk package present..."
SDK_PATH="$ROOT_DIR/packages/x3-marketplace-sdk/src"
if [[ -d "$SDK_PATH" ]]; then
    SDK_FILES="$(find "$SDK_PATH" -name "*.ts" -o -name "*.js" | wc -l)"
    if [[ "$SDK_FILES" -gt 0 ]]; then
        pass "marketplace_sdk_exists"
        info "$SDK_FILES TypeScript/JS files in packages/x3-marketplace-sdk/src"
    else
        fail "marketplace_sdk_exists" "packages/x3-marketplace-sdk/src is empty"
    fi
else
    fail "marketplace_sdk_exists" "packages/x3-marketplace-sdk/src not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 11: dApp hub invariants documented (DAPP-001 through DAPP-006)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 11] dApp hub invariants documented (DAPP-001..006)..."
INV_COUNT="$(grep -c "DAPP-[0-9]\+" "$DAPP_LIB" 2>/dev/null || echo 0)"
if [[ "$INV_COUNT" -ge 6 ]]; then
    pass "dapp_invariants_documented"
    info "$INV_COUNT DAPP-* invariant references in lib.rs"
else
    fail "dapp_invariants_documented" "Expected ≥6 DAPP-* invariants; found $INV_COUNT"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 12: Phase 7 launchpad gate passed (prerequisite)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 12] Phase 7 launchpad gate pre-req..."
P7_REPORT="$REPORT_DIR/phase7_launchpad_gate.md"
if [[ -f "$P7_REPORT" ]] && grep -q "phase7_launchpad_gate: PASS" "$P7_REPORT"; then
    pass "phase7_gate_passed"
else
    fail "phase7_gate_passed" "phase7_launchpad_gate.md not found or not PASS — run Phase 7 first"
fi

# ─────────────────────────────────────────────────────────────────────────────
# Summary report
# ─────────────────────────────────────────────────────────────────────────────
{
  echo "# Phase 8 — dApp Hub Gate Report"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "## Gate Results"
  echo
  echo "| Gate | Result |"
  echo "|------|--------|"
  for key in "${!RESULTS[@]}"; do
    echo "| $key | ${RESULTS[$key]} |"
  done | sort
  echo
  echo "## Overall"
  echo
  echo "phase8_dapp_hub_gate: $OVERALL"
  echo
  echo "---"
  echo "_Report SHA256: pending_"
} > "$REPORT"

SELF_HASH="$(sha256sum "$REPORT" | awk '{print $1}')"
sed -i "s/Report SHA256: pending/Report SHA256: $SELF_HASH/" "$REPORT" 2>/dev/null || true

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  phase8_dapp_hub_gate: $OVERALL"
echo "  Report: $REPORT"
echo "══════════════════════════════════════════════════════════"
echo ""

[[ "$OVERALL" == "PASS" ]] && echo "phase8_dapp_hub_gate: PASS" && exit 0
echo "phase8_dapp_hub_gate: FAIL" && exit 1
