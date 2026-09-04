#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# x3_testnet_health.sh — Comprehensive testnet health check
#
# Checks all testnet services: validators, explorer, indexer, faucet, RPC gateway.
# Reports overall health status and individual component status.
#
# Usage:
#   ./scripts/testnet/x3_testnet_health.sh [--rpc-url URL] [--base-rpc-port PORT]
#       [--count N] [--check-infra]
#
# Environment:
#   RPC_URL           Primary RPC endpoint (default: http://127.0.0.1:9944)
#   BASE_RPC_PORT     Base RPC port (default: 9944)
#   COUNT             Number of validators (default: 7)
#   CHECK_INFRA       Also check infrastructure services (default: 0)
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

RPC_URL="${RPC_URL:-http://127.0.0.1:9944}"
BASE_RPC_PORT="${BASE_RPC_PORT:-9944}"
COUNT="${COUNT:-7}"
CHECK_INFRA="${CHECK_INFRA:-0}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

jsonrpc() {
  local url="$1"
  local method="$2"
  local params="${3:-[]}"
  curl -s --max-time 3 -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"${method}\",\"params\":${params}}" \
    "$url" 2>/dev/null || echo '{"error":"curl failed"}'
}

echo "=========================================="
echo " X3 Testnet Health Check"
echo " $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "=========================================="

# ── Validator Status ─────────────────────────────────────────────────────────
echo ""
echo "── Validators ──────────────────────────────"
printf "%-5s %-7s %-9s %-6s %-12s\n" "NODE" "RPC" "SYNCING" "PEERS" "BLOCK"

VALIDATORS_UP=0
for i in $(seq 0 $((COUNT - 1))); do
  port=$((BASE_RPC_PORT + i))
  url="http://127.0.0.1:${port}"
  node=$((i + 1))

  health=$(jsonrpc "$url" "system_health")
  header=$(jsonrpc "$url" "chain_getHeader")

  python3 -c "
import json, sys
health_raw = '''${health}'''
header_raw = '''${header}'''
try:
    health = json.loads(health_raw)
    hdr = json.loads(header_raw)
    if 'result' in health:
        is_syncing = health['result'].get('isSyncing', True)
        peers = health['result'].get('peers', 0)
        block_num = int(hdr.get('result', {}).get('number', '0x0'), 16) if hdr.get('result') else 0
        print(f\"${node:<5} ${port:<7} {str(is_syncing):<9} {peers:<6} {block_num:<12}\")
        sys.exit(0 if not is_syncing else 2)
    else:
        print(f\"${node:<5} ${port:<7} DOWN\")
        sys.exit(1)
except Exception as e:
    print(f\"${node:<5} ${port:<7} ERROR: {e}\")
    sys.exit(1)
" 2>/dev/null && VALIDATORS_UP=$((VALIDATORS_UP + 1)) || true
done

echo ""
echo "Validators online: ${VALIDATORS_UP}/${COUNT}"

# ── Bridge Status ────────────────────────────────────────────────────────────
echo ""
echo "── Bridge Status ───────────────────────────"

# Check ExternalBridgesEnabled
STORAGE_KEY="0x1ea3c00b772dc6623f323eb3179639f18997eadf5206160f7717460ca1aec5a8"
storage_result=$(jsonrpc "$RPC_URL" "state_getStorage" "[\"${STORAGE_KEY}\"]")
bridges_enabled=$(echo "$storage_result" | python3 -c "
import json,sys
try:
    d = json.load(sys.stdin)
    val = d.get('result')
    print('true' if val == '0x01' else ('false' if val is None else 'unexpected'))
except: print('error')
" 2>/dev/null || echo "error")

if [[ "$bridges_enabled" == "true" ]]; then
  echo -e "  ExternalBridgesEnabled: ${GREEN}true${NC}"
elif [[ "$bridges_enabled" == "false" ]]; then
  echo -e "  ExternalBridgesEnabled: ${RED}false (default)${NC}"
else
  echo -e "  ExternalBridgesEnabled: ${YELLOW}${bridges_enabled}${NC}"
fi

# Check finality
finalized=$(jsonrpc "$RPC_URL" "chain_getFinalizedHead")
best=$(jsonrpc "$RPC_URL" "chain_getHeader")
finality_info=$(python3 -c "
import json, sys
try:
    fin = json.loads('''${finalized}''')
    hdr = json.loads('''${best}''')
    fin_hash = fin.get('result', '')
    best_num = int(hdr.get('result', {}).get('number', '0x0'), 16)
    print(f'Best: {best_num} | Finalized: {fin_hash[:16]}...')
except: print('Unable to determine')
" 2>/dev/null || echo "Unable to determine")
echo "  Finality: ${finality_info}"

# ── Infrastructure Status ────────────────────────────────────────────────────
if [[ "$CHECK_INFRA" == "1" ]]; then
  echo ""
  echo "── Infrastructure ──────────────────────────"

  # Check explorer
  if curl -s --max-time 2 http://localhost:8080/ >/dev/null 2>&1; then
    echo -e "  Explorer: ${GREEN}online${NC} (port 8080)"
  else
    echo -e "  Explorer: ${YELLOW}not detected${NC}"
  fi

  # Check indexer
  if curl -s --max-time 2 http://localhost:4000/graphql >/dev/null 2>&1; then
    echo -e "  Indexer:  ${GREEN}online${NC} (port 4000)"
  else
    echo -e "  Indexer:  ${YELLOW}not detected${NC}"
  fi

  # Check faucet
  if curl -s --max-time 2 http://localhost:3000/health >/dev/null 2>&1; then
    echo -e "  Faucet:   ${GREEN}online${NC} (port 3000)"
  else
    echo -e "  Faucet:   ${YELLOW}not detected${NC}"
  fi

  # Check RPC gateway
  if curl -s --max-time 2 http://localhost:8545/health >/dev/null 2>&1; then
    echo -e "  RPC Gateway: ${GREEN}online${NC} (port 8545)"
  else
    echo -e "  RPC Gateway: ${YELLOW}not detected${NC}"
  fi
fi

# ── Summary ──────────────────────────────────────────────────────────────────
echo ""
echo "── Summary ─────────────────────────────────"
if [[ "$VALIDATORS_UP" -ge 3 ]]; then
  echo -e " Overall: ${GREEN}HEALTHY${NC} (${VALIDATORS_UP}/${COUNT} validators online)"
  exit 0
else
  echo -e " Overall: ${RED}UNHEALTHY${NC} (${VALIDATORS_UP}/${COUNT} validators online)"
  exit 1
fi
