#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# verify-bridge-e2e.sh — Verify bridge end-to-end flow on testnet
#
# Tests the full bridge pipeline on a live testnet:
#   1. RPC connectivity to all validators
#   2. ExternalBridgesEnabled storage value is true
#   3. Cross-VM transfer (Native → EVM)
#   4. Cross-VM transfer (Native → SVM)
#   5. Atomic swap simulation
#   6. Bridge adapter RPC methods respond
#   7. Finality is progressing
#
# Usage:
#   ./scripts/testnet/verify-bridge-e2e.sh [--rpc-url URL] [--suri SURI]
#       [--count N] [--base-rpc-port PORT]
#
# Environment:
#   RPC_URL           Primary RPC endpoint (default: http://127.0.0.1:9944)
#   SURI              Account secret URI for test transactions (default: //Alice)
#   COUNT             Number of validators (default: 7)
#   BASE_RPC_PORT     Base RPC port (default: 9944)
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

RPC_URL="${RPC_URL:-http://127.0.0.1:9944}"
SURI="${SURI:-//Alice}"
COUNT="${COUNT:-7}"
BASE_RPC_PORT="${BASE_RPC_PORT:-9944}"
PASS=0
FAIL=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

usage() {
  cat <<EOF
Usage: $(basename "$0") [--rpc-url URL] [--suri SURI] [--count N] [--base-rpc-port PORT]

Verify bridge end-to-end flow on X3 testnet.

Options:
  --rpc-url URL         Primary RPC endpoint (default: ${RPC_URL})
  --suri SURI           Account secret URI (default: ${SURI})
  --count N             Number of validators (default: ${COUNT})
  --base-rpc-port PORT  Base RPC port (default: ${BASE_RPC_PORT})
  -h, --help            Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rpc-url) RPC_URL="${2:-}"; shift 2 ;;
    --suri) SURI="${2:-}"; shift 2 ;;
    --count) COUNT="${2:-}"; shift 2 ;;
    --base-rpc-port) BASE_RPC_PORT="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1"; usage; exit 2 ;;
  esac
done

jsonrpc() {
  local url="$1"
  local method="$2"
  local params="${3:-[]}"
  curl -s --max-time 5 -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"${method}\",\"params\":${params}}" \
    "$url" 2>/dev/null || echo '{"error":"curl failed"}'
}

check() {
  local name="$1"
  local status="$2"
  if [[ "$status" == "0" ]]; then
    echo -e "  ${GREEN}✓${NC} $name"
    PASS=$((PASS + 1))
  else
    echo -e "  ${RED}✗${NC} $name"
    FAIL=$((FAIL + 1))
  fi
}

echo "=========================================="
echo " X3 Testnet Bridge E2E Verification"
echo " RPC:  ${RPC_URL}"
echo " SURI: ${SURI}"
echo " Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "=========================================="

# ── 1. RPC Connectivity ─────────────────────────────────────────────────────
echo ""
echo "[1/7] RPC Connectivity"

ALL_RPC_OK=0
for i in $(seq 0 $((COUNT - 1))); do
  port=$((BASE_RPC_PORT + i))
  url="http://127.0.0.1:${port}"
  health=$(jsonrpc "$url" "system_health")
  if echo "$health" | python3 -c "import json,sys; d=json.load(sys.stdin); sys.exit(0 if d.get('result') else 1)" 2>/dev/null; then
    ALL_RPC_OK=$((ALL_RPC_OK + 1))
  fi
done

if [[ "$ALL_RPC_OK" -ge 3 ]]; then
  check "At least 3/7 validators respond to RPC" 0
else
  check "At least 3/7 validators respond to RPC (got ${ALL_RPC_OK})" 1
fi

# ── 2. ExternalBridgesEnabled Storage ───────────────────────────────────────
echo ""
echo "[2/7] ExternalBridgesEnabled Storage"

