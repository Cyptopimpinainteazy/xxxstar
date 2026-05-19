#!/usr/bin/env bash
set -euo pipefail

# Local 3-validator launcher using Substrate dev keys (Alice/Bob/Charlie).
# This avoids the `subkey` dependency and is meant for fast MVP validation only.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NODE_BIN="${NODE_BIN:-$ROOT_DIR/target/release/x3-chain-node}"
BASE_DIR="${BASE_DIR:-/tmp/x3-devnet-3v}"
PID_DIR="${PID_DIR:-$BASE_DIR/pids}"
LOG_DIR="${LOG_DIR:-$ROOT_DIR/logs/devnet-3v}"

LISTEN_IP="${LISTEN_IP:-127.0.0.1}"
BASE_P2P_PORT="${BASE_P2P_PORT:-30333}"
BASE_RPC_PORT="${BASE_RPC_PORT:-9944}"
PROMETHEUS="${PROMETHEUS:-0}"
FORCE_AUTHORING="${FORCE_AUTHORING:-0}"
WAIT_FOR_RPC="${WAIT_FOR_RPC:-1}"

wipe=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --wipe)
      wipe=1
      shift
      ;;
    -h|--help)
      echo "Usage: $(basename "$0") [--wipe]"
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 2
      ;;
  esac
done

if [[ ! -x "$NODE_BIN" ]]; then
  echo "Node binary not found: $NODE_BIN" >&2
  exit 1
fi

mkdir -p "$BASE_DIR" "$PID_DIR" "$LOG_DIR"

stop_nodes() {
  shopt -s nullglob
  for pid_file in "$PID_DIR"/node-*.pid; do
    pid="$(cat "$pid_file" 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  shopt -u nullglob
  sleep 1
}

if [[ "$wipe" -eq 1 ]]; then
  stop_nodes
  rm -rf "$BASE_DIR"
  mkdir -p "$BASE_DIR" "$PID_DIR"
fi

wait_for_rpc() {
  local port="$1"
  for _ in $(seq 1 60); do
    if curl -fsS -X POST "http://127.0.0.1:${port}" \
      -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
      >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  return 1
}

start_node() {
  local idx="$1"
  local dev_flag="$2"
  local bootnode="${3:-}"
  local seed=""
  local p2p_port=$((BASE_P2P_PORT + idx - 1))
  local rpc_port=$((BASE_RPC_PORT + idx - 1))
  local prom_port=$((9615 + idx - 1))
  local base_path="$BASE_DIR/node-${idx}"
  local name="x3-devnet-node-$(printf '%02d' "$idx")"
  local log_file="$LOG_DIR/node-${idx}.log"

  mkdir -p "$base_path"

  case "$dev_flag" in
    --alice) seed="//Alice" ;;
    --bob) seed="//Bob" ;;
    --charlie) seed="//Charlie" ;;
    *)
      echo "Unknown dev flag (expected --alice/--bob/--charlie): $dev_flag" >&2
      exit 2
      ;;
  esac

  local boot_args=()
  if [[ -n "$bootnode" ]]; then
    boot_args=(--bootnodes "$bootnode")
  fi

  local prom_args=()
  if [[ "$PROMETHEUS" == "1" ]]; then
    prom_args=(--prometheus-port "$prom_port")
  else
    prom_args=(--no-prometheus)
  fi

  local authoring_args=()
  if [[ "$FORCE_AUTHORING" == "1" ]]; then
    authoring_args=(--force-authoring)
  fi

  # Ensure both Aura and GRANDPA keys exist for each validator on non-Development chains.
  # This is required for GRANDPA finality to progress on local/testnet specs.
  X3_DEV_SEED="$seed" nohup "$NODE_BIN" \
    --chain local3 \
    --base-path "$base_path" \
    --name "$name" \
    --rpc-port "$rpc_port" \
    --rpc-methods=Unsafe \
    --rpc-cors=all \
    --listen-addr "/ip4/${LISTEN_IP}/tcp/${p2p_port}" \
    "${prom_args[@]}" \
    --no-mdns \
    --no-telemetry \
    --validator \
    "${authoring_args[@]}" \
    --allow-private-ip \
    "$dev_flag" \
    "${boot_args[@]}" \
    >"$log_file" 2>&1 &

  echo $! >"$PID_DIR/node-${idx}.pid"
  echo "Started ${name} (p2p=${p2p_port}, rpc=${rpc_port})"

  if [[ "$WAIT_FOR_RPC" == "1" ]]; then
    wait_for_rpc "$rpc_port"
    echo "Node ${name} ready"
  fi
}

echo "Starting node 1 (bootnode, Alice)..."
start_node 1 --alice

peer_id=""
for _ in $(seq 1 60); do
  peer_id="$(curl -s -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"system_localPeerId","params":[]}' \
    "http://127.0.0.1:${BASE_RPC_PORT}" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("result",""))' 2>/dev/null || true)"
  if [[ -n "$peer_id" ]]; then
    break
  fi
  sleep 1
done

if [[ -z "$peer_id" ]]; then
  echo "Failed to detect node-1 peer ID via RPC (system_localPeerId)" >&2
  exit 1
fi

BOOTNODE="/ip4/${LISTEN_IP}/tcp/${BASE_P2P_PORT}/p2p/${peer_id}"
echo "Bootnode: ${BOOTNODE}"

echo "Starting node 2 (Bob)..."
WAIT_FOR_RPC=0 start_node 2 --bob "$BOOTNODE"

echo "Starting node 3 (Charlie)..."
WAIT_FOR_RPC=0 start_node 3 --charlie "$BOOTNODE"

wait_for_rpc "$((BASE_RPC_PORT + 1))"
echo "Node x3-devnet-node-02 ready"
wait_for_rpc "$((BASE_RPC_PORT + 2))"
echo "Node x3-devnet-node-03 ready"

echo "All 3 validators started."
echo "Logs: ${LOG_DIR}/node-*.log"
echo "PIDs: ${PID_DIR}/node-*.pid"
