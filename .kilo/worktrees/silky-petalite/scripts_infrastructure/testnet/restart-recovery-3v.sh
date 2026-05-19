#!/usr/bin/env bash
set -euo pipefail

# Restart recovery check for the local 3-validator devnet.
# - start 3 validators (Alice/Bob/Charlie) on `--chain local3`
# - verify peers + finalized head advances
# - stop validator 2 (Bob), verify block production continues (finality is expected
#   to stall with 3 GRANDPA authorities because GRANDPA requires >2/3 weight)
# - restart validator 2, verify peers + finalized head advances again
#
# This is a script-level gate for Track 1 / Phase 4 before we invest in
# heavier integration-test harness work.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NODE_BIN="${NODE_BIN:-$ROOT_DIR/target/release/x3-chain-node}"
BASE_RPC_PORT="${BASE_RPC_PORT:-9944}"
BASE_P2P_PORT="${BASE_P2P_PORT:-30333}"
LISTEN_IP="${LISTEN_IP:-127.0.0.1}"

RUN_SCRIPT="${RUN_SCRIPT:-$ROOT_DIR/scripts/testnet/run-3-validators-dev.sh}"
STOP_SCRIPT="${STOP_SCRIPT:-$ROOT_DIR/scripts/testnet/stop-3-validators-dev.sh}"
STATUS_SCRIPT="${STATUS_SCRIPT:-$ROOT_DIR/scripts/testnet/status-7-validators.sh}"

FINALITY_WINDOW_SEC="${FINALITY_WINDOW_SEC:-10}"
FINALITY_TIMEOUT_SEC="${FINALITY_TIMEOUT_SEC:-120}"

jsonrpc() {
  local port="$1"
  local method="$2"
  local params="${3:-[]}"
  curl -s --max-time 2 -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"${method}\",\"params\":${params}}" \
    "http://127.0.0.1:${port}" || true
}

finalized_number() {
  local port="$1"
  local finalized_hash
  finalized_hash="$(jsonrpc "$port" "chain_getFinalizedHead" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("result",""))' 2>/dev/null || true)"
  if [[ -z "$finalized_hash" ]]; then
    echo "-1"
    return 0
  fi
  jsonrpc "$port" "chain_getHeader" "[\"${finalized_hash}\"]" | python3 -c 'import json,sys; o=json.load(sys.stdin).get("result"); print(int(o["number"],16) if o and o.get("number") else -1)' 2>/dev/null || echo "-1"
}

best_number() {
  local port="$1"
  jsonrpc "$port" "chain_getHeader" | python3 -c 'import json,sys; o=json.load(sys.stdin).get("result"); print(int(o["number"],16) if o and o.get("number") else -1)' 2>/dev/null || echo "-1"
}

require_finality_progress() {
  local port="$1"
  local start
  local end
  local start_ts
  start_ts="$(date +%s)"
  start="$(finalized_number "$port")"
  while true; do
    sleep "$FINALITY_WINDOW_SEC"
    end="$(finalized_number "$port")"
    if [[ "$start" =~ ^-?[0-9]+$ ]] && [[ "$end" =~ ^-?[0-9]+$ ]] && (( end > start )); then
      return 0
    fi
    local now
    now="$(date +%s)"
    if (( now - start_ts >= FINALITY_TIMEOUT_SEC )); then
      echo "Finality did not advance on port ${port} within ${FINALITY_TIMEOUT_SEC}s (start=${start}, end=${end})" >&2
      return 1
    fi
  done
}

require_best_progress() {
  local port="$1"
  local start_ts
  start_ts="$(date +%s)"
  local start
  start="$(best_number "$port")"
  while true; do
    sleep 2
    local end
    end="$(best_number "$port")"
    if [[ "$start" =~ ^-?[0-9]+$ ]] && [[ "$end" =~ ^-?[0-9]+$ ]] && (( end > start )); then
      return 0
    fi
    local now
    now="$(date +%s)"
    if (( now - start_ts >= FINALITY_TIMEOUT_SEC )); then
      echo "Best block did not advance on port ${port} within ${FINALITY_TIMEOUT_SEC}s (start=${start}, end=${end})" >&2
      return 1
    fi
  done
}

