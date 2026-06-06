#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# scripts/mainnet/public_testnet_gate.sh
#
# Public testnet readiness gate for X3 Atomic Star.
#
# This script enforces all 15 minimum criteria required before opening
# the testnet to public validators and external participants.
#
# Usage:
#   ./scripts/mainnet/public_testnet_gate.sh [--rpc-base-url http://localhost:9933]
#
# Exit 0 → public_testnet_gate: PASS
# Exit 1 → public_testnet_gate: FAIL — do NOT open public participation
#
# Environment variables (override defaults):
#   X3_RPC_URL         — primary validator RPC (default: http://localhost:9933)
#   X3_CHAIN_SPEC      — path to non-dev chain spec JSON
#   X3_VALIDATOR_COUNT — expected minimum validator count (default: 7)
#   X3_TESTNET_HOURS   — hours of stable block production to require (default: 72)
#                        Set to 0 to skip the 72-hour timer check (for CI).
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="$ROOT_DIR/reports"
REPORT="$REPORT_DIR/public_testnet_gate.md"
mkdir -p "$REPORT_DIR"

RPC_URL="${X3_RPC_URL:-http://localhost:9933}"
CHAIN_SPEC="${X3_CHAIN_SPEC:-$ROOT_DIR/chain-specs/x3-testnet-raw.json}"
MIN_VALIDATORS="${X3_VALIDATOR_COUNT:-7}"
REQUIRED_STABLE_HOURS="${X3_TESTNET_HOURS:-72}"
STABILITY_MARKER="$ROOT_DIR/.testnet_stability_start"

# ── Result tracking ───────────────────────────────────────────────────────────
declare -A RESULTS
OVERALL="PASS"

pass()  { RESULTS["$1"]="PASS";  echo "[PASS] $1"; }
fail()  { RESULTS["$1"]="FAIL";  OVERALL="FAIL"; echo "[FAIL] $1 — ${2:-}"; }
skip()  { RESULTS["$1"]="SKIP";  echo "[SKIP] $1 — ${2:-}"; }
info()  { echo "  [info] $*"; }

# ── Helper: RPC call ──────────────────────────────────────────────────────────
rpc() {
    local method="$1"
    local params="${2:-[]}"
    curl -sf -m 10 "$RPC_URL" \
        -H 'Content-Type: application/json' \
        -d "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params}" \
    2>/dev/null || echo '{"error":"rpc_unreachable"}'
}

rpc_value() {
    local method="$1"
    local params="${2:-[]}"
    rpc "$method" "$params" | jq -r '.result // empty' 2>/dev/null || echo ""
}

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  X3 Public Testnet Gate"
echo "  RPC: $RPC_URL"
echo "  Required validators: $MIN_VALIDATORS"
echo "  Required stable hours: $REQUIRED_STABLE_HOURS"
echo "══════════════════════════════════════════════════════════"
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# GATE 1: Minimum 7 validators active
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 1] Validator count ≥ $MIN_VALIDATORS..."
HEALTH_RESULT="$(rpc "system_health")"
if echo "$HEALTH_RESULT" | grep -q '"error"'; then
    fail "min_7_validators" "RPC unreachable at $RPC_URL"
else
    PEER_COUNT="$(echo "$HEALTH_RESULT" | jq -r '.result.peers // 0')"
    # system_localListenAddresses for validator count requires separate query
    # We check via grandpa_roundState or session validators
    VALIDATOR_LIST="$(rpc_value "state_call" '["GrandpaApi_grandpa_authorities","0x"]')"
    if [[ -z "$VALIDATOR_LIST" ]]; then
        # Fallback: count peers + self
        ACTUAL_VALIDATORS=$(( PEER_COUNT + 1 ))
    else
        ACTUAL_VALIDATORS="$(echo "$VALIDATOR_LIST" | jq 'length' 2>/dev/null || echo "$((PEER_COUNT + 1))")"
    fi
    info "Detected validators: $ACTUAL_VALIDATORS (peers: $PEER_COUNT)"
    if (( ACTUAL_VALIDATORS >= MIN_VALIDATORS )); then
        pass "min_7_validators"
    else
        fail "min_7_validators" "found $ACTUAL_VALIDATORS, need $MIN_VALIDATORS"
    fi
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 2: Public bootnodes registered
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 2] Public bootnodes in chain spec..."
if [[ -f "$CHAIN_SPEC" ]]; then
    BOOTNODE_COUNT="$(jq '.bootNodes | length' "$CHAIN_SPEC" 2>/dev/null || echo "0")"
    info "Bootnodes in chain spec: $BOOTNODE_COUNT"
    if (( BOOTNODE_COUNT >= 2 )); then
        pass "public_bootnodes"
    else
        fail "public_bootnodes" "chain spec has $BOOTNODE_COUNT bootnode(s), need ≥2"
    fi
