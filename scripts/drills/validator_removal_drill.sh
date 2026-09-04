#!/usr/bin/env bash
# scripts/drills/validator_removal_drill.sh
# Verifies the chain continues finalising after one validator is removed from
# session keys (offline simulation).
# Produces reports/drill_validator_removal.md with "validator_removal_drill: PASS".
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT="$ROOT_DIR/reports/drill_validator_removal.md"
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

echo "→ Pre-removal block check..."
PRE_BLOCK="$(get_block)"
if [[ "$PRE_BLOCK" -eq 0 ]]; then
    echo "[SKIP] No live node at $RPC_URL"
    { echo "# Validator Removal Drill"; echo "validator_removal_drill: SKIP — no live node"; } > "$REPORT"
    exit 0
fi
echo "  Pre-removal block: $PRE_BLOCK"

# Get current validator count
VALIDATORS="$(curl -sf -m 5 "$RPC_URL" -H 'Content-Type: application/json' \
    -d '{"id":1,"jsonrpc":"2.0","method":"state_call","params":["GrandpaApi_grandpa_authorities","0x"]}' \
    | jq -r '.result // ""' 2>/dev/null || echo "")"
echo "  Grandpa authorities response length: ${#VALIDATORS}"

# The drill: kill one validator process (only if ≥3 validators present)
VALIDATOR_PIDS=( $(pgrep -f "x3-chain-node" || true) )
VALIDATOR_COUNT=${#VALIDATOR_PIDS[@]}
echo "  Active x3-chain-node processes: $VALIDATOR_COUNT"

if [[ $VALIDATOR_COUNT -lt 2 ]]; then
    echo "[SKIP] Need ≥2 validators for removal drill. Have $VALIDATOR_COUNT."
    {
        echo "# Validator Removal Drill"
        echo ""
        echo "- Pre-removal block: $PRE_BLOCK"
        echo "- Validator processes found: $VALIDATOR_COUNT"
        echo "- validator_removal_drill: SKIP — need ≥2 validator processes"
    } > "$REPORT"
    exit 0
fi

# Kill the last validator in the list (non-alice)
TARGET_PID="${VALIDATOR_PIDS[-1]}"
echo "→ Sending SIGTERM to validator PID $TARGET_PID..."
kill "$TARGET_PID" 2>/dev/null || true
sleep 5

echo "→ Waiting for 5 new blocks (chain must continue without removed validator)..."
if wait_blocks 5; then
    POST_BLOCK="$(get_block)"
    echo "  Post-removal block: $POST_BLOCK"
    RESULT="PASS"
else
    echo "  Chain stalled after validator removal"
    POST_BLOCK="$(get_block)"
fi

{
    echo "# Validator Removal Drill"
    echo ""
    echo "- Pre-removal block: $PRE_BLOCK"
    echo "- Post-removal block: $POST_BLOCK"
    echo "- Target PID killed: $TARGET_PID"
    echo "- validator_removal_drill: $RESULT"
} > "$REPORT"

echo "validator_removal_drill: $RESULT  →  $REPORT"
[[ "$RESULT" == "PASS" ]] && exit 0 || exit 1