require_nodes_not_syncing() {
  local start_ts
  start_ts="$(date +%s)"
  while true; do
    local ok=1
    for port in "$BASE_RPC_PORT" "$((BASE_RPC_PORT + 1))" "$((BASE_RPC_PORT + 2))"; do
      if ! jsonrpc "$port" "system_health" | python3 -c 'import json,sys; o=json.load(sys.stdin).get("result",{}); sys.exit(0 if o.get("isSyncing") is False else 2)' 2>/dev/null; then
        ok=0
      fi
    done
    if [[ "$ok" -eq 1 ]]; then
      return 0
    fi
    local now
    now="$(date +%s)"
    if (( now - start_ts >= FINALITY_TIMEOUT_SEC )); then
      echo "Nodes did not reach isSyncing=false within ${FINALITY_TIMEOUT_SEC}s" >&2
      return 1
    fi
    sleep 2
  done
}

cleanup() {
  bash "$STOP_SCRIPT" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "== start devnet =="
bash "$RUN_SCRIPT" --wipe >/dev/null

echo "== peers + finality =="
COUNT=3 MIN_PEERS=2 bash "$STATUS_SCRIPT" --check-peers --min-peers 2 >/dev/null
COUNT=3 bash "$STATUS_SCRIPT" --check-finality --finality-window-sec "$FINALITY_WINDOW_SEC" --finality-timeout-sec "$FINALITY_TIMEOUT_SEC" >/dev/null
require_nodes_not_syncing

echo "== stop node 2 (Bob) =="
pid_file="/tmp/x3-devnet-3v/pids/node-2.pid"
if [[ ! -f "$pid_file" ]]; then
  echo "Missing pid file: $pid_file" >&2
  exit 1
fi
pid="$(cat "$pid_file" 2>/dev/null || true)"
if [[ -z "$pid" ]]; then
  echo "Empty pid in: $pid_file" >&2
  exit 1
fi
kill "$pid" 2>/dev/null || true
sleep 2

echo "== block production continues (node 1) =="
require_best_progress "$BASE_RPC_PORT"

echo "== restart node 2 (Bob) =="
peer_id="$(jsonrpc "$BASE_RPC_PORT" "system_localPeerId" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("result",""))' 2>/dev/null || true)"
if [[ -z "$peer_id" ]]; then
  echo "Failed to fetch node-1 peer id" >&2
  exit 1
fi
bootnode="/ip4/${LISTEN_IP}/tcp/${BASE_P2P_PORT}/p2p/${peer_id}"

# Re-launch node 2 specifically (without wiping state) using the same args/ports.
X3_DEV_SEED="//Bob" nohup "$NODE_BIN" \
  --chain local3 \
  --base-path "/tmp/x3-devnet-3v/node-2" \
  --name "x3-devnet-node-02" \
  --rpc-port "$((BASE_RPC_PORT + 1))" \
  --rpc-methods=Unsafe \
  --rpc-cors=all \
  --listen-addr "/ip4/${LISTEN_IP}/tcp/$((BASE_P2P_PORT + 1))" \
  --no-prometheus \
  --no-mdns \
  --no-telemetry \
  --validator \
  --allow-private-ip \
  --bob \
  --bootnodes "$bootnode" \
  >"$ROOT_DIR/logs/devnet-3v/node-2-restart.log" 2>&1 &

echo $! >"/tmp/x3-devnet-3v/pids/node-2.pid"

for _ in $(seq 1 60); do
  if jsonrpc "$((BASE_RPC_PORT + 1))" "system_health" | python3 -c 'import json,sys; sys.exit(0 if "result" in json.load(sys.stdin) else 2)' 2>/dev/null; then
    break
  fi
  sleep 1
done

echo "== peers + finality after restart =="
COUNT=3 MIN_PEERS=2 bash "$STATUS_SCRIPT" --check-peers --min-peers 2 >/dev/null
COUNT=3 bash "$STATUS_SCRIPT" --check-finality --finality-window-sec "$FINALITY_WINDOW_SEC" --finality-timeout-sec "$FINALITY_TIMEOUT_SEC" >/dev/null

echo "OK: restart recovery passed"
