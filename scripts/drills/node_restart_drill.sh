#!/usr/bin/env bash
# scripts/drills/node_restart_drill.sh
# Kills an active validator node and verifies *that node* recovers.
# Produces reports/drill_node_restart.md with "restart_drill: PASS" on success.
#
# Two things this drill used to get wrong, both measured 2026-09-26:
#
#  * it killed `pgrep -f x3-chain-node | head -1` — whatever node the pattern matched
#    first, which on a host running more than one node is not the node it was checking;
#  * it then started a **new dev node** (`--chain=dev --tmp --alice`) and called the
#    drill passed when that node answered RPC. That proves the binary starts. It says
#    nothing about the node that was killed: a node that resumed its own database, its
#    own chain and its own finalized height is the claim on the tin.
#
# Now: find the node serving the RPC being checked, read its own argv, kill it, relaunch
# it with that same argv (its own base path, chain and keys), and require it to come back
# on the same chain — same hash at the pre-kill height, height resumed, new blocks after.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT="$ROOT_DIR/reports/drill_node_restart.md"
mkdir -p "$ROOT_DIR/reports"

RPC_URL="${X3_RPC_URL:-http://localhost:9933}"
TIMEOUT=120
RESULT="FAIL"
RPC_PORT="${RPC_URL##*:}"; RPC_PORT="${RPC_PORT%%/*}"

get_block() {
    curl -sf -m 5 "$RPC_URL" -H 'Content-Type: application/json' \
        -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
    | jq -r '.result.number // "0x0"' | xargs printf "%d\n" 2>/dev/null || echo "0"
}

hash_of() { # <block number>
    curl -sf -m 5 "$RPC_URL" -H 'Content-Type: application/json' \
        -d "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"chain_getBlockHash\",\"params\":[$1]}" \
    | jq -r '.result // ""'
}

# The pid listening on the RPC port we are interrogating — not "the first node on the
# host". `ss` gives it directly; `lsof` is the fallback.
pid_on_rpc_port() {
    local port="$1" pid
    pid="$(ss -ltnp 2>/dev/null | awk -v p=":${port}" '$4 ~ p' \
           | sed -n 's/.*pid=\([0-9]\+\).*/\1/p' | head -1)"
    if [[ -z "$pid" ]] && command -v lsof >/dev/null 2>&1; then
        pid="$(lsof -ti "tcp:${port}" -sTCP:LISTEN 2>/dev/null | head -1 || true)"
    fi
    printf '%s' "$pid"
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
PRE_FINALIZED="$(curl -sf -m 5 "$RPC_URL" -H 'Content-Type: application/json' \
    -d '{"id":1,"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[]}' \
    | jq -r '.result // ""')"
PRE_FINALIZED_HEIGHT="$(curl -sf -m 5 "$RPC_URL" -H 'Content-Type: application/json' \
    -d "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"chain_getHeader\",\"params\":[\"$PRE_FINALIZED\"]}" \
    | jq -r '.result.number // ""')"
PRE_FINALIZED_HEIGHT=$(( PRE_FINALIZED_HEIGHT ))
PRE_HASH_AT_FINALIZED="$(hash_of "$PRE_FINALIZED_HEIGHT")"
echo "  Pre-kill finalized: #$PRE_FINALIZED_HEIGHT ($PRE_HASH_AT_FINALIZED)"

# The node to kill is the one answering this RPC, found by the port it listens on.
NODE_PID="$(pid_on_rpc_port "$RPC_PORT")"
if [[ -z "$NODE_PID" ]]; then
    echo "[SKIP] no process is listening on the RPC port $RPC_PORT ($RPC_URL)."
    { echo "# Node Restart Drill"; echo "restart_drill: SKIP — nothing listening on RPC port $RPC_PORT"; } > "$REPORT"
    exit 0
fi