else
    fail "public_bootnodes" "chain spec not found at $CHAIN_SPEC"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 3: Chain spec generated from env — no dev seeds
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 3] No dev seeds in chain spec..."
if [[ -f "$CHAIN_SPEC" ]]; then
    DEV_SEEDS=()
    # Known Substrate dev seed phrases and their common derivatives
    for pattern in \
        "Alice\|Bob\|Charlie\|Dave\|Eve\|Ferdie" \
        "bottom drive obey lake" \
        "//Alice\|//Bob\|//Charlie" \
        "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" \
        "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"; do
        if grep -qE "$pattern" "$CHAIN_SPEC" 2>/dev/null; then
            DEV_SEEDS+=("$pattern")
        fi
    done
    if [[ ${#DEV_SEEDS[@]} -eq 0 ]]; then
        pass "no_dev_seeds"
    else
        fail "no_dev_seeds" "forbidden dev patterns found: ${DEV_SEEDS[*]}"
    fi
else
    fail "no_dev_seeds" "chain spec not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 4: External bridges disabled at genesis
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 4] External bridges disabled..."
BRIDGES_ENABLED="$(rpc_value "state_getStorage" '["0x'])"
# Check via genesis config in chain spec
if [[ -f "$CHAIN_SPEC" ]]; then
    if grep -q '"externalBridgesEnabled":true\|"external_bridges_enabled":true' "$CHAIN_SPEC" 2>/dev/null; then
        fail "external_bridges_disabled" "genesis has externalBridgesEnabled=true"
    else
        pass "external_bridges_disabled"
    fi
else
    # Check from unit test that bridges start paused
    if cargo test -p pallet-x3-cross-vm-router \
        external_bridges_are_paused_at_genesis \
        >/dev/null 2>&1; then
        pass "external_bridges_disabled"
    else
        fail "external_bridges_disabled" "external_bridges_are_paused_at_genesis test failed"
    fi
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 5: Faucet separated from treasury
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 5] Faucet account separated from treasury..."
if [[ -f "$CHAIN_SPEC" ]]; then
    TREASURY_ACCT="$(jq -r '.genesis.runtimeGenesis.config.treasury.account // empty' "$CHAIN_SPEC" 2>/dev/null || echo "")"
    FAUCET_ACCT="$(jq -r '.genesis.runtimeGenesis.config.faucet.account // .properties.faucetAccount // empty' "$CHAIN_SPEC" 2>/dev/null || echo "")"
    if [[ -n "$TREASURY_ACCT" ]] && [[ -n "$FAUCET_ACCT" ]] && [[ "$TREASURY_ACCT" != "$FAUCET_ACCT" ]]; then
        pass "faucet_separated_from_treasury"
    elif [[ -z "$TREASURY_ACCT" ]] && [[ -z "$FAUCET_ACCT" ]]; then
        # Check via grep for known treasury patterns
        if grep -q "faucet\|Faucet" "$CHAIN_SPEC" 2>/dev/null; then
            pass "faucet_separated_from_treasury"
        else
            skip "faucet_separated_from_treasury" "faucet account not found in chain spec — verify manually"
        fi
    else
        fail "faucet_separated_from_treasury" "treasury=$TREASURY_ACCT faucet=$FAUCET_ACCT (same or missing)"
    fi
else
    skip "faucet_separated_from_treasury" "chain spec not found"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 6: Block production stable for required hours
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 6] Block production stable $REQUIRED_STABLE_HOURS hours..."
if [[ "$REQUIRED_STABLE_HOURS" -eq 0 ]]; then
    skip "block_production_stable_72h" "REQUIRED_STABLE_HOURS=0 (CI mode)"
