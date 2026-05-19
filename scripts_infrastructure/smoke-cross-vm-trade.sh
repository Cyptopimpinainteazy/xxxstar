#!/usr/bin/env bash
# smoke-cross-vm-trade.sh — end-to-end smoke test for cross-VM atomic trade
# Usage: ./scripts/smoke-cross-vm-trade.sh [rpc-url]
# Exit 0 on success, nonzero on failure.
set -euo pipefail

RPC_URL="${1:-http://127.0.0.1:9944}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Known-good payload built against the patched genesis:
#   caller  = 0xd43593c715fdd31c61141abd04a99fd6822c8558 (Alice dev EVM address)
#   to      = 0x1111111111111111111111111111111111111111
#   value   = 0 (16 LE bytes of zero)
#   data    = empty (data_len = 0)
EVM_PAYLOAD="0xd43593c715fdd31c61141abd04a99fd6822c855811111111111111111111111111111111111111110000000000000000000000000000000000000000"
SVM_PAYLOAD="0x01020304"

echo "=== X3 Cross-VM Smoke Test ==="
echo "RPC: ${RPC_URL}"
echo ""

# ------------------------------------------------------------------ #
# 1. Verify node is reachable                                         #
# ------------------------------------------------------------------ #
echo "[1/3] Checking node health..."
HEALTH=$(curl -sf --max-time 5 -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' \
  "${RPC_URL}" 2>&1) || {
  echo "FAIL: node not reachable at ${RPC_URL}"
  echo "Run: scripts/run-node-fresh.sh"
  exit 1
}
echo "OK: node healthy"

# ------------------------------------------------------------------ #
# 2. Verify all 3 X3 RPC methods are present                         #
# ------------------------------------------------------------------ #
echo "[2/3] Verifying X3 RPC methods..."
"${SCRIPT_DIR}/verify-x3-rpc-methods.sh" "${RPC_URL}"

# ------------------------------------------------------------------ #
# 3. Submit cross-VM transaction and assert result key present        #
# ------------------------------------------------------------------ #
echo "[3/3] Submitting cross-VM trade..."
RESPONSE=$(curl -sS --max-time 10 -H "Content-Type: application/json" \
  -d "{\"id\":100,\"jsonrpc\":\"2.0\",\"method\":\"x3_submitCrossVmTransaction\",\"params\":[{\"evm_payload\":\"${EVM_PAYLOAD}\",\"svm_payload\":\"${SVM_PAYLOAD}\",\"atomic\":true,\"api_key\":\"testnet-default\"}]}" \
  "${RPC_URL}")

echo "Response: ${RESPONSE}"

# Must contain "result" key and no "error" key
if echo "${RESPONSE}" | grep -q '"result"'; then
  TX_HASH=$(echo "${RESPONSE}" | grep -o '"result":"[^"]*"' | cut -d'"' -f4)
  echo ""
  echo "SUCCESS: cross-VM trade submitted"
  echo "  EVM tx hash : ${TX_HASH}"
  exit 0
else
  echo ""
  echo "FAIL: response did not contain 'result' key"
  exit 1
fi
