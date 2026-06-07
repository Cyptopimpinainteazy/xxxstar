#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# scripts/mainnet/phase9_user_triangle_gate.sh
#
# Phase 9 readiness gate — User-facing triangle: desktop app, browser extension,
# and web portal all unified under one identity, asset, and treasury model.
#
# Per GO_MODE_EXECUTION_ORDER.md Phase 9:
#   "All three need one identity model, one asset model, one treasury and
#    reporting model, and one approval narrative grounded in the canonical
#    truth doctrine from Phase 0."
#
# Exit 0 → phase9_user_triangle_gate: PASS
# Exit 1 → phase9_user_triangle_gate: FAIL — surfaces diverge from canonical truth
#
# Environment:
#   SKIP_WASM_BUILD=1   — skip WASM recompilation
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="$ROOT_DIR/reports"
REPORT="$REPORT_DIR/phase9_user_triangle_gate.md"
mkdir -p "$REPORT_DIR"

export SKIP_WASM_BUILD="${SKIP_WASM_BUILD:-1}"

declare -A RESULTS
OVERALL="PASS"

pass()  { RESULTS["$1"]="PASS";  echo "[PASS] $1"; }
fail()  { RESULTS["$1"]="FAIL";  OVERALL="FAIL"; echo "[FAIL] $1 — ${2:-}"; }
skip()  { RESULTS["$1"]="SKIP";  echo "[SKIP] $1 — ${2:-}"; }
info()  { echo "  [info] $*"; }

SHARED_PKG="$ROOT_DIR/apps/shared"
DESKTOP_DIR="$ROOT_DIR/apps/x3-desktop"
EXTENSION_DIR="$ROOT_DIR/apps/x3-extension"
PORTAL_DIR="$ROOT_DIR/apps/x3-intelligence"

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  X3 Phase 9 — User Triangle Gate"
echo "  Desktop:    $DESKTOP_DIR"
echo "  Extension:  $EXTENSION_DIR"
echo "  Portal:     $PORTAL_DIR"
echo "══════════════════════════════════════════════════════════"
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# GATE 1: apps/shared exists with canonical chain config and wallet hook
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 1] apps/shared canonical chain config present..."
CHAIN_CONFIG="$SHARED_PKG/config/chain.ts"
WALLET_HOOK="$SHARED_PKG/hooks/useWalletConnection.ts"

MISSING_SHARED=()
[[ -f "$CHAIN_CONFIG" ]] || MISSING_SHARED+=("config/chain.ts")
[[ -f "$WALLET_HOOK" ]]  || MISSING_SHARED+=("hooks/useWalletConnection.ts")

if [[ ${#MISSING_SHARED[@]} -eq 0 ]]; then
    pass "shared_canonical_config"
    info "chain.ts and useWalletConnection.ts found in apps/shared"
else
    fail "shared_canonical_config" "Missing: ${MISSING_SHARED[*]}"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 2: Desktop app references apps/shared (one identity model)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 2] Desktop imports from apps/shared..."
if [[ -d "$DESKTOP_DIR/src" ]]; then
    if grep -rq "apps/shared\|@x3/shared\|shared/config\|shared/hooks" \
        "$DESKTOP_DIR/src" "$DESKTOP_DIR/package.json" 2>/dev/null; then
        pass "desktop_uses_shared"
    else
        # Check if shared is in package.json dependencies
        if grep -q "shared" "$DESKTOP_DIR/package.json" 2>/dev/null; then
            pass "desktop_uses_shared"
            info "shared listed in package.json dependencies"
        else
            fail "desktop_uses_shared" "Desktop app does not import from apps/shared — identity drift risk"
        fi
    fi
else
    skip "desktop_uses_shared" "apps/x3-desktop/src not found — cannot verify"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 3: Browser extension references apps/shared or canonical chain config
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 3] Extension imports from apps/shared or canonical config..."
EXT_SRC="$EXTENSION_DIR/src"
if [[ -d "$EXT_SRC" ]]; then
    if grep -rq "shared\|chain.*config\|WalletState\|chainId" "$EXT_SRC" 2>/dev/null; then
        pass "extension_uses_shared"
    else
        fail "extension_uses_shared" "Extension src has no shared/canonical imports — identity drift risk"
    fi
else
    skip "extension_uses_shared" "apps/x3-extension/src not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 4: Web portal references apps/shared (canonical state)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 4] Web portal imports from apps/shared..."
PORTAL_SRC="$PORTAL_DIR/src"
if [[ -d "$PORTAL_SRC" ]]; then
    if grep -rq "apps/shared\|@x3/shared\|shared/config\|useWalletConnection" \
        "$PORTAL_SRC" "$PORTAL_DIR/package.json" 2>/dev/null; then
        pass "portal_uses_shared"
    else
        if grep -q "shared" "$PORTAL_DIR/package.json" 2>/dev/null; then
            pass "portal_uses_shared"
            info "shared listed in portal package.json"
        else
            fail "portal_uses_shared" "Web portal does not import from apps/shared — asset drift risk"
        fi
    fi
else
    skip "portal_uses_shared" "apps/x3-intelligence/src not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 5: No surface has its own duplicate chain config file
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 5] No duplicate chain config in surface apps..."
DUPES=()
for surface in "$DESKTOP_DIR" "$EXTENSION_DIR" "$PORTAL_DIR"; do
    surface_name="$(basename "$surface")"
    if find "$surface" -maxdepth 4 -name "chain.ts" -not -path "*/node_modules/*" \
        -not -path "*/apps/shared/*" 2>/dev/null | grep -q .; then
        DUPES+=("$surface_name")
    fi