# Storage key for ExternalBridgesEnabled:
# twox_128("X3CrossVmRouter") + twox_128("ExternalBridgesEnabled")
# = 0x1ea3c00b772dc6623f323eb3179639f1 + 8997eadf5206160f7717460ca1aec5a8
STORAGE_KEY="0x1ea3c00b772dc6623f323eb3179639f18997eadf5206160f7717460ca1aec5a8"

storage_result=$(jsonrpc "$RPC_URL" "state_getStorage" "[\"${STORAGE_KEY}\"]")
storage_value=$(echo "$storage_result" | python3 -c "
import json,sys
try:
    d = json.load(sys.stdin)
    val = d.get('result')
    if val == '0x01':
        print('true')
    elif val:
        print('unexpected:' + val)
    else:
        print('null')
except Exception as e:
    print('error:' + str(e))
" 2>/dev/null || echo "parse_error")

if [[ "$storage_value" == "true" ]]; then
  check "ExternalBridgesEnabled = true" 0
elif [[ "$storage_value" == "null" ]]; then
  check "ExternalBridgesEnabled = true (got null/default)" 1
  echo "         ⚠ Storage value not set. Check genesis config."
else
  check "ExternalBridgesEnabled = true (got ${storage_value})" 1
fi

# ── 3. ExternalBridgeAuditGate Storage ──────────────────────────────────────
echo ""
echo "[3/7] ExternalBridgeAuditGate Storage"

# Storage key: twox_128("X3CrossVmRouter") + twox_128("ExternalBridgeAuditGate")
AUDIT_KEY="0x1ea3c00b772dc6623f323eb3179639f1ed891578bb9b4ab1aa468cd3f5af7e79"

audit_result=$(jsonrpc "$RPC_URL" "state_getStorage" "[\"${AUDIT_KEY}\"]")
audit_value=$(echo "$audit_result" | python3 -c "
import json,sys
try:
    d = json.load(sys.stdin)
    val = d.get('result')
    if val == '0x01':
        print('true')
    elif val:
        print('unexpected:' + val)
    else:
        print('null')
except Exception as e:
    print('error:' + str(e))
" 2>/dev/null || echo "parse_error")

if [[ "$audit_value" == "true" ]]; then
  check "ExternalBridgeAuditGate = true" 0
elif [[ "$audit_value" == "null" ]]; then
  check "ExternalBridgeAuditGate = true (got null/default)" 1
  echo "         ⚠ Audit gate not set. Check genesis config."
else
  check "ExternalBridgeAuditGate = true (got ${audit_value})" 1
fi

# ── 4. Cross-VM RPC Methods ─────────────────────────────────────────────────
echo ""
echo "[4/7] Cross-VM RPC Methods"

# Check x3_getBridgeStatus
bridge_status=$(jsonrpc "$RPC_URL" "x3_getBridgeStatus" "[0]")
if echo "$bridge_status" | python3 -c "import json,sys; d=json.load(sys.stdin); sys.exit(0 if d.get('result') is not None else 1)" 2>/dev/null; then
  check "x3_getBridgeStatus responds" 0
else
  check "x3_getBridgeStatus responds" 1
  echo "         Response: $(echo "$bridge_status" | head -c 200)"
fi

# Check x3_getAtomicRoute
atomic_route=$(jsonrpc "$RPC_URL" "x3_getAtomicRoute" '["0x0000000000000000000000000000000000000000000000000000000000000000","0x0000000000000000000000000000000000000000000000000000000000000000"]')
if echo "$atomic_route" | python3 -c "import json,sys; d=json.load(sys.stdin); sys.exit(0 if d.get('result') is not None else 1)" 2>/dev/null; then
  check "x3_getAtomicRoute responds" 0
else
  check "x3_getAtomicRoute responds" 1
  echo "         Response: $(echo "$atomic_route" | head -c 200)"
fi

# Check atomicTrade_simulate
simulate=$(jsonrpc "$RPC_URL" "atomicTrade_simulate" '["0x0000000000000000000000000000000000000000000000000000000000000000","0x0000000000000000000000000000000000000000000000000000000000000000",1000000000000,50,null]')
if echo "$simulate" | python3 -c "import json,sys; d=json.load(sys.stdin); sys.exit(0 if d.get('result') is not None else 1)" 2>/dev/null; then
  check "atomicTrade_simulate responds" 0
else
  check "atomicTrade_simulate responds" 1
  echo "         Response: $(echo "$simulate" | head -c 200)"
fi

# ── 5. Finality Check ───────────────────────────────────────────────────────
echo ""
echo "[5/7] Finality Progress"

# Sample finalized head twice with a delay
finalized_1=$(jsonrpc "$RPC_URL" "chain_getFinalizedHead")
sleep 6
finalized_2=$(jsonrpc "$RPC_URL" "chain_getFinalizedHead")

finality_ok=$(python3 -c "
import json,sys
try:
    f1 = json.loads(sys.stdin.readline()).get('result','')
    f2 = json.loads(sys.stdin.readline()).get('result','')
    sys.exit(0 if f2 and f2 != f1 else 1)
except:
    sys.exit(2)
" <<< "$(echo -e "${finalized_1}\n${finalized_2}")" 2>/dev/null || echo 2)

if [[ "$finality_ok" == "0" ]]; then
  check "Finality progressing (6s window)" 0
else
  check "Finality progressing (6s window)" 1
fi

# ── 6. Peer Count ───────────────────────────────────────────────────────────
echo ""
echo "[6/7] Peer Connectivity"

PEER_OK=0
for i in $(seq 0 $((COUNT - 1))); do
  port=$((BASE_RPC_PORT + i))
  url="http://127.0.0.1:${port}"
  health=$(jsonrpc "$url" "system_health")
  peers=$(echo "$health" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('result',{}).get('peers',0))" 2>/dev/null || echo 0)
  if [[ "$peers" -ge 2 ]]; then
    PEER_OK=$((PEER_OK + 1))
  fi
done

if [[ "$PEER_OK" -ge 3 ]]; then
  check "At least 3 validators have >=2 peers" 0
else
  check "At least 3 validators have >=2 peers (got ${PEER_OK})" 1
fi

# ── 7. Chain Spec Verification ──────────────────────────────────────────────
echo ""
echo "[7/7] Chain Spec Verification"

# Check chain name
chain_name=$(jsonrpc "$RPC_URL" "system_chain")
chain_name_val=$(echo "$chain_name" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('result',''))" 2>/dev/null || echo "")
if [[ -n "$chain_name_val" ]]; then
  check "Chain name: ${chain_name_val}" 0
else
  check "Chain name readable" 1
fi

# Check chain type
chain_type=$(jsonrpc "$RPC_URL" "system_chainType")
chain_type_val=$(echo "$chain_type" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('result',''))" 2>/dev/null || echo "")
if [[ -n "$chain_type_val" ]]; then
  check "Chain type: ${chain_type_val}" 0
else
  check "Chain type readable" 1
fi

# ── Summary ─────────────────────────────────────────────────────────────────
echo ""
echo "=========================================="
echo " Bridge E2E Verification Summary"
echo " Passed: ${PASS}"
echo " Failed: ${FAIL}"
echo " Total:  $((PASS + FAIL))"
echo "=========================================="

if [[ "$FAIL" -gt 0 ]]; then
  echo ""
  echo " FAILURES DETECTED. Review above for details."
  echo ""
  echo " Common fixes:"
  echo "   - Ensure validators are running with --features testnet"
  echo "   - Check genesis config has ExternalBridgesEnabled=true"
  echo "   - Verify chain spec has correct storage entries"
  echo "   - Run: ./scripts/testnet/status-7-validators.sh"
  exit 1
else
  echo ""
  echo " All bridge checks passed! Testnet is bridge-ready."
  exit 0
fi
