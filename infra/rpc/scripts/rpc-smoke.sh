#!/usr/bin/env bash
# X3 RPC Smoke Test
# Verifies basic connectivity for all chain methods.
# Usage: ./rpc-smoke.sh [gateway_url]
set -euo pipefail

GATEWAY="${1:-http://localhost:8545}"
PASS=0
FAIL=0

green() { echo -e "\033[32m$1\033[0m"; }
red()   { echo -e "\033[31m$1\033[0m"; }

rpc() {
    local method="$1" params="${2:-[]}" id="${3:-1}"
    local resp
    resp=$(curl -s -X POST "$GATEWAY" \
        -H "Content-Type: application/json" \
        --max-time 10 \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":$id}" 2>&1) || {
        red "  FAIL (curl error): $method"
        FAIL=$((FAIL + 1))
        return 1
    }
    # Check for JSON-RPC error
    if echo "$resp" | jq -e '.error' >/dev/null 2>&1; then
        red "  FAIL: $method — $(echo "$resp" | jq -r '.error.message')"
        FAIL=$((FAIL + 1))
        return 1
    fi
    green "  PASS: $method"
    PASS=$((PASS + 1))
}

echo "X3 RPC Smoke Test"
echo "Gateway: $GATEWAY"
echo "=================="
echo ""

# ── EVM Methods ─────────────────────────────────────────────────
echo "EVM:"
rpc "eth_chainId"
rpc "eth_blockNumber"
rpc "eth_getBlockByNumber" '["latest", false]'
rpc "eth_getBalance" '["0x0000000000000000000000000000000000000000", "latest"]'
rpc "eth_estimateGas" '[{"from": "0x0000000000000000000000000000000000000000", "to": "0x0000000000000000000000000000000000000000"}]'
rpc "eth_gasPrice"
rpc "eth_maxPriorityFeePerGas"
rpc "eth_feeHistory" '["0x1", "latest", []]'
rpc "eth_getLogs" '[{"fromBlock": "0x1", "toBlock": "0x2"}]'
rpc "net_version"
rpc "web3_clientVersion"
echo ""

# ── Solana Methods ──────────────────────────────────────────────
echo "Solana:"
rpc "getHealth"
rpc "getSlot"
rpc "getLatestBlockhash" '[{"commitment": "finalized"}]'
rpc "getBlockHeight"
rpc "getGenesisHash"
rpc "getVersion"
echo ""

# ── Bitcoin Methods ─────────────────────────────────────────────
echo "Bitcoin:"
rpc "getblockchaininfo"
rpc "getblockcount"
rpc "getbestblockhash"
rpc "estimatesmartfee" '[6]'
rpc "getnetworkinfo"
echo ""

# ── X3 Methods ──────────────────────────────────────────────────
echo "X3:"
rpc "x3_getHealth"
rpc "x3_getFinalizedHead"
echo ""

# ── TX Methods blocked unless TX_MODE=mainnet ──────────────────
echo "TX (should be blocked in test mode):"
if [ "${TX_MODE:-}" = "mainnet" ]; then
    # In mainnet mode, skip tx tests that could actually send txs
    echo "  SKIP: eth_sendRawTransaction blocked outside mainnet test"
else
    # Verify tx methods are blocked (expect error)
    resp=$(curl -s -X POST "$GATEWAY" \
        -H "Content-Type: application/json" \
        --max-time 5 \
        -d '{"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":["0xdeadbeef"],"id":99}' 2>&1) || true
    if echo "$resp" | jq -e '.error' >/dev/null 2>&1; then
        green "  PASS: eth_sendRawTransaction correctly blocked"
        PASS=$((PASS + 1))
    else
        red "  FAIL: eth_sendRawTransaction should be blocked but got: $(echo "$resp" | jq -c '.')"
        FAIL=$((FAIL + 1))
    fi
fi
echo ""

# ── Summary ─────────────────────────────────────────────────────
echo "=================="
echo "Results: $PASS passed, $FAIL failed"
if [ "$FAIL" -eq 0 ]; then
    green "SMOKE TEST PASSED"
    exit 0
else
    red "SMOKE TEST FAILED"
    exit 1
fi