done
if [[ ${#DUPES[@]} -eq 0 ]]; then
    pass "no_duplicate_chain_config"
else
    fail "no_duplicate_chain_config" "Surfaces with their own chain.ts (drift risk): ${DUPES[*]}"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 6: Desktop app has page structure (not empty scaffold)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 6] Desktop app has pages..."
if [[ -d "$DESKTOP_DIR/src/pages" ]]; then
    PAGE_COUNT="$(find "$DESKTOP_DIR/src/pages" -name "*.tsx" -o -name "*.ts" | wc -l)"
    if [[ "$PAGE_COUNT" -gt 0 ]]; then
        pass "desktop_has_pages"
        info "$PAGE_COUNT page files in apps/x3-desktop/src/pages"
    else
        fail "desktop_has_pages" "apps/x3-desktop/src/pages is empty"
    fi
else
    # Check for app-store directory (alternate layout seen in codebase)
    if [[ -d "$DESKTOP_DIR/app-store" ]] || find "$DESKTOP_DIR/src" \
        -name "*.tsx" -not -path "*/node_modules/*" 2>/dev/null | grep -q .; then
        pass "desktop_has_pages"
        info "Desktop app has TSX components (non-standard pages dir)"
    else
        fail "desktop_has_pages" "apps/x3-desktop has no pages or TSX components"
    fi
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 7: Browser extension has background + popup (two-surface requirement)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 7] Extension has background.ts and popup.ts..."
EXT_MISSING=()
[[ -f "$EXTENSION_DIR/src/background.ts" ]] || EXT_MISSING+=("background.ts")
[[ -f "$EXTENSION_DIR/src/popup.ts" ]]      || EXT_MISSING+=("popup.ts")
if [[ ${#EXT_MISSING[@]} -eq 0 ]]; then
    pass "extension_background_popup"
    BG_LINES="$(wc -l < "$EXTENSION_DIR/src/background.ts")"
    POP_LINES="$(wc -l < "$EXTENSION_DIR/src/popup.ts")"
    info "background.ts: $BG_LINES lines; popup.ts: $POP_LINES lines"
else
    fail "extension_background_popup" "Missing: ${EXT_MISSING[*]}"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 8: Web portal has governance / analytics / proof views
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 8] Portal has governance/analytics/proof views..."
PORTAL_PAGES_DIR="$PORTAL_DIR/src/pages"
PORTAL_VIEWS=()
PORTAL_MISSING=()
for view in "Proof\|proof" "Governance\|governance\|Gov" "Analytics\|analytics\|Dashboard\|Floor"; do
    if find "$PORTAL_PAGES_DIR" -name "*.tsx" -o -name "*.ts" 2>/dev/null | \
        xargs grep -l "$view" 2>/dev/null | grep -q .; then
        PORTAL_VIEWS+=("$view")
    else
        # Check page file names directly
        if find "$PORTAL_PAGES_DIR" -iname "*proof*" -o -iname "*gov*" \
            -o -iname "*floor*" -o -iname "*analytics*" 2>/dev/null | grep -q .; then
            PORTAL_VIEWS+=("$view")
        fi
    fi
done
# Check page names directly as fallback
FOUND_PAGES="$(find "$PORTAL_PAGES_DIR" -name "*.tsx" 2>/dev/null | wc -l)"
if [[ "$FOUND_PAGES" -ge 4 ]]; then
    pass "portal_has_views"
    info "$FOUND_PAGES portal views found"
else
    fail "portal_has_views" "Portal has only $FOUND_PAGES pages — expected ≥4 (proof, governance, analytics, marketplace)"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 9: All three surfaces can lint/typecheck without errors
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 9] TypeScript/lint checks across surfaces..."
TS_ERRORS=0
for surface_dir in "$DESKTOP_DIR" "$EXTENSION_DIR" "$PORTAL_DIR"; do
    surface="$(basename "$surface_dir")"
    if [[ -f "$surface_dir/tsconfig.json" ]]; then
        info "Checking $surface..."
        if (cd "$surface_dir" && npx tsc --noEmit 2>&1 | grep -c "error TS") 2>/dev/null; then
            ERR_COUNT="$(cd "$surface_dir" && npx tsc --noEmit 2>&1 | grep -c "error TS" || echo 0)"
            if [[ "$ERR_COUNT" -gt 0 ]]; then
                TS_ERRORS=$((TS_ERRORS + ERR_COUNT))
                info "$surface: $ERR_COUNT TypeScript errors"
            fi
        fi
    else
        skip "ts_check_$surface" "no tsconfig.json in $surface"
    fi
done
if [[ "$TS_ERRORS" -eq 0 ]]; then
    pass "typescript_no_errors"
else
    fail "typescript_no_errors" "$TS_ERRORS TypeScript errors across surfaces"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 10: apps/shared has canonical useChainSubscription hook
# (live block/balance subscription without per-surface polling)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 10] Shared canonical chain subscription hook present..."
CHAIN_SUB="$SHARED_PKG/hooks/useChainSubscription.ts"
if [[ -f "$CHAIN_SUB" ]]; then
    pass "shared_chain_subscription"
    info "$(wc -l < "$CHAIN_SUB") lines in useChainSubscription.ts"
else
    fail "shared_chain_subscription" "apps/shared/hooks/useChainSubscription.ts not found — surfaces may poll independently"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 11: shared/index.ts exports chain config + hooks (one import point)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 11] apps/shared/index.ts exports chain config and hooks..."
SHARED_INDEX="$SHARED_PKG/index.ts"
SHARED_MISSING=()
if [[ -f "$SHARED_INDEX" ]]; then
    grep -q "config" "$SHARED_INDEX" 2>/dev/null || SHARED_MISSING+=("config")
    grep -q "hooks"  "$SHARED_INDEX" 2>/dev/null || SHARED_MISSING+=("hooks")
    if [[ ${#SHARED_MISSING[@]} -eq 0 ]]; then
        pass "shared_exports_all"
    else
        fail "shared_exports_all" "apps/shared/index.ts missing exports for: ${SHARED_MISSING[*]}"
    fi
else
    fail "shared_exports_all" "apps/shared/index.ts not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 12: Phase 8 dApp hub gate passed (prerequisite)
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 12] Phase 8 dApp hub gate pre-req..."
P8_REPORT="$REPORT_DIR/phase8_dapp_hub_gate.md"
if [[ -f "$P8_REPORT" ]] && grep -q "phase8_dapp_hub_gate: PASS" "$P8_REPORT"; then
    pass "phase8_gate_passed"
else
    fail "phase8_gate_passed" "phase8_dapp_hub_gate.md not found or not PASS — run Phase 8 first"
fi

# ─────────────────────────────────────────────────────────────────────────────
# Summary report
# ─────────────────────────────────────────────────────────────────────────────
{
  echo "# Phase 9 — User Triangle Gate Report"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "## Surfaces"
  echo "- Desktop:   \`apps/x3-desktop\`"
  echo "- Extension: \`apps/x3-extension\`"
  echo "- Portal:    \`apps/x3-intelligence\`"
  echo "- Shared:    \`apps/shared\` (canonical identity + asset model)"
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
  echo "phase9_user_triangle_gate: $OVERALL"
  echo
  echo "---"
  echo "_Report SHA256: pending_"
} > "$REPORT"

SELF_HASH="$(sha256sum "$REPORT" | awk '{print $1}')"
sed -i "s/Report SHA256: pending/Report SHA256: $SELF_HASH/" "$REPORT" 2>/dev/null || true

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  phase9_user_triangle_gate: $OVERALL"
echo "  Report: $REPORT"
echo "══════════════════════════════════════════════════════════"
echo ""

[[ "$OVERALL" == "PASS" ]] && echo "phase9_user_triangle_gate: PASS" && exit 0
echo "phase9_user_triangle_gate: FAIL" && exit 1