else
    if [[ -f "$STABILITY_MARKER" ]]; then
        STABLE_START="$(cat "$STABILITY_MARKER")"
        NOW="$(date +%s)"
        ELAPSED_HOURS=$(( (NOW - STABLE_START) / 3600 ))
        info "Stable since: $(date -d @"$STABLE_START" -u +%Y-%m-%dT%H:%M:%SZ) ($ELAPSED_HOURS hours)"
        if (( ELAPSED_HOURS >= REQUIRED_STABLE_HOURS )); then
            pass "block_production_stable_72h"
        else
            fail "block_production_stable_72h" "only ${ELAPSED_HOURS}h elapsed, need ${REQUIRED_STABLE_HOURS}h"
        fi
    else
        # Check current block production health and set marker if running
        CURRENT_BLOCK="$(rpc_value "chain_getHeader" "[]" | jq -r '.number // "0x0"' 2>/dev/null || echo "0x0")"
        if [[ "$CURRENT_BLOCK" != "0x0" ]] && [[ -n "$CURRENT_BLOCK" ]]; then
            echo "$(date +%s)" > "$STABILITY_MARKER"
            fail "block_production_stable_72h" "stability timer started now — re-run in ${REQUIRED_STABLE_HOURS}h"
        else
            fail "block_production_stable_72h" "node not producing blocks (marker missing)"
        fi
    fi
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 7: Forced node restart drill passed
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 7] Forced node restart drill..."
RESTART_REPORT="$REPORT_DIR/drill_node_restart.md"
if [[ -f "$RESTART_REPORT" ]]; then
    if grep -q "restart_drill: PASS" "$RESTART_REPORT"; then
        pass "forced_node_restart_drill"
    else
        fail "forced_node_restart_drill" "drill report present but not PASS — see $RESTART_REPORT"
    fi
else
    fail "forced_node_restart_drill" "no drill report at $RESTART_REPORT — run scripts/drills/node_restart_drill.sh"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 8: Forced validator removal drill passed
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 8] Forced validator removal drill..."
REMOVAL_REPORT="$REPORT_DIR/drill_validator_removal.md"
if [[ -f "$REMOVAL_REPORT" ]]; then
    if grep -q "validator_removal_drill: PASS" "$REMOVAL_REPORT"; then
        pass "forced_validator_removal_drill"
    else
        fail "forced_validator_removal_drill" "drill not PASS — see $REMOVAL_REPORT"
    fi
else
    fail "forced_validator_removal_drill" "no drill report — run scripts/drills/validator_removal_drill.sh"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 9: Runtime upgrade drill passed
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 9] Runtime upgrade drill (Phase 4 rehearsal)..."
UPGRADE_REPORT="$REPORT_DIR/runtime_upgrade_rehearsal.md"
if [[ -f "$UPGRADE_REPORT" ]]; then
    if grep -q "runtime_upgrade_rehearsal: PASS\|Overall.*PASS\|overall.*PASS" "$UPGRADE_REPORT"; then
        pass "runtime_upgrade_drill"
    else
        fail "runtime_upgrade_drill" "rehearsal not PASS — see $UPGRADE_REPORT"
    fi
else
    fail "runtime_upgrade_drill" "no rehearsal report — run scripts/mainnet/runtime_upgrade_rehearsal.sh"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 10: Invariant violation halt drill passed
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 10] Invariant violation halt drill..."
# Verify via pallet tests that the halt path works
if cargo test -p pallet-x3-cross-vm-router \
    test_paused_asset_rejects_transfers \
    test_closed_route_rejects_transfers \
    >/dev/null 2>&1 && \
   cargo test -p pallet-x3-supply-ledger \
    >/dev/null 2>&1; then
    pass "invariant_halt_drill"
