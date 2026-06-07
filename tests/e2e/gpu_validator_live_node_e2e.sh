#!/usr/bin/env bash

set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROFILE="${PROFILE:-debug}"
RPC_PORT="${X3_GPU_E2E_RPC_PORT:-19933}"
P2P_PORT="${X3_GPU_E2E_P2P_PORT:-30399}"
RPC_URL="${X3_GPU_E2E_RPC_URL:-http://127.0.0.1:${RPC_PORT}}"
TIMEOUT_SECS="${X3_GPU_E2E_TIMEOUT_SECS:-120}"

if [[ "$PROFILE" == "release" ]]; then
  TARGET_DIR="$ROOT/target/release"
  PROFILE_ARGS=(--release)
else
  TARGET_DIR="$ROOT/target/debug"
  PROFILE_ARGS=()
fi

NODE_BIN="$TARGET_DIR/x3-chain-node"
VALIDATOR_BIN="$TARGET_DIR/x3-validator"
WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/x3-gpu-live-e2e.XXXXXX")"
NODE_PID=""
VALIDATOR_A_PID=""
VALIDATOR_B_PID=""

cleanup() {
  set +e
  [[ -n "$VALIDATOR_A_PID" ]] && kill "$VALIDATOR_A_PID" 2>/dev/null || true
  [[ -n "$VALIDATOR_B_PID" ]] && kill "$VALIDATOR_B_PID" 2>/dev/null || true
  [[ -n "$NODE_PID" ]] && kill "$NODE_PID" 2>/dev/null || true
  [[ -n "$NODE_PID" ]] && wait "$NODE_PID" 2>/dev/null || true
  echo "artifacts: $WORKDIR"
}
trap cleanup EXIT

log() {
  printf '[gpu-live-e2e] %s\n' "$*"
}

rpc() {
  local method="$1"
  local params="${2:-[]}"
  curl -fsS \
    -H 'Content-Type: application/json' \
    --data "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"${method}\",\"params\":${params}}" \
    "$RPC_URL"
}

wait_for_rpc() {
  local deadline=$((SECONDS + TIMEOUT_SECS))
  until rpc system_health >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      log "node log follows:"
      tail -n 120 "$WORKDIR/node.log" || true
      echo "timed out waiting for node RPC at $RPC_URL" >&2
      exit 1
    fi
    sleep 1
  done
}

wait_for_finalized_block() {
  local deadline=$((SECONDS + TIMEOUT_SECS))
  while true; do
    local head
    head="$(rpc chain_getFinalizedHead | python3 -c 'import json,sys; print(json.load(sys.stdin).get("result",""))' 2>/dev/null || true)"
    if [[ -n "$head" && "$head" != "0x0000000000000000000000000000000000000000000000000000000000000000" ]]; then
      local number
      number="$(rpc chain_getHeader "[\"$head\"]" | python3 -c 'import json,sys; h=json.load(sys.stdin).get("result") or {}; print(int((h.get("number") or "0x0"), 16))' 2>/dev/null || echo 0)"
      if [[ "$number" =~ ^[0-9]+$ ]] && (( number > 0 )); then
        echo "$number"
        return 0
      fi
    fi

    if (( SECONDS >= deadline )); then
      log "node log follows:"
      tail -n 120 "$WORKDIR/node.log" || true
      echo "timed out waiting for finalized block > 0" >&2
      exit 1
    fi
    sleep 1
  done
}

write_key() {
  local path="$1"
  local byte="$2"
  python3 - "$path" "$byte" <<'PY'
from pathlib import Path
import sys
Path(sys.argv[1]).write_bytes(bytes([int(sys.argv[2])]) * 32)
PY
}

log "building node with gpu-validator feature (SKIP_WASM_BUILD=${SKIP_WASM_BUILD:-unset})"
cargo build -p x3-chain-node --features gpu-validator --bin x3-chain-node "${PROFILE_ARGS[@]}"

log "building validator binary"
cargo build -p x3-gpu-validator-swarm --bin x3-validator "${PROFILE_ARGS[@]}"

log "starting node on $RPC_URL"
"$NODE_BIN" \
  --dev \
  --tmp \
  --rpc-port "$RPC_PORT" \
  --port "$P2P_PORT" \
  --enable-gpu-validator \
  >"$WORKDIR/node.log" 2>&1 &
NODE_PID=$!

wait_for_rpc
FINALIZED="$(wait_for_finalized_block)"
log "observed finalized block $FINALIZED"

KEY_A="$WORKDIR/validator-a.key"
KEY_B="$WORKDIR/validator-b.key"
PROBE_A="$WORKDIR/validator-a.proof.json"
PROBE_B="$WORKDIR/validator-b.proof.json"
AGGREGATE="$WORKDIR/aggregate.json"
TASK_DATA="x3 live gpu-validator finalized-head proof exchange"
TASK_ID="x3-live-gpu-validator-finalized-head-proof"

write_key "$KEY_A" 17
write_key "$KEY_B" 29

log "starting validator probe A"
"$VALIDATOR_BIN" live-probe \
  --rpc-url "$RPC_URL" \
  --validator-id live-validator-a \
  --key-path "$KEY_A" \
  --task-data "$TASK_DATA" \
  --task-id "$TASK_ID" \
  --timeout-secs "$TIMEOUT_SECS" \
  >"$PROBE_A" 2>"$WORKDIR/validator-a.err" &
VALIDATOR_A_PID=$!

log "starting validator probe B"
"$VALIDATOR_BIN" live-probe \
  --rpc-url "$RPC_URL" \
  --validator-id live-validator-b \
  --key-path "$KEY_B" \
  --task-data "$TASK_DATA" \
  --task-id "$TASK_ID" \
  --timeout-secs "$TIMEOUT_SECS" \
  >"$PROBE_B" 2>"$WORKDIR/validator-b.err" &
VALIDATOR_B_PID=$!

wait "$VALIDATOR_A_PID"
VALIDATOR_A_PID=""
wait "$VALIDATOR_B_PID"
VALIDATOR_B_PID=""

log "aggregating validator proofs"
"$VALIDATOR_BIN" aggregate-probes \
  --probe "$PROBE_A" \
  --probe "$PROBE_B" \
  >"$AGGREGATE"

python3 - "$AGGREGATE" <<'PY'
import json
import sys
data = json.load(open(sys.argv[1]))
assert data["ok"] is True
assert data["consensus_count"] >= 2
assert data["finalized_block"] > 0
print(json.dumps(data, indent=2))
PY

log "live node finalized-head two-validator aggregation passed"
