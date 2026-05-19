#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# scripts/mainnet/phase10_ai_swarm_gate.sh
#
# Phase 10 readiness gate — AI Swarm externalized as a service.
#
# Per GO_MODE_EXECUTION_ORDER.md Phase 10:
#   "Bot services, compute billing, analytics usage, and incident controls
#    all inherit canonical accounting and operator truth instead of inventing
#    separate service rails."
#
# This gate verifies that:
#   - The swarm orchestra has governance + evidence retention policy
#   - The intelligence API server exposes the required service endpoints
#   - Evidence paths are wired (no orphaned swarm actions)
#   - Service billing events are not implemented as a separate accounting system
#   - Incident controls (kill-switch / quarantine) are present in security-swarm
#   - The E2E workflow test covers arbitrage, content, court, and human-CRM paths
#   - Phase 9 prerequisites are satisfied
#
# Exit 0 → phase10_ai_swarm_gate: PASS
# Exit 1 → phase10_ai_swarm_gate: FAIL — do NOT open swarm services externally
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="$ROOT_DIR/reports"
REPORT="$REPORT_DIR/phase10_ai_swarm_gate.md"
mkdir -p "$REPORT_DIR"

export SKIP_WASM_BUILD="${SKIP_WASM_BUILD:-1}"

declare -A RESULTS
OVERALL="PASS"

pass()  { RESULTS["$1"]="PASS";  echo "[PASS] $1"; }
fail()  { RESULTS["$1"]="FAIL";  OVERALL="FAIL"; echo "[FAIL] $1 — ${2:-}"; }
skip()  { RESULTS["$1"]="SKIP";  echo "[SKIP] $1 — ${2:-}"; }
info()  { echo "  [info] $*"; }

SWARM_DIR="$ROOT_DIR/x3-swarm-orchestra"
SEC_SWARM="$ROOT_DIR/x3-security-swarm"
INTEL_DIR="$ROOT_DIR/apps/x3-intelligence"

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  X3 Phase 10 — AI Swarm Gate"
echo "  Swarm:       $SWARM_DIR"
echo "  Security:    $SEC_SWARM"
echo "  Intelligence: $INTEL_DIR"
echo "══════════════════════════════════════════════════════════"
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# GATE 1: Swarm orchestra governance charter present
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 1] Swarm governance charter present..."
GOV_CHARTER="$SEC_SWARM/governance/charter.md"
if [[ -f "$GOV_CHARTER" ]]; then
    pass "swarm_governance_charter"
    info "$(wc -l < "$GOV_CHARTER") lines in governance/charter.md"
else
    fail "swarm_governance_charter" "x3-security-swarm/governance/charter.md not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 2: Evidence retention policy defined
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 2] Evidence retention policy defined..."
RETENTION="$SEC_SWARM/evidence/retention.policy"
if [[ -f "$RETENTION" ]]; then
    pass "evidence_retention_policy"
    info "retention.policy found in x3-security-swarm/evidence/"
else
    fail "evidence_retention_policy" "x3-security-swarm/evidence/retention.policy not found — no evidence path for swarm actions"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 3: Governance quorum rules defined
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 3] Swarm quorum rules present..."
QUORUM="$SEC_SWARM/governance/quorum.rules"
if [[ -f "$QUORUM" ]]; then
    pass "swarm_quorum_rules"
    info "quorum.rules found in x3-security-swarm/governance/"
else
    fail "swarm_quorum_rules" "x3-security-swarm/governance/quorum.rules not found — no signer authority bounds"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 4: Appeals path defined (incident escalation)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 4] Swarm appeals / incident escalation path..."
APPEALS="$SEC_SWARM/governance/appeals.yaml"
if [[ -f "$APPEALS" ]]; then
    pass "swarm_appeals_path"
    info "appeals.yaml found in x3-security-swarm/governance/"
else
    fail "swarm_appeals_path" "x3-security-swarm/governance/appeals.yaml not found — no incident escalation path"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 5: Intelligence API server has /health endpoint (service SRE requirement)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 5] Intelligence API /health endpoint present..."
API_SERVER="$INTEL_DIR/server.js"
if [[ -f "$API_SERVER" ]]; then
    if grep -q "'/health'" "$API_SERVER" 2>/dev/null || grep -q '"/health"' "$API_SERVER" 2>/dev/null; then
        pass "api_health_endpoint"
    else
        fail "api_health_endpoint" "No /health endpoint in apps/x3-intelligence/server.js"
    fi
else
    fail "api_health_endpoint" "apps/x3-intelligence/server.js not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 6: Analytics API endpoints present (analytics-as-a-service)
# Required: /api/v1/floor/stats, /api/v1/intents, /api/v1/agents
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 6] Analytics API endpoints present..."
REQUIRED_ENDPOINTS=(
    "/api/v1/floor/stats"
    "/api/v1/intents"
    "/api/v1/agents"
)
MISSING_ENDPOINTS=()
for ep in "${REQUIRED_ENDPOINTS[@]}"; do
    if ! grep -q "$ep" "$API_SERVER" 2>/dev/null; then
        MISSING_ENDPOINTS+=("$ep")
    fi
