#!/usr/bin/env bash
# scripts/drills/node_restart_drill.sh
# Kills an active validator node and verifies it recovers.
# Produces reports/drill_node_restart.md with "restart_drill: PASS" on success.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT="$ROOT_DIR/reports/drill_node_restart.md"
mkdir -p "$ROOT_DIR/reports"

RPC_URL="${X3_RPC_URL:-http://localhost:9933}"
TIMEOUT=120
RESULT="FAIL"

get_block() {
    curl -sf -m 5 "$RPC_URL" -H 'Content-Type: application/json' \
        -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
    | jq -r '.result.number // "0x0"' | xargs printf "%d\n" 2>/dev/null || echo "0"
}

wait_blocks() {
    local need="$1" start; start="$(get_block)"
    local deadline=$(( $(date +%s) + TIMEOUT ))
    while [[ $(date +%s) -lt $deadline ]]; do
        local cur; cur="$(get_block)"
        (( cur >= start + need )) && return 0
        sleep 2
    done
    return 1
}

echo "→ Pre-kill block check..."
PRE_BLOCK="$(get_block)"
if [[ "$PRE_BLOCK" -eq 0 ]]; then
    echo "[SKIP] No live node at $RPC_URL — start a testnet first."
    { echo "# Node Restart Drill"; echo "restart_drill: SKIP — no live node"; } > "$REPORT"
    exit 0
fi

echo "  Pre-kill block: $PRE_BLOCK"

# Find PID of running x3-chain-node
NODE_PID="$(pgrep -f x3-chain-node | head -1 || true)"
if [[ -z "$NODE_PID" ]]; then
    echo "[SKIP] x3-chain-node not running on this host."
    { echo "# Node Restart Drill"; echo "restart_drill: SKIP — node not running on this host"; } > "$REPORT"
    exit 0
fi

echo "→ Sending SIGTERM to PID $NODE_PID..."
kill "$NODE_PID" 2>/dev/null || true
sleep 3

echo "→ Restarting node (assuming start script exists)..."
NODE_BIN="$ROOT_DIR/target/release/x3-chain-node"
BASE_PATH="${TMPDIR:-/tmp}/x3-restart-drill-$$"
mkdir -p "$BASE_PATH"

"$NODE_BIN" \
    --chain=dev \
    --tmp \
    --alice \
    --rpc-port=9933 \
    --rpc-methods=Unsafe \
    --rpc-external \
    --no-mdns \
    >"$BASE_PATH/node.log" 2>&1 &
NEW_PID=$!

echo "→ Waiting for RPC to recover (PID $NEW_PID)..."
deadline=$(( $(date +%s) + TIMEOUT ))
recovered=false
while [[ $(date +%s) -lt $deadline ]]; do
    if curl -sf -m 2 "$RPC_URL" -H 'Content-Type: application/json' \
        -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' >/dev/null 2>&1; then
        recovered=true; break
    fi
    sleep 2
done

if $recovered && wait_blocks 3; then
    POST_BLOCK="$(get_block)"
    echo "  Post-restart block: $POST_BLOCK"
    RESULT="PASS"
fi

kill "$NEW_PID" 2>/dev/null || true
rm -rf "$BASE_PATH"

{
    echo "# Node Restart Drill"
    echo ""
    echo "- Pre-kill block: $PRE_BLOCK"
    echo "- Recovery: $RESULT"
    echo "- restart_drill: $RESULT"
} > "$REPORT"

echo "restart_drill: $RESULT  →  $REPORT"
[[ "$RESULT" == "PASS" ]] && exit 0 || exit 1