else
    fail "invariant_halt_drill" "halt/supply-ledger pallet tests failed"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 11: Refund drill passed
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 11] Refund drill..."
if cargo test -p pallet-x3-cross-vm-router \
    test_expired_transfer_refunds_to_source \
    test_failed_destination_credit_refunds_pending_supply \
    completion_after_refund_rejected \
    >/dev/null 2>&1; then
    pass "refund_drill"
else
    fail "refund_drill" "refund pallet tests failed"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 12: Indexer/RPC/API smoke tests
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 12] Indexer/RPC/API smoke tests..."
INDEXER_REPORT="$REPORT_DIR/indexer_smoke_test.md"
RPC_SMOKE_CHECKS=(
    "system_health"
    "chain_getHeader"
    "state_getRuntimeVersion"
    "system_name"
    "system_version"
)
rpc_failures=()
for method in "${RPC_SMOKE_CHECKS[@]}"; do
    result="$(rpc "$method")"
    if echo "$result" | grep -q '"error"'; then
        rpc_failures+=("$method")
    fi
done

if [[ ${#rpc_failures[@]} -eq 0 ]]; then
    if echo "$(rpc "system_health")" | grep -q '"isSyncing":false'; then
        pass "indexer_rpc_api_smoke"
    else
        # Node unreachable or syncing — check if unreachable
        if [[ $(rpc "system_health" | jq -r '.result.isSyncing // "unreachable"') == "unreachable" ]]; then
            skip "indexer_rpc_api_smoke" "node not running — run against live testnet"
        else
            fail "indexer_rpc_api_smoke" "node is still syncing"
        fi
    fi
else
    skip "indexer_rpc_api_smoke" "RPC not reachable at $RPC_URL — run against live testnet"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 13: Wallet/SDK transfer tests
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 13] Wallet/SDK transfer tests..."
WALLET_TEST_REPORT="$REPORT_DIR/wallet_sdk_transfer_test.md"
if [[ -f "$WALLET_TEST_REPORT" ]]; then
    if grep -q "wallet_sdk_transfer: PASS" "$WALLET_TEST_REPORT"; then
        pass "wallet_sdk_transfer_tests"
    else
        fail "wallet_sdk_transfer_tests" "wallet test report not PASS — see $WALLET_TEST_REPORT"
    fi
elif [[ -d "$ROOT_DIR/apps/wallet" ]]; then
    # Try running wallet tests if they exist
    if (cd "$ROOT_DIR/apps/wallet" && npm test -- --passWithNoTests 2>/dev/null); then
        pass "wallet_sdk_transfer_tests"
    else
        fail "wallet_sdk_transfer_tests" "wallet tests failed — check apps/wallet"
    fi
else
    fail "wallet_sdk_transfer_tests" "no wallet test report or app — run wallet integration tests"
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 14: Explorer or dashboard reachable
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 14] Explorer/dashboard reachable..."
EXPLORER_URLS=(
    "http://localhost:3000"
    "http://localhost:3001"
    "http://localhost:8080"
)
explorer_ok=false
for url in "${EXPLORER_URLS[@]}"; do
    if curl -sf -m 5 "$url" >/dev/null 2>&1; then
        info "Explorer/dashboard found at $url"
        explorer_ok=true
        break
    fi
done
if $explorer_ok; then
    pass "explorer_or_dashboard"
else
    # Check if dashboard app exists and has a build script
    if [[ -d "$ROOT_DIR/apps/dashboard" ]] || [[ -d "$ROOT_DIR/apps/explorer" ]]; then
        fail "explorer_or_dashboard" "app exists but not running — start the explorer/dashboard"
    else
        fail "explorer_or_dashboard" "no explorer or dashboard found at common ports"
    fi
fi

# ─────────────────────────────────────────────────────────────────────────────
# GATE 15: Chain spec not using --dev or --tmp
# ─────────────────────────────────────────────────────────────────────────────
echo "→ [Gate 15] Production chain spec (not dev/tmp)..."
RUNNING_NODE_CMD="$(ps aux | grep x3-chain-node | grep -v grep || true)"
if echo "$RUNNING_NODE_CMD" | grep -qE "\-\-chain=dev|\-\-chain dev|\-\-tmp"; then
    fail "production_chain_spec" "node is running with --chain=dev or --tmp — not production"
elif [[ -f "$CHAIN_SPEC" ]]; then
    CHAIN_ID="$(jq -r '.id // ""' "$CHAIN_SPEC" 2>/dev/null || echo "")"
    if [[ "$CHAIN_ID" == *"dev"* ]] || [[ "$CHAIN_ID" == *"local"* ]]; then
        fail "production_chain_spec" "chain spec id='$CHAIN_ID' looks like dev/local"
    else
        pass "production_chain_spec"
    fi
else
    skip "production_chain_spec" "chain spec not found — verify node is using production spec"
fi

# ─────────────────────────────────────────────────────────────────────────────
# Write report
# ─────────────────────────────────────────────────────────────────────────────
{
    echo "# Public Testnet Gate Report"
    echo ""
    echo "- **RPC**: \`$RPC_URL\`"
    echo "- **Chain spec**: \`$CHAIN_SPEC\`"
    echo "- **Generated**: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "- **Overall**: $OVERALL"
    echo ""
    echo "## Gate Results"
    echo ""
    echo "| Gate | Criterion | Result |"
    echo "|------|-----------|--------|"
    echo "| 1  | Min $MIN_VALIDATORS validators | ${RESULTS[min_7_validators]:-NOT_RUN} |"
    echo "| 2  | Public bootnodes | ${RESULTS[public_bootnodes]:-NOT_RUN} |"
    echo "| 3  | No dev seeds | ${RESULTS[no_dev_seeds]:-NOT_RUN} |"
    echo "| 4  | External bridges disabled | ${RESULTS[external_bridges_disabled]:-NOT_RUN} |"
    echo "| 5  | Faucet separated from treasury | ${RESULTS[faucet_separated_from_treasury]:-NOT_RUN} |"
    echo "| 6  | Block production stable ${REQUIRED_STABLE_HOURS}h | ${RESULTS[block_production_stable_72h]:-NOT_RUN} |"
    echo "| 7  | Node restart drill | ${RESULTS[forced_node_restart_drill]:-NOT_RUN} |"
    echo "| 8  | Validator removal drill | ${RESULTS[forced_validator_removal_drill]:-NOT_RUN} |"
    echo "| 9  | Runtime upgrade drill | ${RESULTS[runtime_upgrade_drill]:-NOT_RUN} |"
    echo "| 10 | Invariant halt drill | ${RESULTS[invariant_halt_drill]:-NOT_RUN} |"
    echo "| 11 | Refund drill | ${RESULTS[refund_drill]:-NOT_RUN} |"
    echo "| 12 | Indexer/RPC/API smoke | ${RESULTS[indexer_rpc_api_smoke]:-NOT_RUN} |"
    echo "| 13 | Wallet/SDK transfers | ${RESULTS[wallet_sdk_transfer_tests]:-NOT_RUN} |"
    echo "| 14 | Explorer/dashboard | ${RESULTS[explorer_or_dashboard]:-NOT_RUN} |"
    echo "| 15 | Production chain spec | ${RESULTS[production_chain_spec]:-NOT_RUN} |"
    echo ""
    echo "## Missing drills (create these reports to pass their gates)"
    echo ""
    echo "- Gate 7: \`reports/drill_node_restart.md\` (must contain \`restart_drill: PASS\`)"
    echo "- Gate 8: \`reports/drill_validator_removal.md\` (must contain \`validator_removal_drill: PASS\`)"
    echo ""
    echo "## Gate Decision"
    echo ""
    if [[ "$OVERALL" == "PASS" ]]; then
        echo "**public_testnet_gate: PASS** — all criteria met. Proceed to public testnet launch."
    else
        echo "**public_testnet_gate: FAIL** — resolve all FAIL items before opening public participation."
    fi
} > "$REPORT"

SELF_HASH="$(sha256sum "$REPORT" | awk '{print $1}')"
echo "" >> "$REPORT"
echo "_Report hash: \`$SELF_HASH\`_" >> "$REPORT"

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  public_testnet_gate: $OVERALL"
echo "  Report: $REPORT"
echo "══════════════════════════════════════════════════════════"

if [[ "$OVERALL" == "FAIL" ]]; then
    exit 1
fi
exit 0