# Its own argv, so the restart runs the same chain, base path and keys instead of a
# fresh dev node. NUL-separated in /proc; the read below is the only portable way.
mapfile -d '' ORIGINAL_ARGV < "/proc/$NODE_PID/cmdline"
if [[ "${#ORIGINAL_ARGV[@]}" -eq 0 ]]; then
    echo "[SKIP] could not read /proc/$NODE_PID/cmdline to restart it."
    { echo "# Node Restart Drill"; echo "restart_drill: SKIP — could not read the node's argv"; } > "$REPORT"
    exit 0
fi
echo "  Node: pid $NODE_PID — ${ORIGINAL_ARGV[*]:0:120}"

echo "→ Sending SIGTERM to PID $NODE_PID..."
kill "$NODE_PID" 2>/dev/null || true
for _ in $(seq 1 20); do kill -0 "$NODE_PID" 2>/dev/null || break; sleep 1; done
if kill -0 "$NODE_PID" 2>/dev/null; then
    echo "  SIGTERM did not stop it within 20s; SIGKILL."
    kill -9 "$NODE_PID" 2>/dev/null || true
    sleep 2
fi

echo "→ Restarting the same node with its own argv..."
BASE_PATH="${TMPDIR:-/tmp}/x3-restart-drill-$$"
mkdir -p "$BASE_PATH"
NEW_LOG="$BASE_PATH/node.log"
"${ORIGINAL_ARGV[@]}" >"$NEW_LOG" 2>&1 &
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

# Recovery is three claims, and the second is the one the old drill never made: the
# node came back, it is on *the same chain* (the hash it had already finalized is
# still the hash at that height), and it is producing again.
RESUMED=false
HASH_SAME=false
if $recovered; then
    deadline=$(( $(date +%s) + TIMEOUT ))
    while [[ $(date +%s) -lt $deadline ]]; do
        now_finalized="$(curl -sf -m 5 "$RPC_URL" -H 'Content-Type: application/json' \
            -d '{"id":1,"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[]}' \
            | jq -r '.result // ""')"
        now_height="$(curl -sf -m 5 "$RPC_URL" -H 'Content-Type: application/json' \
            -d "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"chain_getHeader\",\"params\":[\"$now_finalized\"]}" \
            | jq -r '.result.number // ""')"
        now_height=$(( now_height ))
        if [[ "$now_height" -ge "$PRE_FINALIZED_HEIGHT" ]]; then
            RESUMED=true
            [[ "$(hash_of "$PRE_FINALIZED_HEIGHT")" == "$PRE_HASH_AT_FINALIZED" ]] && HASH_SAME=true
            break
        fi
        sleep 2
    done
fi

if $RESUMED && $HASH_SAME && wait_blocks 3; then
    POST_BLOCK="$(get_block)"
    echo "  Post-restart block: $POST_BLOCK (finalized resumed at #$PRE_FINALIZED_HEIGHT, hash unchanged)"
    RESULT="PASS"
else
    echo "  recovered=$recovered resumed=$RESUMED same_hash_at_pre_kill_height=$HASH_SAME"
fi

# The node is left running on purpose. It is the validator this drill killed, now
# resumed on its own database — stopping it here would take a validator offline at the
# end of a drill whose whole point is that the validator came back.
echo "  Node left running: pid $NEW_PID (log: $NEW_LOG)"

{
    echo "# Node Restart Drill"
    echo ""
    echo "- RPC checked: $RPC_URL (port $RPC_PORT)"
    echo "- Node killed: pid $NODE_PID, restarted from its own argv as pid ${NEW_PID:-none}"
    echo "- Pre-kill best block: $PRE_BLOCK"
    echo "- Pre-kill finalized: #$PRE_FINALIZED_HEIGHT ($PRE_HASH_AT_FINALIZED)"
    echo "- Finalized height resumed: $RESUMED${POST_BLOCK:+ (best block after restart: $POST_BLOCK)}"
    echo "- Hash at #$PRE_FINALIZED_HEIGHT unchanged after restart: $HASH_SAME"
    echo "- Recovery: $RESULT"
    echo "- restart_drill: $RESULT"
    echo "- Log: ${NEW_LOG:-}"
} > "$REPORT"

echo "restart_drill: $RESULT  →  $REPORT"
[[ "$RESULT" == "PASS" ]] && exit 0 || exit 1
