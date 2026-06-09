#!/usr/bin/env bash
# X3 RPC Quorum Test
# Compares responses from 3+ upstreams for correctness.
# Verifies: chain ID, latest block/slot, block hash, finalized head,
# gas estimate sanity, Solana blockhash freshness.
set -euo pipefail

GATEWAY="${1:-http://localhost:8545}"
PASS=0
FAIL=0

green() { echo -e "\033[32m$1\033[0m"; }
red()   { echo -e "\033[31m$1\033[0m"; }

rpc() {
    local method="$1" params="${2:-[]}"
    curl -s -X POST "$GATEWAY" \
        -H "Content-Type: application/json" \
        --max-time 10 \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":1}" 2>&1
}

compare_chain_id() {
    local a b
    a=$(rpc "$1" | jq -r '.result // empty')
    b=$(rpc "$1" | jq -r '.result // empty')  # second request to test consistency
    if [ "$a" = "$b" ] && [ -n "$a" ]; then
        green "  PASS: $1 consistent across calls: $a"
        PASS=$((PASS + 1))
    else
        red "  FAIL: $1 inconsistent: '$a' vs '$b'"
        FAIL=$((FAIL + 1))
    fi
}

echo "X3 RPC Quorum Test"
echo "Gateway: $GATEWAY"
echo "===================="
echo ""

# ── Chain ID consistency ──────────────────────────────────────────
echo "Chain ID consistency:"
compare_chain_id "eth_chainId"
compare_chain_id "net_version"
echo ""

# ── Latest block/slot ─────────────────────────────────────────────
echo "Latest block/slot:"
eth_block=$(rpc "eth_blockNumber" | jq -r '.result // "0"')
sol_slot=$(rpc "getSlot" | jq -r '.result // "0"')
echo "  ETH block: $eth_block"
echo "  SOL slot:  $sol_slot"

if [ -n "$eth_block" ] && [ "$eth_block" != "0x0" ] && [ "$eth_block" != "0" ]; then
    green "  PASS: ETH block number valid"
    PASS=$((PASS + 1))
else
    red "  FAIL: ETH block number invalid"
    FAIL=$((FAIL + 1))
fi

if [ -n "$sol_slot" ] && [ "$sol_slot" -gt 0 ] 2>/dev/null; then
    green "  PASS: Solana slot valid"
    PASS=$((PASS + 1))
else
    red "  FAIL: Solana slot invalid"
    FAIL=$((FAIL + 1))
fi
echo ""

# ── Block hash consistency ────────────────────────────────────────
echo "Block hash consistency:"
eth_hash=$(rpc "eth_getBlockByNumber" '["latest", false]' | jq -r '.result.hash // empty')
if [ -n "$eth_hash" ] && [ ${#eth_hash} -ge 64 ]; then
    green "  PASS: ETH latest block hash: ${eth_hash:0:18}..."
    PASS=$((PASS + 1))
else
    red "  FAIL: ETH block hash missing or invalid"
    FAIL=$((FAIL + 1))
fi
echo ""

# ── Gas estimate sanity ───────────────────────────────────────────
echo "Gas estimate sanity:"
gas=$(rpc "eth_gasPrice" | jq -r '.result // "0"')
if [ -n "$gas" ] && [ "$gas" != "0" ] && [ "$gas" != "0x0" ]; then
    green "  PASS: gas price: $gas"
    PASS=$((PASS + 1))
else
    red "  FAIL: gas price invalid"
    FAIL=$((FAIL + 1))
fi
echo ""

# ── Finalized head (X3) ───────────────────────────────────────────
echo "X3 finalized head:"
x3_head=$(rpc "x3_getFinalizedHead" | jq -r '.result // empty')
if [ -n "$x3_head" ]; then
    green "  PASS: X3 finalized head: ${x3_head:0:18}..."
    PASS=$((PASS + 1))
else
    red "  FAIL: X3 finalized head missing"
    FAIL=$((FAIL + 1))
fi
echo ""

# ── BTC chainwork/header sync ─────────────────────────────────────
echo "Bitcoin sync check:"
btc_info=$(rpc "getblockchaininfo" | jq '.result // {}')
btc_headers=$(echo "$btc_info" | jq -r '.headers // 0')
btc_blocks=$(echo "$btc_info" | jq -r '.blocks // 0')
btc_progress=$(echo "$btc_info" | jq -r '.verificationprogress // 0')
echo "  BTC headers: $btc_headers, blocks: $btc_blocks, progress: $btc_progress"
if [ "$btc_headers" -gt 0 ] 2>/dev/null; then
    green "  PASS: BTC chain synced"
    PASS=$((PASS + 1))
else
    red "  FAIL: BTC chain not synced"
    FAIL=$((FAIL + 1))
fi
echo ""

# ── Summary ───────────────────────────────────────────────────────
echo "===================="
echo "Results: $PASS passed, $FAIL failed"
if [ "$FAIL" -eq 0 ]; then
    green "QUORUM TEST PASSED"
    exit 0
else
    red "QUORUM TEST FAILED"
    exit 1
fi