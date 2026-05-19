#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BASE_DIR="${BASE_DIR:-/tmp/x3-testnet-local}"
LOG_DIR="${LOG_DIR:-/tmp/x3-testnet-logs}"
# Default to all interfaces so dashboard is reachable from remote/local browsers.
CHAINBENCH_HOST="${CHAINBENCH_HOST:-0.0.0.0}"
CHAINBENCH_PORT="${CHAINBENCH_PORT:-7788}"
CHAINBENCH_HEALTH_HOST="${CHAINBENCH_HEALTH_HOST:-127.0.0.1}"
CHAINBENCH_TESTNET_RPC="${CHAINBENCH_TESTNET_RPC:-http://127.0.0.1:9944}"
CHAINBENCH_LIVE_RPC="${CHAINBENCH_LIVE_RPC:-https://rpc.x3star.net}"
CHAINBENCH_ADMIN_KEY="${CHAINBENCH_ADMIN_KEY:-x3-admin-local}"
CHAINBENCH_NODECORE_QUERY_URL="${CHAINBENCH_NODECORE_QUERY_URL:-http://127.0.0.1:9090/queries/ethereum}"
CHAINBENCH_DSHACKLE_PROXY_URL="${CHAINBENCH_DSHACKLE_PROXY_URL:-http://127.0.0.1:8545/eth}"
CHAINBENCH_CHAIN_DB_URL="${CHAINBENCH_CHAIN_DB_URL:-http://127.0.0.1:7070}"
CHAINBENCH_TPS_URL="${CHAINBENCH_TPS_URL:-http://127.0.0.1:3010}"
CHAINBENCH_DEFAULT_CHAIN_ID="${CHAINBENCH_DEFAULT_CHAIN_ID:-eth}"
CHAINBENCH_CHAIN_DB_ADMIN_KEY="${CHAINBENCH_CHAIN_DB_ADMIN_KEY:-${CHAIN_DB_ADMIN_KEY:-${CHAINBENCH_ADMIN_KEY}}}"
CHECK_TELEMETRY="${CHECK_TELEMETRY:-0}"
PROM_HOST="${PROM_HOST:-127.0.0.1}"
PROM_PORT="${PROM_PORT:-9090}"
GRAFANA_HOST="${GRAFANA_HOST:-127.0.0.1}"
GRAFANA_PORT="${GRAFANA_PORT:-3001}"
INFRA_DIR="${INFRA_DIR:-$ROOT_DIR/infra-structure}"
INFRA_START="${INFRA_START:-1}"
DRPC_START="${DRPC_START:-1}"
COUNT="${COUNT:-7}"
NODE_NICE="${NODE_NICE:-10}"
NODE_DB_CACHE_MIB="${NODE_DB_CACHE_MIB:-256}"

cd "$ROOT_DIR"

pkill -f "x3-chain-node" || true
pkill -f "scripts/testnet/chainbench_server.py" || true

if [ "$INFRA_START" != "0" ] && [ -x "$INFRA_DIR/start-all.sh" ]; then
  mkdir -p "$BASE_DIR/chainbench"
  CRED_FILE="$BASE_DIR/chainbench/chain_db_cred.key"
  if [ -z "${CHAIN_DB_CRED_KEY:-}" ]; then
    if [ -f "$CRED_FILE" ]; then
      export CHAIN_DB_CRED_KEY
      CHAIN_DB_CRED_KEY="$(cat "$CRED_FILE")"
    else
      export CHAIN_DB_CRED_KEY
      CHAIN_DB_CRED_KEY="$(python3 - <<'PY'
import os, base64
print(base64.b64encode(os.urandom(32)).decode())
PY
)"
      printf '%s' "$CHAIN_DB_CRED_KEY" > "$CRED_FILE"
    fi
  fi
  if [ -n "$CHAINBENCH_CHAIN_DB_ADMIN_KEY" ]; then
    export CHAIN_DB_ADMIN_KEY="$CHAINBENCH_CHAIN_DB_ADMIN_KEY"
  fi
  bash "$INFRA_DIR/start-all.sh" --no-ui || true
fi

if [ "$DRPC_START" != "0" ] && [ -x "$ROOT_DIR/scripts/drpc/setup-drpc-stack.sh" ]; then
  bash "$ROOT_DIR/scripts/drpc/setup-drpc-stack.sh" || true
fi

COUNT="$COUNT" NODE_NICE="$NODE_NICE" NODE_DB_CACHE_MIB="$NODE_DB_CACHE_MIB" \
  bash scripts/testnet/run-7-validators-local.sh --wipe --base-dir "$BASE_DIR" --log-dir "$LOG_DIR"

mkdir -p "$BASE_DIR"
python3 scripts/testnet/chainbench_server.py \
  --host "$CHAINBENCH_HOST" \
  --port "$CHAINBENCH_PORT" \
  --state-dir "$BASE_DIR/chainbench" \
  --testnet-rpc "$CHAINBENCH_TESTNET_RPC" \
  --live-rpc "$CHAINBENCH_LIVE_RPC" \
  --admin-key "$CHAINBENCH_ADMIN_KEY" \
  --nodecore-query-url "$CHAINBENCH_NODECORE_QUERY_URL" \
  --dshackle-proxy-url "$CHAINBENCH_DSHACKLE_PROXY_URL" \
  --chain-db-url "$CHAINBENCH_CHAIN_DB_URL" \
  --tps-url "$CHAINBENCH_TPS_URL" \
  --default-chain-id "$CHAINBENCH_DEFAULT_CHAIN_ID" \
  --chain-db-admin-key "$CHAINBENCH_CHAIN_DB_ADMIN_KEY" \
  --count "$COUNT" \
  --base-port 9944 \
  > "$LOG_DIR/chainbench-server.log" 2>&1 &

CHAINBENCH_PID=$!
echo "$CHAINBENCH_PID" > "$BASE_DIR/chainbench-server.pid"
trap 'kill $CHAINBENCH_PID 2>/dev/null' EXIT

for _ in $(seq 1 20); do
  if curl -s "http://${CHAINBENCH_HEALTH_HOST}:${CHAINBENCH_PORT}/health" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -s "http://${CHAINBENCH_HEALTH_HOST}:${CHAINBENCH_PORT}/health" >/dev/null 2>&1; then
  echo "Chainbench server failed to start. Log:"
  tail -n 120 "$LOG_DIR/chainbench-server.log" || true
  exit 1
fi

if [[ "$CHECK_TELEMETRY" == "1" ]]; then
  if ! curl -s "http://${PROM_HOST}:${PROM_PORT}/-/healthy" >/dev/null 2>&1; then
    echo "Prometheus health check failed at http://${PROM_HOST}:${PROM_PORT}/-/healthy"
    exit 1
  fi
  if ! curl -s "http://${GRAFANA_HOST}:${GRAFANA_PORT}/api/health" >/dev/null 2>&1; then
    echo "Grafana health check failed at http://${GRAFANA_HOST}:${GRAFANA_PORT}/api/health"
    exit 1
  fi
  echo "Telemetry stack OK (Prometheus + Grafana)"
fi

echo "Chainbench stack ready"
echo "Dashboard: http://${CHAINBENCH_HOST}:${CHAINBENCH_PORT}/"
echo "API health: http://${CHAINBENCH_HOST}:${CHAINBENCH_PORT}/health"
echo "dRPC status: http://${CHAINBENCH_HOST}:${CHAINBENCH_PORT}/api/drpc/status"
echo "Admin key env: CHAINBENCH_ADMIN_KEY=${CHAINBENCH_ADMIN_KEY}"

wait "$CHAINBENCH_PID"