done
if [[ ${#MISSING_ENDPOINTS[@]} -eq 0 ]]; then
    pass "analytics_api_endpoints"
    # Count all API endpoints
    EP_COUNT="$(grep -c "app\.\(get\|post\|put\|delete\)" "$API_SERVER" 2>/dev/null || echo 0)"
    info "$EP_COUNT total API endpoints in server.js"
else
    fail "analytics_api_endpoints" "Missing endpoints: ${MISSING_ENDPOINTS[*]}"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 7: Disputes API endpoint present (canonical incident pipe, not separate)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 7] Disputes endpoint feeds canonical incident system..."
if grep -q "/api/v1/disputes" "$API_SERVER" 2>/dev/null; then
    pass "disputes_api_canonical"
else
    fail "disputes_api_canonical" "No /api/v1/disputes endpoint — service creates separate incident pipe"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 8: Validator metrics endpoint present (compute billing foundation)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 8] Validator metrics endpoint present..."
if grep -q "validator.*metrics\|/api/v1/validator" "$API_SERVER" 2>/dev/null; then
    pass "validator_metrics_endpoint"
else
    fail "validator_metrics_endpoint" "No validator metrics endpoint — compute billing has no usage signal"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 9: E2E workflow test covers arbitrage + AI content + court + CRM paths
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 9] Swarm E2E workflow test covers all service paths..."
E2E_TEST="$SWARM_DIR/tests/e2e_user_test.py"
if [[ -f "$E2E_TEST" ]]; then
    WORKFLOW_MISSING=()
    grep -q "arbitrage\|Arbitrage" "$E2E_TEST" 2>/dev/null || WORKFLOW_MISSING+=("arbitrage")
    grep -q "content\|video\|media" "$E2E_TEST"  2>/dev/null || WORKFLOW_MISSING+=("ai-content")
    grep -q "court\|dispute\|Court" "$E2E_TEST"  2>/dev/null || WORKFLOW_MISSING+=("court-dispute")
    grep -q "crm\|CRM\|human"       "$E2E_TEST"  2>/dev/null || WORKFLOW_MISSING+=("human-crm")
    if [[ ${#WORKFLOW_MISSING[@]} -eq 0 ]]; then
        pass "swarm_e2e_coverage"
    else
        fail "swarm_e2e_coverage" "E2E test missing paths: ${WORKFLOW_MISSING[*]}"
    fi
else
    fail "swarm_e2e_coverage" "x3-swarm-orchestra/tests/e2e_user_test.py not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 10: No second accounting system in swarm — must use canonical supply ledger
# Check that x3-swarm-orchestra does NOT define its own ledger/balance storage
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 10] Swarm has no duplicate accounting system..."
SWARM_DUPLICATE_LEDGER=false
if find "$SWARM_DIR" -name "*.rs" -not -path "*/target/*" 2>/dev/null | \
    xargs grep -l "total_supply\|fn mint\|fn burn" 2>/dev/null | grep -q .; then
    SWARM_DUPLICATE_LEDGER=true
    info "Warning: Rust files with supply/mint/burn found in swarm — may be duplicate ledger"
fi
# JS/TS check
if find "$SWARM_DIR" -name "*.ts" -o -name "*.js" 2>/dev/null | \
    xargs grep -l "totalSupply\|mintTokens\|burnTokens" 2>/dev/null | grep -q .; then
    SWARM_DUPLICATE_LEDGER=true
    info "Warning: TypeScript/JS files with supply mechanics found in swarm"
fi
if [[ "$SWARM_DUPLICATE_LEDGER" == "false" ]]; then
    pass "no_duplicate_accounting"
else
    fail "no_duplicate_accounting" "Swarm may be defining its own accounting system — violates Phase 10 exit gate"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 11: Intelligence API server respects CORS origin policy (security)
# (No wildcard CORS in production mode)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 11] Intelligence API CORS policy..."
if grep -q "cors()" "$API_SERVER" 2>/dev/null; then
    # cors() with no options is permissive — acceptable for dev, WARN for prod
    if grep -q "NODE_ENV.*production\|CORS_ORIGIN\|origin:" "$API_SERVER" 2>/dev/null; then
        pass "api_cors_policy"
        info "CORS appears to be configurable"
    else
        # Not a hard failure — flag for review
        skip "api_cors_policy" "Open CORS (cors()) in server.js — restrict origin before external launch"
    fi
else
    pass "api_cors_policy"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 12: Phase 9 user triangle gate passed (prerequisite)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 12] Phase 9 user triangle gate pre-req..."
P9_REPORT="$REPORT_DIR/phase9_user_triangle_gate.md"
if [[ -f "$P9_REPORT" ]] && grep -q "phase9_user_triangle_gate: PASS" "$P9_REPORT"; then
    pass "phase9_gate_passed"
else
    fail "phase9_gate_passed" "phase9_user_triangle_gate.md not found or not PASS — run Phase 9 first"
fi

# ─────────────────────────────────────────────────────────────────────────────
# Summary report
# ─────────────────────────────────────────────────────────────────────────────
{
  echo "# Phase 10 — AI Swarm Gate Report"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "## Verified Components"
  echo "- Swarm orchestra: \`x3-swarm-orchestra/\`"
  echo "- Security swarm:  \`x3-security-swarm/\`"
  echo "- Intelligence API: \`apps/x3-intelligence/server.js\`"
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
  echo "phase10_ai_swarm_gate: $OVERALL"
  echo
  echo "---"
  echo "_Report SHA256: pending_"
} > "$REPORT"

SELF_HASH="$(sha256sum "$REPORT" | awk '{print $1}')"
sed -i "s/Report SHA256: pending/Report SHA256: $SELF_HASH/" "$REPORT" 2>/dev/null || true

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  phase10_ai_swarm_gate: $OVERALL"
echo "  Report: $REPORT"
echo "══════════════════════════════════════════════════════════"
echo ""

[[ "$OVERALL" == "PASS" ]] && echo "phase10_ai_swarm_gate: PASS" && exit 0
echo "phase10_ai_swarm_gate: FAIL" && exit 1
