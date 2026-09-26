#!/usr/bin/env bash
# scripts/drills/validator_removal_drill.sh
# Remove one validator from a running network and require the network to keep
# *finalizing* without it.
# Produces reports/drill_validator_removal.md with "validator_removal_drill: PASS".
#
# What this drill used to do, and why it was not evidence (measured 2026-09-26):
#
#   * it removed `pgrep -f x3-chain-node | tail -1` — whichever node the pattern
#     matched last on the host, which on a machine running more than one network is not a
#     member of the network the drill is observing. On this box that would have killed a
#     validator of an unrelated seven-node soak;
#   * it then required five new *best* blocks. A chain whose validators keep authoring
#     while finality has stopped is exactly the failure this drill exists to catch, and it
#     would have reported PASS.
#
# Now the validator to remove is named by its own RPC endpoint (`X3_REMOVAL_RPC_URL`,
# default: the next port after the observer), it is found by the pid listening on that
# port, and the check is that the observer's **finalized** height advances while the
# removed validator is verifiably gone (process dead, RPC refused).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT="$ROOT_DIR/reports/drill_validator_removal.md"
mkdir -p "$ROOT_DIR/reports"

RPC_URL="${X3_RPC_URL:-http://localhost:9933}"
OBSERVER_PORT="${RPC_URL##*:}"; OBSERVER_PORT="${OBSERVER_PORT%%/*}"
REMOVAL_RPC_URL="${X3_REMOVAL_RPC_URL:-http://localhost:$(( OBSERVER_PORT + 1 ))}"
REMOVAL_PORT="${REMOVAL_RPC_URL##*:}"; REMOVAL_PORT="${REMOVAL_PORT%%/*}"
NEED_BLOCKS="${X3_REMOVAL_NEED_BLOCKS:-5}"
TIMEOUT=180
RESULT="FAIL"

if [[ "$REMOVAL_PORT" == "$OBSERVER_PORT" ]]; then
    echo "[FAIL] X3_REMOVAL_RPC_URL must name a different validator than X3_RPC_URL." >&2
    { echo "# Validator Removal Drill"; echo "validator_removal_drill: FAIL — removal endpoint is the observer"; } > "$REPORT"
    exit 1
fi

rpc() { # <url> <method> [params]
    curl -sf -m 5 "$1" -H 'Content-Type: application/json' \
        -d "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"$2\",\"params\":${3:-[]}}" 2>/dev/null
}

finalized_height() { # <url>
    local head
    head="$(rpc "$1" chain_getFinalizedHead | jq -r '.result // ""')"
    [[ -n "$head" ]] || { echo 0; return; }
    rpc "$1" chain_getHeader "[\"$head\"]" | jq -r '.result.number // "0x0"' \
        | xargs printf "%d\n" 2>/dev/null || echo 0
}

pid_on_port() {
    local port="$1" pid
    pid="$(ss -ltnp 2>/dev/null | awk -v p=":${port}" '$4 ~ p' \
           | sed -n 's/.*pid=\([0-9]\+\).*/\1/p' | head -1)"
    if [[ -z "$pid" ]] && command -v lsof >/dev/null 2>&1; then
        pid="$(lsof -ti "tcp:${port}" -sTCP:LISTEN 2>/dev/null | head -1 || true)"
    fi
    printf '%s' "$pid"
}

info() { echo "  $*"; }

echo "→ Pre-removal check (observer $RPC_URL, removing $REMOVAL_RPC_URL)"
PRE_HEIGHT="$(finalized_height "$RPC_URL")"
if [[ "$PRE_HEIGHT" -eq 0 ]]; then
    echo "[SKIP] no live node at $RPC_URL"
    { echo "# Validator Removal Drill"; echo "validator_removal_drill: SKIP — no live node at the observer RPC"; } > "$REPORT"
    exit 0
fi

# How many validators may this set lose and still finalize?
#
# Substrate's GRANDPA threshold is `n - (n - 1) / 3` votes, not "more than two thirds":
# for three authorities it is **all three**, for four it is three, for seven it is five.
# A drill that removes one validator from a three-validator set and then demands finality
# is asserting something the protocol does not do — measured 2026-09-26: the remaining two
# kept authoring (best 1297/1298) and could not finalize, which is correct behaviour for
# that set size and the arithmetic reason a public testnet needs seven.
AUTHORITIES_HEX="$(rpc "$RPC_URL" state_call '["GrandpaApi_grandpa_authorities","0x"]' \
    | jq -r '.result // ""')"
