#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_PATH="${BIN_PATH:-$ROOT_DIR/bin/x3-chain-node-fresh}"
RPC_PORT="${RPC_PORT:-9944}"
RPC_HOST="${RPC_HOST:-127.0.0.1}"
RPC_URL="${RPC_URL:-http://$RPC_HOST:$RPC_PORT}"
LOG_FILE="${LOG_FILE:-/tmp/x3-node-fresh.log}"
STARTUP_TIMEOUT_SECS="${STARTUP_TIMEOUT_SECS:-30}"

if [[ ! -x "$BIN_PATH" ]]; then
  echo "ERROR: fresh node binary not found or not executable: $BIN_PATH"
  echo "Build/copy it first, for example:"
  echo "  install -m 0755 artifacts/node/x3-chain-node bin/x3-chain-node-fresh"
  exit 2
fi

echo "Stopping any existing dev node on RPC port $RPC_PORT ..."
pkill -f "x3-chain-node-fresh --dev --tmp --rpc-port $RPC_PORT" || true
sleep 1

echo "Starting fresh node ..."
"$BIN_PATH" \
  --dev \
  --tmp \
  --rpc-port "$RPC_PORT" \
  --rpc-methods unsafe \
  --unsafe-rpc-external \
  --rpc-cors all > "$LOG_FILE" 2>&1 &

NODE_PID=$!
echo "Node PID: $NODE_PID"
echo "Log file: $LOG_FILE"

echo "Waiting for RPC health on $RPC_URL ..."
deadline=$((SECONDS + STARTUP_TIMEOUT_SECS))
while (( SECONDS < deadline )); do
  if curl -fsS -H "Content-Type: application/json" \
    -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' \
    "$RPC_URL" >/dev/null; then
    break
  fi
  sleep 1
done

if ! curl -fsS -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' \
  "$RPC_URL" >/dev/null; then
  echo "ERROR: RPC did not become ready within ${STARTUP_TIMEOUT_SECS}s at $RPC_URL"
  echo "Last log lines:"
  tail -n 40 "$LOG_FILE" || true
  exit 7
fi

echo "RPC is healthy. Verifying required X3 methods ..."
"$ROOT_DIR/scripts/verify-x3-rpc-methods.sh" "$RPC_URL"

echo "Node started and verified at $RPC_URL (pid=$NODE_PID)"