AUTHORITY_COUNT="$(python3 - "$AUTHORITIES_HEX" <<'PY'
import sys
raw = sys.argv[1] if len(sys.argv) > 1 else ""
raw = raw[2:] if raw.startswith("0x") else raw
if not raw:
    print(0); raise SystemExit
first = int(raw[:2], 16)
mode = first & 0b11
if mode == 0:
    print(first >> 2)
elif mode == 1 and len(raw) >= 4:
    print(int.from_bytes(bytes.fromhex(raw[:4]), "little") >> 2)
else:
    print(0)
PY
)"
if [[ "${AUTHORITY_COUNT:-0}" -gt 0 ]]; then
    THRESHOLD=$(( AUTHORITY_COUNT - (AUTHORITY_COUNT - 1) / 3 ))
    info "authorities: $AUTHORITY_COUNT, GRANDPA threshold: $THRESHOLD (n - (n-1)/3)"
    if (( AUTHORITY_COUNT - 1 < THRESHOLD )); then
        {
            echo "# Validator Removal Drill"
            echo ""
            echo "- Observer: $RPC_URL"
            echo "- Authorities: $AUTHORITY_COUNT, GRANDPA threshold: $THRESHOLD (n - (n-1)/3)"
            echo "- Removing one validator leaves $(( AUTHORITY_COUNT - 1 )), below the threshold"
            echo "- validator_removal_drill: SKIP — this set cannot lose a validator by design"
            echo ""
            echo "This is the arithmetic that makes the public testnet a seven-validator"
            echo "network: at seven, the threshold is five and two may be lost. Run this drill"
            echo "against a seven-authority network to exercise the removal path."
        } > "$REPORT"
        echo "validator_removal_drill: SKIP — a $AUTHORITY_COUNT-validator set needs $THRESHOLD of them to finalize" \
            "→  $REPORT"
        exit 0
    fi
fi

TARGET_PID="$(pid_on_port "$REMOVAL_PORT")"
if [[ -z "$TARGET_PID" ]]; then
    echo "[SKIP] nothing is listening on the removal RPC port $REMOVAL_PORT."
    { echo "# Validator Removal Drill"; echo "validator_removal_drill: SKIP — no validator on $REMOVAL_RPC_URL"; } > "$REPORT"
    exit 0
fi

mapfile -d '' TARGET_ARGV < "/proc/$TARGET_PID/cmdline" || true
info "observer finalized height: #$PRE_HEIGHT"
info "removing pid $TARGET_PID (${TARGET_ARGV[*]:0:100})"

echo "→ Sending SIGTERM to validator pid $TARGET_PID..."
kill "$TARGET_PID" 2>/dev/null || true
for _ in $(seq 1 20); do kill -0 "$TARGET_PID" 2>/dev/null || break; sleep 1; done
if kill -0 "$TARGET_PID" 2>/dev/null; then
    kill -9 "$TARGET_PID" 2>/dev/null || true
    sleep 2
fi

# The removal has to be real before the check means anything: process gone and its RPC
# no longer answering.
GONE=false
if ! kill -0 "$TARGET_PID" 2>/dev/null && ! rpc "$REMOVAL_RPC_URL" system_health >/dev/null 2>&1; then
    GONE=true
fi
info "removed validator is gone (process dead, RPC refusing): $GONE"

echo "→ Waiting for the observer's finalized height to advance by $NEED_BLOCKS without it..."
deadline=$(( $(date +%s) + TIMEOUT ))
POST_HEIGHT="$PRE_HEIGHT"
while [[ $(date +%s) -lt $deadline ]]; do
    POST_HEIGHT="$(finalized_height "$RPC_URL")"
    (( POST_HEIGHT >= PRE_HEIGHT + NEED_BLOCKS )) && break
    sleep 2
done

if $GONE && (( POST_HEIGHT >= PRE_HEIGHT + NEED_BLOCKS )); then
    RESULT="PASS"
else
    info "finalized #$PRE_HEIGHT -> #$POST_HEIGHT (needed +$NEED_BLOCKS)"
fi

{
    echo "# Validator Removal Drill"
    echo ""
    echo "- Observer: $RPC_URL"
    echo "- Removed validator: $REMOVAL_RPC_URL (pid $TARGET_PID)"
    echo "- Removed before the check: $GONE"
    echo "- Finalized height before: #$PRE_HEIGHT"
    echo "- Finalized height after:  #$POST_HEIGHT"
    echo "- Required advance: +$NEED_BLOCKS finalized blocks without the removed validator"
    echo "- validator_removal_drill: $RESULT"
} > "$REPORT"

echo "validator_removal_drill: $RESULT  →  $REPORT"
[[ "$RESULT" == "PASS" ]] && exit 0 || exit 1
